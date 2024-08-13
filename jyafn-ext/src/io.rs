//! In-out convenince for reading raw function parameters.

use byte_slice_cast::*;
use std::mem::MaybeUninit;

/// A convenience wrapper over the input data pointer, given the information on its size.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Input<'a>(&'a [u64]);

impl<'a> Input<'a> {
    /// Creates a new input.
    ///
    /// # Safety
    ///
    /// Make sure that `input` points to a slice with _memory size_ of `8 * n_slots` at
    /// least. Failing to do so, reads from bad memory may occur.
    pub unsafe fn new(input: *const u8, n_slots: usize) -> Self {
        Self(std::slice::from_raw_parts(input as *const u64, n_slots))
    }

    /// Gets the data at index `idx` as an `f64`.
    pub fn get_f64(&self, idx: usize) -> f64 {
        f64::from_ne_bytes(self.0[idx].to_ne_bytes())
    }

    /// Gets the data at index `idx` as an `u64`.
    pub fn get_u64(&self, idx: usize) -> u64 {
        self.0[idx]
    }

    /// Gets the data at index `idx` as an `i64`.
    pub fn get_i64(&self, idx: usize) -> i64 {
        self.0[idx] as i64
    }

    /// Gets the data at index `idx` as a `bool`.
    pub fn get_bool(&self, idx: usize) -> bool {
        self.0[idx] == 1
    }

    /// Represents itself as a slice of `f64`s.
    pub fn as_f64_slice(&self) -> &[f64] {
        self.0
            .as_byte_slice()
            .as_slice_of()
            .expect("f64 and u64 have the same size")
    }

    /// Represents itself as a slice of `u64`s.
    pub fn as_u64_slice(&self) -> &[u64] {
        self.0
    }

    /// Represents itself as a slice of `i64`s.
    pub fn as_i64_slice(&self) -> &[i64] {
        self.0
            .as_byte_slice()
            .as_slice_of()
            .expect("i64 and u64 have the same size")
    }
}

/// A reader over an [`Input`], which may help you in reading chunks of data from the input.
#[derive(Debug, Clone, Copy)]
pub struct InputReader<'a> {
    position: usize,
    input: Input<'a>,
}

impl<'a> InputReader<'a> {
    /// Creates a new reader from a given input, starting at 0.
    pub fn new(input: Input<'a>) -> Self {
        Self { position: 0, input }
    }

    /// Creates a new reader from a given input, starting at a given position.
    pub fn new_at(position: usize, input: Input<'a>) -> Self {
        Self { position, input }
    }

    /// Reads a float, advancing one position.
    pub fn read_f64(&mut self) -> f64 {
        let read = self.input.get_f64(self.position);
        self.position += 1;
        read
    }

    /// Reads an usigned integer, advancing one position.
    pub fn read_u64(&mut self) -> u64 {
        let read = self.input.get_u64(self.position);
        self.position += 1;
        read
    }

    /// Reads an integer, advancing one position.
    pub fn read_i64(&mut self) -> i64 {
        let read = self.input.get_i64(self.position);
        self.position += 1;
        read
    }

    /// Iterates through `n` floats, advancing the reader.
    pub fn iter_n_f64(&'a mut self, n: usize) -> impl 'a + Iterator<Item = f64> {
        (0..n).map(|_| self.read_f64())
    }

    /// Iterates through `n` unsigned integers, advancing the reader.
    pub fn iter_n_u64(&'a mut self, n: usize) -> impl 'a + Iterator<Item = u64> {
        (0..n).map(|_| self.read_u64())
    }

    /// Iterates through `n` integers, advancing the reader.
    pub fn iter_n_i64(&'a mut self, n: usize) -> impl 'a + Iterator<Item = i64> {
        (0..n).map(|_| self.read_i64())
    }

    /// Reads `n` floats, advancing the reader.
    pub fn read_n_f64(&mut self, n: usize) -> Vec<f64> {
        (0..n).map(|_| self.read_f64()).collect()
    }

    /// Reads `n` unsigned integers, advancing the reader.
    pub fn read_n_u64(&mut self, n: usize) -> Vec<u64> {
        (0..n).map(|_| self.read_u64()).collect()
    }

    /// Reads `n` integers, advancing the reader.
    pub fn read_n_i64(&mut self, n: usize) -> Vec<i64> {
        (0..n).map(|_| self.read_i64()).collect()
    }

    /// Reads `n` floats into a slice, advancing the reader.
    pub fn read_f64_into(&mut self, slice: &mut [f64]) {
        for item in slice {
            *item = self.read_f64();
        }
    }

    /// Reads `n` unsigned integers into a slice, advancing the reader.
    pub fn read_u64_into(&mut self, slice: &mut [u64]) {
        for item in slice {
            *item = self.read_u64();
        }
    }

    /// Reads `n` integers into a slice, advancing the reader.
    pub fn read_i64_into(&mut self, slice: &mut [i64]) {
        for item in slice {
            *item = self.read_i64();
        }
    }
}

/// A convenience wrapper over the output data pointer, given the information on its size.
#[derive(Debug)]
pub struct OutputBuilder<'a> {
    position: usize,
    slice: &'a mut [MaybeUninit<u64>],
}

impl<'a> Drop for OutputBuilder<'a> {
    fn drop(&mut self) {
        // This prevents any uninitialized memory from ever being read.
        while self.position < self.slice.len() {
            self.push_u64(0)
        }
    }
}

impl<'a> OutputBuilder<'a> {
    /// Creates a new input.
    ///
    /// # Safety
    ///
    /// Make sure that `output` points to a slice with _memory size_ of `8 * n_slots` at
    /// least. Failing to do so, writes to bad memory may occur.
    pub unsafe fn new(output: *mut u8, n_slots: usize) -> Self {
        Self {
            position: 0,
            slice: std::slice::from_raw_parts_mut(output as *mut MaybeUninit<u64>, n_slots),
        }
    }

    pub fn push_f64(&mut self, val: f64) {
        self.slice[self.position].write(u64::from_ne_bytes(val.to_ne_bytes()));
        self.position += 1;
    }

    pub fn push_u64(&mut self, val: u64) {
        self.slice[self.position].write(val);
        self.position += 1;
    }

    pub fn push_i64(&mut self, val: i64) {
        self.slice[self.position].write(val as u64);
        self.position += 1;
    }

    pub fn push_bool(&mut self, val: bool) {
        self.slice[self.position].write(val as u64);
        self.position += 1;
    }

    pub fn copy_from_f64(&mut self, src: &[f64]) {
        for &val in src {
            self.push_f64(val);
        }
    }

    pub fn copy_from_u64(&mut self, src: &[u64]) {
        for &val in src {
            self.push_u64(val);
        }
    }

    pub fn copy_from_i64(&mut self, src: &[i64]) {
        for &val in src {
            self.push_i64(val);
        }
    }

    pub fn copy_from_bool(&mut self, src: &[bool]) {
        for &val in src {
            self.push_bool(val);
        }
    }
}
