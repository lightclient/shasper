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

use primitives::{Slot, Balance, Timestamp};

pub const CYCLE_LENGTH: Slot = 4;
pub const BASE_REWARD_QUOTIENT: Balance = 32;
pub const INACTIVITY_PENALTY_QUOTIENT: Balance = 16777216;
pub const INCLUDER_REWARD_QUOTIENT: Balance = 8;
pub const MIN_ATTESTATION_INCLUSION_DELAY: Slot = 0;
pub const WHISTLEBLOWER_REWARD_QUOTIENT: Balance = 512;
pub const SLOT_DURATION: Timestamp = 12;
pub const SLOT_INHERENT_EXTRINSIC_INDEX: u32 = 0;
pub const RANDAO_INHERENT_EXTRINSIC_INDEX: u32 = 1;
pub const ATTESTATION_EXTRINSIC_START_INDEX: u32 = 2;
