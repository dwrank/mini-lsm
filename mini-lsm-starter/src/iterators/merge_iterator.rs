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

use std::cmp::{self};
use std::collections::BinaryHeap;
use std::collections::binary_heap::PeekMut;

use anyhow::Result;

use crate::key::KeySlice;

use super::StorageIterator;

struct HeapWrapper<I: StorageIterator>(pub usize, pub Box<I>);

impl<I: StorageIterator> PartialEq for HeapWrapper<I> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == cmp::Ordering::Equal
    }
}

impl<I: StorageIterator> Eq for HeapWrapper<I> {}

impl<I: StorageIterator> PartialOrd for HeapWrapper<I> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<I: StorageIterator> Ord for HeapWrapper<I> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.1
            .key()
            .cmp(&other.1.key())
            .then(self.0.cmp(&other.0))
            .reverse()
    }
}

/// Merge multiple iterators of the same type. If the same key occurs multiple times in some
/// iterators, prefer the one with smaller index.
pub struct MergeIterator<I: StorageIterator> {
    iters: BinaryHeap<HeapWrapper<I>>,
    current: Option<HeapWrapper<I>>,
}

impl<I: StorageIterator> MergeIterator<I> {
    pub fn create(iters: Vec<Box<I>>) -> Self {
        let mut heap = BinaryHeap::new();

        for (i, iter) in iters.into_iter().enumerate() {
            if iter.is_valid() {
                heap.push(HeapWrapper(i, iter));
            }
        }

        let current = heap.pop();

        MergeIterator {
            iters: heap,
            current,
        }
    }

    // Skips duplicate keys
    // If the next key on the binary heap is the same as the current key,
    // then the iterator holding that (key, value) is skipped by calling
    // next.  The result is that the next key returned from the binary heap
    // comes after current's key.
    //
    // If a binary heap's iterator's next call returns an Err, then that
    // iterator is popped and the Err is returned.
    // If the iterator has reached it's end, then it is popped.
    // If current is none, then the end is reached and Ok is returned.
    fn skip_duplicate_keys(&mut self) -> Result<()> {
        let Some(current) = self.current.as_ref() else {
            return Ok(());
        };

        while let Some(mut inner) = self.iters.peek_mut() {
            if inner.1.key() != current.1.key() {
                break;
            }

            if let Err(e) = inner.1.next() {
                PeekMut::pop(inner);
                return Err(e);
            } else if !inner.1.is_valid() {
                PeekMut::pop(inner);
            }
        }

        Ok(())
    }
}

impl<I: 'static + for<'a> StorageIterator<KeyType<'a> = KeySlice<'a>>> StorageIterator
    for MergeIterator<I>
{
    type KeyType<'a> = KeySlice<'a>;

    fn key(&self) -> KeySlice<'_> {
        self.current
            .as_ref()
            .map_or(KeySlice::from_slice(b""), |heap_wrapper| {
                heap_wrapper.1.key()
            })
    }

    fn value(&self) -> &[u8] {
        self.current
            .as_ref()
            .map_or(b"", |heap_wrapper| heap_wrapper.1.value())
    }

    fn is_valid(&self) -> bool {
        self.current
            .as_ref()
            .is_some_and(|heap_wrapper| heap_wrapper.1.is_valid())
    }

    // Moves current's iter to the next (key, value) iterator.
    // There may initially be duplicate keys on the binary heap,
    // so first off those are removed.
    // If current is None then the end is reached and Ok is returned.
    // If current is not None then next is called on it's iter.  If
    // next returns an Err then the Err is returned.
    // If current's iter is not valid after next is called, then it is
    // discarded and the next iter from the binary heap takes it's place.
    // If current's iter is valid, then a comparison is made with the top
    // of the binary heap to determine the new current.
    fn next(&mut self) -> Result<()> {
        if let Err(e) = self.skip_duplicate_keys() {
            self.current = None;
            return Err(e);
        }

        let Some(mut current) = self.current.take() else {
            return Ok(());
        };

        current.1.next()?;

        if current.1.is_valid() {
            if let Some(mut top) = self.iters.peek_mut() {
                // because the Ord impl calls .reverse, > means comes first not bigger key.
                // peek_mut() gives a PeekMut guard that wraps and derefs to HeapWrapper<I>.
                // After swapping top and current, the heap's top slot holds the old current
                // which may be out of order.  When the top guard is dropped, PeekMut::drop
                // notices &mut access was taken to the top, and sifts that element down to
                // the correct position.
                if *top > current {
                    std::mem::swap(&mut *top, &mut current);
                }
            }
            self.current = Some(current);
        } else {
            self.current = self.iters.pop();
        }

        Ok(())
    }
}
