// Copyright (c) 2022-2026 Alex Chi Z
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use bytes::BufMut;

use crate::key::{KeySlice, KeyVec};

use super::Block;

/// Builds a block.
pub struct BlockBuilder {
    /// Offsets of each key-value entries.
    offsets: Vec<u16>,
    /// All serialized key-value pairs in the block.
    data: Vec<u8>,
    /// The expected block size.
    block_size: usize,
    /// The first key in the block
    first_key: KeyVec,
}

impl BlockBuilder {
    /// Creates a new block builder.
    pub fn new(block_size: usize) -> Self {
        Self {
            offsets: Vec::new(),
            data: Vec::new(),
            block_size,
            first_key: KeyVec::new(),
        }
    }

    /// Adds a key-value pair to the block. Returns false when the block is full.
    /// You may find the `bytes::BufMut` trait useful for manipulating binary data.
    #[must_use]
    pub fn add(&mut self, key: KeySlice, value: &[u8]) -> bool {
        // 2 bytes for num of elems + 2 for each offset + data len
        let curr_size = 2 + 2 * self.offsets.len() + self.data.len();

        // 2 bytes for key len + 2 for value len + 2 for offset + key len + value len
        let add_size = 6 + key.len() + value.len();

        // the first entry must always fit, so only do a size check
        // if the entry is not the first
        if !self.is_empty() {
            if curr_size + add_size > self.block_size {
                return false;
            }
        } else {
            // this is the first entry
            self.first_key.set_from_slice(key);
        }

        self.offsets.push(self.data.len() as u16);
        self.data.put_u16(key.len() as u16);
        self.data.put_slice(key.into_inner());
        self.data.put_u16(value.len() as u16);
        self.data.put_slice(value);

        true
    }

    /// Check if there is no key-value pair in the block.
    pub fn is_empty(&self) -> bool {
        self.offsets.is_empty()
    }

    /// Finalize the block.
    pub fn build(self) -> Block {
        Block {
            data: self.data,
            offsets: self.offsets,
        }
    }
}
