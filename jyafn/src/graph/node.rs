use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};
use std::cmp::PartialEq;
use std::fmt::{self, Display};

use crate::{Error, Op};

use super::Graph;
use super::Type;

/// A reference to a value in a graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, GetSize)]
pub enum Ref {
    /// A reference to the input of a given id.
    Input(usize),
    /// A constant value of a given type and given binary representation.
    Const(Type, u64),
    /// A reference to a node of a given id.
    Node(usize),
}

impl Display for Ref {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input(id) => write!(f, "input {id}"),
            Self::Const(ty, val) => write!(f, "const {}", ty.print(*val)),
            Self::Node(id) => write!(f, "node {id}"),
        }
    }
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

    /// Represents this ref as an f64, if it is a constant.
    pub fn as_f64(self) -> Option<f64> {
        if let Self::Const(Type::Float, c) = self {
            Some(f64::from_ne_bytes(u64::to_ne_bytes(c)))
        } else {
            None
        }
    }

    /// Represents this ref as an f64, if it is a constant.
    pub fn as_bool(self) -> Option<bool> {
        if let Self::Const(Type::Bool, c) = self {
            Some(c == 1)
        } else {
            None
        }
    }
}

/// A node of the computational graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// The operation that this node performs.
    pub(crate) op: Box<dyn Op>,
    /// The inputs of the operation.
    pub(crate) args: Vec<Ref>,
    /// The single output of the operation.
    pub(crate) ty: Type,
}

impl PartialEq for Node {
    fn eq(&self, other: &Node) -> bool {
        self.op.is_eq(other.op.as_ref()) && self.args == other.args && self.ty == other.ty
    }
}

impl GetSize for Node {
    fn get_heap_size(&self) -> usize {
        self.op.get_size()
    }
}

impl Node {
    /// Creates a new node.
    pub(crate) fn init<O: Op>(
        node_id: usize,
        graph: &Graph,
        mut op: O,
        args: Vec<Ref>,
    ) -> Result<Node, Error> {
        let arg_types = args.iter().map(|r| graph.type_of(*r)).collect::<Vec<_>>();
        let Some(ty) = op.annotate(node_id, graph, &arg_types) else {
            return Err(Error::Type(Box::new(op), arg_types));
        };

        Ok(Node {
            op: Box::new(op),
            args,
            ty,
        })
    }
}
