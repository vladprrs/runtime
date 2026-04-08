pub mod dag_pass;
pub mod schema_pass;
pub mod semantic_pass;
pub mod version_pass;

pub use dag_pass::MaterializedDag;

use std::path::Path;

use crate::error::SdgError;
use crate::types::ServiceDefinition;

/// Fully validated SDG with materialized DAG, ready for runtime use.
#[derive(Debug)]
pub struct ValidatedSdg {
    /// The deserialized service definition.
    pub definition: ServiceDefinition,
    /// The materialized computation DAG with pre-computed topological order.
    pub dag: MaterializedDag,
}

/// Load and validate an SDG file from the given path.
///
/// Runs validation passes in strict order (per D-28):
/// 1. JSON Schema conformance
/// 2. Version compatibility
/// 3. Semantic validation
/// 4. DAG materialization
///
/// Each pass collects all errors. Later passes do not run if earlier pass fails (D-29).
pub fn load(path: &Path) -> Result<ValidatedSdg, Vec<SdgError>> {
    // Read file
    let content = std::fs::read_to_string(path).map_err(|e| {
        vec![SdgError::FileRead {
            path: path.to_owned(),
            source: e,
        }]
    })?;

    // Parse JSON
    let raw: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        vec![SdgError::JsonParse {
            message: e.to_string(),
        }]
    })?;

    validate(&raw)
}

