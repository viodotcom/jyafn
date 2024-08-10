use serde_derive::{Deserialize, Serialize};

/// The `strptime` format for ISO 8601, the standard used in the [`Layout::DateTime`]
/// variant.
pub const ISOFORMAT: &str = "%Y-%m-%dT%H:%M:%S%.f";

/// A struct is a kind of layout of _ordered_ key-value pairs. Each value is layed out
/// sequentially in memory.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Struct(pub Vec<(String, Layout)>);

/// A layout is a how jyafn makes the correspondence of structured data (like, but not
/// necessarily exactly JSON) and buffers of binary data.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layout {
    /// An empty value.
    #[default]
    Unit,
    /// A floating point number. Jyafn does not support integers directly.
    Scalar,
    /// A boolean. Can be either true or false. This is represented as u64 1 or 0
    /// respectively. All other values are invalid.
    Bool,
    /// A date-time with a given format string. Internally, this is represented as a
    /// timestamp integer in microseconds.
    DateTime(String),
    /// An imutable piece of text.
    Symbol,
    /// An ordered sequence of values, layed out in memory sequentially.
    Struct(Struct),
    /// A layout repeated a given number of times.
    List(Box<Layout>, usize),
}

impl From<Struct> for Layout {
    fn from(fields: Struct) -> Layout {
        Layout::Struct(fields)
    }
}

/// Builds a [`Layout`] usng the jyafn layout display notation.
///
/// # Usage
///
/// This declares a struct layout with two fields: `x`, a scalar and `y` a date.
/// ```
/// layout!({
///     x: scalar,
///     y: datetime "%Y-%m-%d"
/// })
/// ```
#[macro_export]
macro_rules! layout {
    ({$($key:tt : $ty:tt),*}) => {
        $crate::Layout::Struct($crate::r#struct!($($key : $ty),*))
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
    (datetime) => {
        $crate::Layout::DateTime($crate::ISOFORMAT.to_string())
    };
    (symbol) => {
        $crate::Layout::Symbol
    };
    ([$element:tt; $size:expr]) => {
        $crate::Layout::List(Box::new($crate::layout!($element)), $size)
    }
}

/// Builds a [`Struct`] layout out of a collection of keys and values.
#[macro_export]
macro_rules! r#struct {
    ($($key:tt : $ty:tt),*) => {
        $crate::Struct(vec![$(
            $crate::struct_field!($key : $ty)
        ),*])
    };
}

/// Builds a [`Struct`] field, given a key and a value layout.
#[macro_export]
macro_rules! struct_field {
    ($key:literal : $ty:tt) => {
        ($key.to_string(), $crate::layout!($ty))
    };
    ($key:ident : $ty:tt) => {
        (stringify!($key).to_string(), $crate::layout!($ty))
    };
}
