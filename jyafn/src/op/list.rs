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
        func.assign_instr(
            data_ptr.clone(),
            qbe::Type::Long,
            qbe::Instr::Copy(output.clone()),
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
                qbe::Instr::Add(
                    data_ptr.clone(),
                    qbe::Value::Const(self.element.size() as u64),
                ),
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Index {
    pub element: Type,
    pub n_elements: usize,
    pub error: usize,
}

#[typetag::serde]
impl Op for Index {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        if args.len() != 2 || args[1] != Type::Float {
            return None;
        }

        let Type::Ptr { origin } = args[0] else {
            return None;
        };

        let origin_op = graph.nodes.get(origin)?.op.downcast_ref::<List>()?;
        if origin_op.element != self.element || origin_op.n_elements != self.n_elements {
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
        let displacement = qbe::Value::Temporary(unique_for(output.clone(), "index.displacement"));
        let test_bounds = qbe::Value::Temporary(unique_for(output.clone(), "index.test_bounds"));
        let out_of_bounds = unique_for(output.clone(), "index.out_of_bounds");
        let in_bounds = unique_for(output.clone(), "index.in_bounds");

        func.assign_instr(
            displacement.clone(),
            qbe::Type::Long,
            qbe::Instr::Dtoui(args[1].render()),
        );
        func.assign_instr(
            test_bounds.clone(),
            qbe::Type::Long,
            qbe::Instr::Cmp(
                qbe::Type::Long,
                qbe::Cmp::Uge,
                displacement.clone(),
                qbe::Value::Const(self.n_elements as u64),
            ),
        );
        func.add_instr(qbe::Instr::Jnz(
            test_bounds,
            out_of_bounds.clone(),
            in_bounds.clone(),
        ));

        func.add_block(out_of_bounds);
        super::render_return_error(
            func,
            qbe::Value::Global(format!("{namespace}.error.{}", self.error)),
        );

        func.add_block(in_bounds);
        func.assign_instr(
            displacement.clone(),
            qbe::Type::Long,
            qbe::Instr::Mul(
                displacement.clone(),
                qbe::Value::Const(self.element.size() as u64),
            ),
        );
        func.assign_instr(
            displacement.clone(),
            qbe::Type::Long,
            qbe::Instr::Add(displacement.clone(), args[0].render()),
        );
        func.assign_instr(
            output,
            self.element.render(),
            qbe::Instr::Load(self.element.render(), displacement),
        );
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct IndexOf {
    pub element: Type,
    pub n_elements: usize,
}

#[typetag::serde]
impl Op for IndexOf {
    impl_op! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        if args.len() != 2 || args[1] != self.element {
            return None;
        }

        let Type::Ptr { origin } = args[0] else {
            return None;
        };

        let origin_op = graph.nodes.get(origin)?.op.downcast_ref::<List>()?;
        if origin_op.element != self.element || origin_op.n_elements != self.n_elements {
            return None;
        }

        Some(Type::Float)
    }

    fn render_into(
        &self,
        graph: &Graph,
        output: qbe::Value,
        args: &[Ref],
        func: &mut qbe::Function,
        namespace: &str,
    ) {
        let displacement =
            qbe::Value::Temporary(unique_for(output.clone(), "indexof.displacement"));
        let end_if = unique_for(output.clone(), "indexof.if.end");
        func.assign_instr(
            displacement.clone(),
            qbe::Type::Long,
            qbe::Instr::Copy(args[0].render()),
        );

        for i in 0..self.n_elements {
            let element =
                qbe::Value::Temporary(unique_for(output.clone(), &format!("indexof.element{i}")));
            let test =
                qbe::Value::Temporary(unique_for(output.clone(), &format!("indexof.test{i}")));
            let found = unique_for(output.clone(), "indexof.if.found");
            let next_if = unique_for(output.clone(), "indexof.if.next");

            // Compare:
            func.assign_instr(
                element.clone(),
                self.element.render(),
                qbe::Instr::Load(self.element.render(), displacement.clone()),
            );
            func.assign_instr(
                test.clone(),
                qbe::Type::Long,
                qbe::Instr::Cmp(
                    self.element.render(),
                    qbe::Cmp::Eq,
                    element.clone(),
                    args[1].render(),
                ),
            );
            func.add_instr(qbe::Instr::Jnz(test, found.clone(), next_if.clone()));

            // If equal:
            func.add_block(found);
            func.assign_instr(
                output.clone(),
                Type::Float.render(),
                qbe::Instr::Copy(Ref::from(i as f64).render()),
            );
            func.add_instr(qbe::Instr::Jmp(end_if.clone()));

            // If different:
            func.add_block(next_if);
            func.assign_instr(
                displacement.clone(),
                qbe::Type::Long,
                qbe::Instr::Add(
                    displacement.clone(),
                    qbe::Value::Const(self.element.size() as u64),
                ),
            );
        }

        func.assign_instr(
            output.clone(),
            Type::Float.render(),
            qbe::Instr::Copy(Ref::from(-1.0).render()),
        );
        func.add_block(end_if);
    }
}
