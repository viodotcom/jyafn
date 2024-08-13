//! This module has the main structs that control how information is sent into functions
//! and read from functions in a safe way.

mod decode;
mod encode;
mod ref_value;
mod symbols;
mod visitor;

pub use decode::{Decode, Decoder, ZeroDecoder};
pub use encode::Encode;
pub use ref_value::RefValue;
pub use symbols::{symbol_hash, Sym, Symbols};
pub use visitor::Visitor;

pub(crate) use symbols::SymbolsView;

use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{self, Display};

use crate::size::{InSlots, Size, Unit};
use crate::Error;

use super::{Ref, Type};

/// The `strptime` format for ISO 8601, the standard used in the [`Layout::DateTime`]
/// variant.
pub const ISOFORMAT: &str = "%Y-%m-%dT%H:%M:%S%.f";

/// A struct is a kind of layout of _ordered_ key-value pairs. Each value is layed out
/// sequentially in memory.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, GetSize)]
pub struct Struct(pub Vec<(String, Layout)>);

impl Display for Struct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ ")?;

        for (name, field) in &self.0[0..self.0.len() - 1] {
            if name.contains(":") {
                write!(f, "{name:?}: {field}, ")?;
            } else {
                write!(f, "{name}: {field}, ")?;
            }
        }

        if let Some((name, field)) = self.0.last() {
            if name.contains(":") {
                write!(f, "{name:?}: {field} ")?;
            } else {
                write!(f, "{name}: {field} ")?;
            }
        }

        write!(f, "}}")?;
        Ok(())
    }
}

impl Struct {
    /// The size in slots of this struct.
    pub fn size(&self) -> Size {
        self.0.iter().map(|(_, layout)| layout.size()).sum()
    }

    /// Inserts a new key-value field in this struct.
    pub fn insert(&mut self, name: String, field: Layout) {
        self.0.push((name, field))
    }

    /// Returns the slots of this struct.
    pub fn slots(&self) -> Vec<Type> {
        self.0
            .iter()
            .flat_map(|(_, field)| field.slots())
            .collect::<Vec<_>>()
    }

    /// Prints this struct in a pretty way (recursive part).
    fn pretty_recursive(&self, buf: &mut String, indent: &mut String) {
        *indent += "    ";
        *buf += "{";

        for (name, field) in &self.0 {
            buf.push('\n');
            *buf += indent;
            *buf += name;
            *buf += ": ";
            field.pretty_recursive(buf, indent);
            *buf += ",";
        }

        indent.truncate(indent.len() - 4);
        buf.push('\n');
        *buf += indent;
        *buf += "}";
    }

    /// Prints this struct in a pretty way.
    pub fn pretty(&self) -> String {
        let mut buf = String::new();
        self.pretty_recursive(&mut buf, &mut String::new());
        buf
    }

    /// Tests whether this struct contains all the same fields and values than another
    /// structure. If one field diverges in type, it must at least be the superset of the
    /// corresponding field in the other struct.
    ///
    /// The idea behind this function is that if `A` is superset of `B`, then a value of
    /// `B` can represent a value of `A` without the need to "come up with" new values.
    pub fn is_superset(&self, other: &Struct) -> bool {
        let self_keys = self.0.iter().map(|(name, _)| name).collect::<BTreeSet<_>>();
        let other_keys = other
            .0
            .iter()
            .map(|(name, _)| name)
            .collect::<BTreeSet<_>>();

        if !self_keys.is_superset(&other_keys) {
            return false;
        }

        for name in other_keys {
            let (_, self_field) = self.0.iter().find(|&(n, _)| n == name).expect("key exists");
            let (_, other_field) = other
                .0
                .iter()
                .find(|&(n, _)| n == name)
                .expect("key exists");

            if !self_field.is_superset(other_field) {
                return false;
            }
        }

        true
    }
}

/// A layout is a how jyafn makes the correspondence of structured data (like, but not
/// necessarily exactly JSON) and buffers of binary data. See also the [`crate::layout!`] macro
/// for an easy way to declare layouts.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, GetSize)]
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
    /// An ordered sequence of named values, layed out in memory sequentially.
    Struct(Struct),
    /// An ordered sequence of unnamed values, layed out in memory sequentially.
    Tuple(Vec<Layout>),
    /// A layout repeated a given number of times.
    List(Box<Layout>, usize),
}

impl From<Struct> for Layout {
    fn from(fields: Struct) -> Layout {
        Layout::Struct(fields)
    }
}

impl Display for Layout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Layout::Unit => write!(f, "unit"),
            Layout::Scalar => write!(f, "scalar"),
            Layout::Bool => write!(f, "bool"),
            Layout::DateTime(format) if format == ISOFORMAT => write!(f, "datetime"),
            Layout::DateTime(format) => write!(f, "datetime {format:?}"),
            Layout::Symbol => write!(f, "symbol"),
            Layout::Struct(fields) if f.alternate() => write!(f, "{fields:#}"),
            Layout::Struct(fields) => write!(f, "{fields}"),
            Layout::Tuple(fields) => write!(
                f,
                "({})",
                fields
                    .iter()
                    .map(|field| field.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Layout::List(element, size) if element.as_ref() == &Layout::Scalar => {
                write!(f, "[{size}]")
            }
            Layout::List(element, size) => write!(f, "[{element}; {size}]"),
        }
    }
}

