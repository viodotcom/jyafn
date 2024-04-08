use serde_derive::{Deserialize, Serialize};

use crate::{Graph, Ref, Type};

use super::{unique_for, Op};

#[derive(Debug, Serialize, Deserialize)]
pub struct Add;

#[typetag::serde]
impl Op for Add {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Float,
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
            Type::Float.render(),
            qbe::Instr::Add(args[0].render(), args[1].render()),
        )
    }

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        if Ref::from(0.0) == args[0] {
            return Some(args[1]);
        }

        if Ref::from(0.0) == args[1] {
            return Some(args[0]);
        }

        None
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sub;

#[typetag::serde]
impl Op for Sub {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Float,
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
            Type::Float.render(),
            qbe::Instr::Sub(args[0].render(), args[1].render()),
        )
    }

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        if let Ref::Const(Type::Float, 0) = args[1] {
            return Some(args[0]);
        }

        None
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Mul;

#[typetag::serde]
impl Op for Mul {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Float,
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
            Type::Float.render(),
            qbe::Instr::Mul(args[0].render(), args[1].render()),
        )
    }

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        if Ref::from(1.0) == args[0] {
            return Some(args[1]);
        }

        if Ref::from(1.0) == args[1] {
            return Some(args[0]);
        }

        None
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Div;

#[typetag::serde]
impl Op for Div {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Float,
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
            Type::Float.render(),
            qbe::Instr::Div(args[0].render(), args[1].render()),
        )
    }

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        if Ref::from(1.0) == args[1] {
            return Some(args[0]);
        }

        None
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rem;

#[typetag::serde]
impl Op for Rem {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Float,
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
            Type::Float.render(),
            qbe::Instr::Rem(args[0].render(), args[1].render()),
        )
    }

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        if Ref::from(1.0) == args[1] {
            return Some(args[0]);
        }

        None
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Neg;

#[typetag::serde]
impl Op for Neg {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float] => Type::Float,
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
            Type::Float.render(),
            qbe::Instr::Neg(args[0].render()),
        )
    }

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        if Ref::from(0.0) == args[0] {
            return Some(Ref::from(0.0));
        }

        None
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Abs;

#[typetag::serde]
impl Op for Abs {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float] => Type::Float,
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
        let test_temp = qbe::Value::Temporary(unique_for(output.clone(), "abs.test"));
        func.assign_instr(
            test_temp.clone(),
            qbe::Type::Byte,
            qbe::Instr::Cmp(
                Type::Float.render(),
                qbe::Cmp::Ge,
                args[0].render(),
                qbe::Value::Const(0),
            ),
        );

        let true_side = unique_for(output.clone(), "abs.if.true");
        let false_side = unique_for(output.clone(), "abs.if.false");
        let end_side = unique_for(output.clone(), "abs.if.end");

        func.add_instr(qbe::Instr::Jnz(
            test_temp,
            true_side.clone(),
            false_side.clone(),
        ));

        func.add_block(true_side);
        func.assign_instr(
            output.clone(),
            Type::Float.render(),
            qbe::Instr::Copy(args[0].render()),
        );
        func.add_instr(qbe::Instr::Jmp(end_side.clone()));

        func.add_block(false_side);
        func.assign_instr(
            output,
            Type::Float.render(),
            qbe::Instr::Neg(args[0].render()),
        );

        func.add_block(end_side);
    }
}
