use byte_slice_cast::*;
use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::convert::AsRef;
use std::ops::Deref;

pub const BUFFER_SIZE: usize = 4 * std::mem::size_of::<u64>();
#[derive(Debug, Serialize, Deserialize)]
pub struct Buffer(SmallVec<[u8; BUFFER_SIZE]>);

impl GetSize for Buffer {
    fn get_heap_size(&self) -> usize {
        if self.0.spilled() {
            self.0.capacity()
        } else {
            0
        }
    }
}

impl Deref for Buffer {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.0
    }
}

type BoxBuffer = Box<[u8]>;

#[derive(Debug, Clone)]
pub struct Visitor(pub(crate) BoxBuffer, usize);

impl From<Buffer> for Visitor {
    fn from(b: Buffer) -> Visitor {
        Visitor(b.iter().copied().collect(), 0)
    }
}

impl AsRef<[u8]> for Visitor {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Visitor {
    pub(crate) fn new(size: usize) -> Visitor {
        Visitor(vec![0; size * 8].into_boxed_slice(), 0)
    }

    pub fn into_inner(self) -> Buffer {
        Buffer(self.0.into_vec().into())
    }

    pub(crate) fn reset(&mut self) {
        self.1 = 0
    }

    pub fn push(&mut self, val: f64) {
        self.0.as_mut_slice_of::<f64>().unwrap()[self.1] = val;
        self.1 += 1;
    }

    pub fn pop(&mut self) -> f64 {
        let top = self.0.as_mut_slice_of::<f64>().unwrap()[self.1];
        self.1 += 1;
        top
    }

    pub fn push_int(&mut self, val: i64) {
        self.0.as_mut_slice_of::<i64>().unwrap()[self.1] = val;
        self.1 += 1;
    }

    pub fn pop_int(&mut self) -> i64 {
        let top = self.0.as_mut_slice_of::<i64>().unwrap()[self.1];
        self.1 += 1;
        top
    }
}
