use std::{borrow::Cow, ffi::CStr};

use serde_derive::{Deserialize, Serialize};

use crate::{graph::Builder, impl_op, Graph, Ref, Type};

use super::Op;

/// Converts a float to a boolean. This is equivalent to `a != 0`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToBool;

#[typetag::serde]
impl Op for ToBool {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float] => Type::Bool,
            _ => return None,
        })
    }

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let x = args[0].load(0, builder);
        builder.func.assign_instr(
            x.render(),
            Type::Bool.render(),
            qbe::Instr::Cmp(
                Type::Float.render(),
                qbe::Cmp::Ne,
                x.render(),
                qbe::Value::Const(0),
            ),
        );
        output.store(x.render(), builder)
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if let Some(x) = args[0].as_f64() {
            return Some((x != 0.0).into());
        }

        None
    }

    fn eval(&self, graph: &Graph, args: &[u64]) -> Result<u64, Cow<'static, CStr>> {
        Ok((f64::from_ne_bytes(args[0].to_ne_bytes()) != 0.0) as u64)
    }
}

/// Converts a boolean to a float. This is equivalent to `if a then 1.0 else 0.0`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToFloat;

#[typetag::serde]
impl Op for ToFloat {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Bool] => Type::Float,
            _ => return None,
        })
    }

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let x = args[0].load(0, builder);
        builder.func.assign_instr(
            x.render(),
            Type::Float.render(),
            qbe::Instr::Ultof(x.render()),
        );
        output.store(x.render(), builder)
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if let Some(x) = args[0].as_bool() {
            return Some((x as i64 as f64).into());
        }

        None
    }

    fn eval(&self, graph: &Graph, args: &[u64]) -> Result<u64, Cow<'static, CStr>> {
        Ok(u64::from_ne_bytes(
            ((args[0] != 0) as u64 as f64).to_ne_bytes(),
        ))
    }
}
