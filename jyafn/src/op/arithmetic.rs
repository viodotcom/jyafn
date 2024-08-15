use std::{borrow::Cow, ffi::CStr};

use serde_derive::{Deserialize, Serialize};

use crate::{graph::Builder, impl_op, Graph, Ref, Type};

use super::Op;

/// Implements `a + b`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Add;

#[typetag::serde]
impl Op for Add {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Float,
            _ => return None,
        })
    }

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let x = args[0].load(0, builder);
        let y = args[1].load(1, builder);
        builder.func.assign_instr(
            x.render(),
            Type::Float.render(),
            qbe::Instr::Add(x.render(), y.render()),
        );
        output.store(x.render(), builder)
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if Ref::from(0.0) == args[0] {
            return Some(args[1]);
        }

        if Ref::from(0.0) == args[1] {
            return Some(args[0]);
        }

        if let Some((x, y)) = args[0].as_f64().zip(args[1].as_f64()) {
            return Some((x + y).into());
        }

        None
    }

    fn eval(&self, graph: &Graph, args: &[u64]) -> Result<u64, Cow<'static, CStr>> {
        Ok(u64::from_ne_bytes(
            (f64::from_ne_bytes(args[0].to_ne_bytes()) + f64::from_ne_bytes(args[1].to_ne_bytes()))
                .to_ne_bytes(),
        ))
    }
}

/// Implements `a - b`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sub;

#[typetag::serde]
impl Op for Sub {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Float,
            _ => return None,
        })
    }

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let x = args[0].load(0, builder);
        let y = args[1].load(1, builder);
        builder.func.assign_instr(
            x.render(),
            Type::Float.render(),
            qbe::Instr::Sub(x.render(), y.render()),
        );
        output.store(x.render(), builder)
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if let Ref::Const(Type::Float, 0) = args[1] {
            return Some(args[0]);
        }

        if let Some((x, y)) = args[0].as_f64().zip(args[1].as_f64()) {
            return Some((x - y).into());
        }

        None
    }

    fn eval(&self, graph: &Graph, args: &[u64]) -> Result<u64, Cow<'static, CStr>> {
        Ok(u64::from_ne_bytes(
            (f64::from_ne_bytes(args[0].to_ne_bytes()) - f64::from_ne_bytes(args[1].to_ne_bytes()))
                .to_ne_bytes(),
        ))
    }
}

/// Implements `a * b`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mul;

#[typetag::serde]
impl Op for Mul {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Float,
            _ => return None,
        })
    }

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let x = args[0].load(0, builder);
        let y = args[1].load(1, builder);
        builder.func.assign_instr(
            x.render(),
            Type::Float.render(),
            qbe::Instr::Mul(x.render(), y.render()),
        );
        output.store(x.render(), builder)
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if Ref::from(1.0) == args[0] {
            return Some(args[1]);
        }

        if Ref::from(1.0) == args[1] {
            return Some(args[0]);
        }

        if let Some((x, y)) = args[0].as_f64().zip(args[1].as_f64()) {
            return Some((x * y).into());
        }

        None
    }

    fn eval(&self, graph: &Graph, args: &[u64]) -> Result<u64, Cow<'static, CStr>> {
        Ok(u64::from_ne_bytes(
            (f64::from_ne_bytes(args[0].to_ne_bytes()) * f64::from_ne_bytes(args[1].to_ne_bytes()))
                .to_ne_bytes(),
        ))
    }
}

/// Implements `a / b`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Div;

#[typetag::serde]
impl Op for Div {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Float,
            _ => return None,
        })
    }

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let x = args[0].load(0, builder);
        let y = args[1].load(1, builder);
        builder.func.assign_instr(
            x.render(),
            Type::Float.render(),
            qbe::Instr::Div(x.render(), y.render()),
        );
        output.store(x.render(), builder)
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if Ref::from(1.0) == args[1] {
            return Some(args[0]);
        }

        if let Some((x, y)) = args[0].as_f64().zip(args[1].as_f64()) {
            return Some((x / y).into());
        }

        None
    }

    fn eval(&self, graph: &Graph, args: &[u64]) -> Result<u64, Cow<'static, CStr>> {
        Ok(u64::from_ne_bytes(
            (f64::from_ne_bytes(args[0].to_ne_bytes()) / f64::from_ne_bytes(args[1].to_ne_bytes()))
                .to_ne_bytes(),
        ))
    }
}

/// Implements `a % b`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rem;

#[typetag::serde]
impl Op for Rem {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Float,
            _ => return None,
        })
    }

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        // `rem` does not work for floats in QBE. So, we need to resort to pfuncs!
        super::call::Call("rem".to_string()).render_into(output, args, builder)
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if let Some((x, y)) = args[0].as_f64().zip(args[1].as_f64()) {
            return Some((x % y).into());
        }

        None
    }

    fn eval(&self, graph: &Graph, args: &[u64]) -> Result<u64, Cow<'static, CStr>> {
        Ok(u64::from_ne_bytes(
            (f64::from_ne_bytes(args[0].to_ne_bytes()) % f64::from_ne_bytes(args[1].to_ne_bytes()))
                .to_ne_bytes(),
        ))
    }
}

/// Implements `-a`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Neg;

#[typetag::serde]
impl Op for Neg {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float] => Type::Float,
            _ => return None,
        })
    }

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let x = args[0].load(0, builder);
        builder.func.assign_instr(
            x.render(),
            Type::Float.render(),
            qbe::Instr::Neg(x.render()),
        );
        output.store(x.render(), builder)
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if let Some(x) = args[0].as_f64() {
            return Some((-x).into());
        }

        None
    }

    fn eval(&self, graph: &Graph, args: &[u64]) -> Result<u64, Cow<'static, CStr>> {
        Ok(u64::from_ne_bytes(
            (-f64::from_ne_bytes(args[0].to_ne_bytes())).to_ne_bytes(),
        ))
    }
}

/// Implements `|a|`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Abs;

#[typetag::serde]
impl Op for Abs {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float] => Type::Float,
            _ => return None,
        })
    }

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        // `best to use pfuncs!
        super::call::Call("abs".to_string()).render_into(output, args, builder)
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if let Some(x) = args[0].as_f64() {
            return Some(x.abs().into());
        }

        None
    }

    fn eval(&self, graph: &Graph, args: &[u64]) -> Result<u64, Cow<'static, CStr>> {
        Ok(u64::from_ne_bytes(
            f64::from_ne_bytes(args[0].to_ne_bytes())
                .abs()
                .to_ne_bytes(),
        ))
    }
}
