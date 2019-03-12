// Copyright 2018 Parity Technologies (UK) Ltd.
// This file is part of Substrate Shasper.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

use primitives::{H256, ValidatorId, BitField};
use ssz::Hashable;
use ssz_derive::Ssz;
use crate::{Gwei, Slot, Epoch, Timestamp, ValidatorIndex, Shard};
use crate::eth1::{Eth1Data, Eth1DataVote, Deposit};
use crate::slashing::SlashableAttestation;
use crate::attestation::{
	PendingAttestation, Crosslink, AttestationDataAndCustodyBit,
	AttestationData,
};
use crate::validator::Validator;
use crate::block::{BeaconBlock, BeaconBlockHeader};
use crate::consts::{
	SLOTS_PER_HISTORICAL_ROOT, LATEST_SLASHED_EXIT_LENGTH,
	LATEST_ACTIVE_INDEX_ROOTS_LENGTH, SHARD_COUNT,
	LATEST_RANDAO_MIXES_LENGTH, DOMAIN_DEPOSIT,
	ACTIVATION_EXIT_DELAY, MIN_SEED_LOOKAHEAD,
	GENESIS_EPOCH, GENESIS_START_SHARD, GENESIS_SLOT,
	GENESIS_FORK_VERSION, MIN_DEPOSIT_AMOUNT,
	MAX_DEPOSIT_AMOUNT, DOMAIN_ATTESTATION, MAX_INDICES_PER_SLASHABLE_VOTE,
	SLOTS_PER_EPOCH, MIN_VALIDATOR_WITHDRAWABILITY_DELAY, WHISTLEBLOWER_REWARD_QUOTIENT,
};
use crate::error::Error;
use crate::util::{
	Hasher, bls_domain, slot_to_epoch, hash3, to_bytes, bls_aggregate_pubkeys,
	bls_verify_multiple, shuffling, is_power_of_two, epoch_committee_count,
	epoch_start_slot,
};

#[derive(Ssz)]
pub struct BeaconState {
	// Misc
	pub slot: Slot,
	pub genesis_time: Timestamp,
	pub fork: Fork, // For versioning hard forks

	// Validator registry
	pub validator_registry: Vec<Validator>,
	pub validator_balances: Vec<u64>,
	pub validator_registry_update_epoch: Epoch,

	// Randomness and committees
	pub latest_randao_mixes: [H256; LATEST_RANDAO_MIXES_LENGTH],
	pub previous_shuffling_start_shard: Shard,
	pub current_shuffling_start_shard: Shard,
	pub previous_shuffling_epoch: Epoch,
	pub current_shuffling_epoch: Epoch,
	pub previous_shuffling_seed: H256,
	pub current_shuffling_seed: H256,

	// Finality
	pub previous_epoch_attestations: Vec<PendingAttestation>,
	pub current_epoch_attestations: Vec<PendingAttestation>,
	pub previous_justified_epoch: Epoch,
	pub justified_epoch: Epoch,
	pub justification_bitfield: u64,
	pub finalized_epoch: Epoch,

	// Recent state
	pub latest_crosslinks: [Crosslink; SHARD_COUNT],
	pub latest_block_roots: [H256; SLOTS_PER_HISTORICAL_ROOT],
	pub latest_state_roots: [H256; SLOTS_PER_HISTORICAL_ROOT],
	pub latest_active_index_roots: [H256; LATEST_ACTIVE_INDEX_ROOTS_LENGTH],
	pub latest_slashed_balances: [u64; LATEST_SLASHED_EXIT_LENGTH], // Balances slashed at every withdrawal period
	pub latest_block_header: BeaconBlockHeader,
	pub historical_roots: Vec<H256>,

	// Ethereum 1.0 chain data
	pub latest_eth1_data: Eth1Data,
	pub eth1_data_votes: Vec<Eth1DataVote>,
	pub deposit_index: u64,
}

pub struct HistoricalBatch {
	/// Block roots
	pub block_roots: [H256; SLOTS_PER_HISTORICAL_ROOT],
	/// State roots
	pub state_roots: [H256; SLOTS_PER_HISTORICAL_ROOT],
}

#[derive(Ssz)]
pub struct Fork {
	/// Previous fork version
	pub previous_version: u64,
	/// Current fork version
	pub current_version: u64,
	/// Fork epoch number
	pub epoch: u64,
}

