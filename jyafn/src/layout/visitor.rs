use byte_slice_cast::*;
use std::convert::AsRef;

use crate::size::Size;

/// A builder of binary data to be sent to and from functions. This represents a sequence
/// of slots of 64-bit data that can be grown by pushing more 64-bid data into it.
#[derive(Debug, Clone)]
pub struct Visitor(pub(crate) Box<[u8]>, usize);

impl AsRef<[u8]> for Visitor {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Visitor {
    pub(crate) fn new(size: Size) -> Visitor {
        Visitor(vec![0; size.in_bytes()].into_boxed_slice(), 0)
    }

    pub(crate) fn into_inner(self) -> Box<[u8]> {
        self.0
    }

    pub(crate) fn reset(&mut self) {
        self.1 = 0
    }

    /// Pushes a new scalar value into the visitor.
    pub fn push(&mut self, val: f64) {
        self.0.as_mut_slice_of::<f64>().unwrap()[self.1] = val;
        self.1 += 1;
    }

    /// Removes a scalar value from the end of the visitor, shrinking it by 1 slot.
    pub fn pop(&mut self) -> f64 {
        let top = self.0.as_mut_slice_of::<f64>().unwrap()[self.1];
        self.1 += 1;
        top
    }

    /// Pushes a new integer value into the visitor.
    pub fn push_int(&mut self, val: i64) {
        self.0.as_mut_slice_of::<i64>().unwrap()[self.1] = val;
        self.1 += 1;
    }

    /// Removes an integer value from the end of the visitor, shrinking it by 1 slot.
    pub fn pop_int(&mut self) -> i64 {
        let top = self.0.as_mut_slice_of::<i64>().unwrap()[self.1];
        self.1 += 1;
        top
    }
}
