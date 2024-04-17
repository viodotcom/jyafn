//! Graph optimizations (those not covered by qbe).

use std::collections::BTreeSet;

use crate::{Graph, Node, Ref};

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
        .chain(
            // Operations that must always be used, such as assert.
            nodes
                .iter()
                .enumerate()
                .filter(|(_, node)| node.op.must_use())
                .map(|(id, _)| id),
        )
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

fn reverse(nodes: &[Node]) -> Vec<Vec<usize>> {
    let mut reversed = nodes.iter().map(|_| vec![]).collect::<Vec<_>>();
    let mut visited = nodes.iter().map(|_| false).collect::<Vec<_>>();
    let mut stack = vec![];

    while let Some(start) = visited.iter().position(|&visited| !visited) {
        stack.push(start);

        while let Some(node) = stack.pop() {
            if !visited[node] {
                visited[node] = true;
                for &arg in &nodes[node].args {
                    if let Ref::Node(arg_id) = arg {
                        reversed[arg_id].push(node);
                        stack.push(arg_id);
                    }
                }
            }
        }
    }

    reversed
}

/// This optimization is also a no-no for QBE, but here at `jyafn` we play fast and loose
/// with operation order, because side-effects are undefined behavior.
fn find_branches(
    nodes: &[Node],
    reversed: &[Vec<usize>],
    choose: usize,
) -> (BTreeSet<usize>, BTreeSet<usize>) {
    let is_accessible_later =
        |node_id: usize| reversed[node_id].iter().any(|&other| other >= choose);

    // Initial state
    let mut queue = BTreeSet::new();
    let mut true_nodes = BTreeSet::new();
    let mut false_nodes = BTreeSet::new();
    let mut false_in_queue = 0;
    let mut true_in_queue = 0;

    // Extract arguments:
    if let Ref::Node(condition) = nodes[choose].args[0] {
        queue.insert(condition);
        true_nodes.insert(condition);
        false_nodes.insert(condition);
        true_in_queue += 1;
        false_in_queue += 1;
    }
    if let Ref::Node(t_node) = nodes[choose].args[1] {
        queue.insert(t_node);
        true_nodes.insert(t_node);
        true_in_queue += 1;
    }
    if let Ref::Node(f_node) = nodes[choose].args[2] {
        queue.insert(f_node);
        false_nodes.insert(f_node);
        false_in_queue += 1;
    }

    while let Some(node_id) = queue.pop_last() {
        let args = &nodes[node_id].args;

        if true_nodes.contains(&node_id) {
            true_in_queue -= 1;
            for &arg in args {
                if let Ref::Node(arg_id) = arg {
                    if true_nodes.insert(arg_id) {
                        queue.insert(arg_id);
                        true_in_queue += 1;

                        // Sneaky: using logic op short-circuiting.
                        if is_accessible_later(arg_id) && false_nodes.insert(arg_id) {
                            false_in_queue += 1;
                        }
                    }
                }
            }
        }

        if false_nodes.contains(&node_id) {
            false_in_queue -= 1;
            for &arg in args {
                if let Ref::Node(arg_id) = arg {
                    if false_nodes.insert(arg_id) {
                        queue.insert(arg_id);
                        false_in_queue += 1;

                        // Sneaky: using logic op short-circuiting.
                        if is_accessible_later(arg_id) && true_nodes.insert(arg_id) {
                            true_in_queue += 1;
                        }
                    }
                }
            }
        }

        // If feverything is reachable from true and from false, there is no point in
        // pursuing the search further. This helps to cut the search waaay earlier in
        // case of big graphs.
        if true_in_queue == queue.len() && false_in_queue == queue.len() {
            break;
        }
    }

    (
        true_nodes.difference(&false_nodes).copied().collect(),
        false_nodes.difference(&true_nodes).copied().collect(),
    )
}

pub enum StatementOrConditional {
    Statement(usize),
    Conditional {
        node_id: usize,
        condition: Ref,
        true_side: Statements,
        false_side: Statements,
    },
}

pub struct Statements(Vec<StatementOrConditional>);

impl Statements {
    pub fn build(nodes: &[Node]) -> Statements {
        let reversed = reverse(nodes);
        let all_node_ids = (0..nodes.len()).collect::<BTreeSet<_>>();

        return do_build(all_node_ids, &reversed, nodes);

        fn do_build(
            mut node_ids: BTreeSet<usize>,
            reversed: &[Vec<usize>],
            nodes: &[Node],
        ) -> Statements {
            let mut buffer = vec![];

            while let Some(node_id) = node_ids.pop_last() {
                if nodes[node_id].op.as_any().is::<crate::op::Choose>() {
                    let condition = nodes[node_id].args[0];
                    let (true_side, false_side) = find_branches(nodes, reversed, node_id);

                    true_side.iter().chain(false_side.iter()).for_each(|n| {
                        node_ids.remove(n);
                    });

                    buffer.push(StatementOrConditional::Conditional {
                        node_id,
                        condition,
                        true_side: do_build(true_side, reversed, nodes),
                        false_side: do_build(false_side, reversed, nodes),
                    });
                } else {
                    buffer.push(StatementOrConditional::Statement(node_id));
                }
            }

            // Remember: we traversed the node in descending order. So, need to
            // disinvert...
            Statements(buffer.into_iter().rev().collect())
        }
    }

    pub fn render_into(
        &self,
        graph: &Graph,
        reachable: &[bool],
        func: &mut qbe::Function,
        namespace: &str,
    ) {
        for statement in &self.0 {
            match statement {
                &StatementOrConditional::Statement(node_id) if reachable[node_id] => {
                    let node = &graph.nodes[node_id];
                    node.op.render_into(
                        graph,
                        Ref::Node(node_id).render(),
                        &node.args,
                        func,
                        namespace,
                    )
                }
                StatementOrConditional::Conditional {
                    node_id,
                    condition,
                    true_side,
                    false_side,
                } => {
                    let output = Ref::Node(*node_id).render();
                    let node = &graph.nodes[*node_id];
                    let true_label = format!("if.true_n{node_id}");
                    let false_label = format!("if.false_n{node_id}");
                    let end_label = format!("if.end_n{node_id}");

                    func.add_instr(qbe::Instr::Jnz(
                        condition.render(),
                        true_label.clone(),
                        false_label.clone(),
                    ));

                    func.add_block(true_label);
                    true_side.render_into(graph, reachable, func, namespace);
                    func.assign_instr(
                        output.clone(),
                        node.ty.render(),
                        qbe::Instr::Copy(node.args[1].render()),
                    );
                    func.add_instr(qbe::Instr::Jmp(end_label.clone()));

                    func.add_block(false_label);
                    false_side.render_into(graph, reachable, func, namespace);
                    func.assign_instr(
                        output,
                        node.ty.render(),
                        qbe::Instr::Copy(node.args[2].render()),
                    );

                    func.add_block(end_label);
                }
                _ => {}
            }
        }
    }
}
