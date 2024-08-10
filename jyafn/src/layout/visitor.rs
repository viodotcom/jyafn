use byte_slice_cast::*;

use crate::size::Size;

/// A builder of binary data to be sent to and from functions. This represents a sequence
/// of slots of 64-bit data that can be grown by pushing more 64-bid data into it.
#[derive(Debug, Clone)]
pub struct Visitor(pub(crate) Box<[u8]>, isize);

impl From<Box<[u8]>> for Visitor {
    fn from(value: Box<[u8]>) -> Self {
        let len = value.len();
        Visitor(value, len as isize)
    }
}

impl Visitor {
    pub fn new(size: Size) -> Visitor {
        Visitor(vec![0; size.in_bytes()].into_boxed_slice(), 0)
    }

    pub fn into_inner(self) -> Box<[u8]> {
        self.0
    }

    pub fn reset(&mut self) {
        self.1 = 0
    }

    pub fn set_full(&mut self) {
        self.1 = self.0.len() as isize;
    }

    pub fn buffer(&self) -> &[u8] {
        &self.0
    }

    pub fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }

    /// Pushes a new scalar value into the visitor.
    pub fn push(&mut self, val: f64) {
        self.0.as_mut_slice_of::<f64>().unwrap()[self.1 as usize] = val;
        self.1 += 1;
    }

    /// Removes a scalar value from the end of the visitor, shrinking it by 1 slot.
    pub fn pop(&mut self) -> f64 {
        let top = self.0.as_mut_slice_of::<f64>().unwrap()[self.1 as usize];
        self.1 -= 1;
        top
    }

    /// Pushes a new integer value into the visitor.
    pub fn push_int(&mut self, val: i64) {
        self.0.as_mut_slice_of::<i64>().unwrap()[self.1 as usize] = val;
        self.1 += 1;
    }

    /// Removes an integer value from the end of the visitor, shrinking it by 1 slot.
    pub fn pop_int(&mut self) -> i64 {
        let top = self.0.as_mut_slice_of::<i64>().unwrap()[self.1 as usize];
        self.1 -= 1;
        top
    }

    /// Pushes a new unsigned integer value into the visitor.
    pub fn push_uint(&mut self, val: u64) {
        self.0.as_mut_slice_of::<u64>().unwrap()[self.1 as usize] = val;
        self.1 += 1;
    }

    /// Removes an integer value from the end of the visitor, shrinking it by 1 slot.
    pub fn pop_uint(&mut self) -> u64 {
        let top = self.0.as_mut_slice_of::<u64>().unwrap()[self.1 as usize];
        self.1 -= 1;
        top
    }
}
