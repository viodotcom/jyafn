mod node;

pub use node::{Node, Ref, Type};

use serde_derive::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    error::Error as StdError,
    fmt::Debug,
    sync::atomic::{AtomicUsize, Ordering},
    sync::Arc,
};

use super::{
    layout, mapping,
    op::{self, Op},
    r#const::Const,
    Error,
};

const GRAPH_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Graph {
    pub(crate) name: String,
    pub(crate) metadata: HashMap<String, String>,
    pub(crate) input_layout: layout::Struct,
    pub(crate) output_layout: layout::Layout,
    pub(crate) inputs: Vec<Type>,
    pub(crate) nodes: Vec<Node>,
    pub(crate) outputs: Vec<Ref>,
    pub(crate) symbols: layout::Symbols,
    pub(crate) errors: Vec<String>,
    pub(crate) mappings: HashMap<String, Arc<mapping::Mapping>>,
}

impl Graph {
    pub fn new_with_name(name: String) -> Graph {
        Graph {
            name,
            ..Default::default()
        }
    }

    pub fn new() -> Graph {
        let graph_id = GRAPH_ID.fetch_add(1, Ordering::Relaxed);
        Graph::new_with_name(format!("g{graph_id}"))
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn metadata(&mut self) -> &HashMap<String, String> {
        &self.metadata
    }

    pub fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.metadata
    }

    pub fn dump(&self) -> Vec<u8> {
        bincode::serialize(self).expect("can always serialize")
    }

