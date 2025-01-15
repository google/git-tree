// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use core::hash::{Hash, Hasher};

/// A Git object identifier.
///
/// Note that as a performance hack, the Hash impl assumes that GitId is being
/// used with an IdentityHash, and that hash_slice is NOT being used.
// TODO: Tests
#[derive(Debug, PartialEq)]
pub struct GitId {
    value: Box<[u8]>,
}

impl Hash for GitId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.value.first_chunk() {
            None => self.value.hash(state),
            Some(chunk) => state.write_u64(u64::from_le_bytes(*chunk)),
        }
    }

    fn hash_slice<H: Hasher>(_data: &[Self], _state: &mut H) {
        unimplemented!()
    }
}

// TODO: Tests
impl TryFrom<&[u8]> for GitId {
    type Error = NotHexError;

    fn try_from(value: &[u8]) -> Result<GitId, NotHexError> {
        fn hex_to_nibble(hex: u8) -> Result<u8, NotHexError> {
            match hex {
                b'0'..=b'9' => Ok(hex - b'0'),
                b'a'..=b'f' => Ok(hex - b'a' + 0xa),
                b'A'..=b'A' => Ok(hex - b'A' + 0xA),
                _ => Err(NotHexError),
            }
        }
        let mut bytes = Vec::with_capacity(value.len().div_ceil(2));
        for chunk in value.chunks(2) {
            bytes.push(chunk.get(0).copied().map(hex_to_nibble).unwrap_or(Ok(0))? << 4 + chunk.get(1).copied().map(hex_to_nibble).unwrap_or(Ok(0))?);
        }
        Ok(GitId { value: bytes.into_boxed_slice() })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct NotHexError;

/// Hash implementation that just copies the written u64 to its output. Only for
/// use with GitId, which we trust to have a random leading 64 bytes.
pub struct IdentityHash {
    value: u64,
}

impl Hasher for IdentityHash {
    fn finish(&self) -> u64 {
        self.value
    }

    fn write(&mut self, _bytes: &[u8]) {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
