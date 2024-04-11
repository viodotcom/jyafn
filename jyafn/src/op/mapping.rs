use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};

use crate::{Graph, Ref, Type};

use super::{unique_for, Op};

#[derive(Debug, Serialize, Deserialize, GetSize)]
pub(crate) struct CallMapping {
    pub name: String,
}

#[typetag::serde]
impl Op for CallMapping {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
        if let Some(mapping) = graph.mappings.get(&self.name) {
            if mapping.key_layout().slots() == args {
                return Some(Type::Ptr);
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
    ) {
        func.assign_instr(
            output.clone(),
            Type::Ptr.render(),
            qbe::Instr::Call(
                qbe::Value::Global(format!("mapping.{}", self.name)),
                args.iter()
                    .map(|&r| (graph.type_of(r).render(), r.render()))
                    .collect(),
            ),
        );
    }

    fn get_size(&self) -> usize {
        GetSize::get_size(self)
    }
}

#[derive(Debug, Serialize, Deserialize, GetSize)]
pub(crate) struct LoadMappingValue {
    pub mapping: String,
    pub error_code: u64,
    pub slot: usize,
}

#[typetag::serde]
impl Op for LoadMappingValue {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
        if args.len() != 1 || args[0] != Type::Ptr {
            return None;
        }

        if let Some(mapping) = graph.mappings.get(&self.mapping) {
            let slots = mapping.value_layout().slots();
            return slots.get(self.slot).copied();
        }

        None
    }

    fn render_into(
        &self,
        graph: &Graph,
        output: qbe::Value,
        args: &[Ref],
        func: &mut qbe::Function,
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
        // +1 because returning 0 is success.
        func.add_instr(qbe::Instr::Ret(Some(qbe::Value::Const(
            self.error_code + 1,
        ))));
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
}

#[derive(Debug, Serialize, Deserialize, GetSize)]
pub(crate) struct LoadOrDefaultMappingValue {
    pub mapping: String,
    pub error_code: u64,
    pub slot: usize,
}

#[typetag::serde]
impl Op for LoadOrDefaultMappingValue {
    fn annotate(&mut self, graph: &Graph, args: &[Type]) -> Option<Type> {
        if args.len() != 2 || args[0] != Type::Ptr {
            return None;
        }

        if let Some(mapping) = graph.mappings.get(&self.mapping) {
            let slots = mapping.value_layout().slots();
            let slot_type = slots.get(self.slot)?;
            if slot_type == &args[1] {
                return Some(args[1]);
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
}
