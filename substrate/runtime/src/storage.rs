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

use primitives::{
	BlockNumber, Hash, Balance, ValidatorId, CheckedAttestation, AttestationContext,
	KeccakHasher,
};
use runtime_support::storage_items;
use runtime_support::storage::{StorageValue, StorageMap};
use runtime_support::storage::unhashed::{self, StorageVec};
use casper::{randao, committee};
use crate::state::ValidatorRecord;
use crate::{UncheckedExtrinsic, utils};

storage_items! {
	pub Number: b"sys:num" => default BlockNumber;
	pub Slot: b"sys:slot" => default primitives::Slot;
	pub LastSlot: b"sys:lastslot" => default primitives::Slot;
	pub ParentHash: b"sys:parenthash" => default Hash;
	pub Digest: b"sys:digest" => default super::Digest;
	pub CasperContext: b"sys:caspercontext" => default casper::CasperProcess<AttestationContext>;
	pub GenesisSlot: b"sys:genesisslot" => default primitives::Slot;
	pub LatestBlockHashes: b"sys:latestblockhashes" => map [primitives::Slot => Hash];
	pub Randao: b"sys:randao" => default randao::RandaoProducer<KeccakHasher>;
	pub Committee: b"sys:committee" => default committee::CommitteeProcess<KeccakHasher>;
}

pub struct UncheckedExtrinsics;
impl unhashed::StorageVec for UncheckedExtrinsics {
	type Item = Option<UncheckedExtrinsic>;
	const PREFIX: &'static [u8] = b"sys:extrinsics";
}

pub struct PendingAttestations;
impl unhashed::StorageVec for PendingAttestations {
	type Item = Option<CheckedAttestation>;
	const PREFIX: &'static [u8] = b"sys:pendingattestations";
}

pub fn note_parent_hash() {
	let slot = Slot::get();
	let last_slot = LastSlot::get();

	let hash = ParentHash::get();
	for s in last_slot..slot {
		LatestBlockHashes::insert(s, hash);
	}
}

pub const VALIDATORS_PREFIX: &[u8] = b"sys:validators";

pub struct Validators;
impl unhashed::StorageVec for Validators {
	type Item = Option<ValidatorRecord>;
	const PREFIX: &'static [u8] = VALIDATORS_PREFIX;
}

pub fn add_balance(validator_id: &ValidatorId, balance: Balance) {
	if let Some((index, Some(mut record))) = Validators::items().into_iter()
		.enumerate()
		.find(|(_, record)| record.as_ref().map(|r| &r.validator_id == validator_id).unwrap_or(false))
	{
		record.balance += balance;
		Validators::set_item(index as u32, &Some(record));
	}
}

pub fn sub_balance(validator_id: &ValidatorId, balance: Balance) {
	if let Some((index, Some(mut record))) = Validators::items().into_iter()
		.enumerate()
		.find(|(_, record)| record.as_ref().map(|r| &r.validator_id == validator_id).unwrap_or(false))
	{
		record.balance -= balance;
		Validators::set_item(index as u32, &Some(record));
	}
}

pub fn penalize_validator(validator_id: &ValidatorId, balance: Balance) {
	if let Some((index, Some(mut record))) = Validators::items().into_iter()
		.enumerate()
		.find(|(_, record)| record.as_ref().map(|r| &r.validator_id == validator_id).unwrap_or(false))
	{
		record.balance -= balance;
		record.valid_to = utils::slot_to_epoch(Number::get());
		Validators::set_item(index as u32, &Some(record));
	}
}
