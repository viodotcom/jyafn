use serde_derive::{Deserialize, Serialize};

use crate::{impl_op, Graph, Ref, Type};

use super::{unique_for, Op};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct List {
    pub element: Type,
    pub n_elements: usize,
}

#[typetag::serde]
impl Op for List {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        if args.len() == self.n_elements && args.iter().all(|&arg| arg == self.element) {
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
        let data_ptr = qbe::Value::Temporary(unique_for(output.clone(), "list.data_ptr"));
        func.assign_instr(
            output.clone(),
            qbe::Type::Long,
            qbe::Instr::Alloc8((self.element.size() * self.n_elements) as u64),
        );

        for arg in args {
            func.add_instr(qbe::Instr::Store(
                self.element.render(),
                data_ptr.clone(),
                arg.render(),
            ));
            func.assign_instr(
                data_ptr.clone(),
                qbe::Type::Long,
                qbe::Instr::Add(data_ptr.clone(), qbe::Value::Const(self.element.size() as u64)),
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Index {
    element: Type,
}

#[typetag::serde]
impl Op for Index {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        if args.len() != 2 && args[1] != Type::Float {
            return None;
        }

        let Type::Ptr { origin } = args[0] else {
            return None;
        };

        let origin_op = graph.nodes.get(origin)?.op.downcast_ref::<List>()?;
        if origin_op.element != self.element {
            return None;
        }

        Some(self.element)
    }

    fn render_into(
        &self,
        graph: &Graph,
        output: qbe::Value,
        args: &[Ref],
        func: &mut qbe::Function,
        namespace: &str,
    ) {
    }
}
