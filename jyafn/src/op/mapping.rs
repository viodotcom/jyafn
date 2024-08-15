use byte_slice_cast::AsByteSlice;
use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};

use crate::{graph::Builder, impl_is_eq, Graph, Ref, Type};

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

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let out_reg = qbe::Value::Temporary(format!("returned"));
        let call_args = args
            .iter()
            .enumerate()
            .map(|(i, &r)| {
                (
                    builder.graph.type_of(r).render(),
                    r.load(i, builder).render(),
                )
            })
            .collect();
        builder.func.assign_instr(
            out_reg.clone(),
            Type::Ptr { origin: usize::MAX }.render(),
            qbe::Instr::Call(
                qbe::Value::Global(format!("{}.mapping.{}", builder.namespace, self.name)),
                call_args,
            ),
        );
        output.store(out_reg, builder)
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

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let ty = builder.graph.mappings[&self.mapping].value_layout().slots()[self.slot];
        let addr = qbe::Value::Temporary(unique_for(output, "loadmapping.addr"));
        let false_side = unique_for(output, "loadmapping.found.false");
        let true_side = unique_for(output, "loadmapping.found.true");

        let key = args[0].load(0, builder);

        builder.func.add_instr(qbe::Instr::Jnz(
            key.render(),
            true_side.clone(),
            false_side.clone(),
        ));
        builder.func.add_block(false_side);
        super::render_return_error(
            builder.func,
            qbe::Value::Global(format!("{}.error.{}", builder.namespace, self.error_code)),
        );
        builder.func.add_block(true_side);

        builder.func.assign_instr(
            addr.clone(),
            qbe::Type::Long,
            qbe::Instr::Add(key.render(), qbe::Value::Const((self.slot * 8) as u64)),
        );
        builder.func.assign_instr(
            addr.clone(),
            ty.render(),
            qbe::Instr::Load(ty.render(), addr.clone()),
        );
        output.store(addr, builder)
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

    fn render_into(&self, output: Ref, args: &[Ref], builder: &mut Builder) {
        let ty = builder.graph.mappings[&self.mapping].value_layout().slots()[self.slot];
        let addr = qbe::Value::Temporary(unique_for(output, "loadmappingdefault.addr"));
        let false_side = unique_for(output, "loadmappingdefault.found.false");
        let true_side = unique_for(output, "loadmappingdefault.found.true");
        let end_if = unique_for(output, "loadmappingdefault.found.end");

        let key = args[0].load(0, builder);

        builder.func.add_instr(qbe::Instr::Jnz(
            key.render(),
            true_side.clone(),
            false_side.clone(),
        ));

        builder.func.add_block(false_side);
        let default = args[1].load(1, builder);
        builder.func.assign_instr(
            addr.clone(),
            ty.render(),
            qbe::Instr::Copy(default.render()),
        );
        builder.func.add_instr(qbe::Instr::Jmp(end_if.clone()));

        builder.func.add_block(true_side);
        builder.func.assign_instr(
            addr.clone(),
            qbe::Type::Long,
            qbe::Instr::Add(key.render(), qbe::Value::Const((self.slot * 8) as u64)),
        );
        builder.func.assign_instr(
            addr.clone(),
            ty.render(),
            qbe::Instr::Load(ty.render(), addr.clone()),
        );
        output.store(addr, builder);

        builder.func.add_block(end_if);
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
