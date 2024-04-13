use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};

use crate::{Graph, Ref, Type};

use super::{unique_for, Op};

#[derive(Debug, Serialize, Deserialize, GetSize)]
pub(crate) struct Assert(pub u64);

#[typetag::serde]
impl Op for Assert {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Bool] => Type::Bool,
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
        let false_side = unique_for(output.clone(), "assert.if.false");
        let true_side = unique_for(output.clone(), "assert.if.true");

        func.add_instr(qbe::Instr::Jnz(
            args[0].render(),
            true_side.clone(),
            false_side.clone(),
        ));
        func.add_block(false_side);
        // +1 because returning 0 is success.
        func.add_instr(qbe::Instr::Ret(Some(qbe::Value::Const(self.0 + 1))));
        func.add_block(true_side);
    }

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        if let Some(true) = args[0].as_bool() {
            Some(Ref::from(true))
        } else {
            None
        }
    }

    fn must_use(&self) -> bool {
        true
    }

    fn is_illegal(&self, args: &[Ref]) -> bool {
        matches!(args[0].as_bool(), Some(false))
    }

    fn get_size(&self) -> usize {
        GetSize::get_size(self)
    }
}

/// The ternary operator. Unfortunately, this a naive version where both sides of the
/// ternary are calculated. Further design optimization is needed to elliminate this grave
/// shortcomming.
#[derive(Debug, Serialize, Deserialize)]
pub struct Choose;

#[typetag::serde]
impl Op for Choose {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Bool, a, b] if a == b => *a,
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
        let true_side = unique_for(output.clone(), "choose.if.true");
        let false_side = unique_for(output.clone(), "choose.if.false");
        let end_side = unique_for(output.clone(), "choose.if.end");

        func.add_instr(qbe::Instr::Jnz(
            args[0].render(),
            true_side.clone(),
            false_side.clone(),
        ));

        func.add_block(true_side);
        func.assign_instr(
            output.clone(),
            Type::Float.render(),
            qbe::Instr::Copy(args[1].render()),
        );
        func.add_instr(qbe::Instr::Jmp(end_side.clone()));

        func.add_block(false_side);
        func.assign_instr(
            output,
            Type::Float.render(),
            qbe::Instr::Copy(args[2].render()),
        );

        func.add_block(end_side);
    }

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        if Ref::from(true) == args[0] {
            return Some(args[1]);
        }

        if Ref::from(false) == args[1] {
            return Some(args[2]);
        }

        if args[1] == args[2] {
            return Some(args[1]);
        }

        None
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Not;

#[typetag::serde]
impl Op for Not {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Bool] => Type::Bool,
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
            qbe::Instr::Xor(args[0].render(), qbe::Value::Const(1)),
        )
    }

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        if Ref::from(true) == args[0] {
            return Some(Ref::from(false));
        }

        if Ref::from(false) == args[1] {
            return Some(Ref::from(true));
        }

        None
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct And;

#[typetag::serde]
impl Op for And {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Bool, Type::Bool] => Type::Bool,
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
            qbe::Instr::And(args[0].render(), args[1].render()),
        )
    }

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        if let (Some(a), Some(b)) = (args[0].as_bool(), args[1].as_bool()) {
            Some(Ref::from(a && b))
        } else {
            None
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Or;

#[typetag::serde]
impl Op for Or {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Bool, Type::Bool] => Type::Bool,
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
            qbe::Instr::Or(args[0].render(), args[1].render()),
        )
    }

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        if let (Some(a), Some(b)) = (args[0].as_bool(), args[1].as_bool()) {
            Some(Ref::from(a || b))
        } else {
            None
        }
    }
}