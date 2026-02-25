use petgraph::algo::is_cyclic_directed;
use petgraph::graph::DiGraph;
use std::collections::HashMap;

use crate::domain::plan::ResolvedTask;
use crate::error::{Result, WiggumError};

/// Validates that the task dependency graph is a DAG (no cycles).
/// Returns topologically sorted task slugs if valid.
///
/// # Errors
///
/// Returns an error if a cycle is detected or a task references an unknown dependency.
pub fn validate_dag(tasks: &[ResolvedTask]) -> Result<Vec<String>> {
    let (graph, slug_to_node) = build_graph(tasks)?;

    if is_cyclic_directed(&graph) {
        let cycle_desc = find_cycle_description(&graph, &slug_to_node);
        return Err(WiggumError::CycleDetected(cycle_desc));
    }

    // Topological sort
    let sorted = petgraph::algo::toposort(&graph, None).map_err(|e| {
        let node = graph.node_weight(e.node_id()).copied().unwrap_or("unknown");
        WiggumError::CycleDetected(format!("cycle involves task: {node}"))
    })?;

    Ok(sorted
        .into_iter()
        .filter_map(|idx| graph.node_weight(idx).map(|s| (*s).to_string()))
        .collect())
}

/// Compute parallel task groups — tasks at the same depth level
/// that can execute concurrently. Returns groups ordered by depth.
///
/// # Errors
///
/// Returns an error if the dependency graph contains a cycle or unknown dependency.
pub fn parallel_groups(tasks: &[ResolvedTask]) -> Result<Vec<Vec<String>>> {
    if tasks.is_empty() {
        return Ok(Vec::new());
    }

    let (graph, _) = build_graph(tasks)?;

    if is_cyclic_directed(&graph) {
        let cycle_desc = find_cycle_description(&graph, &HashMap::new());
        return Err(WiggumError::CycleDetected(cycle_desc));
    }

    // Compute depth for each node: longest path from any root
    let mut depth: HashMap<petgraph::graph::NodeIndex, usize> = HashMap::new();
    let sorted = petgraph::algo::toposort(&graph, None).map_err(|e| {
        let node = graph[e.node_id()];
        WiggumError::CycleDetected(format!("cycle involves task: {node}"))
    })?;

    for &node in &sorted {
        let max_parent_depth = graph
            .neighbors_directed(node, petgraph::Direction::Incoming)
            .map(|parent| depth.get(&parent).copied().unwrap_or(0) + 1)
            .max()
            .unwrap_or(0);
        depth.insert(node, max_parent_depth);
    }

    // Group by depth
    let max_depth = depth.values().copied().max().unwrap_or(0);
    let mut groups: Vec<Vec<String>> = vec![Vec::new(); max_depth + 1];
    for &node in &sorted {
        let d = depth.get(&node).copied().unwrap_or(0);
        if let (Some(group), Some(slug)) = (groups.get_mut(d), graph.node_weight(node)) {
            group.push((*slug).to_string());
        }
    }

    Ok(groups)
}

type DagResult<'a> = (
    DiGraph<&'a str, ()>,
    HashMap<&'a str, petgraph::graph::NodeIndex>,
);

fn build_graph(tasks: &[ResolvedTask]) -> Result<DagResult<'_>> {
    let mut graph = DiGraph::<&str, ()>::new();
    let mut slug_to_node: HashMap<&str, petgraph::graph::NodeIndex> = HashMap::new();

    for task in tasks {
        let idx = graph.add_node(task.slug.as_str());
        slug_to_node.insert(task.slug.as_str(), idx);
    }

    for task in tasks {
        let Some(&task_node) = slug_to_node.get(task.slug.as_str()) else {
            continue;
        };
        for dep in &task.depends_on {
            let dep_node =
                *slug_to_node
                    .get(dep.as_str())
                    .ok_or_else(|| WiggumError::UnknownDependency {
                        referenced: dep.clone(),
                        referencing: task.slug.clone(),
                    })?;
            graph.add_edge(dep_node, task_node, ());
        }
    }

    Ok((graph, slug_to_node))
}

fn find_cycle_description(
    graph: &DiGraph<&str, ()>,
    _slug_to_node: &HashMap<&str, petgraph::graph::NodeIndex>,
) -> String {
    // Simple approach: try toposort which gives us the offending node
    match petgraph::algo::toposort(graph, None) {
        Err(cycle) => {
            let node_name = graph
                .node_weight(cycle.node_id())
                .copied()
                .unwrap_or("unknown");
            // Walk neighbors to describe the cycle
            let neighbors: Vec<&str> = graph
                .neighbors(cycle.node_id())
                .filter_map(|n| graph.node_weight(n).copied())
                .collect();
            format!(
                "{node_name} is involved in a cycle (connects to: {})",
                neighbors.join(", ")
            )
        }
        Ok(_) => "unknown cycle".to_string(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::plan::ResolvedTask;

    fn make_task(number: u32, slug: &str, depends_on: &[&str]) -> ResolvedTask {
        ResolvedTask {
            number,
            slug: slug.to_string(),
            title: format!("Task {slug}"),
            goal: format!("Goal for {slug}"),
            depends_on: depends_on.iter().map(ToString::to_string).collect(),
            hints: vec![],
            test_hints: vec![],
            must_haves: vec![],
            phase_name: "Phase 1".to_string(),
            phase_order: 1,
        }
    }

    #[test]
    fn valid_dag() {
        let tasks = vec![
            make_task(1, "scaffold", &[]),
            make_task(2, "domain", &["scaffold"]),
            make_task(3, "infra", &["domain"]),
        ];
        let result = validate_dag(&tasks);
        assert!(result.is_ok());
        let sorted = result.unwrap();
        assert_eq!(sorted, vec!["scaffold", "domain", "infra"]);
    }

    #[test]
    fn detects_cycle() {
        let tasks = vec![
            make_task(1, "a", &["c"]),
            make_task(2, "b", &["a"]),
            make_task(3, "c", &["b"]),
        ];
        let result = validate_dag(&tasks);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cycle"), "Expected cycle error, got: {err}");
    }

    #[test]
    fn no_tasks_is_valid() {
        let tasks: Vec<ResolvedTask> = vec![];
        let result = validate_dag(&tasks);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn diamond_dependency_is_valid() {
        let tasks = vec![
            make_task(1, "base", &[]),
            make_task(2, "left", &["base"]),
            make_task(3, "right", &["base"]),
            make_task(4, "top", &["left", "right"]),
        ];
        let result = validate_dag(&tasks);
        assert!(result.is_ok());
    }
}