impl Default for Fork {
	fn default() -> Self {
		Self {
			previous_version: GENESIS_FORK_VERSION,
			current_version: GENESIS_FORK_VERSION,
			epoch: GENESIS_EPOCH,
		}
	}
}

impl BeaconState {
	pub fn current_epoch(&self) -> Epoch {
		slot_to_epoch(self.slot)
	}

	pub fn previous_epoch(&self) -> Epoch {
		self.current_epoch().saturating_sub(1)
	}

	pub fn delayed_activation_exit_epoch(&self) -> u64 {
		self.current_epoch() + 1 + ACTIVATION_EXIT_DELAY
	}

	pub fn randao_mix(&self, epoch: Epoch) -> Result<H256, Error> {
		if self.current_epoch().saturating_sub(LATEST_RANDAO_MIXES_LENGTH as u64) >= epoch ||
			epoch > self.current_epoch()
		{
			return Err(Error::EpochOutOfRange)
		}

		Ok(self.latest_randao_mixes[(epoch % LATEST_RANDAO_MIXES_LENGTH as u64) as usize])
	}

	pub fn block_root(&self, slot: Slot) -> Result<H256, Error> {
		if slot >= self.slot || self.slot > slot + SLOTS_PER_HISTORICAL_ROOT as u64 {
			return Err(Error::SlotOutOfRange)
		}
		Ok(self.latest_block_roots[(slot % SLOTS_PER_HISTORICAL_ROOT as u64) as usize])
	}

	pub fn state_root(&self, slot: Slot) -> Result<H256, Error> {
		if slot >= self.slot || self.slot > slot + SLOTS_PER_HISTORICAL_ROOT as u64 {
			return Err(Error::SlotOutOfRange)
		}
		Ok(self.latest_state_roots[(slot % SLOTS_PER_HISTORICAL_ROOT as u64) as usize])
	}

	pub fn active_index_root(&self, epoch: Epoch) -> Result<H256, Error> {
		if self.current_epoch().saturating_sub(
			LATEST_ACTIVE_INDEX_ROOTS_LENGTH as u64 - ACTIVATION_EXIT_DELAY
		) >= epoch || epoch > self.current_epoch() + ACTIVATION_EXIT_DELAY {
			return Err(Error::EpochOutOfRange)
		}

		Ok(self.latest_active_index_roots[(epoch % LATEST_ACTIVE_INDEX_ROOTS_LENGTH as u64) as usize])
	}

	pub fn seed(&self, epoch: Epoch) -> Result<H256, Error> {
		Ok(hash3(
			self.randao_mix(epoch.saturating_sub(MIN_SEED_LOOKAHEAD))?.as_ref(),
			self.active_index_root(epoch)?.as_ref(),
			to_bytes(epoch).as_ref()
		))
	}

	pub fn beacon_proposer_index(&self, slot: Slot, registry_change: bool) -> Result<ValidatorIndex, Error> {
		let epoch = slot_to_epoch(slot);
		let current_epoch = self.current_epoch();
		let previous_epoch = self.previous_epoch();
		let next_epoch = current_epoch + 1;

		if previous_epoch > epoch || epoch > next_epoch {
			return Err(Error::EpochOutOfRange)
		}

		let (first_committee, _) = self.crosslink_committees_at_slot(slot, registry_change)?[0].clone();
		Ok(first_committee[(slot % first_committee.len() as u64) as usize])
	}

	pub fn validator_by_id(&self, validator_id: &ValidatorId) -> Option<&Validator> {
		for validator in &self.validator_registry {
			if &validator.pubkey == validator_id {
				return Some(validator)
			}
		}

		None
	}

	fn effective_balance(&self, index: ValidatorIndex) -> Gwei {
		core::cmp::min(self.validator_balances[index as usize], MIN_DEPOSIT_AMOUNT)
	}

	fn activate_validator(&mut self, index: ValidatorIndex, is_genesis: bool) {
		let delayed_activation_exit_epoch = self.delayed_activation_exit_epoch();
		self.validator_registry[index as usize].activate(delayed_activation_exit_epoch, is_genesis);
	}

	pub fn initiate_validator_exit(&mut self, index: ValidatorIndex) {
		self.validator_registry[index as usize].initiate_exit();
	}

