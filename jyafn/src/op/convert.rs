use serde_derive::{Deserialize, Serialize};

use crate::{impl_op, Graph, Ref, Type};

use super::Op;

/// Converts a float to a boolean. This is equivalent to `a == 1`.
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
                qbe::Cmp::Eq,
                args[0].render(),
                qbe::Value::Const(1),
            ),
        )
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
}
