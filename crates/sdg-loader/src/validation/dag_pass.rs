use std::collections::HashMap;

use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};

use crate::error::SdgError;
use crate::types::{ComputationNode, ServiceDefinition};

/// Per D-37: the 5 implicit aggregate fields that users cannot declare.
const IMPLICIT_FIELDS: &[(&str, &str)] = &[
    ("id", "uuid"),
    ("state", "string"),
    ("created_at", "datetime"),
    ("updated_at", "datetime"),
    ("version", "integer"),
];

/// Per D-39: the valid context paths available in computation DAGs.
const VALID_CONTEXT_PATHS: &[(&str, &str)] = &[
    ("actor.id", "uuid"),
    ("actor.email", "string"),
    ("actor.roles", "string[]"),
    ("timestamp", "datetime"),
    ("correlation_id", "uuid"),
];

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

/// Describes the expected type for a port on a computation node.
#[derive(Debug, Clone)]
enum PortType {
    /// Exact type match, e.g. "boolean", "uuid", "string", "integer", "number".
    Exact(&'static str),
    /// Array of a specific element type, e.g. Array("number") means "number[]".
    Array(&'static str),
    /// Any array type (T[]).
    AnyArray,
    /// Any type (generic type parameter T).
    Any,
    /// Variadic indexed port with expected type, e.g. "in" on and/or nodes.
    Variadic(&'static str),
    /// String or any array type (for the `length` node).
    StringOrArray,
}

/// Returns the valid input ports for a given node type per the spec function catalog.
/// Returns `None` for unknown node types.
fn valid_ports(node_type: &str) -> Option<Vec<(&'static str, PortType)>> {
    match node_type {
        // Leaf nodes (no input ports)
        "field" | "command" | "context" | "literal" => Some(vec![]),

        // Lookup
        "lookup" => Some(vec![("id", PortType::Exact("uuid"))]),
        "lookup_many" => Some(vec![("ids", PortType::Exact("uuid[]"))]),

        // Collection
        "filter" | "count" | "min" | "max" => Some(vec![("items", PortType::AnyArray)]),
        "sum" => Some(vec![("items", PortType::Array("number"))]),
        "any" | "all" => Some(vec![("items", PortType::Array("boolean"))]),
        "contains" => Some(vec![
            ("collection", PortType::AnyArray),
            ("item", PortType::Any),
        ]),
        "length" => Some(vec![("value", PortType::StringOrArray)]),

        // Comparison (all output boolean)
        "eq" | "neq" | "gt" | "lt" | "gte" | "lte" => {
            Some(vec![("left", PortType::Any), ("right", PortType::Any)])
        }

        // Logic
        "and" | "or" => Some(vec![("in", PortType::Variadic("boolean"))]),
        "not" => Some(vec![("value", PortType::Exact("boolean"))]),

        // Arithmetic (all output number)
        "add" | "sub" | "mul" | "div" => Some(vec![
            ("left", PortType::Exact("number")),
            ("right", PortType::Exact("number")),
        ]),

        // String
        "concat" => Some(vec![
            ("left", PortType::Exact("string")),
            ("right", PortType::Exact("string")),
        ]),
        "str_contains" => Some(vec![
            ("haystack", PortType::Exact("string")),
            ("needle", PortType::Exact("string")),
        ]),
        "str_len" => Some(vec![("value", PortType::Exact("string"))]),

        _ => None,
    }
}

/// Resolve the output type of a computation node.
/// Returns `None` if the type cannot be determined (e.g., unknown field, passthrough).
fn resolve_output_type(node: &ComputationNode, definition: &ServiceDefinition) -> Option<String> {
    match node.node_type.as_str() {
        "field" => {
            let field_name = node.params.get("name")?.as_str()?;
            // Check implicit fields first
            for &(name, ftype) in IMPLICIT_FIELDS {
                if name == field_name {
                    return Some(ftype.to_string());
                }
            }
            // Check all aggregates' fields
            for aggregate in definition.model.aggregates.values() {
                if let Some(field_def) = aggregate.fields.get(field_name) {
                    return Some(field_def.field_type.clone());
                }
            }
            None
        }
        "command" => {
            let cmd_field_name = node.params.get("name")?.as_str()?;
            // Check all transitions' command fields across all aggregates
            for aggregate in definition.model.aggregates.values() {
                for transition in aggregate.transitions.values() {
                    if let Some(cmd) = &transition.command {
                        if let Some(field_def) = cmd.fields.get(cmd_field_name) {
                            return Some(field_def.field_type.clone());
                        }
                    }
                }
            }
            None
        }
        "context" => {
            let path = node.params.get("path")?.as_str()?;
            for &(ctx_path, ctx_type) in VALID_CONTEXT_PATHS {
                if ctx_path == path {
                    return Some(ctx_type.to_string());
                }
            }
            None
        }
        "literal" => node.params.get("output_type")?.as_str().map(String::from),
        "lookup" => {
            let agg_name = node.params.get("aggregate")?.as_str()?;
            let pick = node.params.get("pick")?.as_str()?;
            // Check implicit fields first
            for &(name, ftype) in IMPLICIT_FIELDS {
                if name == pick {
                    return Some(ftype.to_string());
                }
            }
            let aggregate = definition.model.aggregates.get(agg_name)?;
            let field_def = aggregate.fields.get(pick)?;
            Some(field_def.field_type.clone())
        }
        "lookup_many" => {
            let agg_name = node.params.get("aggregate")?.as_str()?;
            let pick = node.params.get("pick")?.as_str()?;
            // Check implicit fields first
            for &(name, ftype) in IMPLICIT_FIELDS {
                if name == pick {
                    return Some(format!("{ftype}[]"));
                }
            }
            let aggregate = definition.model.aggregates.get(agg_name)?;
            let field_def = aggregate.fields.get(pick)?;
            Some(format!("{}[]", field_def.field_type))
        }
        // Fixed output types
        "count" | "str_len" | "length" => Some("integer".to_string()),
        "sum" | "add" | "sub" | "mul" | "div" => Some("number".to_string()),
        "any" | "all" | "eq" | "neq" | "gt" | "lt" | "gte" | "lte" | "and" | "or" | "not"
        | "contains" | "str_contains" => Some("boolean".to_string()),
        "concat" => Some("string".to_string()),
        // Passthrough types (filter, min, max) and unknown types
        _ => None,
    }
}

/// Check whether a source output type is compatible with a port's expected type.
fn is_type_compatible(source_type: &str, port_type: &PortType) -> bool {
    match port_type {
        PortType::Exact(expected) => {
            if source_type == *expected {
                return true;
            }
            // "integer" and "float" are subtypes of "number"
            if *expected == "number" && (source_type == "integer" || source_type == "float") {
                return true;
            }
            false
        }
        PortType::Array(expected_elem) => {
            // Source must be T[] and element type must match
            if let Some(elem) = source_type.strip_suffix("[]") {
                if elem == *expected_elem {
                    return true;
                }
                // "integer[]" and "float[]" match Array("number")
                if *expected_elem == "number" && (elem == "integer" || elem == "float") {
                    return true;
                }
                return false;
            }
            false
        }
        PortType::AnyArray => source_type.ends_with("[]"),
        PortType::Any => true,
        PortType::Variadic(expected) => {
            if source_type == *expected {
                return true;
            }
            // Same subtype rules as Exact
            if *expected == "number" && (source_type == "integer" || source_type == "float") {
                return true;
            }
            false
        }
        PortType::StringOrArray => source_type == "string" || source_type.ends_with("[]"),
    }
}

/// Format a `PortType` as a human-readable expected type string.
fn port_type_display(port_type: &PortType) -> String {
    match port_type {
        PortType::Array(t) => format!("{t}[]"),
        PortType::AnyArray => "T[] (any array)".to_string(),
        PortType::Any => "T (any)".to_string(),
        PortType::Exact(t) | PortType::Variadic(t) => (*t).to_string(),
        PortType::StringOrArray => "string or T[]".to_string(),
    }
}

/// Pass 4: Materialize the computation DAG from the SDG definition.
///
/// Builds a `petgraph::DiGraph`, validates edge references, detects cycles
/// via topological sort, type-checks edges, and produces a pre-computed
/// evaluation order.
pub fn materialize_dags(definition: &ServiceDefinition) -> Result<MaterializedDag, Vec<SdgError>> {
    let computations = &definition.computations;

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
    let topo_order = match toposort(&graph, None) {
        Ok(order) => order,
        Err(cycle) => {
            let cycle_node = graph[cycle.node_id()].clone();
            return Err(vec![SdgError::DagCycle { node: cycle_node }]);
        }
    };

    // Build a node-ID-to-ComputationNode map for O(1) lookup.
    let comp_node_map: HashMap<&str, &ComputationNode> = computations
        .nodes
        .iter()
        .map(|n| (n.id.as_str(), n))
        .collect();

    // Edge type-checking: validate port names and type compatibility.
    let mut type_errors = Vec::new();

    for edge in &computations.edges {
        let Some(target_node) = comp_node_map.get(edge.to.as_str()) else {
            continue; // Already caught by DagEdgeReference above
        };

        // Get valid ports for the target node type.
        let Some(ports) = valid_ports(&target_node.node_type) else {
            continue; // Unknown node type; semantic pass handles this.
        };

        // Check if the port name is valid.
        // For variadic ports, the edge.port should match the port name (e.g., "in").
        let Some((_, port_type)) = ports.iter().find(|(name, _)| *name == edge.port.as_str())
        else {
            type_errors.push(SdgError::DagInvalidPort {
                port: edge.port.clone(),
                node: edge.to.clone(),
                node_type: target_node.node_type.clone(),
            });
            continue;
        };

        // Resolve the source node's output type.
        let Some(source_node) = comp_node_map.get(edge.from.as_str()) else {
            continue;
        };

        let Some(source_type) = resolve_output_type(source_node, definition) else {
            continue; // Unresolvable type; skip (semantic pass catches root cause).
        };

        // Check type compatibility.
        if !is_type_compatible(&source_type, port_type) {
            type_errors.push(SdgError::TypeMismatch {
                path: format!(
                    "computations.edges[{} -> {}.{}]",
                    edge.from, edge.to, edge.port
                ),
                expected: port_type_display(port_type),
                found: source_type,
            });
        }
    }

    if !type_errors.is_empty() {
        return Err(type_errors);
    }

    Ok(MaterializedDag {
        graph,
        topo_order,
        node_map,
    })
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
    fn make_definition(nodes: Vec<ComputationNode>, edges: Vec<Edge>) -> ServiceDefinition {
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

    fn make_node(
        id: &str,
        node_type: &str,
        params: serde_json::Map<String, serde_json::Value>,
    ) -> ComputationNode {
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
        str_params.insert(
            "value".to_string(),
            serde_json::Value::String("hello".to_string()),
        );
        str_params.insert(
            "output_type".to_string(),
            serde_json::Value::String("string".to_string()),
        );

        let nodes = vec![
            make_node("str_val", "literal", str_params),
            make_node("logic_and", "and", serde_json::Map::new()),
        ];
        let edges = vec![make_edge("str_val", "logic_and", "in", Some(0))];
        let def = make_definition(nodes, edges);

        let result = materialize_dags(&def);
        let errors = result.expect_err("should fail type-checking");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SdgError::TypeMismatch { .. })),
            "should have TypeMismatch error, got: {errors:?}"
        );
    }

    #[test]
    fn test_invalid_port_name_on_not_node() {
        // literal(boolean) -> not(port="input") should fail: "not" only has port "value"
        let mut bool_params = serde_json::Map::new();
        bool_params.insert("value".to_string(), serde_json::Value::Bool(true));
        bool_params.insert(
            "output_type".to_string(),
            serde_json::Value::String("boolean".to_string()),
        );

        let nodes = vec![
            make_node("val1", "literal", bool_params),
            make_node("negate", "not", serde_json::Map::new()),
        ];
        let edges = vec![make_edge("val1", "negate", "input", None)];
        let def = make_definition(nodes, edges);

        let result = materialize_dags(&def);
        let errors = result.expect_err("should fail port validation");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SdgError::DagInvalidPort { .. })),
            "should have DagInvalidPort error, got: {errors:?}"
        );
    }

    #[test]
    fn test_compatible_uuid_to_lookup_id() {
        // context(path="actor.id", output=uuid) -> lookup(port="id", expects uuid) => no error
        let mut ctx_params = serde_json::Map::new();
        ctx_params.insert(
            "path".to_string(),
            serde_json::Value::String("actor.id".to_string()),
        );
        let mut lookup_params = serde_json::Map::new();
        lookup_params.insert(
            "aggregate".to_string(),
            serde_json::Value::String("Thing".to_string()),
        );
        lookup_params.insert(
            "pick".to_string(),
            serde_json::Value::String("title".to_string()),
        );

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
        int_params.insert(
            "output_type".to_string(),
            serde_json::Value::String("integer".to_string()),
        );

        let nodes = vec![
            make_node("int_val", "literal", int_params),
            make_node("counter", "count", serde_json::Map::new()),
        ];
        let edges = vec![make_edge("int_val", "counter", "items", None)];
        let def = make_definition(nodes, edges);

        let result = materialize_dags(&def);
        let errors = result.expect_err("should fail type-checking");
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, SdgError::TypeMismatch { .. })),
            "should have TypeMismatch error (integer is not an array), got: {errors:?}"
        );
    }

    #[test]
    fn test_variadic_boolean_port_accepts_boolean() {
        // literal(boolean) -> and(port="in", index=0) => no error
        let mut bool_params = serde_json::Map::new();
        bool_params.insert("value".to_string(), serde_json::Value::Bool(true));
        bool_params.insert(
            "output_type".to_string(),
            serde_json::Value::String("boolean".to_string()),
        );
        let mut bool_params2 = serde_json::Map::new();
        bool_params2.insert("value".to_string(), serde_json::Value::Bool(false));
        bool_params2.insert(
            "output_type".to_string(),
            serde_json::Value::String("boolean".to_string()),
        );

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
        str_params1.insert(
            "value".to_string(),
            serde_json::Value::String("hello".to_string()),
        );
        str_params1.insert(
            "output_type".to_string(),
            serde_json::Value::String("string".to_string()),
        );
        let mut str_params2 = serde_json::Map::new();
        str_params2.insert(
            "value".to_string(),
            serde_json::Value::String(" world".to_string()),
        );
        str_params2.insert(
            "output_type".to_string(),
            serde_json::Value::String("string".to_string()),
        );

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
