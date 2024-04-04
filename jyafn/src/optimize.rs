//! Graph optimizations (those not covered by qbe).

use super::{Graph, Node, Ref};

/// Even though QBE can make a good job of finding unused data, sometimes it cannot
/// optimize everything out. One example are pfuncs. Since, fot QBE, the call might as
/// well result in something somewhere being mutated, it never optimizes a call away. We,
/// however know that pfuncs are immutable and can get rid of them.
pub fn find_reachable(outputs: &[Ref], nodes: &[Node]) -> Vec<bool> {
    let mut stack = outputs
        .iter()
        .filter_map(|r| {
            if let &Ref::Node(node_id) = r {
                Some(node_id)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let mut reachable = vec![false; nodes.len()];

    while let Some(node_id) = stack.pop() {
        if !reachable[node_id] {
            reachable[node_id] = true;
            for &arg in &nodes[node_id].args {
                if let Ref::Node(other_node_id) = arg {
                    stack.push(other_node_id);
                }
            }
        }
    }

    reachable
}

pub fn const_eval(graph: &mut Graph) {
    let mut visited = vec![false; graph.nodes.len()];

    fn search(graph: &mut Graph, visited: &mut [bool], node_id: usize) -> Ref {
        visited[node_id] = true;
        let mut new_args = graph.nodes[node_id].args.clone();

        for r#ref in &mut new_args {
            if let Ref::Node(other) = *r#ref {
                if !visited[other] {
                    *r#ref = search(graph, visited, other);
                }
            }
        }

        let node = &mut graph.nodes[node_id];
        node.args = new_args;

        if let Some(evald) = node.op.const_eval(&node.args) {
            evald
        } else {
            Ref::Node(node_id)
        }
    }

    let mut new_outputs = graph.outputs.clone();

    for output in &mut new_outputs {
        if let Ref::Node(node_id) = *output {
            *output = search(graph, &mut visited, node_id);
        }
    }

    graph.outputs = new_outputs;
}