impl Layout {
    /// The size in slots of this struct.
    pub fn size(&self) -> Size {
        #[allow(clippy::erasing_op)]
        match self {
            Layout::Unit => 0 * InSlots::UNIT,
            Layout::Scalar => 1 * InSlots::UNIT,
            Layout::Bool => 1 * InSlots::UNIT,
            Layout::DateTime(_) => 1 * InSlots::UNIT,
            Layout::Symbol => 1 * InSlots::UNIT,
            Layout::Struct(fields) => fields.size(),
            Layout::Tuple(fields) => fields.iter().map(Layout::size).sum(),
            Layout::List(element, size) => *size * element.size(),
        }
    }

    /// Returns the slots of this struct.
    pub fn slots(&self) -> Vec<Type> {
        match self {
            Layout::Unit => vec![],
            Layout::Scalar => vec![Type::Float],
            Layout::Bool => vec![Type::Bool],
            Layout::DateTime(_) => vec![Type::DateTime],
            Layout::Symbol => vec![Type::Symbol],
            Layout::Struct(fields) => fields.slots(),
            Layout::Tuple(fields) => fields.iter().flat_map(Layout::slots).collect(),
            Layout::List(element, size) => [element.slots()]
                .into_iter()
                .cycle()
                .take(*size)
                .flatten()
                .collect(),
        }
    }

    fn build_ref_value_inner<I>(&self, it: &mut I) -> Option<RefValue>
    where
        I: Iterator<Item = Ref>,
    {
        Some(match self {
            Layout::Unit => RefValue::Unit,
            Layout::Scalar => RefValue::Scalar(it.next()?),
            Layout::Bool => RefValue::Bool(it.next()?),
            Layout::DateTime(_) => RefValue::DateTime(it.next()?),
            Layout::Symbol => RefValue::Symbol(it.next()?),
            Layout::Struct(fields) => RefValue::Struct(
                fields
                    .0
                    .iter()
                    .map(|(name, field)| {
                        Some((name.clone(), field.build_ref_value_inner(it.by_ref())?))
                    })
                    .collect::<Option<_>>()?,
            ),
            Layout::Tuple(fields) => RefValue::Tuple(
                fields
                    .iter()
                    .map(|field| field.build_ref_value_inner(it.by_ref()))
                    .collect::<Option<_>>()?,
            ),
            Layout::List(element, size) => RefValue::List(
                (0..*size)
                    .map(|_| element.build_ref_value_inner(it.by_ref()))
                    .collect::<Option<Vec<_>>>()?,
            ),
        })
    }

    /// Builds a structured [`RefValue`] out of unstructured data, represented as an
    /// iterator of [`Ref`]s. Returns `None` if the representation is not possible (i.e.
    /// a type error).
    pub fn build_ref_value<I>(&self, it: I) -> Option<RefValue>
    where
        I: IntoIterator<Item = Ref>,
    {
        self.build_ref_value_inner(&mut it.into_iter())
    }

    /// Prints this layout in a pretty way (recursive part).
    fn pretty_recursive(&self, buf: &mut String, indent: &mut String) {
        if let Layout::Struct(strct) = self {
            strct.pretty_recursive(buf, indent)
        } else {
            *buf += &self.to_string();
        }
    }

    /// Prints this layout in a pretty way.
    pub fn pretty(&self) -> String {
        let mut buf = String::new();
        self.pretty_recursive(&mut buf, &mut String::new());
        buf
    }

    /// Tests whether this layout contains all the same fields and values than another
    /// layout. If one field diverges in type, it must at least be the superset of the
    /// corresponding field in the other layout.
    ///
    /// The idea behind this function is that if `A` is superset of `B`, then a value of
    /// `B` can represent a value of `A` without the need to "come up with" new values.
    pub fn is_superset(&self, other: &Layout) -> bool {
        match (self, other) {
            (Layout::Struct(self_struct), Layout::Struct(other_struct)) => {
                self_struct.is_superset(other_struct)
            }
            (Layout::List(self_item, self_len), Layout::List(other_item, other_len))
                if self_len == other_len =>
            {
                self_item.is_superset(other_item)
            }
            _ => self == other,
        }
    }

    pub fn encode<E: Encode, S: Sym>(&self, msg: &E, symbols: &mut S) -> Result<Box<[u8]>, Error> {
        let mut visitor = Visitor::new(self.size());
        msg.visit(self, symbols, &mut visitor)
            .map_err(|err| Error::EncodeError(Box::new(err)))?;
        Ok(visitor.into_inner())
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
    ({$($key:literal : $ty:tt),*}) => {
        $crate::r#struct!($($key : $ty),*)
    };
    (($($ty:tt),*)) => {
        $crate::layout::Tuple(vec![$($ty),*])
    };
    (unit) => {
        $crate::layout::Layout::Unit
    };
    (scalar) => {
        $crate::layout::Layout::Scalar
    };
    (bool) => {
        $crate::layout::Layout::Bool
    };
    (datetime $format:expr) => {
        $crate::layout::Layout::DateTime($format.to_string())
    };
    (datetime) => {
        $crate::layout::Layout::DateTime($crate::ISOFORMAT.to_string())
    };
    (symbol) => {
        $crate::layout::Layout::Symbol
    };
    ([$element:tt; $size:expr]) => {
        $crate::layout::Layout::List(Box::new($crate::layout!($element)), $size)
    }
}

/// Builds a [`Struct`] layout out of a collection of keys and values.
#[macro_export]
macro_rules! r#struct {
    ($($key:tt : $ty:tt),*) => {
        $crate::layout::Struct(vec![$(
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
