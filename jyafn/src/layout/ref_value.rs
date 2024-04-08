use std::collections::HashMap;

use crate::Ref;

use super::{Layout, Struct};

#[derive(Debug)]
pub enum RefValue {
    Unit,
    Scalar(Ref),
    Bool(Ref),
    Symbol(Ref),
    Struct(HashMap<String, RefValue>),
    List(Vec<RefValue>),
}

impl RefValue {
    pub fn putative_layout(&self) -> Layout {
        match self {
            Self::Unit => Layout::Unit,
            Self::Scalar(_) => Layout::Scalar,
            Self::Bool(_) => Layout::Bool,
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
                if let Some(first) = list.get(0) {
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
