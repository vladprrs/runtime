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
    computations: &ComputationsDefinition,
) -> Result<MaterializedDag, Vec<SdgError>> {
    // If no nodes and no edges, return empty DAG.
    if computations.nodes.is_empty() && computations.edges.is_empty() {
        return Ok(MaterializedDag {
            graph: DiGraph::new(),
            topo_order: Vec::new(),
            node_map: HashMap::new(),
        });
    }

    let mut graph = DiGraph::new();
    let mut node_map = HashMap::new();
    let mut errors = Vec::new();

    // Add nodes.
    for node in &computations.nodes {
        let idx = graph.add_node(node.id.clone());
        node_map.insert(node.id.clone(), idx);
    }

    // Add edges with validation.
    for edge in &computations.edges {
        let src = node_map.get(&edge.from);
        let dst = node_map.get(&edge.to);
        match (src, dst) {
            (Some(&src_idx), Some(&dst_idx)) => {
                graph.add_edge(src_idx, dst_idx, edge.port.clone());
            }
            _ => {
                errors.push(SdgError::DagEdgeReference {
                    from: edge.from.clone(),
                    to: edge.to.clone(),
                });
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // `toposort` detects cycles and returns topological order in one pass.
    match toposort(&graph, None) {
        Ok(order) => Ok(MaterializedDag {
            graph,
            topo_order: order,
            node_map,
        }),
        Err(cycle) => {
            let cycle_node = graph[cycle.node_id()].clone();
            Err(vec![SdgError::DagCycle { node: cycle_node }])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        Aggregate, ApiConfig, ComputationNode, ComputationsDefinition, Edge, FieldDefinition,
        ModelDefinition, ServiceDefinition, ServiceInfo,
    };
    use std::collections::HashMap;

    /// Helper: build a minimal `ServiceDefinition` from nodes and edges.
    fn make_definition(
        nodes: Vec<ComputationNode>,
        edges: Vec<Edge>,
    ) -> ServiceDefinition {
        let mut fields = HashMap::new();
        fields.insert(
            "title".to_string(),
            FieldDefinition {
                field_type: "string".to_string(),
                required: false,
                default: None,
                min: None,
                max: None,
                min_length: None,
                max_length: None,
                pattern: None,
                format: None,
                references: None,
                description: None,
            },
        );
        let mut aggregates = HashMap::new();
        aggregates.insert(
            "Thing".to_string(),
            Aggregate {
                fields,
                states: vec!["Active".to_string()],
                initial_state: None,
                transitions: HashMap::new(),
            },
        );
        ServiceDefinition {
            schema_version: "2.0.0".to_string(),
            service: ServiceInfo {
                name: "test".to_string(),
                description: String::new(),
                owner: "test".to_string(),
            },
            model: ModelDefinition { aggregates },
            computations: ComputationsDefinition { nodes, edges },
            api: ApiConfig::default(),
        }
    }

    /// Helper: build a simple definition from (id, type) nodes and (from, to, port) edges.
    fn make_simple_definition(
        nodes: Vec<(&str, &str)>,
        edges: Vec<(&str, &str, &str)>,
    ) -> ServiceDefinition {
        let comp_nodes: Vec<ComputationNode> = nodes
            .into_iter()
            .map(|(id, nt)| ComputationNode {
                id: id.to_string(),
                node_type: nt.to_string(),
                params: serde_json::Map::new(),
            })
            .collect();
        let comp_edges: Vec<Edge> = edges
            .into_iter()
            .map(|(from, to, port)| Edge {
                from: from.to_string(),
                to: to.to_string(),
                port: port.to_string(),
                index: None,
            })
            .collect();
        make_definition(comp_nodes, comp_edges)
    }

    fn make_node(id: &str, node_type: &str, params: serde_json::Map<String, serde_json::Value>) -> ComputationNode {
        ComputationNode {
            id: id.to_string(),
            node_type: node_type.to_string(),
            params,
        }
    }

    fn make_edge(from: &str, to: &str, port: &str, index: Option<u32>) -> Edge {
        Edge {
            from: from.to_string(),
            to: to.to_string(),
            port: port.to_string(),
            index,
        }
    }

    #[test]
    fn test_canonical_fixture_materializes() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let content = std::fs::read_to_string(format!(
            "{manifest_dir}/fixtures/valid_task_tracker.sdg.json"
        ))
        .expect("fixture must exist");
        let definition: ServiceDefinition =
            serde_json::from_str(&content).expect("fixture must deserialize");
        let result = materialize_dags(&definition);
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
        let definition: ServiceDefinition =
            serde_json::from_str(&content).expect("fixture must deserialize");
        let dag = materialize_dags(&definition).expect("should materialize");
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
        let definition: ServiceDefinition =
            serde_json::from_str(&content).expect("fixture must deserialize");
        let dag = materialize_dags(&definition).expect("should materialize");
        assert_eq!(
            dag.topo_order.len(),
            definition.computations.nodes.len(),
            "topo_order should contain all nodes"
        );
    }

    #[test]
    fn test_dag_cycle_detected() {
        let def = make_simple_definition(
            vec![("a", "literal"), ("b", "eq"), ("c", "eq")],
            vec![("a", "b", "left"), ("b", "c", "left"), ("c", "a", "left")],
        );
        let result = materialize_dags(&def);
        let errors = result.expect_err("cycle should be detected");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SdgError::DagCycle { .. })),
            "should have DagCycle error, got: {errors:?}"
        );
    }

    #[test]
    fn test_dag_edge_nonexistent_source() {
        let def = make_simple_definition(vec![("a", "eq")], vec![("nonexistent", "a", "left")]);
        let result = materialize_dags(&def);
        let errors = result.expect_err("nonexistent source should fail");
        assert!(
            errors.iter().any(
                |e| matches!(e, SdgError::DagEdgeReference { from, .. } if from == "nonexistent")
            ),
            "should have DagEdgeReference for nonexistent source, got: {errors:?}"
        );
    }

    #[test]
    fn test_dag_edge_nonexistent_target() {
        let def = make_simple_definition(vec![("a", "eq")], vec![("a", "nonexistent", "left")]);
        let result = materialize_dags(&def);
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
        let mut def = make_simple_definition(vec![], vec![]);
        def.computations = ComputationsDefinition::default();
        let result = materialize_dags(&def);
        let dag = result.expect("empty computations should produce empty DAG");
        assert_eq!(dag.graph.node_count(), 0);
        assert_eq!(dag.topo_order.len(), 0);
        assert!(dag.node_map.is_empty());
    }

    // --- Edge type-checking tests ---

    #[test]
    fn test_type_mismatch_string_to_boolean_port() {
        // literal(output_type="string") -> and(port="in") should fail: string vs boolean
        let mut str_params = serde_json::Map::new();
        str_params.insert("value".to_string(), serde_json::Value::String("hello".to_string()));
        str_params.insert("output_type".to_string(), serde_json::Value::String("string".to_string()));

        let nodes = vec![
            make_node("str_val", "literal", str_params),
            make_node("logic_and", "and", serde_json::Map::new()),
        ];
        let edges = vec![make_edge("str_val", "logic_and", "in", Some(0))];
        let def = make_definition(nodes, edges);

        let result = materialize_dags(&def);
        let errors = result.expect_err("should fail type-checking");
        assert!(
            errors.iter().any(|e| matches!(e, SdgError::TypeMismatch { .. })),
            "should have TypeMismatch error, got: {errors:?}"
        );
    }

    #[test]
    fn test_invalid_port_name_on_not_node() {
        // literal(boolean) -> not(port="input") should fail: "not" only has port "value"
        let mut bool_params = serde_json::Map::new();
        bool_params.insert("value".to_string(), serde_json::Value::Bool(true));
        bool_params.insert("output_type".to_string(), serde_json::Value::String("boolean".to_string()));

        let nodes = vec![
            make_node("val1", "literal", bool_params),
            make_node("negate", "not", serde_json::Map::new()),
        ];
        let edges = vec![make_edge("val1", "negate", "input", None)];
        let def = make_definition(nodes, edges);

        let result = materialize_dags(&def);
        let errors = result.expect_err("should fail port validation");
        assert!(
            errors.iter().any(|e| matches!(e, SdgError::DagInvalidPort { .. })),
            "should have DagInvalidPort error, got: {errors:?}"
        );
    }

    #[test]
    fn test_compatible_uuid_to_lookup_id() {
        // context(path="actor.id", output=uuid) -> lookup(port="id", expects uuid) => no error
        let mut ctx_params = serde_json::Map::new();
        ctx_params.insert("path".to_string(), serde_json::Value::String("actor.id".to_string()));
        let mut lookup_params = serde_json::Map::new();
        lookup_params.insert("aggregate".to_string(), serde_json::Value::String("Thing".to_string()));
        lookup_params.insert("pick".to_string(), serde_json::Value::String("title".to_string()));

        let nodes = vec![
            make_node("actor_id", "context", ctx_params),
            make_node("actor_lookup", "lookup", lookup_params),
        ];
        let edges = vec![make_edge("actor_id", "actor_lookup", "id", None)];
        let def = make_definition(nodes, edges);

        let result = materialize_dags(&def);
        assert!(
            result.is_ok(),
            "uuid to lookup.id should be compatible, got: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_type_mismatch_integer_to_count_items() {
        // literal(output_type="integer") -> count(port="items") should fail: integer is not array
        let mut int_params = serde_json::Map::new();
        int_params.insert("value".to_string(), serde_json::Value::from(42));
        int_params.insert("output_type".to_string(), serde_json::Value::String("integer".to_string()));

        let nodes = vec![
            make_node("int_val", "literal", int_params),
            make_node("counter", "count", serde_json::Map::new()),
        ];
        let edges = vec![make_edge("int_val", "counter", "items", None)];
        let def = make_definition(nodes, edges);

        let result = materialize_dags(&def);
        let errors = result.expect_err("should fail type-checking");
        assert!(
            errors.iter().any(|e| matches!(e, SdgError::TypeMismatch { .. })),
            "should have TypeMismatch error (integer is not an array), got: {errors:?}"
        );
    }

    #[test]
    fn test_variadic_boolean_port_accepts_boolean() {
        // literal(boolean) -> and(port="in", index=0) => no error
        let mut bool_params = serde_json::Map::new();
        bool_params.insert("value".to_string(), serde_json::Value::Bool(true));
        bool_params.insert("output_type".to_string(), serde_json::Value::String("boolean".to_string()));
        let mut bool_params2 = serde_json::Map::new();
        bool_params2.insert("value".to_string(), serde_json::Value::Bool(false));
        bool_params2.insert("output_type".to_string(), serde_json::Value::String("boolean".to_string()));

        let nodes = vec![
            make_node("b1", "literal", bool_params),
            make_node("b2", "literal", bool_params2),
            make_node("logic_and", "and", serde_json::Map::new()),
        ];
        let edges = vec![
            make_edge("b1", "logic_and", "in", Some(0)),
            make_edge("b2", "logic_and", "in", Some(1)),
        ];
        let def = make_definition(nodes, edges);

        let result = materialize_dags(&def);
        assert!(
            result.is_ok(),
            "boolean to and.in should be compatible, got: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_valid_edges_pass_type_check() {
        // literal(string) -> concat(left), literal(string) -> concat(right) => no error
        let mut str_params1 = serde_json::Map::new();
        str_params1.insert("value".to_string(), serde_json::Value::String("hello".to_string()));
        str_params1.insert("output_type".to_string(), serde_json::Value::String("string".to_string()));
        let mut str_params2 = serde_json::Map::new();
        str_params2.insert("value".to_string(), serde_json::Value::String(" world".to_string()));
        str_params2.insert("output_type".to_string(), serde_json::Value::String("string".to_string()));

        let nodes = vec![
            make_node("s1", "literal", str_params1),
            make_node("s2", "literal", str_params2),
            make_node("joined", "concat", serde_json::Map::new()),
        ];
        let edges = vec![
            make_edge("s1", "joined", "left", None),
            make_edge("s2", "joined", "right", None),
        ];
        let def = make_definition(nodes, edges);

        let result = materialize_dags(&def);
        assert!(
            result.is_ok(),
            "string to concat ports should be compatible, got: {:?}",
            result.err()
        );
    }
}
