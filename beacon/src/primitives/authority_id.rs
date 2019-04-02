// Copyright 2017-2018 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

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

#[cfg(feature = "serde")]
use serde::{Serialize, Serializer, Deserialize, Deserializer};
use fixed_hash::construct_fixed_hash;
#[cfg(feature = "serde")]
use impl_serde::serialize as bytes;

const SIZE: usize = 48;

construct_fixed_hash! {
	/// Fixed 384-bit hash.
	pub struct H384(SIZE);
}

pub type ValidatorId = H384;

#[cfg(feature = "serde")]
impl Serialize for H384 {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
		bytes::serialize(&self.0, serializer)
	}
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for H384 {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
		bytes::deserialize_check_len(deserializer, bytes::ExpectedLen::Exact(SIZE))
			.map(|x| H384::from_slice(&x))
	}
}

#[cfg(feature = "parity-codec")]
impl codec::Encode for H384 {
	fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
		self.0.using_encoded(f)
	}
}

#[cfg(feature = "parity-codec")]
impl codec::Decode for H384 {
	fn decode<I: codec::Input>(input: &mut I) -> Option<Self> {
		<[u8; SIZE] as codec::Decode>::decode(input).map(H384)
	}
}

impl ssz::Encode for H384 {
	fn encode_to<W: ::ssz::Output>(&self, dest: &mut W) {
		dest.write(self.as_ref())
	}
}
impl ssz::Decode for H384 {
	fn decode_as<I: ::ssz::Input>(input: &mut I) -> Option<(Self, usize)> {
		let mut vec = [0u8; SIZE];
		if input.read(&mut vec[..SIZE]) != SIZE {
			None
		} else {
			Some((H384::from(&vec), SIZE))
		}
	}
}

impl ssz::Prefixable for H384 {
	fn prefixed() -> bool {
		false
	}
}

impl ssz::Composite for H384 { }

impl ssz::Hashable for H384 {
	fn hash<H: ::hash_db::Hasher>(&self) -> H::Out {
		ssz::hash::merkleize::<H>(ssz::hash::chunkify(self.as_ref()))
	}
}
