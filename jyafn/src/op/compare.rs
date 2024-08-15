use std::{borrow::Cow, ffi::CStr};

use serde_derive::{Deserialize, Serialize};

use crate::{graph::Builder, impl_op, Graph, Ref, Type};

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

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let x = args[0].load(0, builder);
        let y = args[1].load(1, builder);
        builder.func.assign_instr(
            x.render(),
            Type::Bool.render(),
            qbe::Instr::Cmp(
                self.0.expect("already annotated").render(),
                qbe::Cmp::Eq,
                x.render(),
                y.render(),
            ),
        );
        output.store(x.render(), builder)
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if let Some((x, y)) = args[0].as_f64().zip(args[1].as_f64()) {
            Some(Ref::from(x == y))
        } else {
            None
        }
    }

    fn eval(&self, graph: &Graph, args: &[u64]) -> Result<u64, Cow<'static, CStr>> {
        Ok((args[0] == args[1]) as u64)
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

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let x = args[0].load(0, builder);
        let y = args[1].load(1, builder);
        builder.func.assign_instr(
            x.render(),
            Type::Bool.render(),
            qbe::Instr::Cmp(Type::Float.render(), qbe::Cmp::Gt, x.render(), y.render()),
        );
        output.store(x.render(), builder)
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if let Some((x, y)) = args[0].as_f64().zip(args[1].as_f64()) {
            Some(Ref::from(x > y))
        } else {
            None
        }
    }

    fn eval(&self, graph: &Graph, args: &[u64]) -> Result<u64, Cow<'static, CStr>> {
        Ok(
            (f64::from_ne_bytes(args[0].to_ne_bytes()) > f64::from_ne_bytes(args[1].to_ne_bytes()))
                as u64,
        )
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

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let x = args[0].load(0, builder);
        let y = args[1].load(1, builder);
        builder.func.assign_instr(
            x.render(),
            Type::Bool.render(),
            qbe::Instr::Cmp(Type::Float.render(), qbe::Cmp::Lt, x.render(), y.render()),
        );
        output.store(x.render(), builder)
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if let Some((x, y)) = args[0].as_f64().zip(args[1].as_f64()) {
            Some(Ref::from(x < y))
        } else {
            None
        }
    }

    fn eval(&self, graph: &Graph, args: &[u64]) -> Result<u64, Cow<'static, CStr>> {
        Ok(
            (f64::from_ne_bytes(args[0].to_ne_bytes()) < f64::from_ne_bytes(args[1].to_ne_bytes()))
                as u64,
        )
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

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let x = args[0].load(0, builder);
        let y = args[1].load(1, builder);
        builder.func.assign_instr(
            x.render(),
            Type::Bool.render(),
            qbe::Instr::Cmp(Type::Float.render(), qbe::Cmp::Ge, x.render(), y.render()),
        );
        output.store(x.render(), builder)
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if let Some((x, y)) = args[0].as_f64().zip(args[1].as_f64()) {
            Some(Ref::from(x >= y))
        } else {
            None
        }
    }

    fn eval(&self, graph: &Graph, args: &[u64]) -> Result<u64, Cow<'static, CStr>> {
        Ok(
            (f64::from_ne_bytes(args[0].to_ne_bytes()) >= f64::from_ne_bytes(args[1].to_ne_bytes()))
                as u64,
        )
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

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let x = args[0].load(0, builder);
        let y = args[1].load(1, builder);
        builder.func.assign_instr(
            x.render(),
            Type::Bool.render(),
            qbe::Instr::Cmp(Type::Float.render(), qbe::Cmp::Le, x.render(), y.render()),
        );
        output.store(x.render(), builder)
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if let Some((x, y)) = args[0].as_f64().zip(args[1].as_f64()) {
            Some(Ref::from(x <= y))
        } else {
            None
        }
    }

    fn eval(&self, graph: &Graph, args: &[u64]) -> Result<u64, Cow<'static, CStr>> {
        Ok(
            (f64::from_ne_bytes(args[0].to_ne_bytes()) <= f64::from_ne_bytes(args[1].to_ne_bytes()))
                as u64,
        )
    }
}
