use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};
use std::cmp::PartialEq;
use std::fmt::{self, Display};

use crate::{Error, Op};

use super::{Builder, Graph};
use super::{Type, SLOT_SIZE};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Register(pub usize);

impl Register {
    pub fn render(self) -> qbe::Value {
        qbe::Value::Temporary(format!("a{}", self.0))
    }
}

impl Ref {
    pub(crate) fn load(self, arg_num: usize, builder: &mut Builder) -> Register {
        let ty = builder.graph.type_of(self);
        let arg_reg = Register(arg_num);

        match self {
            Ref::Input(input_id) => {
                let arg_reg = Register(arg_num);
                let stack_ptr = qbe::Value::Temporary("stkptr".to_owned());
                builder.func.assign_instr(
                    stack_ptr.clone(),
                    qbe::Type::Long,
                    qbe::Instr::Add(
                        qbe::Value::Temporary("in".to_owned()),
                        qbe::Value::Const((SLOT_SIZE.in_bytes() * input_id) as u64),
                    ),
                );
                builder.func.assign_instr(
                    arg_reg.render(),
                    ty.render(),
                    qbe::Instr::Load(ty.render(), stack_ptr),
                );
            }
            Ref::Const(_, r#const) => {
                builder.func.assign_instr(
                    arg_reg.render(),
                    ty.render(),
                    qbe::Instr::Copy(qbe::Value::Const(r#const)),
                );
            }
            Ref::Node(node_id) => {
                let stack_ptr = qbe::Value::Temporary("stkptr".to_owned());
                builder.func.assign_instr(
                    stack_ptr.clone(),
                    qbe::Type::Long,
                    qbe::Instr::Add(
                        qbe::Value::Temporary("stack".to_owned()),
                        qbe::Value::Const(
                            (SLOT_SIZE.in_bytes() * builder.stack_slots[node_id]) as u64,
                        ),
                    ),
                );
                builder.func.assign_instr(
                    arg_reg.render(),
                    ty.render(),
                    qbe::Instr::Load(ty.render(), stack_ptr),
                );
            }
        }

        arg_reg
    }

    pub(crate) fn store(self, val: qbe::Value, builder: &mut Builder) {
        match self {
            Ref::Input(_) => {
                panic!("cannot store an input reference")
            }
            Ref::Const(_, _) => {
                panic!("cannot store a const reference")
            }
            Ref::Node(node_id) => {
                let ty = builder.graph.type_of(self);
                let stack_ptr = qbe::Value::Temporary("stkptr".to_owned());
                builder.func.assign_instr(
                    stack_ptr.clone(),
                    qbe::Type::Long,
                    qbe::Instr::Add(
                        qbe::Value::Temporary("stack".to_owned()),
                        qbe::Value::Const(
                            (SLOT_SIZE.in_bytes() * builder.stack_slots[node_id]) as u64,
                        ),
                    ),
                );
                builder
                    .func
                    .add_instr(qbe::Instr::Store(ty.render(), stack_ptr, val))
            }
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
