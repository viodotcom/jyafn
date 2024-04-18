mod check;
mod compile;
mod node;
mod serde;

pub use node::{Node, Ref, Type};

use get_size::GetSize;
use serde_derive::{Deserialize, Serialize};
use std::{
    cmp::PartialEq,
    collections::HashMap,
    error::Error as StdError,
    fmt::Debug,
    sync::atomic::{AtomicUsize, Ordering},
    sync::Arc,
};

use super::{
    layout::{Encode, Layout, RefValue, Struct, Symbols, Visitor},
    mapping,
    op::{self, Op},
    r#const::Const,
    Context, Error,
};

static GRAPH_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Default, Clone, Serialize, Deserialize, GetSize)]
pub struct Graph {
    pub(crate) name: String,
    pub(crate) metadata: HashMap<String, String>,
    pub(crate) input_layout: Struct,
    pub(crate) output_layout: Layout,
    pub(crate) inputs: Vec<Type>,
    pub(crate) nodes: Vec<Node>,
    pub(crate) outputs: Vec<Ref>,
    pub(crate) symbols: Symbols,
    pub(crate) errors: Vec<String>,
    pub(crate) mappings: HashMap<String, Arc<mapping::Mapping>>,
    pub(crate) subgraphs: Vec<Graph>,
}

