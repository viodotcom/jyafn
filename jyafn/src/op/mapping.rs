use byte_slice_cast::AsByteSlice;
use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};

use crate::{impl_is_eq, Graph, Ref, Type};

use super::{unique_for, Op};

/// Implements `mappgin[key]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, GetSize)]
pub(crate) struct CallMapping {
    pub name: String,
    #[serde(default)]
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub value_slots: Option<Vec<Type>>,
}

#[typetag::serde]
impl Op for CallMapping {
    impl_is_eq! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        if let Some(mapping) = graph.mappings.get(&self.name) {
            if mapping.key_layout().slots() == args {
                self.value_slots = Some(mapping.value_layout().slots());
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
        func.assign_instr(
            output.clone(),
            Type::Ptr { origin: usize::MAX }.render(),
            qbe::Instr::Call(
                qbe::Value::Global(format!("{namespace}.mapping.{}", self.name)),
                args.iter()
                    .map(|&r| (graph.type_of(r).render(), r.render()))
                    .collect(),
            ),
        );
    }

    fn get_size(&self) -> usize {
        GetSize::get_size(self)
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        let key = args
            .iter()
            .copied()
            .map(|arg| {
                if let Ref::Const(_, repr) = arg {
                    Some(repr)
                } else {
                    None
                }
            })
            .collect::<Option<Vec<_>>>()?;
        let key_ptr = if let Some(value) = graph.mappings[&self.name].get(key.as_byte_slice()) {
            value.as_ptr() as usize as u64
        } else {
            0
        };

        Some(Ref::Const(Type::Ptr { origin: usize::MAX }, key_ptr))
    }
}

/// Loads the value of a mapping call for a given slot or yields an error if none was
/// found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, GetSize)]
pub(crate) struct LoadMappingValue {
    pub mapping: String,
    pub error_code: u64,
    pub slot: usize,
}

#[typetag::serde]
impl Op for LoadMappingValue {
    impl_is_eq! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        if args.len() != 1 {
            return None;
        }

        let Type::Ptr { origin } = args[0] else {
            return None;
        };

        // Check if the origin is legit...
        let call_mapping_op = graph.nodes.get(origin)?.op.downcast_ref::<CallMapping>()?;
        if call_mapping_op.name != self.mapping {
            return None;
        }

        let mapping = graph.mappings.get(&self.mapping)?;
        let slots = mapping.value_layout().slots();

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
        let ty = graph.mappings[&self.mapping].value_layout().slots()[self.slot];
        let addr = unique_for(output.clone(), "loadmapping.addr");

        let false_side = unique_for(output.clone(), "loadmapping.found.false");
        let true_side = unique_for(output.clone(), "loadmapping.found.true");

        func.add_instr(qbe::Instr::Jnz(
            args[0].render(),
            true_side.clone(),
            false_side.clone(),
        ));
        func.add_block(false_side);
        super::render_return_error(
            func,
            qbe::Value::Global(format!("{namespace}.error.{}", self.error_code)),
        );
        func.add_block(true_side);

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

    fn get_size(&self) -> usize {
        GetSize::get_size(self)
    }

    fn is_illegal(&self, graph: &Graph, args: &[Ref]) -> bool {
        // If const is zero = value not found.
        // If const not zero = hardcoding pointers?! sus...
        matches!(args[0], Ref::Const(_, _))
    }
}

/// Loads the value of a mapping call for a given slot or yields an error if none was
/// found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, GetSize)]
pub(crate) struct LoadOrDefaultMappingValue {
    pub mapping: String,
    pub error_code: u64,
    pub slot: usize,
}

#[typetag::serde]
impl Op for LoadOrDefaultMappingValue {
    impl_is_eq! {}

    fn annotate(&mut self, self_id: usize, graph: &Graph, args: &[Type]) -> Option<Type> {
        if args.len() != 2 {
            return None;
        }

        let Type::Ptr { origin } = args[0] else {
            return None;
        };

        // Check if the origin is legit...
        let call_mapping_op = graph.nodes.get(origin)?.op.downcast_ref::<CallMapping>()?;
        if call_mapping_op.name != self.mapping {
            return None;
        }

        let slots = call_mapping_op.value_slots.as_ref()?;

        let slot = *slots.get(self.slot)?;
        if slot == args[1] {
            Some(slot)
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
        let ty = graph.mappings[&self.mapping].value_layout().slots()[self.slot];
        let addr = unique_for(output.clone(), "loadmappingdefault.addr");

        let false_side = unique_for(output.clone(), "loadmappingdefault.found.false");
        let true_side = unique_for(output.clone(), "loadmappingdefault.found.true");
        let end_if = unique_for(output.clone(), "loadmappingdefault.found.end");

        func.add_instr(qbe::Instr::Jnz(
            args[0].render(),
            true_side.clone(),
            false_side.clone(),
        ));

        func.add_block(false_side);
        func.assign_instr(
            output.clone(),
            ty.render(),
            qbe::Instr::Copy(args[1].render()),
        );
        func.add_instr(qbe::Instr::Jmp(end_if.clone()));

        func.add_block(true_side);
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

        func.add_block(end_if);
    }

    fn get_size(&self) -> usize {
        GetSize::get_size(self)
    }

    fn const_eval(&self, graph: &Graph, args: &[Ref]) -> Option<Ref> {
        if matches!(args[0], Ref::Const(_, 0)) {
            Some(args[1])
        } else {
            None
        }
    }

    fn is_illegal(&self, graph: &Graph, args: &[Ref]) -> bool {
        if let Ref::Const(_, ptr) = args[0] {
            // If const not zero = hardcoding pointers?! sus...
            if ptr != 0 {
                return true;
            }
        }

        false
    }
}