	pub fn slash_validator(&mut self, index: ValidatorIndex) -> Result<(), Error> {
		if self.slot >= epoch_start_slot(self.validator_registry[index as usize].withdrawable_epoch) {
			return Err(Error::ValidatorNotWithdrawable);
		}
		self.exit_validator(index);

		self.latest_slashed_balances[(self.current_epoch() % LATEST_SLASHED_EXIT_LENGTH as u64) as usize] += self.effective_balance(index);

		let whistleblower_index = self.beacon_proposer_index(self.slot, false)?;
		let whistleblower_reward = self.effective_balance(index) / WHISTLEBLOWER_REWARD_QUOTIENT;
		self.validator_balances[whistleblower_index as usize] += whistleblower_reward;
		self.validator_balances[index as usize] -= whistleblower_reward;
		self.validator_registry[index as usize].slashed = true;
		self.validator_registry[index as usize].withdrawable_epoch = self.current_epoch() + LATEST_SLASHED_EXIT_LENGTH as u64;

		Ok(())
	}

	pub fn prepare_validator_for_withdrawal(&mut self, index: ValidatorIndex) {
		self.validator_registry[index as usize].withdrawable_epoch = self.current_epoch() + MIN_VALIDATOR_WITHDRAWABILITY_DELAY;
	}

	pub fn exit_validator(&mut self, index: ValidatorIndex) {
		let delayed_activation_exit_epoch = self.delayed_activation_exit_epoch();
		self.validator_registry[index as usize].exit(delayed_activation_exit_epoch);
	}

	pub fn push_deposit(&mut self, deposit: Deposit) -> Result<(), Error> {
		if deposit.index != self.deposit_index {
			return Err(Error::DepositIndexMismatch)
		}

		if !deposit.is_merkle_valid(&self.latest_eth1_data.deposit_root) {
			return Err(Error::DepositMerkleInvalid)
		}

		self.deposit_index += 1;

		if !deposit.is_proof_valid(
			bls_domain(&self.fork, self.current_epoch(), DOMAIN_DEPOSIT)
		) {
			return Err(Error::DepositProofInvalid)
		}

		match self.validator_by_id(&deposit.deposit_data.deposit_input.pubkey) {
			Some(validator) => {
				if validator.withdrawal_credentials != deposit.deposit_data.deposit_input.withdrawal_credentials {
					return Err(Error::DepositWithdrawalCredentialsMismatch)
				}
			},
			None => {

			},
		}

		Ok(())
	}

	pub fn active_validator_indices(&self, epoch: Epoch) -> Vec<ValidatorIndex> {
		self.validator_registry.iter()
			.enumerate()
			.filter(|(_, v)| v.is_active(epoch))
			.map(|(i, _)| i as u64)
			.collect::<Vec<_>>()
	}

	pub fn genesis(deposits: Vec<Deposit>, genesis_time: Timestamp, latest_eth1_data: Eth1Data) -> Result<Self, Error> {
		let mut state = Self {
			slot: GENESIS_SLOT,
			genesis_time,
			fork: Fork::default(),

			validator_registry: Vec::new(),
			validator_balances: Vec::new(),
			validator_registry_update_epoch: GENESIS_EPOCH,

			latest_randao_mixes: [H256::default(); LATEST_RANDAO_MIXES_LENGTH],
			previous_shuffling_start_shard: GENESIS_START_SHARD,
			current_shuffling_start_shard: GENESIS_START_SHARD,
			previous_shuffling_epoch: GENESIS_EPOCH,
			current_shuffling_epoch: GENESIS_EPOCH,
			previous_shuffling_seed: H256::default(),
			current_shuffling_seed: H256::default(),

			previous_epoch_attestations: Vec::new(),
			current_epoch_attestations: Vec::new(),
			previous_justified_epoch: GENESIS_EPOCH,
			justified_epoch: GENESIS_EPOCH,
			justification_bitfield: 0,
			finalized_epoch: GENESIS_EPOCH,

			latest_crosslinks: unsafe {
				let mut ret: [Crosslink; SHARD_COUNT] = core::mem::uninitialized();
				for item in &mut ret[..] {
					core::ptr::write(item, Crosslink::default());
				}
				ret
			},
			latest_block_roots: [H256::default(); SLOTS_PER_HISTORICAL_ROOT],
			latest_state_roots: [H256::default(); SLOTS_PER_HISTORICAL_ROOT],
			latest_active_index_roots: [H256::default(); LATEST_ACTIVE_INDEX_ROOTS_LENGTH],
			latest_slashed_balances: [0; LATEST_SLASHED_EXIT_LENGTH],
			latest_block_header: BeaconBlockHeader::with_state_root(&BeaconBlock::empty(), H256::default()),
			historical_roots: Vec::new(),

			latest_eth1_data,
			eth1_data_votes: Vec::new(),
			deposit_index: 0,
		};

		for deposit in deposits {
			state.push_deposit(deposit)?;
		}

		for validator_index in 0..(state.validator_registry.len() as u64) {
			if state.effective_balance(validator_index) >= MAX_DEPOSIT_AMOUNT {
				state.activate_validator(validator_index, true);
			}
		}

		let genesis_active_index_root = state.active_validator_indices(GENESIS_EPOCH).hash::<Hasher>();
		for index in 0..LATEST_ACTIVE_INDEX_ROOTS_LENGTH {
			state.latest_active_index_roots[index] = genesis_active_index_root;
		}
		state.current_shuffling_seed = state.seed(GENESIS_EPOCH)?;

		Ok(state)
	}

