use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};

use crate::{impl_is_eq, impl_op, Graph, Ref, Type};

use super::{unique_for, Op};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, GetSize)]
pub(crate) struct CallResource {
    pub name: String,
    pub method: String,
}

#[typetag::serde]
impl Op for CallResource {
    impl_is_eq! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        if let Some(method) = graph
            .resources
            .get(&self.name)
            .and_then(|r| r.get_method(&self.method))
        {
            if method.input_layout.slots() == args {
                return Some(Type::Ptr { origin: self_id });
            }
        }

        None
    }

    fn render_into(
        &self,
        graph: &Graph,
        output: qbe::Value,
        args: &[Ref],
        func: &mut qbe::Function,
        namespace: &str,
    ) {
        let resource = &graph.resources[&self.name];
        let method = resource
            .get_method(&self.method)
            .expect("node already annotated");

        let input_ptr = qbe::Value::Temporary(unique_for(output.clone(), "callresource.input"));
        let output_ptr = qbe::Value::Temporary(unique_for(output.clone(), "callresource.output"));
        let data_ptr = qbe::Value::Temporary(unique_for(output.clone(), "callresource.data"));
        let status = qbe::Value::Temporary(unique_for(output.clone(), "callresource.status"));
        let raise_side = unique_for(output.clone(), "callresource.raise");
        let end_side = unique_for(output.clone(), "callresource.end");

        let input_size = method.input_layout.slots().len() as u64;
        let output_size = method.output_layout.slots().len() as u64;

        func.assign_instr(
            input_ptr.clone(),
            qbe::Type::Long,
            qbe::Instr::Alloc8(input_size * 8),
        );
        func.assign_instr(
            output_ptr.clone(),
            qbe::Type::Long,
            qbe::Instr::Alloc8(output_size * 8),
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
                    qbe::Value::Const(graph.type_of(arg).size() as u64),
                ),
            );
        }

        func.assign_instr(
            status.clone(),
            qbe::Type::Long,
            qbe::Instr::Call(
                qbe::Value::Const(method.fn_ptr as *const () as u64),
                vec![
                    (
                        qbe::Type::Long,
                        qbe::Value::Const(resource.get_raw_ptr() as u64),
                    ),
                    (qbe::Type::Long, input_ptr),
                    (qbe::Type::Long, qbe::Value::Const(input_size)),
                    (qbe::Type::Long, output_ptr.clone()),
                    (qbe::Type::Long, qbe::Value::Const(output_size)),
                ],
            ),
        );

        func.add_instr(qbe::Instr::Jnz(
            status.clone(),
            raise_side.clone(),
            end_side.clone(),
        ));
        func.add_block(raise_side);
        // This status is already a `*mut FnError`. So, no need to make.
        func.add_instr(qbe::Instr::Ret(Some(status)));
        func.add_block(end_side);
        func.assign_instr(output, qbe::Type::Long, qbe::Instr::Copy(output_ptr));
    }

    fn get_size(&self) -> usize {
        GetSize::get_size(self)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LoadMethodOutput {
    pub(crate) return_type: Type,
    pub(crate) slot: usize,
}

#[typetag::serde]
impl Op for LoadMethodOutput {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        if args.len() != 1 {
            return None;
        }

        let Type::Ptr { origin } = args[0] else {
            return None;
        };
        // Ensure origin call exists.
        graph.nodes.get(origin)?.op.downcast_ref::<CallResource>()?;

        // There needed to be some more strict checking here, but leave it for the future.

        Some(self.return_type)
    }

    fn render_into(
        &self,
        graph: &Graph,
        output: qbe::Value,
        args: &[Ref],
        func: &mut qbe::Function,
        namespace: &str,
    ) {
        let addr = unique_for(output.clone(), "loadmethodoutput.addr");

        func.assign_instr(
            qbe::Value::Temporary(addr.clone()),
            qbe::Type::Long,
            qbe::Instr::Add(args[0].render(), qbe::Value::Const((self.slot * 8) as u64)),
        );
        func.assign_instr(
            output,
            self.return_type.render(),
            qbe::Instr::Load(self.return_type.render(), qbe::Value::Temporary(addr)),
        );
    }

    fn must_use(&self) -> bool {
        true
    }
}
