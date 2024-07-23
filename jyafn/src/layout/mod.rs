mod decode;
mod encode;
mod ref_value;
mod symbols;
mod visitor;

pub use decode::{Decode, Decoder, ZeroDecoder};
pub use encode::Encode;
pub use ref_value::RefValue;
pub use symbols::{Sym, Symbols};
pub use visitor::{Buffer, Visitor, BUFFER_SIZE};

pub(crate) use symbols::SymbolsView;

use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{self, Display};

use super::{Ref, Type};

pub const ISOFORMAT: &str = "%Y-%m-%dT%H:%M:%S%.f";

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
    pub fn size(&self) -> usize {
        self.0.iter().map(|(_, layout)| layout.size()).sum()
    }

    pub fn insert(&mut self, name: String, field: Layout) {
        self.0.push((name, field))
    }

    pub fn slots(&self) -> Vec<Type> {
        self.0
            .iter()
            .flat_map(|(_, field)| field.slots())
            .collect::<Vec<_>>()
    }

    fn pretty_recursive(&self, buf: &mut String, indent: &mut String) {
        *indent += "    ";
        *buf += "{";

        for (name, field) in &self.0 {
            buf.push('\n');
            *buf += indent;
            *buf += &name;
            *buf += ": ";
            field.pretty_recursive(buf, indent);
            *buf += ",";
        }

        indent.truncate(indent.len() - 4);
        buf.push('\n');
        *buf += indent;
        *buf += "}";
    }

    pub fn pretty(&self) -> String {
        let mut buf = String::new();
        self.pretty_recursive(&mut buf, &mut String::new());
        buf
    }

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
            let (_, self_field) = self
                .0
                .iter()
                .filter(|&(n, _)| n == name)
                .next()
                .expect("key exists");
            let (_, other_field) = other
                .0
                .iter()
                .filter(|&(n, _)| n == name)
                .next()
                .expect("key exists");

            if !self_field.is_superset(other_field) {
                return false;
            }
        }

        true
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, GetSize)]
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
            Layout::List(element, size) if element.as_ref() == &Layout::Scalar => {
                write!(f, "[{size}]")
            }
            Layout::List(element, size) => write!(f, "[{element}; {size}]"),
        }
    }
}

impl Layout {
    pub fn size(&self) -> usize {
        match self {
            Layout::Unit => 0,
            Layout::Scalar => 1,
            Layout::Bool => 1,
            Layout::DateTime(_) => 1,
            Layout::Symbol => 1,
            Layout::Struct(fields) => fields.size(),
            Layout::List(element, size) => size * element.size(),
        }
    }

    pub fn slots(&self) -> Vec<Type> {
        match self {
            Layout::Unit => vec![],
            Layout::Scalar => vec![Type::Float],
            Layout::Bool => vec![Type::Bool],
            Layout::DateTime(_) => vec![Type::DateTime],
            Layout::Symbol => vec![Type::Symbol],
            Layout::Struct(fields) => fields.slots(),
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
            Layout::List(element, size) => RefValue::List(
                (0..*size)
                    .map(|_| element.build_ref_value_inner(it.by_ref()))
                    .collect::<Option<Vec<_>>>()?,
            ),
        })
    }

    pub fn build_ref_value<I>(&self, it: I) -> Option<RefValue>
    where
        I: IntoIterator<Item = Ref>,
    {
        self.build_ref_value_inner(&mut it.into_iter())
    }

    fn pretty_recursive(&self, buf: &mut String, indent: &mut String) {
        if let Layout::Struct(strct) = self {
            strct.pretty_recursive(buf, indent)
        } else {
            *buf += &self.to_string();
        }
    }

    pub fn pretty(&self) -> String {
        let mut buf = String::new();
        self.pretty_recursive(&mut buf, &mut String::new());
        buf
    }

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
    };
}