	pub fn update_cache(&mut self) {
		let previous_slot_state_root = self.hash::<Hasher>();

		self.latest_state_roots[(self.slot % SLOTS_PER_HISTORICAL_ROOT as u64) as usize] = previous_slot_state_root;

		if self.latest_block_header.state_root == H256::default() {
			self.latest_block_header.state_root = previous_slot_state_root;
		}

		self.latest_block_roots[(self.slot % SLOTS_PER_HISTORICAL_ROOT as u64) as usize] = self.latest_block_header.hash::<Hasher>();
	}

	pub fn previous_epoch_committee_count(&self) -> usize {
		let previous_active_validators = self.active_validator_indices(self.previous_shuffling_epoch);
		epoch_committee_count(previous_active_validators.len())
	}

	pub fn current_epoch_committee_count(&self) -> usize {
		let current_active_validators = self.active_validator_indices(self.current_shuffling_epoch);
		epoch_committee_count(current_active_validators.len())
	}

	pub fn next_epoch_committee_count(&self) -> usize {
		let next_active_validators = self.active_validator_indices(self.current_epoch() + 1);
		epoch_committee_count(next_active_validators.len())
	}

	pub fn crosslink_committees_at_slot(&self, slot: Slot, registry_change: bool) -> Result<Vec<(Vec<ValidatorIndex>, Shard)>, Error> {
		let epoch = slot_to_epoch(slot);
		let current_epoch = self.current_epoch();
		let previous_epoch = self.previous_epoch();
		let next_epoch = current_epoch + 1;

		if previous_epoch > epoch || epoch > next_epoch {
			return Err(Error::EpochOutOfRange);
		}

		let (committees_per_epoch, seed, shuffling_epoch, shuffling_start_shard) = if epoch == current_epoch {
			let committees_per_epoch = self.current_epoch_committee_count();
			let seed = self.current_shuffling_seed;
			let shuffling_epoch = self.current_shuffling_epoch;
			let shuffling_start_shard = self.current_shuffling_start_shard;

			(committees_per_epoch, seed, shuffling_epoch, shuffling_start_shard)
		} else if epoch == previous_epoch {
			let committees_per_epoch = self.previous_epoch_committee_count();
			let seed = self.previous_shuffling_seed;
			let shuffling_epoch = self.previous_shuffling_epoch;
			let shuffling_start_shard = self.previous_shuffling_start_shard;

			(committees_per_epoch, seed, shuffling_epoch, shuffling_start_shard)
		} else {
			let epochs_since_last_registry_update = current_epoch - self.validator_registry_update_epoch;

			if registry_change {
				let committees_per_epoch = self.next_epoch_committee_count();
				let seed = self.seed(next_epoch)?;
				let shuffling_epoch = next_epoch;
				let current_committees_per_epoch = self.current_epoch_committee_count();
				let shuffling_start_shard = (self.current_shuffling_start_shard + current_committees_per_epoch as u64) % SHARD_COUNT as u64;

				(committees_per_epoch, seed, shuffling_epoch, shuffling_start_shard)
			} else if epochs_since_last_registry_update > 1 && is_power_of_two(epochs_since_last_registry_update) {
				let committees_per_epoch = self.next_epoch_committee_count();
				let seed = self.seed(next_epoch)?;
				let shuffling_epoch = next_epoch;
				let shuffling_start_shard = self.current_shuffling_start_shard;

				(committees_per_epoch, seed, shuffling_epoch, shuffling_start_shard)
			} else {
				let committees_per_epoch = self.current_epoch_committee_count();
				let seed = self.current_shuffling_seed;
				let shuffling_epoch = self.current_shuffling_epoch;
				let shuffling_start_shard = self.current_shuffling_start_shard;

				(committees_per_epoch, seed, shuffling_epoch, shuffling_start_shard)
			}
		};

		let active_validators = self.active_validator_indices(shuffling_epoch);
		let shuffling = shuffling(&seed, active_validators);
		let offset = slot % SLOTS_PER_EPOCH;
		let committees_per_slot = committees_per_epoch as u64 / SLOTS_PER_EPOCH;
		let slot_start_shard = (shuffling_start_shard + committees_per_slot * offset) % SHARD_COUNT as u64;

		let mut ret = Vec::new();
		for i in 0..committees_per_slot {
			ret.push((shuffling[(committees_per_slot * offset + i as u64) as usize].clone(),
					  (slot_start_shard + i as u64) % SHARD_COUNT as u64));
		}
		Ok(ret)
	}

