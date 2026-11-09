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

mod builder;
mod iterator;

use bytes::{Buf, BufMut, BytesMut};

pub use builder::BlockBuilder;
use bytes::Bytes;
pub use iterator::BlockIterator;

/// A block is the smallest unit of read and caching in LSM tree. It is a collection of sorted key-value pairs.
pub struct Block {
    pub(crate) data: Vec<u8>,
    pub(crate) offsets: Vec<u16>,
}

impl Block {
    /// Encode the internal data to the data layout illustrated in the course
    /// Note: You may want to recheck if any of the expected field is missing from your output
    pub fn encode(&self) -> Bytes {
        let mut buf = BytesMut::new();

        buf.extend_from_slice(&self.data);

        for &offset in self.offsets.iter() {
            buf.put_u16(offset);
        }

        buf.put_u16(self.offsets.len() as u16);

        buf.into()
    }

    /// Decode from the data layout, transform the input `data` to a single `Block`
    pub fn decode(data: &[u8]) -> Self {
        let len = data.len();
        assert!(len >= 2, "block too short: {len} bytes");

        let bytes: [u8; 2] = data[len - 2..].try_into().unwrap();
        let num_elems = u16::from_be_bytes(bytes) as usize;
        // 2 bytes for number of elements + for each element:
        // 2 for offset, 2 for key len, 2 for value len
        let min_block_size = 2 + 6 * num_elems;
        assert!(
            len >= min_block_size,
            "corrupted block: block len is {len}, {num_elems} entries need at least {min_block_size} bytes"
        );

        let offset_start = len - num_elems * 2 - 2;

        Self {
            data: data[..offset_start].to_vec(),
            offsets: data[offset_start..len - 2]
                .chunks(2)
                .map(|mut chunk| chunk.get_u16())
                .collect(),
        }
    }
}
