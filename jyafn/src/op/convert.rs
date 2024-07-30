use serde_derive::{Deserialize, Serialize};

use crate::{impl_op, Graph, Ref, Type};

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
                qbe::Cmp::Ne,
                args[0].render(),
                qbe::Value::Const(0),
            ),
        )
    }

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        if let Some(x) = args[0].as_f64() {
            return Some((x != 0.0).into());
        }

        None
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
            qbe::Instr::Ultof(args[0].render()),
        )
    }

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        if let Some(x) = args[0].as_bool() {
            return Some((x as i64 as f64).into());
        }

        None
    }
}