    pub fn load(bytes: &[u8]) -> Result<Self, Error> {
        bincode::deserialize(bytes).map_err(Error::Deserialization)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).expect("can always serialize")
    }

    pub fn from_json(json: &str) -> Result<Self, Error> {
        serde_json::from_str(json).map_err(Error::JsonDeserialization)
    }

    pub fn type_of(&self, reference: Ref) -> Type {
        match reference {
            Ref::Node(node_id) => self.nodes[node_id].ty,
            Ref::Input(input_id) => self.inputs[input_id],
            Ref::Const(ty, _) => ty,
        }
    }

    pub fn r#const<C: Const>(&mut self, r#const: C) -> Ref {
        Ref::Const(r#const.annotate(), r#const.render().into())
    }

    pub fn insert<O: Op>(&mut self, op: O, args: Vec<Ref>) -> Result<Ref, Error> {
        let current_id = self.nodes.len();
        self.nodes.push(Node::init(&self, op, args)?);

        Ok(Ref::Node(current_id))
    }

    fn push_input(&mut self, ty: Type) -> Ref {
        let current_id = self.inputs.len();
        self.inputs.push(ty);

        Ref::Input(current_id)
    }

    fn alloc_input(&mut self, layout: &layout::Layout) -> layout::RefValue {
        match layout {
            layout::Layout::Unit => layout::RefValue::Unit,
            layout::Layout::Scalar => layout::RefValue::Scalar(self.push_input(Type::Float)),
            layout::Layout::Bool => layout::RefValue::Bool(self.push_input(Type::Bool)),
            layout::Layout::Struct(fields) => layout::RefValue::Struct(
                fields
                    .0
                    .iter()
                    .map(|(name, field)| (name.clone(), self.alloc_input(field)))
                    .collect(),
            ),
            layout::Layout::Symbol => layout::RefValue::Symbol(self.push_input(Type::Symbol)),
            layout::Layout::List(element, size) => {
                layout::RefValue::List((0..*size).map(|_| self.alloc_input(element)).collect())
            }
        }
    }

    pub fn scalar_input(&mut self, name: String) -> Ref {
        self.input_layout.insert(name, layout::Layout::Scalar);
        self.push_input(Type::Float)
    }

    pub fn vec_input(&mut self, name: String, size: usize) -> Vec<Ref> {
        self.input_layout.insert(
            name,
            layout::Layout::List(Box::new(layout::Layout::Scalar), size),
        );
        (0..size).map(|_| self.push_input(Type::Float)).collect()
    }

    pub fn symbol_input(&mut self, name: String) -> Ref {
        self.input_layout.insert(name, layout::Layout::Symbol);
        self.push_input(Type::Symbol)
    }

    pub fn input(&mut self, name: String, layout: layout::Layout) -> layout::RefValue {
        let val = self.alloc_input(&layout);
        self.input_layout.insert(name, layout);
        val
    }

    pub fn output(&mut self, value: layout::RefValue, layout: layout::Layout) -> Result<(), Error> {
        self.outputs = value.output_vec(&layout).ok_or_else(|| Error::BadValue {
            expected: layout.clone(),
            got: value,
        })?;
        self.output_layout = layout;
        Ok(())
    }

    fn push_error(&mut self, error: String) -> usize {
        if let Some(error_id) = self.errors.iter().position(|e| e == &error) {
            error_id
        } else {
            let error_id = self.errors.len();
            self.errors.push(error);
            error_id
        }
    }

    pub fn assert(&mut self, test: Ref, error_msg: String) -> Result<Ref, Error> {
        let error_id = self.push_error(error_msg);
        self.insert(op::Assert(error_id as u64), vec![test])
    }

    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    pub fn push_symbol(&mut self, name: String) -> Ref {
        Ref::Const(Type::Symbol, self.symbols.push(name) as u64)
    }

    pub fn symbols(&self) -> &[String] {
        self.symbols.as_ref()
    }

    pub fn insert_mapping<I, K, V, E>(
        &mut self,
        name: String,
        key_layout: layout::Layout,
        value_layout: layout::Layout,
        items: I,
    ) -> Result<(), E>
    where
        K: layout::Encode<Err = E>,
        V: layout::Encode<Err = E>,
        E: 'static + StdError + Send,
        I: IntoIterator<Item = Result<(K, V), E>>,
    {
        let mut mapping = mapping::Mapping::new(key_layout, value_layout);
        let mut key_visitor = layout::Visitor::new(mapping.key_layout().size());
        let mut value_visitor = layout::Visitor::new(mapping.value_layout().size());

        for item in items {
            let (key, value) = item?;
            key_visitor.reset();
            key.visit(mapping.key_layout(), &mut self.symbols, &mut key_visitor)?;
            value_visitor.reset();
            value.visit(
                mapping.value_layout(),
                &mut self.symbols,
                &mut value_visitor,
            )?;

            mapping.insert(
                key_visitor.clone().into_inner(),
                value_visitor.clone().into_inner(),
            );
        }

        self.mappings.insert(name, Arc::new(mapping));

        Ok(())
    }

    pub fn mappings(&self) -> &HashMap<String, Arc<mapping::Mapping>> {
        &self.mappings
    }

    pub fn mapping_contains(
        &mut self,
        name: &str,
        key: layout::RefValue,
    ) -> Result<layout::RefValue, Error> {
        let mapping = self.mappings.get(name).unwrap().clone();
        let Some(key_args) = key.output_vec(mapping.key_layout()) else {
            return Err(Error::BadValue {
                expected: mapping.key_layout().clone(),
                got: key,
            });
        };

        let value_pointer = self.insert(
            op::CallMapping {
                name: name.to_string(),
            },
            key_args,
        )?;
        let not_contains =
            self.insert(op::Eq(None), vec![value_pointer, Ref::Const(Type::Int, 0)])?;

        Ok(layout::RefValue::Scalar(
            self.insert(op::Not, vec![not_contains])?,
        ))
    }

    pub fn call_mapping(
        &mut self,
        name: &str,
        key: layout::RefValue,
    ) -> Result<layout::RefValue, Error> {
        let mapping = self.mappings.get(name).unwrap().clone();
        let Some(key_args) = key.output_vec(mapping.key_layout()) else {
            return Err(Error::BadValue {
                expected: mapping.key_layout().clone(),
                got: key,
            });
        };
        let error_code = self.push_error(format!("Key error calling mapping {name}")) as u64;

        let value_pointer = self.insert(
            op::CallMapping {
                name: name.to_string(),
            },
            key_args,
        )?;

        let values = mapping
            .value_layout()
            .slots()
            .iter()
            .enumerate()
            .map(|(id, _)| {
                self.insert(
                    op::LoadMappingValue {
                        mapping: name.to_string(),
                        error_code,
                        slot: id,
                    },
                    vec![value_pointer],
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(mapping.value_layout().build_ref_value(values).unwrap())
    }

    pub fn call_mapping_default(
        &mut self,
        name: &str,
        key: layout::RefValue,
        default: layout::RefValue,
    ) -> Result<layout::RefValue, Error> {
        let mapping = self.mappings.get(name).unwrap().clone();
        let Some(key_args) = key.output_vec(mapping.key_layout()) else {
            return Err(Error::BadValue {
                expected: mapping.key_layout().clone(),
                got: key,
            });
        };
        let Some(default_args) = default.output_vec(mapping.value_layout()) else {
            return Err(Error::BadValue {
                expected: mapping.value_layout().clone(),
                got: default,
            });
        };
        let error_code = self.push_error(format!("Key error calling mapping {name}")) as u64;

        let value_pointer = self.insert(
            op::CallMapping {
                name: name.to_string(),
            },
            key_args,
        )?;

        let values = mapping
            .value_layout()
            .slots()
            .iter()
            .zip(default_args)
            .enumerate()
            .map(|(id, (_, default_arg))| {
                self.insert(
                    op::LoadOrDefaultMappingValue {
                        mapping: name.to_string(),
                        error_code,
                        slot: id,
                    },
                    vec![value_pointer, default_arg],
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(mapping.value_layout().build_ref_value(values).unwrap())
    }
}
