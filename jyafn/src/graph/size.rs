//! Utilities for dealing with memory sizes without fantastically messing up the units.

use std::{
    iter::Sum,
    ops::{Add, Mul},
};

/// A size of something in memory. This is just a newtype on top of a `usize` that is
/// also type-checked to make sure that we are representing that size in bytes, in jyafn
/// slots, etc... and not fantastically mess up the units.
#[derive(Debug, Clone, Copy, Default)]
pub struct Size(usize);

impl Size {
    /// Gets this size represented in _bytes_.
    pub const fn in_bytes(self) -> usize {
        self.0
    }
}

impl Mul<Size> for usize {
    type Output = Size;
    fn mul(self, other: Size) -> Size {
        Size(self * other.0)
    }
}

impl Add<Size> for Size {
    type Output = Size;
    fn add(self, other: Size) -> Size {
        Size(self.0 + other.0)
    }
}

impl Sum for Size {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let mut sum = Size::default();
        for el in iter {
            sum = sum + el;
        }
        sum
    }
}

/// Represents a unit of memory size to be used in [`Size`].
pub trait Unit: Send + Sync + Copy {
    /// The size of "1 unit".
    const UNIT: Size;
}

/// The unit of "1 byte".
#[derive(Debug, Clone, Copy)]
pub struct InBytes;

impl Unit for InBytes {
    const UNIT: Size = Size(1);
}

/// The unit of "1 jyafn slot".
#[derive(Debug, Clone, Copy)]
pub struct InSlots;

const SLOT_SIZE: usize = 8;

impl Unit for InSlots {
    const UNIT: Size = Size(SLOT_SIZE);
}
