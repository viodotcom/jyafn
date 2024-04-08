use serde_derive::{Deserialize, Serialize};

use crate::{Graph, Ref, Type};

use super::Op;

#[derive(Debug, Serialize, Deserialize)]
pub struct ToBool;

#[typetag::serde]
impl Op for ToBool {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
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
    ) {
        func.assign_instr(
            output,
            Type::Bool.render(),
            qbe::Instr::Cmp(
                Type::Float.render(),
                qbe::Cmp::Eq,
                args[0].render(),
                qbe::Value::Const(0),
            ),
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToFloat;

#[typetag::serde]
impl Op for ToFloat {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
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
    ) {
        func.assign_instr(
            output,
            Type::Bool.render(),
            qbe::Instr::Ultof(args[0].render()),
        )
    }
}
