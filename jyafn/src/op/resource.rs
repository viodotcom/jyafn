use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};

use crate::{graph::SLOT_SIZE, impl_is_eq, impl_op, resource::ResourceMethod, Graph, Ref, Type};

use super::{unique_for, Op};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, GetSize)]
pub(crate) struct CallResource {
    pub name: String,
    pub method: String,
    #[serde(default)]
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub resolved: Option<ResourceMethod>,
}

#[typetag::serde]
impl Op for CallResource {
    impl_is_eq! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        let method = graph.resources.get(&self.name)?.get_method(&self.method)?;

        if method.input_layout.slots() == args {
            self.resolved = Some(method);
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
                    qbe::Value::Const(SLOT_SIZE.in_bytes() as u64),
                ),
            );
        }

        func.assign_instr(
            status.clone(),
            qbe::Type::Long,
            qbe::Instr::Call(
                qbe::Value::Const(method.fn_ptr.0 as *const () as u64),
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
        super::render_return_allocated_error(func, status);
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
        let call_resource = graph.nodes.get(origin)?.op.downcast_ref::<CallResource>()?;

        // Ensure return type matches.
        let call_resource_type = call_resource
            .resolved
            .as_ref()?
            .output_layout
            .slots()
            .get(self.slot)
            .copied()?;

        if call_resource_type == self.return_type {
            Some(self.return_type)
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

    fn is_illegal(&self, graph: &Graph, args: &[Ref]) -> bool {
        // If const is zero = null pointer.
        // If const not zero = hardcoding pointers?! sus...
        matches!(args[0], Ref::Const(_, _))
    }
}
