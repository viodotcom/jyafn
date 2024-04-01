use super::Type;

use std::fmt::Debug;

#[typetag::serde(tag = "type")]
pub trait Const: 'static + Debug + Send {
    fn annotate(&self) -> Type;
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
