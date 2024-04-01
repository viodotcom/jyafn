use super::{pfunc, Ref, Type};

use serde_derive::{Deserialize, Serialize};
use std::fmt::Debug;

#[typetag::serde(tag = "type")]
pub trait Op: 'static + Debug + Send + Sync {
    fn annotate(&self, args: &[Type]) -> Option<Type>;
    fn render_into(&self, output: qbe::Value, args: &[Ref], func: &mut qbe::Function);
}

fn unique_for(v: qbe::Value, prefix: &str) -> String {
    let qbe::Value::Temporary(name) = v else {
        panic!("Can only get unique names for temporaries; got {v}")
    };

    format!("{prefix}_{name}")
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Add;

#[typetag::serde]
impl Op for Add {
    fn annotate(&self, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Float,
            _ => return None,
        })
    }

    fn render_into(&self, output: qbe::Value, args: &[Ref], func: &mut qbe::Function) {
        func.assign_instr(
            output,
            Type::Float.render(),
            qbe::Instr::Add(args[0].render(), args[1].render()),
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Sub;

#[typetag::serde]
impl Op for Sub {
    fn annotate(&self, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Float,
            _ => return None,
        })
    }

    fn render_into(&self, output: qbe::Value, args: &[Ref], func: &mut qbe::Function) {
        func.assign_instr(
            output,
            Type::Float.render(),
            qbe::Instr::Sub(args[0].render(), args[1].render()),
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Mul;

#[typetag::serde]
impl Op for Mul {
    fn annotate(&self, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Float,
            _ => return None,
        })
    }

    fn render_into(&self, output: qbe::Value, args: &[Ref], func: &mut qbe::Function) {
        func.assign_instr(
            output,
            Type::Float.render(),
            qbe::Instr::Mul(args[0].render(), args[1].render()),
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Div;

#[typetag::serde]
impl Op for Div {
    fn annotate(&self, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Float,
            _ => return None,
        })
    }

    fn render_into(&self, output: qbe::Value, args: &[Ref], func: &mut qbe::Function) {
        func.assign_instr(
            output,
            Type::Float.render(),
            qbe::Instr::Div(args[0].render(), args[1].render()),
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Neg;

#[typetag::serde]
impl Op for Neg {
    fn annotate(&self, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float] => Type::Float,
            _ => return None,
        })
    }

    fn render_into(&self, output: qbe::Value, args: &[Ref], func: &mut qbe::Function) {
        func.assign_instr(
            output,
            Type::Float.render(),
            qbe::Instr::Neg(args[0].render()),
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Call(pub String);

#[typetag::serde]
impl Op for Call {
    fn annotate(&self, args: &[Type]) -> Option<Type> {
        let pfunc = pfunc::get(&self.0)?;
        if pfunc.signature() == args {
            Some(pfunc.returns())
        } else {
            None
        }
    }

    fn render_into(&self, output: qbe::Value, args: &[Ref], func: &mut qbe::Function) {
        let pfunc = pfunc::get(&self.0).expect("pfunc existence already checked");
        func.assign_instr(
            output,
            pfunc.returns().render(),
            qbe::Instr::Call(
                qbe::Value::Const(pfunc.location() as u64),
                pfunc
                    .signature()
                    .into_iter()
                    .zip(args)
                    .map(|(ty, arg)| (ty.render(), arg.render()))
                    .collect(),
            ),
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Abs;

#[typetag::serde]
impl Op for Abs {
    fn annotate(&self, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float] => Type::Float,
            _ => return None,
        })
    }

    fn render_into(&self, output: qbe::Value, args: &[Ref], func: &mut qbe::Function) {
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

/// The ternary operator. Unfortunately, this a naive version where both sides of the
/// ternary are calculated. Further design optimization is needed to elliminate this grave
/// shortcomming.
#[derive(Debug, Serialize, Deserialize)]
pub struct Choose;

#[typetag::serde]
impl Op for Choose {
    fn annotate(&self, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Bool, a, b] if a == b => *a,
            _ => return None,
        })
    }

    fn render_into(&self, output: qbe::Value, args: &[Ref], func: &mut qbe::Function) {
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Not;

#[typetag::serde]
impl Op for Not {
    fn annotate(&self, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Bool] => Type::Bool,
            _ => return None,
        })
    }

    fn render_into(&self, output: qbe::Value, args: &[Ref], func: &mut qbe::Function) {
        func.assign_instr(
            output,
            Type::Bool.render(),
            qbe::Instr::Xor(args[0].render(), qbe::Value::Const(1)),
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct And;

#[typetag::serde]
impl Op for And {
    fn annotate(&self, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Bool, Type::Bool] => Type::Bool,
            _ => return None,
        })
    }

    fn render_into(&self, output: qbe::Value, args: &[Ref], func: &mut qbe::Function) {
        func.assign_instr(
            output,
            Type::Bool.render(),
            qbe::Instr::And(args[0].render(), args[1].render()),
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Or;

#[typetag::serde]
impl Op for Or {
    fn annotate(&self, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Bool, Type::Bool] => Type::Bool,
            _ => return None,
        })
    }

    fn render_into(&self, output: qbe::Value, args: &[Ref], func: &mut qbe::Function) {
        func.assign_instr(
            output,
            Type::Bool.render(),
            qbe::Instr::Or(args[0].render(), args[1].render()),
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Eq;

#[typetag::serde]
impl Op for Eq {
    fn annotate(&self, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Bool,
            _ => return None,
        })
    }

    fn render_into(&self, output: qbe::Value, args: &[Ref], func: &mut qbe::Function) {
        func.assign_instr(
            output,
            Type::Bool.render(),
            qbe::Instr::Cmp(
                Type::Float.render(),
                qbe::Cmp::Eq,
                args[0].render(),
                args[1].render(),
            ),
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Gt;

#[typetag::serde]
impl Op for Gt {
    fn annotate(&self, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Bool,
            _ => return None,
        })
    }

    fn render_into(&self, output: qbe::Value, args: &[Ref], func: &mut qbe::Function) {
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Lt;

#[typetag::serde]
impl Op for Lt {
    fn annotate(&self, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Bool,
            _ => return None,
        })
    }

    fn render_into(&self, output: qbe::Value, args: &[Ref], func: &mut qbe::Function) {
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ge;

#[typetag::serde]
impl Op for Ge {
    fn annotate(&self, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Bool,
            _ => return None,
        })
    }

    fn render_into(&self, output: qbe::Value, args: &[Ref], func: &mut qbe::Function) {
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Le;

#[typetag::serde]
impl Op for Le {
    fn annotate(&self, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float, Type::Float] => Type::Bool,
            _ => return None,
        })
    }

    fn render_into(&self, output: qbe::Value, args: &[Ref], func: &mut qbe::Function) {
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToBool;

#[typetag::serde]
impl Op for ToBool {
    fn annotate(&self, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Float] => Type::Bool,
            _ => return None,
        })
    }

    fn render_into(&self, output: qbe::Value, args: &[Ref], func: &mut qbe::Function) {
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
    fn annotate(&self, args: &[Type]) -> Option<Type> {
        Some(match args {
            [Type::Bool] => Type::Float,
            _ => return None,
        })
    }

    fn render_into(&self, output: qbe::Value, args: &[Ref], func: &mut qbe::Function) {
        func.assign_instr(
            output,
            Type::Bool.render(),
            qbe::Instr::Ultof(args[0].render()),
        )
    }
}
