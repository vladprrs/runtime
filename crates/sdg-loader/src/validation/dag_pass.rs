use std::collections::HashMap;

use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};

use crate::error::SdgError;
use crate::types::ComputationsDefinition;

/// A materialized computation DAG with pre-computed topological order.
/// Produced by Pass 4 of the validation pipeline.
#[derive(Debug)]
pub struct MaterializedDag {
    /// The directed graph. Nodes hold computation node IDs, edges hold port names.
    pub graph: DiGraph<String, String>,
    /// Pre-computed topological order for runtime evaluation.
    pub topo_order: Vec<NodeIndex>,
    /// Map from node ID string to graph `NodeIndex` for O(1) lookup.
    pub node_map: HashMap<String, NodeIndex>,
}

/// Pass 4: Materialize the computation DAG from the SDG definition.
///
/// Builds a `petgraph::DiGraph`, validates edge references, detects cycles
/// via topological sort, and produces a pre-computed evaluation order.
pub fn materialize_dags(
    _computations: &ComputationsDefinition,
) -> Result<MaterializedDag, Vec<SdgError>> {
    // TODO: implement
    let _ = _computations;
    Err(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ComputationNode, ComputationsDefinition, Edge};

    /// Helper: build `ComputationsDefinition` from nodes and edges.
    fn make_computations(nodes: Vec<(&str, &str)>, edges: Vec<(&str, &str, &str)>) -> ComputationsDefinition {
        ComputationsDefinition {
            nodes: nodes
                .into_iter()
                .map(|(id, nt)| ComputationNode {
                    id: id.to_string(),
                    node_type: nt.to_string(),
                    params: serde_json::Map::new(),
                })
                .collect(),
            edges: edges
                .into_iter()
                .map(|(from, to, port)| Edge {
                    from: from.to_string(),
                    to: to.to_string(),
                    port: port.to_string(),
                    index: None,
                })
                .collect(),
        }
    }

    #[test]
    fn test_canonical_fixture_materializes() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let content = std::fs::read_to_string(format!(
            "{manifest_dir}/fixtures/valid_task_tracker.sdg.json"
        ))
        .expect("fixture must exist");
        let definition: crate::types::ServiceDefinition =
            serde_json::from_str(&content).expect("fixture must deserialize");
        let result = materialize_dags(&definition.computations);
        assert!(
            result.is_ok(),
            "canonical fixture should materialize, got: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_dag_node_count() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let content = std::fs::read_to_string(format!(
            "{manifest_dir}/fixtures/valid_task_tracker.sdg.json"
        ))
        .expect("fixture must exist");
        let definition: crate::types::ServiceDefinition =
            serde_json::from_str(&content).expect("fixture must deserialize");
        let dag = materialize_dags(&definition.computations).expect("should materialize");
        assert_eq!(
            dag.graph.node_count(),
            definition.computations.nodes.len(),
            "graph node count should match computation nodes"
        );
    }

    #[test]
    fn test_dag_topo_order_complete() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let content = std::fs::read_to_string(format!(
            "{manifest_dir}/fixtures/valid_task_tracker.sdg.json"
        ))
        .expect("fixture must exist");
        let definition: crate::types::ServiceDefinition =
            serde_json::from_str(&content).expect("fixture must deserialize");
        let dag = materialize_dags(&definition.computations).expect("should materialize");
        assert_eq!(
            dag.topo_order.len(),
            definition.computations.nodes.len(),
            "topo_order should contain all nodes"
        );
    }

    #[test]
    fn test_dag_cycle_detected() {
        let computations = make_computations(
            vec![("a", "literal"), ("b", "eq"), ("c", "eq")],
            vec![("a", "b", "left"), ("b", "c", "left"), ("c", "a", "left")],
        );
        let result = materialize_dags(&computations);
        let errors = result.expect_err("cycle should be detected");
        assert!(
            errors.iter().any(|e| matches!(e, SdgError::DagCycle { .. })),
            "should have DagCycle error, got: {errors:?}"
        );
    }

    #[test]
    fn test_dag_edge_nonexistent_source() {
        let computations = make_computations(
            vec![("a", "eq")],
            vec![("nonexistent", "a", "left")],
        );
        let result = materialize_dags(&computations);
        let errors = result.expect_err("nonexistent source should fail");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SdgError::DagEdgeReference { from, .. } if from == "nonexistent")),
            "should have DagEdgeReference for nonexistent source, got: {errors:?}"
        );
    }

    #[test]
    fn test_dag_edge_nonexistent_target() {
        let computations = make_computations(
            vec![("a", "eq")],
            vec![("a", "nonexistent", "left")],
        );
        let result = materialize_dags(&computations);
        let errors = result.expect_err("nonexistent target should fail");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SdgError::DagEdgeReference { to, .. } if to == "nonexistent")),
            "should have DagEdgeReference for nonexistent target, got: {errors:?}"
        );
    }

    #[test]
    fn test_empty_computations() {
        let computations = ComputationsDefinition::default();
        let result = materialize_dags(&computations);
        let dag = result.expect("empty computations should produce empty DAG");
        assert_eq!(dag.graph.node_count(), 0);
        assert_eq!(dag.topo_order.len(), 0);
        assert!(dag.node_map.is_empty());
    }
}
