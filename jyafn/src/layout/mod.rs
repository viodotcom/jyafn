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

use serde_derive::{Deserialize, Serialize};
use std::fmt::{self, Display};

use super::{Ref, Type};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Struct(pub Vec<(String, Layout)>);

impl Display for Struct {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ ")?;

        for (name, field) in &self.0[0..self.0.len() - 1] {
            if field == &Layout::Scalar {
                write!(f, "{name:?}, ")?;
            } else {
                write!(f, "{name}: {field}, ")?;
            }
        }

        if let Some((name, field)) = self.0.last() {
            if field == &Layout::Scalar {
                write!(f, "{name:?} ")?;
            } else {
                write!(f, "{name:?}: {field} ")?;
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
            .map(|(_, field)| field.slots())
            .flatten()
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layout {
    #[default]
    Unit,
    Scalar,
    Bool,
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
            Layout::Scalar => write!(f, "f64"),
            Layout::Bool => write!(f, "bool"),
            Layout::Symbol => write!(f, "symbol"),
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
}
