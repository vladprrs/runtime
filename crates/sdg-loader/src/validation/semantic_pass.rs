use std::collections::HashSet;

use crate::error::SdgError;
use crate::suggestions::suggestion_or_empty;
use crate::types::ServiceDefinition;

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

/// Per D-15 through D-20: the valid computation node types from the function catalog.
const VALID_NODE_TYPES: &[&str] = &[
    "field",
    "command",
    "context",
    "literal",
    "lookup",
    "lookup_many",
    "filter",
    "count",
    "sum",
    "min",
    "max",
    "any",
    "all",
    "contains",
    "length",
    "eq",
    "neq",
    "gt",
    "lt",
    "gte",
    "lte",
    "and",
    "or",
    "not",
    "add",
    "sub",
    "mul",
    "div",
    "concat",
    "str_contains",
    "str_len",
];

/// Pass 3: Semantic validation of cross-references within the typed `ServiceDefinition`.
///
/// Checks state references, guard/`auto_fields` refs, node types, duplicate IDs,
/// context paths (D-39), implicit field conflicts (D-37), field-node names,
/// and literal `output_type` params (D-36).
///
/// All errors are collected (not short-circuited) within this pass (per D-29).
pub fn validate_semantics(definition: &ServiceDefinition) -> Vec<SdgError> {
    let mut errors = Vec::new();

    let node_ids: Vec<&str> = definition
        .computations
        .nodes
        .iter()
        .map(|n| n.id.as_str())
        .collect();

    validate_duplicate_node_ids(definition, &mut errors);
    validate_aggregates(definition, &node_ids, &mut errors);
    validate_computation_nodes(definition, &mut errors);

    errors
}

/// Check for duplicate computation node IDs.
fn validate_duplicate_node_ids(definition: &ServiceDefinition, errors: &mut Vec<SdgError>) {
    let mut seen_ids = HashSet::new();
    for node in &definition.computations.nodes {
        if !seen_ids.insert(node.id.as_str()) {
            errors.push(SdgError::DuplicateNodeId {
                path: format!("computations.nodes[id={}]", node.id),
                node_id: node.id.clone(),
            });
        }
    }
}

/// Validate aggregate fields, state references, guards, and `auto_fields`.
fn validate_aggregates(
    definition: &ServiceDefinition,
    node_ids: &[&str],
    errors: &mut Vec<SdgError>,
) {
    for (agg_name, aggregate) in &definition.model.aggregates {
        let path_prefix = format!("model.aggregates.{agg_name}");

        // D-37: Check no user-declared field conflicts with implicit fields.
        for field_name in aggregate.fields.keys() {
            if IMPLICIT_FIELDS
                .iter()
                .any(|(name, _)| *name == field_name.as_str())
            {
                errors.push(SdgError::ImplicitFieldConflict {
                    path: format!("{path_prefix}.fields.{field_name}"),
                    name: field_name.clone(),
                });
            }
        }

        let state_names: Vec<&str> = aggregate.states.iter().map(String::as_str).collect();

        for (trans_name, transition) in &aggregate.transitions {
            let trans_path = format!("{path_prefix}.transitions.{trans_name}");

            // Validate "from" state references.
            let from_states = match &transition.from {
                crate::types::StateRef::Single(s) => vec![s.as_str()],
                crate::types::StateRef::Multiple(v) => v.iter().map(String::as_str).collect(),
            };
            for state in &from_states {
                if !state_names.contains(state) {
                    errors.push(SdgError::InvalidStateReference {
                        path: format!("{trans_path}.from"),
                        name: (*state).to_string(),
                        aggregate: agg_name.clone(),
                        suggestion: suggestion_or_empty(state, &state_names),
                    });
                }
            }

            // Validate "to" state (unless "$same" per D-06).
            if transition.to != "$same" && !state_names.contains(&transition.to.as_str()) {
                errors.push(SdgError::InvalidStateReference {
                    path: format!("{trans_path}.to"),
                    name: transition.to.clone(),
                    aggregate: agg_name.clone(),
                    suggestion: suggestion_or_empty(&transition.to, &state_names),
                });
            }

            // Validate guard references a computation node that exists.
            if let Some(guard_id) = &transition.guard {
                if !node_ids.contains(&guard_id.as_str()) {
                    errors.push(SdgError::SemanticError {
                        path: format!("{trans_path}.guard"),
                        message: format!(
                            "guard references non-existent computation node '{guard_id}'{}",
                            suggestion_or_empty(guard_id, node_ids)
                        ),
                    });
                }
            }

            // Validate `auto_fields` reference computation nodes that exist.
            for (field_name, node_id) in &transition.auto_fields {
                if !node_ids.contains(&node_id.as_str()) {
                    errors.push(SdgError::SemanticError {
                        path: format!("{trans_path}.auto_fields.{field_name}"),
                        message: format!(
                            "auto_field references non-existent computation node '{node_id}'{}",
                            suggestion_or_empty(node_id, node_ids)
                        ),
                    });
                }
            }
        }
    }
}

