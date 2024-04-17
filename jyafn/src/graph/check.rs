//! Checkings to be made every time a graph is received from an external source and might
//! be corrupted in ways that mere deserialization cannot detect, be they malicious or
//! unintentional.

use crate::Error;

use super::{Graph, Ref, Type};

/// This function mutates the graph because some checks fix the state of the graph.
pub fn run_checks(graph: &mut Graph) -> Result<(), Error> {
    topsort(graph)?;
    types(graph)?;
    pointers(graph)?;

    Ok(())
}

/// This function mutates the graph because some operations are mutated by the interence
/// of the input parameters.
fn types(graph: &mut Graph) -> Result<(), Error> {
    let checked_nodes = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(node_id, node)| {
            let mut node = node.clone();
            let arg_types = node
                .args
                .iter()
                .map(|&r| graph.type_of(r))
                .collect::<Vec<_>>();
            if let Some(ty) = node.op.annotate(node_id, &*graph, &arg_types) {
                if ty == node.ty {
                    return Ok(node);
                }
            }

            return Err(Error::Type(node.op, arg_types));
        })
        .collect::<Result<Vec<_>, _>>()?;

    graph.nodes = checked_nodes;

    Ok(())
}

/// Checks whether the nodes are ordered in topological order.
fn topsort(graph: &Graph) -> Result<(), Error> {
    for (node_id, node) in graph.nodes.iter().enumerate() {
        for arg in &node.args {
            if let &Ref::Node(arg_id) = arg {
                if arg_id >= node_id {
                    return Err(format!(
                        "graph topsort violated: node {node_id} references node {arg_id}"
                    )
                    .into());
                }
            }
        }
    }

    Ok(())
}

/// Checks that no pointers are present in the output.
fn pointers(graph: &Graph) -> Result<(), Error> {
    for &output in &graph.outputs {
        if matches!(graph.type_of(output), Type::Ptr { .. }) {
            return Err(format!("Found pointer type in output").into());
        }
    }

    Ok(())
}
