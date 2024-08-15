use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};

use crate::{
    graph::{Builder, Register, SLOT_SIZE},
    impl_is_eq, impl_op, pfunc, Graph, Ref, Type,
};

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

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let pfunc = pfunc::get(&self.0).expect("pfunc existence already checked");
        let pfunc_args = pfunc
            .signature()
            .iter()
            .zip(args)
            .enumerate()
            .map(|(i, (ty, arg))| (ty.render(), arg.load(i, builder).render()))
            .collect();
        let out_reg = Register(0);
        builder.func.assign_instr(
            out_reg.render(),
            pfunc.returns().render(),
            qbe::Instr::Call(qbe::Value::Const(pfunc.location() as u64), pfunc_args),
        );
        output.store(out_reg.render(), builder)
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

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let subgraph = &builder.graph.subgraphs[self.0];
        let input_ptr = qbe::Value::Temporary(unique_for(output, "callgraph.input"));
        let output_ptr = qbe::Value::Temporary(unique_for(output, "callgraph.output"));
        let data_ptr = qbe::Value::Temporary(unique_for(output, "callgraph.data"));
        let status = qbe::Value::Temporary(unique_for(output, "callgraph.status"));
        let raise_side = unique_for(output, "callgraph.raise");
        let end_side = unique_for(output, "callgraph.end");

        builder.func.assign_instr(
            input_ptr.clone(),
            qbe::Type::Long,
            qbe::Instr::Alloc8(
                builder.graph.subgraphs[self.0]
                    .inputs
                    .iter()
                    .map(|ty| SLOT_SIZE.in_bytes())
                    .sum::<usize>() as u64,
            ),
        );
        builder.func.assign_instr(
            output_ptr.clone(),
            qbe::Type::Long,
            qbe::Instr::Alloc8(
                builder.graph.subgraphs[self.0]
                    .output_layout
                    .size()
                    .in_bytes() as u64,
            ),
        );

        builder.func.assign_instr(
            data_ptr.clone(),
            qbe::Type::Long,
            qbe::Instr::Copy(input_ptr.clone()),
        );

        for &arg in args {
            let arg_reg = arg.load(0, builder);
            builder.func.add_instr(qbe::Instr::Store(
                builder.graph.type_of(arg).render(),
                data_ptr.clone(),
                arg_reg.render(),
            ));
            builder.func.assign_instr(
                data_ptr.clone(),
                qbe::Type::Long,
                qbe::Instr::Add(
                    data_ptr.clone(),
                    qbe::Value::Const(SLOT_SIZE.in_bytes() as u64),
                ),
            );
        }

        builder.func.assign_instr(
            status.clone(),
            qbe::Type::Long,
            qbe::Instr::Call(
                qbe::Value::Global(format!("{}.graph.{}", builder.namespace, self.0)),
                vec![
                    (qbe::Type::Long, input_ptr),
                    (qbe::Type::Long, output_ptr.clone()),
                ],
            ),
        );

        builder.func.add_instr(qbe::Instr::Jnz(
            status.clone(),
            raise_side.clone(),
            end_side.clone(),
        ));
        builder.func.add_block(raise_side);
        super::render_return_error(builder.func, status);
        builder.func.add_block(end_side);
        output.store(output_ptr, builder);
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

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let ty = builder.graph.subgraphs[self.subgraph].output_layout.slots()[self.slot];
        let arg_reg = args[0].load(0, builder);
        builder.func.assign_instr(
            arg_reg.render(),
            qbe::Type::Long,
            qbe::Instr::Add(arg_reg.render(), qbe::Value::Const((self.slot * 8) as u64)),
        );
        builder.func.assign_instr(
            arg_reg.render(),
            ty.render(),
            qbe::Instr::Load(ty.render(), arg_reg.render()),
        );
        output.store(arg_reg.render(), builder);
    }

    fn must_use(&self) -> bool {
        true
    }
}