/// Validate computation node types, context paths, field names, and literal params.
fn validate_computation_nodes(definition: &ServiceDefinition, errors: &mut Vec<SdgError>) {
    for node in &definition.computations.nodes {
        let node_path = format!("computations.nodes[id={}]", node.id);

        // Validate node type is known.
        if !VALID_NODE_TYPES.contains(&node.node_type.as_str()) {
            errors.push(SdgError::UnknownNodeType {
                path: node_path.clone(),
                node_type: node.node_type.clone(),
                suggestion: suggestion_or_empty(&node.node_type, VALID_NODE_TYPES),
            });
        }

        // D-39: Validate context paths.
        if node.node_type == "context" {
            if let Some(path_val) = node.params.get("path").and_then(|v| v.as_str()) {
                let valid_paths: Vec<&str> = VALID_CONTEXT_PATHS.iter().map(|(p, _)| *p).collect();
                if !valid_paths.contains(&path_val) {
                    errors.push(SdgError::InvalidContextPath {
                        path: node_path.clone(),
                        context_path: path_val.to_string(),
                        suggestion: suggestion_or_empty(path_val, &valid_paths),
                    });
                }
            }
        }

        // Validate field-node references: name param must be non-empty.
        if node.node_type == "field" {
            if let Some(field_name) = node.params.get("name").and_then(|v| v.as_str()) {
                if field_name.is_empty() {
                    errors.push(SdgError::SemanticError {
                        path: node_path.clone(),
                        message: "field node has empty 'name' param".to_string(),
                    });
                }
            }
        }

        // D-36: Validate literal nodes have `output_type` param.
        if node.node_type == "literal" && !node.params.contains_key("output_type") {
            errors.push(SdgError::SemanticError {
                path: node_path.clone(),
                message: "literal node must have 'output_type' param (per D-36)".to_string(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        Aggregate, ApiConfig, ComputationNode, ComputationsDefinition, FieldDefinition,
        ModelDefinition, ServiceDefinition, ServiceInfo, StateRef, Transition,
    };
    use std::collections::HashMap;

    /// Helper: create a minimal valid `ServiceDefinition` for testing.
    fn minimal_definition() -> ServiceDefinition {
        let mut fields = HashMap::new();
        fields.insert(
            "title".to_string(),
            FieldDefinition {
                field_type: "string".to_string(),
                required: true,
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

        let mut transitions = HashMap::new();
        transitions.insert(
            "Create".to_string(),
            Transition {
                from: StateRef::Single("Created".to_string()),
                to: "Created".to_string(),
                command: None,
                guard: None,
                auto_fields: HashMap::new(),
                event_name: None,
                description: None,
            },
        );

        let mut aggregates = HashMap::new();
        aggregates.insert(
            "Task".to_string(),
            Aggregate {
                fields,
                states: vec![
                    "Created".to_string(),
                    "InProgress".to_string(),
                    "Done".to_string(),
                ],
                initial_state: Some("Created".to_string()),
                transitions,
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
            computations: ComputationsDefinition::default(),
            api: ApiConfig::default(),
        }
    }

    /// Helper: add a computation node to a definition.
    fn add_node(def: &mut ServiceDefinition, id: &str, node_type: &str) {
        def.computations.nodes.push(ComputationNode {
            id: id.to_string(),
            node_type: node_type.to_string(),
            params: serde_json::Map::new(),
        });
    }

    /// Helper: add a computation node with params.
    fn add_node_with_params(
        def: &mut ServiceDefinition,
        id: &str,
        node_type: &str,
        params: serde_json::Map<String, serde_json::Value>,
    ) {
        def.computations.nodes.push(ComputationNode {
            id: id.to_string(),
            node_type: node_type.to_string(),
            params,
        });
    }

    // --- Canonical fixture test ---

    #[test]
    fn test_valid_fixture_passes_semantics() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let content = std::fs::read_to_string(format!(
            "{manifest_dir}/fixtures/valid_task_tracker.sdg.json"
        ))
        .expect("fixture must exist");
        let definition: ServiceDefinition =
            serde_json::from_str(&content).expect("fixture must deserialize");
        let errors = validate_semantics(&definition);
        assert!(
            errors.is_empty(),
            "canonical fixture should pass semantic validation, got: {errors:?}"
        );
    }

    // --- State reference tests ---

    #[test]
    fn test_invalid_from_state() {
        let mut def = minimal_definition();
        let task = def.model.aggregates.get_mut("Task").unwrap();
        task.transitions.insert(
            "Bad".to_string(),
            Transition {
                from: StateRef::Single("Creatd".to_string()),
                to: "Done".to_string(),
                command: None,
                guard: None,
                auto_fields: HashMap::new(),
                event_name: None,
                description: None,
            },
        );

        let errors = validate_semantics(&def);
        assert!(!errors.is_empty(), "should catch invalid from state");
        let has_state_error = errors.iter().any(|e| {
            matches!(e, SdgError::InvalidStateReference { name, suggestion, .. }
                if name == "Creatd" && suggestion.contains("Created"))
        });
        assert!(
            has_state_error,
            "should have InvalidStateReference with suggestion for 'Creatd', got: {errors:?}"
        );
    }

    #[test]
    fn test_invalid_to_state() {
        let mut def = minimal_definition();
        let task = def.model.aggregates.get_mut("Task").unwrap();
        task.transitions.insert(
            "Bad".to_string(),
            Transition {
                from: StateRef::Single("Created".to_string()),
                to: "Doen".to_string(),
                command: None,
                guard: None,
                auto_fields: HashMap::new(),
                event_name: None,
                description: None,
            },
        );

        let errors = validate_semantics(&def);
        assert!(!errors.is_empty(), "should catch invalid to state");
        let has_state_error = errors.iter().any(|e| {
            matches!(e, SdgError::InvalidStateReference { name, suggestion, .. }
                if name == "Doen" && suggestion.contains("Done"))
        });
        assert!(
            has_state_error,
            "should have InvalidStateReference with suggestion for 'Doen', got: {errors:?}"
        );
    }

    #[test]
    fn test_same_sentinel_accepted() {
        let mut def = minimal_definition();
        let task = def.model.aggregates.get_mut("Task").unwrap();
        task.transitions.insert(
            "Update".to_string(),
            Transition {
                from: StateRef::Single("Created".to_string()),
                to: "$same".to_string(),
                command: None,
                guard: None,
                auto_fields: HashMap::new(),
                event_name: None,
                description: None,
            },
        );

        let errors = validate_semantics(&def);
        let same_errors: Vec<_> = errors
            .iter()
            .filter(
                |e| matches!(e, SdgError::InvalidStateReference { name, .. } if name == "$same"),
            )
            .collect();
        assert!(
            same_errors.is_empty(),
            "'$same' should be accepted as a valid 'to' state, got: {same_errors:?}"
        );
    }

    // --- Guard and auto_fields tests ---

    #[test]
    fn test_guard_nonexistent_node() {
        let mut def = minimal_definition();
        let task = def.model.aggregates.get_mut("Task").unwrap();
        task.transitions.insert(
            "Guarded".to_string(),
            Transition {
                from: StateRef::Single("Created".to_string()),
                to: "Done".to_string(),
                command: None,
                guard: Some("nonexistent".to_string()),
                auto_fields: HashMap::new(),
                event_name: None,
                description: None,
            },
        );

        let errors = validate_semantics(&def);
        assert!(!errors.is_empty(), "should catch nonexistent guard node");
        let has_guard_error = errors.iter().any(|e| {
            matches!(e, SdgError::SemanticError { message, .. } if message.contains("guard") && message.contains("nonexistent"))
        });
        assert!(
            has_guard_error,
            "should have SemanticError about guard referencing nonexistent node, got: {errors:?}"
        );
    }

    #[test]
    fn test_auto_fields_nonexistent_node() {
        let mut def = minimal_definition();
        let task = def.model.aggregates.get_mut("Task").unwrap();
        let mut auto_fields = HashMap::new();
        auto_fields.insert("some_field".to_string(), "nonexistent".to_string());
        task.transitions.insert(
            "AutoFielded".to_string(),
            Transition {
                from: StateRef::Single("Created".to_string()),
                to: "Done".to_string(),
                command: None,
                guard: None,
                auto_fields,
                event_name: None,
                description: None,
            },
        );

        let errors = validate_semantics(&def);
        assert!(
            !errors.is_empty(),
            "should catch nonexistent auto_fields node"
        );
        let has_auto_error = errors.iter().any(|e| {
            matches!(e, SdgError::SemanticError { message, .. } if message.contains("auto_field") && message.contains("nonexistent"))
        });
        assert!(
            has_auto_error,
            "should have SemanticError about auto_field referencing nonexistent node, got: {errors:?}"
        );
    }

    // --- Node type tests ---

    #[test]
    fn test_unknown_node_type() {
        let mut def = minimal_definition();
        add_node(&mut def, "bad_node", "foobar");

        let errors = validate_semantics(&def);
        assert!(!errors.is_empty(), "should catch unknown node type");
        let has_type_error = errors.iter().any(
            |e| matches!(e, SdgError::UnknownNodeType { node_type, .. } if node_type == "foobar"),
        );
        assert!(
            has_type_error,
            "should have UnknownNodeType for 'foobar', got: {errors:?}"
        );
    }

    #[test]
    fn test_duplicate_node_ids() {
        let mut def = minimal_definition();
        add_node(&mut def, "dup_id", "eq");
        add_node(&mut def, "dup_id", "neq");

        let errors = validate_semantics(&def);
        assert!(!errors.is_empty(), "should catch duplicate node IDs");
        let has_dup_error = errors
            .iter()
            .any(|e| matches!(e, SdgError::DuplicateNodeId { node_id, .. } if node_id == "dup_id"));
        assert!(
            has_dup_error,
            "should have DuplicateNodeId for 'dup_id', got: {errors:?}"
        );
    }

    // --- Context path tests ---

    #[test]
    fn test_invalid_context_path() {
        let mut def = minimal_definition();
        let mut params = serde_json::Map::new();
        params.insert(
            "path".to_string(),
            serde_json::Value::String("actor.name".to_string()),
        );
        add_node_with_params(&mut def, "ctx_bad", "context", params);

        let errors = validate_semantics(&def);
        assert!(!errors.is_empty(), "should catch invalid context path");
        let has_ctx_error = errors.iter().any(|e| {
            matches!(e, SdgError::InvalidContextPath { context_path, .. } if context_path == "actor.name")
        });
        assert!(
            has_ctx_error,
            "should have InvalidContextPath for 'actor.name', got: {errors:?}"
        );
    }

    #[test]
    fn test_valid_context_paths() {
        let mut def = minimal_definition();
        let valid_paths = [
            "actor.id",
            "actor.email",
            "actor.roles",
            "timestamp",
            "correlation_id",
        ];
        for (i, path) in valid_paths.iter().enumerate() {
            let mut params = serde_json::Map::new();
            params.insert(
                "path".to_string(),
                serde_json::Value::String(path.to_string()),
            );
            add_node_with_params(&mut def, &format!("ctx_{i}"), "context", params);
        }

        let errors = validate_semantics(&def);
        let ctx_errors: Vec<_> = errors
            .iter()
            .filter(|e| matches!(e, SdgError::InvalidContextPath { .. }))
            .collect();
        assert!(
            ctx_errors.is_empty(),
            "valid context paths should not produce errors, got: {ctx_errors:?}"
        );
    }

    // --- Implicit field conflict tests ---

    #[test]
    fn test_implicit_field_conflict_id() {
        let mut def = minimal_definition();
        let task = def.model.aggregates.get_mut("Task").unwrap();
        task.fields.insert(
            "id".to_string(),
            FieldDefinition {
                field_type: "uuid".to_string(),
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

        let errors = validate_semantics(&def);
        let has_implicit_error = errors
            .iter()
            .any(|e| matches!(e, SdgError::ImplicitFieldConflict { name, .. } if name == "id"));
        assert!(
            has_implicit_error,
            "should catch implicit field conflict for 'id', got: {errors:?}"
        );
    }

    #[test]
    fn test_implicit_field_conflict_state() {
        let mut def = minimal_definition();
        let task = def.model.aggregates.get_mut("Task").unwrap();
        task.fields.insert(
            "state".to_string(),
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

        let errors = validate_semantics(&def);
        let has_implicit_error = errors
            .iter()
            .any(|e| matches!(e, SdgError::ImplicitFieldConflict { name, .. } if name == "state"));
        assert!(
            has_implicit_error,
            "should catch implicit field conflict for 'state', got: {errors:?}"
        );
    }

    #[test]
    fn test_implicit_field_conflict_created_at() {
        let mut def = minimal_definition();
        let task = def.model.aggregates.get_mut("Task").unwrap();
        task.fields.insert(
            "created_at".to_string(),
            FieldDefinition {
                field_type: "datetime".to_string(),
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

        let errors = validate_semantics(&def);
        let has_implicit_error = errors.iter().any(
            |e| matches!(e, SdgError::ImplicitFieldConflict { name, .. } if name == "created_at"),
        );
        assert!(
            has_implicit_error,
            "should catch implicit field conflict for 'created_at', got: {errors:?}"
        );
    }

    // --- Literal node test ---

    #[test]
    fn test_literal_missing_output_type() {
        let mut def = minimal_definition();
        let mut params = serde_json::Map::new();
        params.insert("value".to_string(), serde_json::Value::from(42));
        // No output_type param
        add_node_with_params(&mut def, "bad_literal", "literal", params);

        let errors = validate_semantics(&def);
        let has_literal_error = errors.iter().any(|e| {
            matches!(e, SdgError::SemanticError { message, .. } if message.contains("output_type"))
        });
        assert!(
            has_literal_error,
            "should catch literal node missing output_type, got: {errors:?}"
        );
    }

    // --- Error collection test ---

    #[test]
    fn test_collects_all_errors() {
        let mut def = minimal_definition();
        // Add multiple problems:
        // 1. Invalid state reference
        let task = def.model.aggregates.get_mut("Task").unwrap();
        task.transitions.insert(
            "BadState".to_string(),
            Transition {
                from: StateRef::Single("Creatd".to_string()),
                to: "Doen".to_string(),
                command: None,
                guard: None,
                auto_fields: HashMap::new(),
                event_name: None,
                description: None,
            },
        );
        // 2. Unknown node type
        add_node(&mut def, "bad1", "foobar");
        // 3. Duplicate node ID
        add_node(&mut def, "bad1", "eq");

        let errors = validate_semantics(&def);
        assert!(
            errors.len() > 1,
            "should collect multiple errors, got {} errors: {errors:?}",
            errors.len()
        );
    }
}
