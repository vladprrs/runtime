use std::collections::HashSet;

use crate::error::SdgError;
use crate::suggestions::suggestion_or_empty;
use crate::types::ServiceDefinition;

const IMPLICIT_FIELDS: &[(&str, &str)] = &[
    ("id", "uuid"),
    ("state", "string"),
    ("created_at", "datetime"),
    ("updated_at", "datetime"),
    ("version", "integer"),
];

const VALID_CONTEXT_PATHS: &[(&str, &str)] = &[
    ("actor.id", "uuid"),
    ("actor.email", "string"),
    ("actor.roles", "string[]"),
    ("timestamp", "datetime"),
    ("correlation_id", "uuid"),
];

const VALID_NODE_TYPES: &[&str] = &[
    "field", "command", "context", "literal", "lookup", "lookup_many", "filter", "count", "sum",
    "min", "max", "any", "all", "contains", "length", "eq", "neq", "gt", "lt", "gte", "lte",
    "and", "or", "not", "add", "sub", "mul", "div", "concat", "str_contains", "str_len",
];

/// Pass 3: Semantic validation of cross-references within the typed `ServiceDefinition`.
///
/// Checks:
/// - State references in transitions (from/to) exist in the aggregate's states
/// - Guard and auto_fields reference existing computation nodes
/// - Computation node types are from the known catalog
/// - No duplicate computation node IDs
/// - Context nodes reference valid context paths (per D-39)
/// - No user-declared fields shadow implicit fields (per D-37)
/// - Field nodes have non-empty name params
/// - Literal nodes have output_type param (per D-36)
///
/// All errors are collected (not short-circuited) within this pass (per D-29).
pub fn validate_semantics(definition: &ServiceDefinition) -> Vec<SdgError> {
    // TODO: implement
    let _ = (
        definition,
        IMPLICIT_FIELDS,
        VALID_CONTEXT_PATHS,
        VALID_NODE_TYPES,
        suggestion_or_empty,
    );
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        Aggregate, ApiConfig, CommandDefinition, ComputationNode, ComputationsDefinition, Edge,
        FieldDefinition, ModelDefinition, ServiceDefinition, ServiceInfo, StateRef, Transition,
    };
    use std::collections::HashMap;

    /// Helper: create a minimal valid ServiceDefinition for testing.
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
        // Filter to only InvalidStateReference errors for "$same"
        let same_errors: Vec<_> = errors
            .iter()
            .filter(|e| matches!(e, SdgError::InvalidStateReference { name, .. } if name == "$same"))
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
        assert!(!errors.is_empty(), "should catch nonexistent auto_fields node");
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
        let has_type_error = errors.iter().any(|e| {
            matches!(e, SdgError::UnknownNodeType { node_type, .. } if node_type == "foobar")
        });
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
        let has_dup_error = errors.iter().any(|e| {
            matches!(e, SdgError::DuplicateNodeId { node_id, .. } if node_id == "dup_id")
        });
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
        let valid_paths = ["actor.id", "actor.email", "actor.roles", "timestamp", "correlation_id"];
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
        let has_implicit_error = errors.iter().any(|e| {
            matches!(e, SdgError::ImplicitFieldConflict { name, .. } if name == "id")
        });
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
        let has_implicit_error = errors.iter().any(|e| {
            matches!(e, SdgError::ImplicitFieldConflict { name, .. } if name == "state")
        });
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
        let has_implicit_error = errors.iter().any(|e| {
            matches!(e, SdgError::ImplicitFieldConflict { name, .. } if name == "created_at")
        });
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