	fn total_balance(&self, indices: &[ValidatorIndex]) -> Gwei {
		indices.iter().fold(0, |sum, index| {
			sum + self.effective_balance(*index)
		})
	}

	pub fn current_total_balance(&self) -> Gwei {
		self.total_balance(&self.active_validator_indices(self.current_epoch())[..])
	}

	pub fn previous_total_balance(&self) -> Gwei {
		self.total_balance(&self.active_validator_indices(self.previous_epoch())[..])
	}

	pub fn attestation_participants(&self, attestation: &AttestationData, bitfield: &BitField) -> Result<Vec<ValidatorIndex>, Error> {
		let crosslink_committees = self.crosslink_committees_at_slot(attestation.slot, false)?;

		let matched_committees = crosslink_committees.iter().filter(|(_, s)| s == &attestation.shard).collect::<Vec<_>>();
		if matched_committees.len() == 0 {
			return Err(Error::AttestationShardInvalid);
		}

		let crosslink_committee = matched_committees[0];
		if bitfield.count() != crosslink_committee.0.len() {
			return Err(Error::AttestationBitFieldInvalid);
		}

		let mut participants = Vec::new();
		for (i, validator_index) in crosslink_committee.0.iter().enumerate() {
			if bitfield.has_voted(i) {
				participants.push(*validator_index);
			}
		}
		Ok(participants)
	}

	pub fn verify_slashable_attestation(&self, slashable: &SlashableAttestation) -> bool {
		if slashable.custody_bitfield.count() != 0 {
			return false;
		}

		if slashable.validator_indices.len() == 0 {
			return false;
		}

		for i in 0..(slashable.validator_indices.len() - 1) {
			if slashable.validator_indices[i] > slashable.validator_indices[i + 1] {
				return false;
			}
		}

		if slashable.custody_bitfield.count() != slashable.validator_indices.len() {
			return false;
		}

		if slashable.validator_indices.len() > MAX_INDICES_PER_SLASHABLE_VOTE {
			return false;
		}

		let mut custody_bit_0_indices = Vec::new();
		let mut custody_bit_1_indices = Vec::new();
		for (i, validator_index) in slashable.validator_indices.iter().enumerate() {
			if !slashable.custody_bitfield.has_voted(i) {
				custody_bit_0_indices.push(validator_index);
			} else {
				custody_bit_1_indices.push(validator_index);
			}
		}

		bls_verify_multiple(
			&[
				bls_aggregate_pubkeys(&custody_bit_0_indices.iter().map(|i| self.validator_registry[**i as usize].pubkey).collect::<Vec<_>>()[..]),
				bls_aggregate_pubkeys(&custody_bit_1_indices.iter().map(|i| self.validator_registry[**i as usize].pubkey).collect::<Vec<_>>()[..]),
			],
			&[
				AttestationDataAndCustodyBit {
					data: slashable.data.clone(),
					custody_bit: false,
				}.hash::<Hasher>(),
				AttestationDataAndCustodyBit {
					data: slashable.data.clone(),
					custody_bit: true,
				}.hash::<Hasher>(),
			],
			&slashable.aggregate_signature,
			bls_domain(&self.fork, slot_to_epoch(slashable.data.slot), DOMAIN_ATTESTATION)
		)
	}
}
