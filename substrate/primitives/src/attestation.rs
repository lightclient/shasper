use crypto::bls;
use codec::{Encode, Decode};
#[cfg(feature = "std")]
use serde_derive::{Serialize, Deserialize};
use casper::context::{Attestation, SlotAttestation, BalanceContext, SlotContext};
use crate::{Slot, Hash, Epoch, ValidatorId, Signature, Balance};

#[derive(Eq, PartialEq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct UnsignedAttestation {
	pub slot: Slot,
	pub slot_block_hash: Hash,
	pub source_epoch: Epoch,
	pub source_epoch_block_hash: Hash,
	pub target_epoch: Epoch,
	pub target_epoch_block_hash: Hash,
	pub validator_indexes: Vec<u32>,
}

impl UnsignedAttestation {
	pub fn sign_with(self, secrets: &[&bls::Secret]) -> UncheckedAttestation {
		assert!(secrets.len() == self.validator_indexes.len());

		let to_sign = self.encode();
		let mut signature = bls::AggregateSignature::new();
		for secret in secrets {
			signature.add(&bls::Signature::new(&to_sign[..], 0, &secret));
		}

		UncheckedAttestation {
			signature: crate::from_aggregate_signature(signature),
			data: self,
		}
	}
}

#[derive(Eq, PartialEq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct UncheckedAttestation {
	pub data: UnsignedAttestation,
	pub signature: Signature,
}

impl UncheckedAttestation {
	pub fn aggregate(&mut self, validator_index: u32, secret: &bls::Secret) -> bool {
		if self.data.validator_indexes.contains(&validator_index) {
			return false;
		}

		let mut signature = bls::AggregateSignature::new();
		signature.add(&match crate::into_signature(&self.signature) {
			Some(signature) => signature,
			None => return false,
		});
		let to_sign = self.data.encode();
		signature.add(&bls::Signature::new(&to_sign[..], 0, &secret));

		self.signature = crate::from_aggregate_signature(signature);

		true
	}
}

#[derive(Eq, PartialEq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug, Serialize, Deserialize))]
pub struct CheckedAttestation {
	pub data: UnsignedAttestation,
	pub is_slot_canon: bool,
	pub is_source_canon: bool,
	pub is_target_canon: bool,
	pub validator_ids: Vec<ValidatorId>,
	pub inclusion_distance: Slot,
}

impl Attestation for CheckedAttestation {
	type ValidatorId = ValidatorId;
	type Epoch = Epoch;

	fn validator_ids(&self) -> Vec<ValidatorId> {
		self.validator_ids.clone()
	}

	fn is_source_canon(&self) -> bool {
		self.is_source_canon
	}

	fn is_target_canon(&self) -> bool {
		self.is_target_canon
	}

	fn source_epoch(&self) -> Epoch {
		self.data.source_epoch
	}

	fn target_epoch(&self) -> Epoch {
		self.data.target_epoch
	}
}

impl SlotAttestation for CheckedAttestation {
	type Slot = Slot;

	fn slot(&self) -> Slot {
		self.data.slot
	}

	fn is_slot_canon(&self) -> bool {
		self.is_slot_canon
	}

	fn inclusion_distance(&self) -> Slot {
		self.inclusion_distance
	}
}

#[derive(Eq, PartialEq, Clone, Encode, Decode)]
pub struct AttestationContext;

impl BalanceContext for AttestationContext {
	type Attestation = CheckedAttestation;
	type Balance = Balance;
}

impl SlotContext for AttestationContext { }
