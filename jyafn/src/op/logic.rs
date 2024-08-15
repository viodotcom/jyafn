use std::{borrow::Cow, ffi::CStr};

use serde_derive::{Deserialize, Serialize};

use crate::{graph::Builder, impl_op, Graph, Ref, Type};

use super::{unique_for, Op};

/// Implements an assertion. If the input is `false`, this operation will raise a runtime
/// error.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Assert(pub u64);

#[typetag::serde]
impl Op for Assert {
    impl_op! {}
    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Bool] => Type::Bool,
            _ => return None,
        })
    }

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let false_side = unique_for(output, "assert.if.false");
        let true_side = unique_for(output, "assert.if.true");

        let test = args[0].load(0, builder);
        builder.func.add_instr(qbe::Instr::Jnz(
            test.render(),
            true_side.clone(),
            false_side.clone(),
        ));
        builder.func.add_block(false_side);
        super::render_return_error(
            builder.func,
            qbe::Value::Global(format!("{}.error.{}", builder.namespace, self.0)),
        );
        builder.func.add_block(true_side);
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if let Some(true) = args[0].as_bool() {
            Some(Ref::from(true))
        } else {
            None
        }
    }

    fn must_use(&self) -> bool {
        true
    }

    fn is_illegal(&self, graph: &Graph, args: &[Ref]) -> bool {
        matches!(args[0].as_bool(), Some(false))
    }

    fn eval(&self, graph: &Graph, args: &[u64]) -> Result<u64, Cow<'static, CStr>> {
        if args[0] == 0 {
            Err(crate::utils::make_safe_c_str(graph.errors()[self.0 as usize].clone()).into())
        } else {
            Ok(0)
        }
    }
}

/// The ternary operator. This implements `if a then b else c`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Choose;

#[typetag::serde]
impl Op for Choose {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Bool, a, b] if a == b => *a,
            _ => return None,
        })
    }

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let true_side = unique_for(output.clone(), "choose.if.true");
        let false_side = unique_for(output.clone(), "choose.if.false");
        let end_side = unique_for(output.clone(), "choose.if.end");

        let test = args[0].load(0, builder);
        builder.func.add_instr(qbe::Instr::Jnz(
            test.render(),
            true_side.clone(),
            false_side.clone(),
        ));

        builder.func.add_block(true_side);
        let loaded = args[1].load(0, builder);
        output.store(loaded.render(), builder);
        builder.func.add_instr(qbe::Instr::Jmp(end_side.clone()));

        builder.func.add_block(false_side);
        let loaded = args[2].load(0, builder);
        output.store(loaded.render(), builder);

        builder.func.add_block(end_side);
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
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

    fn eval(&self, graph: &Graph, args: &[u64]) -> Result<u64, Cow<'static, CStr>> {
        Ok(if args[0] != 0 { args[1] } else { args[2] })
    }
}

/// Implements `!a`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Not;

#[typetag::serde]
impl Op for Not {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Bool] => Type::Bool,
            _ => return None,
        })
    }

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let x = args[0].load(0, builder);
        builder.func.assign_instr(
            x.render(),
            Type::Bool.render(),
            qbe::Instr::Xor(x.render(), qbe::Value::Const(1)),
        );
        output.store(x.render(), builder)
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if Ref::from(true) == args[0] {
            return Some(Ref::from(false));
        }

        if Ref::from(false) == args[1] {
            return Some(Ref::from(true));
        }

        None
    }
    fn eval(&self, graph: &Graph, args: &[u64]) -> Result<u64, Cow<'static, CStr>> {
        Ok((args[0] == 0) as u64)
    }
}

/// Implements `a && b`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct And;

#[typetag::serde]
impl Op for And {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Bool, Type::Bool] => Type::Bool,
            _ => return None,
        })
    }

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let x = args[0].load(0, builder);
        let y = args[1].load(1, builder);
        builder.func.assign_instr(
            x.render(),
            Type::Float.render(),
            qbe::Instr::And(x.render(), y.render()),
        );
        output.store(x.render(), builder)
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if let Some((a, b)) = args[0].as_bool().zip(args[1].as_bool()) {
            Some(Ref::from(a && b))
        } else {
            None
        }
    }

    fn eval(&self, graph: &Graph, args: &[u64]) -> Result<u64, Cow<'static, CStr>> {
        Ok((args[0] != 0 && args[1] != 0) as u64)
    }
}

/// Implements `a || b`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Or;

#[typetag::serde]
impl Op for Or {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Bool, Type::Bool] => Type::Bool,
            _ => return None,
        })
    }

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let x = args[0].load(0, builder);
        let y = args[1].load(1, builder);
        builder.func.assign_instr(
            x.render(),
            Type::Float.render(),
            qbe::Instr::Or(x.render(), y.render()),
        );
        output.store(x.render(), builder)
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if let Some((a, b)) = args[0].as_bool().zip(args[1].as_bool()) {
            Some(Ref::from(a || b))
        } else {
            None
        }
    }

    fn eval(&self, graph: &Graph, args: &[u64]) -> Result<u64, Cow<'static, CStr>> {
        Ok((args[0] != 0 || args[1] != 0) as u64)
    }
}
