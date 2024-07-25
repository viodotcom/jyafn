//! Constant values in the computational graph. Constants need to have a type and a binary
//! representation as a 64-bit peice of data.

use super::Type;

use std::fmt::Debug;

/// A constant. Constants need to have a type and a binary representation as a 64-bit
/// peice of data.
#[typetag::serde(tag = "type")]
pub trait Const: 'static + Debug + Send {
    /// The primitive. type of this constant.
    fn annotate(&self) -> Type;
    /// The binary representation of this constant.
    fn render(&self) -> u64;
}

#[typetag::serde]
impl Const for f64 {
    fn annotate(&self) -> Type {
        Type::Float
    }

    fn render(&self) -> u64 {
        u64::from_ne_bytes(self.to_ne_bytes())
    }
}

#[typetag::serde]
impl Const for bool {
    fn annotate(&self) -> Type {
        Type::Bool
    }

    fn render(&self) -> u64 {
        match *self {
            true => 1,
            false => 0,
        }
    }
}
