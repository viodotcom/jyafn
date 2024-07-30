use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};
use std::fmt::{self, Display};

use crate::Error;

use super::size::{InSlots, Size, Unit};

/// The primitive types of data that can be represented in the computational graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, GetSize)]
#[repr(u8)]
pub enum Type {
    /// A floating point number.
    Float,
    /// A boolean.
    Bool,
    /// An _id_ referencing a piece of imutable text "somewhere".
    Symbol,
    /// A pointer, with an origin node id. Pointers _cannot_ appear in the public
    /// interface of a graph.
    Ptr { origin: usize },
    /// An integer timestamp in microseconds.
    DateTime,
}

impl TryFrom<u8> for Type {
    type Error = Error;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Type::Float),
            1 => Ok(Type::Bool),
            2 => Ok(Type::Symbol),
            3 => Ok(Type::Ptr { origin: usize::MAX }),
            4 => Ok(Type::DateTime),
            _ => Err(format!("{v} is not a valid type id"))?,
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Float => write!(f, "scalar"),
            Type::Bool => write!(f, "bool"),
            Type::Symbol => write!(f, "symbol"),
            Type::Ptr { origin } => write!(f, "ptr@{origin}"),
            Type::DateTime => write!(f, "datetime"),
        }
    }
}

/// All slots in jyafn are 64 bits long.
pub const SLOT_SIZE: Size = InSlots::UNIT;

impl Type {
    pub(crate) fn render(self) -> qbe::Type<'static> {
        match self {
            Type::Float => qbe::Type::Double,
            Type::Bool => qbe::Type::Long,
            Type::Symbol => qbe::Type::Long,
            Type::Ptr { .. } => qbe::Type::Long,
            Type::DateTime => qbe::Type::Long,
        }
    }

    pub(crate) fn print(self, val: u64) -> String {
        match self {
            Type::Float => format!("{}", f64::from_ne_bytes(val.to_ne_bytes())),
            Type::Bool => format!("{}", val == 1),
            Type::Symbol => format!("{val}"),
            Type::Ptr { .. } => format!("{val:#x}"),
            Type::DateTime => {
                if let Some(date) =
                    chrono::DateTime::<chrono::Utc>::from_timestamp_micros(val as i64)
                {
                    format!("{date}",)
                } else {
                    "<invalid datetime>".to_string()
                }
            }
        }
    }
}
