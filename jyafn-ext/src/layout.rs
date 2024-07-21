use serde_derive::{Deserialize, Serialize};

pub const ISOFORMAT: &str = "%Y-%m-%dT%H:%M:%S%.f";

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Struct(pub Vec<(String, Layout)>);

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layout {
    #[default]
    Unit,
    Scalar,
    Bool,
    DateTime(String),
    Symbol,
    Struct(Struct),
    List(Box<Layout>, usize),
}

impl From<Struct> for Layout {
    fn from(fields: Struct) -> Layout {
        Layout::Struct(fields)
    }
}

#[macro_export]
macro_rules! layout {
    ({$($key:literal : $ty:tt),*}) => {
        $crate::r#struct!($($key : $ty),*)
    };
    (unit) => {
        $crate::Layout::Unit
    };
    (scalar) => {
        $crate::Layout::Scalar
    };
    (bool) => {
        $crate::Layout::Bool
    };
    (datetime $format:expr) => {
        $crate::Layout::DateTime($format.to_string())
    };
    (symbol) => {
        $crate::Layout::Symbol
    };
    ([$element:tt; $size:expr]) => {
        $crate::Layout::List(Box::new($crate::layout!($element)), $size)
    }

}

#[macro_export]
macro_rules! r#struct {
    ($($key:tt : $ty:tt),*) => {
        $crate::Struct(vec![$(
            $crate::struct_field!($key : $ty)
        ),*])
    };
}

#[macro_export]
macro_rules! struct_field {
    ($key:literal : $ty:tt) => {
        ($key.to_string(), $crate::layout!($ty))
    };
    ($key:ident : $ty:tt) => {
        (stringify!($key).to_string(), $crate::layout!($ty))
    }
}
