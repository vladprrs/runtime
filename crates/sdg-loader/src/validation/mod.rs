pub mod schema_pass;
pub mod version_pass;

use std::path::Path;

use crate::error::SdgError;
use crate::types::ServiceDefinition;

/// Load and validate an SDG file from the given path.
///
/// Runs validation passes in strict order (per D-28):
/// 1. JSON Schema conformance
/// 2. Version compatibility
/// 3. Semantic validation (wired in Plan 03)
/// 4. DAG materialization (wired in Plan 03)
///
/// Each pass collects all errors. Later passes do not run if earlier pass fails (D-29).
pub fn load(path: &Path) -> Result<ServiceDefinition, Vec<SdgError>> {
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
pub fn validate(raw: &serde_json::Value) -> Result<ServiceDefinition, Vec<SdgError>> {
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

    // Pass 3: Semantic validation (placeholder -- wired in Plan 03)
    // Pass 4: DAG materialization (placeholder -- wired in Plan 03)

    Ok(definition)
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
        let sd = result.expect("canonical fixture should load successfully");
        assert_eq!(sd.service.name, "task-tracker-extended");
        assert_eq!(sd.model.aggregates.len(), 2);
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

        let sd = validate(&raw).expect("canonical fixture should validate");
        assert_eq!(sd.service.name, "task-tracker-extended");
    }

    // --- Fixture tests (Task 2) ---

    fn fixture_path() -> std::path::PathBuf {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        std::path::PathBuf::from(format!(
            "{manifest_dir}/fixtures/valid_task_tracker.sdg.json"
        ))
    }

    fn load_fixture() -> ServiceDefinition {
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
        let sd = load_fixture();
        assert_eq!(sd.model.aggregates.len(), 2);
    }

    #[test]
    fn test_fixture_task_states() {
        let sd = load_fixture();
        let task = sd.model.aggregates.get("Task").expect("Task aggregate");
        assert_eq!(
            task.states,
            vec!["Created", "InProgress", "Done", "Cancelled"]
        );
    }

    #[test]
    fn test_fixture_user_states() {
        let sd = load_fixture();
        let user = sd.model.aggregates.get("User").expect("User aggregate");
        assert_eq!(user.states, vec!["Active", "Deactivated"]);
    }

    #[test]
    fn test_fixture_task_fields() {
        let sd = load_fixture();
        let task = sd.model.aggregates.get("Task").expect("Task aggregate");
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
        let sd = load_fixture();
        // The spec example has ~22 nodes
        assert!(
            sd.computations.nodes.len() >= 22,
            "expected at least 22 computation nodes, got {}",
            sd.computations.nodes.len()
        );
        // Check key nodes exist
        let node_ids: Vec<&str> = sd
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
        let sd = load_fixture();
        // The spec example has ~22 edges
        assert!(
            sd.computations.edges.len() >= 22,
            "expected at least 22 computation edges, got {}",
            sd.computations.edges.len()
        );
    }

    #[test]
    fn test_fixture_complete_guard() {
        let sd = load_fixture();
        let task = sd.model.aggregates.get("Task").expect("Task aggregate");
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
        let sd = load_fixture();
        let task = sd.model.aggregates.get("Task").expect("Task aggregate");
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
        let sd = result.expect("fixture should load from file path");
        assert_eq!(sd.service.name, "task-tracker-extended");
    }
}
