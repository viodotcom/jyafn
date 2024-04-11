use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};

use crate::{pfunc, Graph, Ref, Type};

use super::Op;

#[derive(Debug, Serialize, Deserialize)]
pub struct Call(pub String);

#[typetag::serde]
impl Op for Call {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
        let pfunc = pfunc::get(&self.0)?;
        if pfunc.signature() == args {
            Some(pfunc.returns())
        } else {
            None
        }
    }

    fn render_into(
        &self,
        graph: &Graph,
        output: qbe::Value,
        args: &[Ref],
        func: &mut qbe::Function,
    ) {
        let pfunc = pfunc::get(&self.0).expect("pfunc existence already checked");
        func.assign_instr(
            output,
            pfunc.returns().render(),
            qbe::Instr::Call(
                qbe::Value::Const(pfunc.location() as u64),
                pfunc
                    .signature()
                    .iter()
                    .zip(args)
                    .map(|(ty, arg)| (ty.render(), arg.render()))
                    .collect(),
            ),
        )
    }

    fn const_eval(&self, args: &[Ref]) -> Option<Ref> {
        let pfunc = pfunc::get(&self.0).expect("pfunc existence already checked");
        let const_args = args
            .iter()
            .copied()
            .map(Ref::as_f64)
            .collect::<Option<Vec<_>>>()?;
        (pfunc.const_eval.0)(&const_args).map(|v| v.into())
    }

    fn get_size(&self) -> usize {
        self.0.get_size()
    }
}
