use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};

use crate::{graph::SLOT_SIZE, impl_is_eq, impl_op, pfunc, Graph, Ref, Type};

use super::{unique_for, Op};

/// Calls a pure function, given its name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, GetSize)]
pub struct Call(pub String);

#[typetag::serde]
impl Op for Call {
    impl_is_eq! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
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
        namespace: &str,
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

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
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

/// Calls a sub-graph by its id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallGraph(pub usize);

#[typetag::serde]
impl Op for CallGraph {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        let subgraph = graph.subgraphs.get(self.0)?;
        if subgraph.inputs == args {
            Some(Type::Ptr { origin: self_id })
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
        namespace: &str,
    ) {
        let subgraph = &graph.subgraphs[self.0];
        let input_ptr = qbe::Value::Temporary(unique_for(output.clone(), "callgraph.input"));
        let output_ptr = qbe::Value::Temporary(unique_for(output.clone(), "callgraph.output"));
        let data_ptr = qbe::Value::Temporary(unique_for(output.clone(), "callgraph.data"));
        let status = qbe::Value::Temporary(unique_for(output.clone(), "callgraph.status"));
        let raise_side = unique_for(output.clone(), "callgraph.raise");
        let end_side = unique_for(output.clone(), "callgraph.end");

        func.assign_instr(
            input_ptr.clone(),
            qbe::Type::Long,
            qbe::Instr::Alloc8(
                graph.subgraphs[self.0]
                    .inputs
                    .iter()
                    .map(|ty| SLOT_SIZE.in_bytes())
                    .sum::<usize>() as u64,
            ),
        );
        func.assign_instr(
            output_ptr.clone(),
            qbe::Type::Long,
            qbe::Instr::Alloc8(graph.subgraphs[self.0].output_layout.size().in_bytes() as u64),
        );

        func.assign_instr(
            data_ptr.clone(),
            qbe::Type::Long,
            qbe::Instr::Copy(input_ptr.clone()),
        );

        for &arg in args {
            func.add_instr(qbe::Instr::Store(
                graph.type_of(arg).render(),
                data_ptr.clone(),
                arg.render(),
            ));
            func.assign_instr(
                data_ptr.clone(),
                qbe::Type::Long,
                qbe::Instr::Add(
                    data_ptr.clone(),
                    qbe::Value::Const(SLOT_SIZE.in_bytes() as u64),
                ),
            );
        }

        func.assign_instr(
            status.clone(),
            qbe::Type::Long,
            qbe::Instr::Call(
                qbe::Value::Global(format!("{namespace}.graph.{}", self.0)),
                vec![
                    (qbe::Type::Long, input_ptr),
                    (qbe::Type::Long, output_ptr.clone()),
                ],
            ),
        );

        func.add_instr(qbe::Instr::Jnz(
            status.clone(),
            raise_side.clone(),
            end_side.clone(),
        ));
        func.add_block(raise_side);
        super::render_return_error(func, status);
        func.add_block(end_side);
        func.assign_instr(output, qbe::Type::Long, qbe::Instr::Copy(output_ptr));
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LoadSubgraphOutput {
    pub(crate) subgraph: usize,
    pub(crate) slot: usize,
}

#[typetag::serde]
impl Op for LoadSubgraphOutput {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        if args.len() != 1 {
            return None;
        }

        let Type::Ptr { origin } = args[0] else {
            return None;
        };
        let origin_op = graph.nodes.get(origin)?.op.downcast_ref::<CallGraph>()?;
        if self.subgraph != origin_op.0 {
            return None;
        }

        let subgraph = graph.subgraphs.get(self.slot)?;
        let slots = subgraph.output_layout.slots();

        slots.get(self.slot).copied()
    }

    fn render_into(
        &self,
        graph: &Graph,
        output: qbe::Value,
        args: &[Ref],
        func: &mut qbe::Function,
        namespace: &str,
    ) {
        let ty = graph.subgraphs[self.subgraph].output_layout.slots()[self.slot];
        let addr = unique_for(output.clone(), "loadsubgraphoutput.addr");

        func.assign_instr(
            qbe::Value::Temporary(addr.clone()),
            qbe::Type::Long,
            qbe::Instr::Add(args[0].render(), qbe::Value::Const((self.slot * 8) as u64)),
        );
        func.assign_instr(
            output,
            ty.render(),
            qbe::Instr::Load(ty.render(), qbe::Value::Temporary(addr)),
        );
    }

    fn must_use(&self) -> bool {
        true
    }
}
