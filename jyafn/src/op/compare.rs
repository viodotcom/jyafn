use serde_derive::{Deserialize, Serialize};

use crate::{impl_op, Graph, Ref, Type};

use super::Op;

/// Implements `a == b`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Eq(pub Option<Type>);

#[typetag::serde]
impl Op for Eq {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => {
                self.0 = Some(Type::Float);
                Type::Bool
            }
            [Type::Symbol, Type::Symbol] => {
                self.0 = Some(Type::Symbol);
                Type::Bool
            }
            [Type::Ptr { origin }, Type::Ptr { .. }] => {
                self.0 = Some(Type::Ptr { origin: *origin });
                Type::Bool
            }
            _ => return None,
        })
    }

    fn render_into(
        &self,
        graph: &Graph,
        output: qbe::Value,
        args: &[Ref],
        func: &mut qbe::Function,
        namespace: &str,
    ) {
        func.assign_instr(
            output,
            Type::Bool.render(),
            qbe::Instr::Cmp(
                self.0.expect("already annotated").render(),
                qbe::Cmp::Eq,
                args[0].render(),
                args[1].render(),
            ),
        )
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if let Some((x, y)) = args[0].as_f64().zip(args[1].as_f64()) {
            Some(Ref::from(x == y))
        } else {
            None
        }
    }
}

/// Implements `a > b`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Gt;

#[typetag::serde]
impl Op for Gt {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Bool,
            _ => return None,
        })
    }

    fn render_into(
        &self,
        graph: &Graph,
        output: qbe::Value,
        args: &[Ref],
        func: &mut qbe::Function,
        namespace: &str,
    ) {
        func.assign_instr(
            output,
            Type::Bool.render(),
            qbe::Instr::Cmp(
                Type::Float.render(),
                qbe::Cmp::Gt,
                args[0].render(),
                args[1].render(),
            ),
        )
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if let Some((x, y)) = args[0].as_f64().zip(args[1].as_f64()) {
            Some(Ref::from(x > y))
        } else {
            None
        }
    }
}

/// Implements `a < b`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lt;

#[typetag::serde]
impl Op for Lt {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Bool,
            _ => return None,
        })
    }

    fn render_into(
        &self,
        graph: &Graph,
        output: qbe::Value,
        args: &[Ref],
        func: &mut qbe::Function,
        namespace: &str,
    ) {
        func.assign_instr(
            output,
            Type::Bool.render(),
            qbe::Instr::Cmp(
                Type::Float.render(),
                qbe::Cmp::Lt,
                args[0].render(),
                args[1].render(),
            ),
        )
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if let Some((x, y)) = args[0].as_f64().zip(args[1].as_f64()) {
            Some(Ref::from(x < y))
        } else {
            None
        }
    }
}

/// Implements `a >= b`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ge;

#[typetag::serde]
impl Op for Ge {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Bool,
            _ => return None,
        })
    }

    fn render_into(
        &self,
        graph: &Graph,
        output: qbe::Value,
        args: &[Ref],
        func: &mut qbe::Function,
        namespace: &str,
    ) {
        func.assign_instr(
            output,
            Type::Bool.render(),
            qbe::Instr::Cmp(
                Type::Float.render(),
                qbe::Cmp::Ge,
                args[0].render(),
                args[1].render(),
            ),
        )
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if let Some((x, y)) = args[0].as_f64().zip(args[1].as_f64()) {
            Some(Ref::from(x >= y))
        } else {
            None
        }
    }
}

/// Implements `a <= b`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Le;

#[typetag::serde]
impl Op for Le {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Bool,
            _ => return None,
        })
    }

    fn render_into(
        &self,
        graph: &Graph,
        output: qbe::Value,
        args: &[Ref],
        func: &mut qbe::Function,
        namespace: &str,
    ) {
        func.assign_instr(
            output,
            Type::Bool.render(),
            qbe::Instr::Cmp(
                Type::Float.render(),
                qbe::Cmp::Le,
                args[0].render(),
                args[1].render(),
            ),
        )
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if let Some((x, y)) = args[0].as_f64().zip(args[1].as_f64()) {
            Some(Ref::from(x <= y))
        } else {
            None
        }
    }
}
