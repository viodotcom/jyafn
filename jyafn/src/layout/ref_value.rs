use std::collections::HashMap;
use std::fmt::{self, Display};

use crate::{Graph, Ref};

use super::{Layout, Struct, ISOFORMAT};

#[derive(Debug)]
pub enum RefValue {
    Unit,
    Scalar(Ref),
    Bool(Ref),
    DateTime(Ref),
    Symbol(Ref),
    Struct(HashMap<String, RefValue>),
    List(Vec<RefValue>),
}

impl Display for RefValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => write!(f, "unit"),
            Self::Scalar(s) => write!(f, "scalar {s}"),
            Self::Bool(s) => write!(f, "bool {s}"),
            Self::DateTime(s) => write!(f, "datetime {s}"),
            Self::Symbol(s) => write!(f, "symbol {s}"),
            Self::Struct(fields) => {
                write!(f, "{{ ")?;
                for (name, field) in fields {
                    write!(f, "{name}: {field}, ")?;
                }
                write!(f, "}}")
            }
            Self::List(list) => {
                write!(f, "[ ")?;
                for field in list {
                    write!(f, "{field}, ")?;
                }
                write!(f, "]")
            }
        }
    }
}

impl RefValue {
    pub fn putative_layout(&self) -> Layout {
        match self {
            Self::Unit => Layout::Unit,
            Self::Scalar(_) => Layout::Scalar,
            Self::Bool(_) => Layout::Bool,
            Self::DateTime(_) => Layout::DateTime(ISOFORMAT.to_string()),
            Self::Symbol(_) => Layout::Symbol,
            Self::Struct(fields) => Layout::Struct(Struct({
                let mut strct = fields
                    .iter()
                    .map(|(name, field)| (name.clone(), field.putative_layout()))
                    .collect::<Vec<_>>();
                strct.sort_unstable_by_key(|(n, _)| n.clone());
                strct
            })),
            Self::List(list) => {
                if let Some(first) = list.first() {
                    Layout::List(Box::new(first.putative_layout()), list.len())
                } else {
                    Layout::List(Box::new(Layout::Scalar), 0)
                }
            }
        }
    }

    pub fn output_vec(&self, layout: &Layout) -> Option<Vec<Ref>> {
        let mut buffer = vec![];
        self.build_output_vec(layout, &mut buffer)?;
        Some(buffer)
    }

    fn build_output_vec(&self, layout: &Layout, buf: &mut Vec<Ref>) -> Option<()> {
        match (self, layout) {
            (Self::Unit, Layout::Unit) => {}
            (Self::Scalar(s), Layout::Scalar) => buf.push(*s),
            (Self::Bool(s), Layout::Bool) => buf.push(*s),
            (Self::DateTime(s), Layout::DateTime(_)) => buf.push(*s),
            (Self::Symbol(s), Layout::Symbol) => buf.push(*s),
            (Self::Struct(vals), Layout::Struct(fields)) => {
                for (name, field) in &fields.0 {
                    vals.get(name)?.build_output_vec(field, buf);
                }
            }
            (Self::List(list), Layout::List(element, size)) if list.len() == *size => {
                for item in list {
                    item.build_output_vec(element, buf)?;
                }
            }
            _ => return None,
        }

        Some(())
    }
}
