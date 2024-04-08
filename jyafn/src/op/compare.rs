use serde_derive::{Deserialize, Serialize};

use crate::{Graph, Ref, Type};

use super::Op;

#[derive(Debug, Serialize, Deserialize)]
pub struct Eq(pub Option<Type>);

#[typetag::serde]
impl Op for Eq {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => {
                self.0 = Some(Type::Float);
                Type::Bool
            }
            [Type::Symbol, Type::Symbol] => {
                self.0 = Some(Type::Symbol);
                Type::Bool
            }
            [Type::Int, Type::Int] => {
                self.0 = Some(Type::Int);
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

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        if let (Some(a), Some(b)) = (args[0].as_f64(), args[1].as_f64()) {
            Some(Ref::from(a == b))
        } else {
            None
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Gt;

#[typetag::serde]
impl Op for Gt {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
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

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        if let (Some(a), Some(b)) = (args[0].as_f64(), args[1].as_f64()) {
            Some(Ref::from(a > b))
        } else {
            None
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Lt;

#[typetag::serde]
impl Op for Lt {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
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

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        if let (Some(a), Some(b)) = (args[0].as_f64(), args[1].as_f64()) {
            Some(Ref::from(a < b))
        } else {
            None
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ge;

#[typetag::serde]
impl Op for Ge {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
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

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        if let (Some(a), Some(b)) = (args[0].as_f64(), args[1].as_f64()) {
            Some(Ref::from(a >= b))
        } else {
            None
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Le;

#[typetag::serde]
impl Op for Le {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
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

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        if let (Some(a), Some(b)) = (args[0].as_f64(), args[1].as_f64()) {
            Some(Ref::from(a <= b))
        } else {
            None
        }
    }
}