/// Validate raw JSON without loading from file.
/// Useful for testing or when JSON is already in memory.
pub fn validate(raw: &serde_json::Value) -> Result<ValidatedSdg, Vec<SdgError>> {
    // Pass 1: Schema conformance
    let schema_errors = schema_pass::validate_schema(raw);
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    // Pass 2: Version compatibility
    let version_errors = version_pass::validate_version(raw);
    if !version_errors.is_empty() {
        return Err(version_errors);
    }

    // Deserialize into typed structs
    let definition: ServiceDefinition = serde_json::from_value(raw.clone()).map_err(|e| {
        vec![SdgError::Deserialization {
            message: e.to_string(),
        }]
    })?;

    // Pass 3: Semantic validation
    let semantic_errors = semantic_pass::validate_semantics(&definition);
    if !semantic_errors.is_empty() {
        return Err(semantic_errors);
    }

    // Pass 4: DAG materialization + cycle detection
    let dag = dag_pass::materialize_dags(&definition.computations)?;

    Ok(ValidatedSdg { definition, dag })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_fixture_path() -> std::path::PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        std::path::PathBuf::from(format!(
            "{manifest_dir}/../../specs/003-sdg-v2-format/examples/task-tracker-extended.sdg.json"
        ))
    }

    #[test]
    fn test_load_canonical_fixture() {
        let result = load(&canonical_fixture_path());
        let validated = result.expect("canonical fixture should load successfully");
        assert_eq!(validated.definition.service.name, "task-tracker-extended");
        assert_eq!(validated.definition.model.aggregates.len(), 2);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let path = Path::new("/tmp/nonexistent_sdg_test_file_12345.sdg.json");
        let result = load(path);
        let errors = result.expect_err("nonexistent file should fail");
        assert_eq!(errors.len(), 1);
        assert!(
            matches!(&errors[0], SdgError::FileRead { .. }),
            "expected FileRead error, got: {:?}",
            errors[0]
        );
    }

    #[test]
    fn test_load_schema_failure_prevents_version_check() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("bad.sdg.json");
        std::fs::write(&path, r#"{"bad":"json"}"#).expect("write temp file");

        let result = load(&path);
        let errors = result.expect_err("invalid schema should fail");
        // All errors should be schema errors, not version errors
        for error in &errors {
            assert!(
                matches!(error, SdgError::SchemaViolation { .. }),
                "expected SchemaViolation (not version error), got: {error:?}"
            );
        }
    }

    #[test]
    fn test_load_wrong_version_after_valid_schema() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("wrong_version.sdg.json");
        let content = serde_json::json!({
            "schema_version": "1.0.0",
            "service": { "name": "test" },
            "model": { "aggregates": {} }
        });
        std::fs::write(&path, serde_json::to_string_pretty(&content).unwrap())
            .expect("write temp file");

        let result = load(&path);
        let errors = result.expect_err("wrong version should fail");
        assert_eq!(errors.len(), 1);
        assert!(
            matches!(&errors[0], SdgError::IncompatibleVersion { .. }),
            "expected IncompatibleVersion, got: {:?}",
            errors[0]
        );
    }

    #[test]
    fn test_validate_canonical_json() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = format!(
            "{manifest_dir}/../../specs/003-sdg-v2-format/examples/task-tracker-extended.sdg.json"
        );
        let content = std::fs::read_to_string(&fixture_path).expect("canonical fixture must exist");
        let raw: serde_json::Value =
            serde_json::from_str(&content).expect("fixture must be valid JSON");

        let validated = validate(&raw).expect("canonical fixture should validate");
        assert_eq!(validated.definition.service.name, "task-tracker-extended");
    }

    // --- Fixture tests (from Plan 02) ---

    fn fixture_path() -> std::path::PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        std::path::PathBuf::from(format!(
            "{manifest_dir}/fixtures/valid_task_tracker.sdg.json"
        ))
    }

    fn load_fixture() -> ValidatedSdg {
        let content = std::fs::read_to_string(fixture_path()).expect("fixture file must exist");
        let raw: serde_json::Value =
            serde_json::from_str(&content).expect("fixture must be valid JSON");
        validate(&raw).expect("fixture must validate and deserialize")
    }

    #[test]
    fn test_fixture_schema_validates() {
        let content = std::fs::read_to_string(fixture_path()).expect("fixture file must exist");
        let raw: serde_json::Value =
            serde_json::from_str(&content).expect("fixture must be valid JSON");
        let errors = schema_pass::validate_schema(&raw);
        assert!(
            errors.is_empty(),
            "fixture should pass schema validation, got: {errors:?}"
        );
    }

    #[test]
    fn test_fixture_deserializes() {
        let validated = load_fixture();
        assert_eq!(validated.definition.model.aggregates.len(), 2);
    }

    #[test]
    fn test_fixture_task_states() {
        let validated = load_fixture();
        let task = validated
            .definition
            .model
            .aggregates
            .get("Task")
            .expect("Task aggregate");
        assert_eq!(
            task.states,
            vec!["Created", "InProgress", "Done", "Cancelled"]
        );
    }

    #[test]
    fn test_fixture_user_states() {
        let validated = load_fixture();
        let user = validated
            .definition
            .model
            .aggregates
            .get("User")
            .expect("User aggregate");
        assert_eq!(user.states, vec!["Active", "Deactivated"]);
    }

    #[test]
    fn test_fixture_task_fields() {
        let validated = load_fixture();
        let task = validated
            .definition
            .model
            .aggregates
            .get("Task")
            .expect("Task aggregate");
        let field_names: Vec<&String> = task.fields.keys().collect();
        for expected in &[
            "title",
            "description",
            "author_id",
            "assignee_id",
            "priority",
            "linked_task_ids",
        ] {
            assert!(
                field_names.contains(&&expected.to_string()),
                "Task should have field '{expected}'"
            );
        }
        assert_eq!(
            task.fields.len(),
            6,
            "Task should have 6 user-defined fields"
        );
    }

    #[test]
    fn test_fixture_computation_nodes() {
        let validated = load_fixture();
        // The spec example has ~22 nodes
        assert!(
            validated.definition.computations.nodes.len() >= 22,
            "expected at least 22 computation nodes, got {}",
            validated.definition.computations.nodes.len()
        );
        // Check key nodes exist
        let node_ids: Vec<&str> = validated
            .definition
            .computations
            .nodes
            .iter()
            .map(|n| n.id.as_str())
            .collect();
        for expected in &[
            "actor_id",
            "can_complete",
            "is_assignee",
            "zero",
            "all_linked_done",
        ] {
            assert!(
                node_ids.contains(expected),
                "expected node '{expected}' to exist"
            );
        }
    }

    #[test]
    fn test_fixture_computation_edges() {
        let validated = load_fixture();
        // The spec example has ~22 edges
        assert!(
            validated.definition.computations.edges.len() >= 22,
            "expected at least 22 computation edges, got {}",
            validated.definition.computations.edges.len()
        );
    }

    #[test]
    fn test_fixture_complete_guard() {
        let validated = load_fixture();
        let task = validated
            .definition
            .model
            .aggregates
            .get("Task")
            .expect("Task aggregate");
        let complete = task
            .transitions
            .get("Complete")
            .expect("Complete transition");
        assert_eq!(
            complete.guard.as_deref(),
            Some("can_complete"),
            "Complete transition should have guard 'can_complete'"
        );
    }

    #[test]
    fn test_fixture_create_auto_fields() {
        let validated = load_fixture();
        let task = validated
            .definition
            .model
            .aggregates
            .get("Task")
            .expect("Task aggregate");
        let create = task.transitions.get("Create").expect("Create transition");
        assert_eq!(
            create.auto_fields.get("author_id").map(String::as_str),
            Some("actor_id"),
            "Create transition should have auto_fields author_id -> actor_id"
        );
    }

    #[test]
    fn test_fixture_load_from_file() {
        let result = load(&fixture_path());
        let validated = result.expect("fixture should load from file path");
        assert_eq!(validated.definition.service.name, "task-tracker-extended");
    }

    // --- Pipeline integration tests (Plan 03) ---

    #[test]
    fn test_full_pipeline_canonical_fixture() {
        let result = load(&fixture_path());
        let validated = result.expect("canonical fixture should pass full pipeline");
        assert_eq!(validated.definition.service.name, "task-tracker-extended");
        assert!(
            !validated.dag.topo_order.is_empty(),
            "topo_order should not be empty for fixture with computation nodes"
        );
    }

    #[test]
    fn test_pipeline_halts_at_semantic() {
        // Create a file that passes schema + version but has semantic errors.
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("semantic_error.sdg.json");
        let content = serde_json::json!({
            "schema_version": "2.0.0",
            "service": { "name": "test" },
            "model": {
                "aggregates": {
                    "Task": {
                        "fields": { "title": { "type": "string" } },
                        "states": ["Created", "Done"],
                        "transitions": {
                            "Bad": {
                                "from": "Creatd",
                                "to": "Done"
                            }
                        }
                    }
                }
            }
        });
        std::fs::write(&path, serde_json::to_string_pretty(&content).unwrap())
            .expect("write temp file");

        let result = load(&path);
        let errors = result.expect_err("semantic error should fail");
        // Errors should be semantic (not DAG)
        for error in &errors {
            assert_eq!(
                error.pass(),
                "semantic",
                "expected semantic error, got {}: {error:?}",
                error.pass()
            );
        }
    }

    #[test]
    fn test_pipeline_load_returns_validated_sdg() {
        let result = load(&fixture_path());
        let validated = result.expect("fixture should load");
        // Verify both definition and dag are present.
        assert!(!validated.definition.service.name.is_empty());
        assert_eq!(
            validated.dag.graph.node_count(),
            validated.definition.computations.nodes.len()
        );
    }
}
