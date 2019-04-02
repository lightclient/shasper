#[cfg_attr(feature = "std", derive(Debug))]
pub enum Error {
	DepositIndexMismatch,
	DepositMerkleInvalid,
	DepositProofInvalid,
	DepositWithdrawalCredentialsMismatch,
	EpochOutOfRange,
	SlotOutOfRange,
	AttestationShardInvalid,
	AttestationBitFieldInvalid,
	ValidatorNotWithdrawable,
	ValidatorAttestationNotFound,
	BlockSlotInvalid,
	BlockPreviousRootInvalid,
	BlockSignatureInvalid,
	RandaoSignatureInvalid,
	ProposerSlashingInvalidSlot,
	ProposerSlashingSameHeader,
	ProposerSlashingAlreadySlashed,
	ProposerSlashingInvalidSignature,
	AttesterSlashingSameAttestation,
	AttesterSlashingNotSlashable,
	AttesterSlashingInvalid,
	AttesterSlashingEmptyIndices,
	AttestationTooFarInHistory,
	AttestationSubmittedTooQuickly,
	AttestationIncorrectJustifiedEpochOrBlockRoot,
	AttestationIncorrectCrosslinkData,
	AttestationEmptyAggregation,
	AttestationEmptyCustody,
	AttestationInvalidShard,
	AttestationInvalidCustody,
	AttestationInvalidSignature,
	AttestationInvalidCrosslink,
	VoluntaryExitAlreadyExited,
	VoluntaryExitAlreadyInitiated,
	VoluntaryExitNotYetValid,
	VoluntaryExitNotLongEnough,
	VoluntaryExitInvalidSignature,
	TransferNoFund,
	TransferNotValidSlot,
	TransferNotWithdrawable,
	TransferInvalidPublicKey,
	TransferInvalidSignature,
	TooManyProposerSlashings,
	TooManyAttesterSlashings,
	TooManyAttestations,
	TooManyDeposits,
	TooManyVoluntaryExits,
	TooManyTransfers,
}