impl PartialEq for Graph {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.metadata == other.metadata
            && self.input_layout == other.input_layout
            && self.output_layout == other.output_layout
            && self.inputs == other.inputs
            && self.nodes == other.nodes
            && self.outputs == other.outputs
            && self.symbols == other.symbols
            && self.errors == other.errors
            && (self.mappings.len() == other.mappings.len()
                && self.mappings.iter().all(|(k, v)| {
                    other
                        .mappings
                        .get(k)
                        .map(|other_v| Arc::ptr_eq(v, other_v))
                        .unwrap_or(false)
                }))
            && self.subgraphs == other.subgraphs
    }
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

    pub fn metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    pub fn input_layout(&self) -> &Struct {
        &self.input_layout
    }

    pub fn output_layout(&self) -> &Layout {
        &self.output_layout
    }

    pub fn metadata_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.metadata
    }

    pub fn type_of(&self, reference: Ref) -> Type {
        match reference {
            Ref::Node(node_id) => self.nodes[node_id].ty,
            Ref::Input(input_id) => self.inputs[input_id],
            Ref::Const(ty, _) => ty,
        }
    }

    pub fn r#const<C: Const>(&mut self, r#const: C) -> Ref {
        Ref::Const(r#const.annotate(), r#const.render())
    }

    pub fn insert<O: Op>(&mut self, op: O, args: Vec<Ref>) -> Result<Ref, Error> {
        let current_id = self.nodes.len();
        // Need to do this (quite inefficient way) because of borrowing.
        let error_msg = format!("initializing node for {op:?} on {args:?}");

        self.nodes
            .push(Node::init(current_id, self, op, args).with_context(|| error_msg)?);

        Ok(Ref::Node(current_id))
    }

    fn push_input(&mut self, ty: Type) -> Ref {
        let current_id = self.inputs.len();
        self.inputs.push(ty);

        Ref::Input(current_id)
    }

    fn alloc_input(&mut self, layout: &Layout) -> RefValue {
        match layout {
            Layout::Unit => RefValue::Unit,
            Layout::Scalar => RefValue::Scalar(self.push_input(Type::Float)),
            Layout::Bool => RefValue::Bool(self.push_input(Type::Bool)),
            Layout::DateTime(_) => RefValue::Bool(self.push_input(Type::DateTime)),
            Layout::Symbol => RefValue::Symbol(self.push_input(Type::Symbol)),
            Layout::Struct(fields) => RefValue::Struct(
                fields
                    .0
                    .iter()
                    .map(|(name, field)| (name.clone(), self.alloc_input(field)))
                    .collect(),
            ),
            Layout::List(element, size) => {
                RefValue::List((0..*size).map(|_| self.alloc_input(element)).collect())
            }
        }
    }

    pub fn input(&mut self, name: String, layout: Layout) -> RefValue {
        let val = self.alloc_input(&layout);
        self.input_layout.insert(name, layout);
        val
    }

    pub fn output(&mut self, value: RefValue, layout: Layout) -> Result<(), Error> {
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

    pub fn insert_mapping<S, I, K, V, E>(
        &mut self,
        name: String,
        key_layout: Layout,
        value_layout: Layout,
        storage_type: S,
        items: I,
    ) -> Result<(), E>
    where
        S: 'static + mapping::StorageType,
        K: Encode<Err = E>,
        V: Encode<Err = E>,
        E: 'static + StdError + Send,
        I: IntoIterator<Item = Result<(K, V), E>>,
    {
        let mut mapping = mapping::Mapping::new(key_layout, value_layout, storage_type)
            .expect("didn't find a good way to treat this error yet");
        let mut key_visitor = Visitor::new(mapping.key_layout().size());
        let mut value_visitor = Visitor::new(mapping.value_layout().size());

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

    pub fn mappings_mut(&mut self) -> &mut HashMap<String, Arc<mapping::Mapping>> {
        &mut self.mappings
    }

    pub fn mapping_contains(&mut self, name: &str, key: RefValue) -> Result<RefValue, Error> {
        let mapping = self
            .mappings
            .get(name)
            .ok_or_else(|| format!("no such mapping {name}"))?
            .clone();
        let Some(key_args) = key.output_vec(mapping.key_layout()) else {
            return Err(Error::BadValue {
                expected: mapping.key_layout().clone(),
                got: key,
            })
            .with_context(|| format!("getting key argument for \"contains\" on mapping {name}"));
        };

        let value_pointer = self.insert(
            op::CallMapping {
                name: name.to_string(),
            },
            key_args,
        )?;
        let not_contains = self.insert(
            op::Eq(None),
            vec![
                value_pointer,
                Ref::Const(Type::Ptr { origin: usize::MAX }, 0),
            ],
        )?;

        Ok(RefValue::Scalar(self.insert(op::Not, vec![not_contains])?))
    }

    pub fn call_mapping(&mut self, name: &str, key: RefValue) -> Result<RefValue, Error> {
        let mapping = self
            .mappings
            .get(name)
            .ok_or_else(|| format!("no such mapping {name}"))?
            .clone();
        let Some(key_args) = key.output_vec(mapping.key_layout()) else {
            return Err(Error::BadValue {
                expected: mapping.key_layout().clone(),
                got: key,
            })
            .with_context(|| format!("getting key argument for call on mapping {name}"));
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

        Ok(mapping
            .value_layout()
            .build_ref_value(values)
            .ok_or_else(|| format!("building ref-value for call on mapping of {name}"))?)
    }

    pub fn call_mapping_default(
        &mut self,
        name: &str,
        key: RefValue,
        default: RefValue,
    ) -> Result<RefValue, Error> {
        let mapping = self
            .mappings
            .get(name)
            .ok_or_else(|| format!("no such mapping {name}"))?
            .clone();
        let Some(key_args) = key.output_vec(mapping.key_layout()) else {
            return Err(Error::BadValue {
                expected: mapping.key_layout().clone(),
                got: key,
            })
            .with_context(|| format!("getting key for call-default on mapping {name}"));
        };
        let Some(default_args) = default.output_vec(mapping.value_layout()) else {
            return Err(Error::BadValue {
                expected: mapping.value_layout().clone(),
                got: default,
            })
            .with_context(|| {
                format!("getting default argument for call-default on mapping {name}")
            });
        };
        let error_code = self.push_error(format!("key error calling mapping {name}")) as u64;

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

        Ok(mapping
            .value_layout()
            .build_ref_value(values)
            .ok_or_else(|| format!("building ref-value for call with default on mapping {name}"))?)
    }

    pub fn insert_subgraph(&mut self, subgraph: Graph) -> usize {
        if let Some(exitsting) = self.subgraphs.iter().position(|g| g == &subgraph) {
            return exitsting;
        }

        let graph_id = self.subgraphs.len();
        self.subgraphs.push(subgraph);
        graph_id
    }

    pub fn call_graph(&mut self, graph_id: usize, args: RefValue) -> Result<RefValue, Error> {
        let subgraph = self
            .subgraphs
            .get(graph_id)
            .ok_or_else(|| format!("no subgraph of id {graph_id}"))?
            .clone();
        let Some(args) = args.output_vec(&Layout::Struct(subgraph.input_layout.clone())) else {
            return Err(Error::BadValue {
                expected: Layout::Struct(subgraph.input_layout.clone()),
                got: args,
            })
            .with_context(|| format!("calling subgraph {}", subgraph.name()));
        };
        let output_pointer = self.insert(op::CallGraph(graph_id), args)?;

        let values = subgraph
            .output_layout
            .slots()
            .iter()
            .enumerate()
            .map(|(id, _)| {
                self.insert(
                    op::LoadSubgraphOutput {
                        subgraph: graph_id,
                        slot: id,
                    },
                    vec![output_pointer],
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(subgraph
            .output_layout
            .build_ref_value(values)
            .ok_or_else(|| {
                format!(
                    "building ref-value for call on subgraph {}",
                    subgraph.name()
                )
            })?)
    }

    pub fn indexed_list(&mut self, list: Vec<Ref>) -> Result<IndexedList, Error> {
        let element = list
            .first()
            .map(|&f| self.type_of(f))
            .unwrap_or(Type::Float);
        let n_elements = list.len();
        let list = self.insert(
            op::List {
                element,
                n_elements,
            },
            list,
        )?;
        let error = self.push_error(format!("Index out of bounds"));

        Ok(IndexedList {
            list,
            element,
            n_elements,
            error,
        })
    }
}

#[derive(Clone)]
pub struct IndexedList {
    list: Ref,
    element: Type,
    n_elements: usize,
    error: usize,
}

impl IndexedList {
    pub fn get(&self, graph: &mut Graph, idx: Ref) -> Result<Ref, Error> {
        graph.insert(
            op::Index {
                element: self.element,
                n_elements: self.n_elements,
                error: self.error,
            },
            vec![self.list, idx],
        )
    }
}
