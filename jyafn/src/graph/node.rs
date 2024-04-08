use serde_derive::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    sync::Arc,
};

use crate::{Error, Op};

use super::Graph;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Type {
    Float,
    Bool,
    Symbol,
    Int,
}

impl TryFrom<u8> for Type {
    type Error = Error;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == Type::Float as u8 => Ok(Type::Float),
            x if x == Type::Bool as u8 => Ok(Type::Bool),
            x if x == Type::Symbol as u8 => Ok(Type::Symbol),
            x if x == Type::Int as u8 => Ok(Type::Int),
            _ => Err(format!("{v} is not a valid type id"))?,
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Float => write!(f, "f64"),
            Type::Bool => write!(f, "bool"),
            Type::Symbol => write!(f, "symbol"),
            Type::Int => write!(f, "i64"),
        }
    }
}

impl Type {
    pub(crate) fn render(self) -> qbe::Type<'static> {
        match self {
            Type::Float => qbe::Type::Double,
            Type::Bool => qbe::Type::Long,
            Type::Symbol => qbe::Type::Long,
            Type::Int => qbe::Type::Long,
        }
    }

    pub(crate) fn size(self) -> usize {
        match self {
            Type::Float => 8,
            Type::Bool => 8,
            Type::Symbol => 8,
            Type::Int => 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ref {
    Input(usize),
    Const(Type, u64),
    Node(usize),
}

impl From<f64> for Ref {
    fn from(v: f64) -> Ref {
        Ref::Const(Type::Float, u64::from_ne_bytes(v.to_ne_bytes()))
    }
}

impl From<bool> for Ref {
    fn from(v: bool) -> Ref {
        Ref::Const(Type::Bool, if v { 1 } else { 0 })
    }
}

impl Ref {
    pub(crate) fn render(self) -> qbe::Value {
        match self {
            Ref::Input(input_id) => qbe::Value::Temporary(format!("i{input_id}")),
            Ref::Const(_, r#const) => qbe::Value::Const(r#const),
            Ref::Node(node_id) => qbe::Value::Temporary(format!("n{node_id}")),
        }
    }

    pub fn as_f64(self) -> Option<f64> {
        if let Self::Const(Type::Float, c) = self {
            Some(f64::from_ne_bytes(u64::to_ne_bytes(c)))
        } else {
            None
        }
    }

    pub fn as_bool(self) -> Option<bool> {
        if let Self::Const(Type::Bool, c) = self {
            Some(if c == 1 { true } else { false })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub(crate) op: Arc<dyn Op>,
    pub(crate) args: Vec<Ref>,
    pub(crate) ty: Type,
}

impl Node {
    pub(crate) fn init<O: Op>(graph: &Graph, mut op: O, args: Vec<Ref>) -> Result<Node, Error> {
        let arg_types = args.iter().map(|r| graph.type_of(*r)).collect::<Vec<_>>();
        let Some(ty) = op.annotate(graph, &arg_types) else {
            return Err(Error::Type(Box::new(op), arg_types));
        };

        Ok(Node {
            op: Arc::new(op),
            args,
            ty,
        })
    }
}
