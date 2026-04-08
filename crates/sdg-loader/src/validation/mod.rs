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
pub fn load(_path: &Path) -> Result<ServiceDefinition, Vec<SdgError>> {
    todo!("RED: implement load pipeline")
}

/// Validate raw JSON without loading from file.
/// Useful for testing or when JSON is already in memory.
pub fn validate(_raw: &serde_json::Value) -> Result<ServiceDefinition, Vec<SdgError>> {
    todo!("RED: implement validate pipeline")
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
        let content =
            std::fs::read_to_string(&fixture_path).expect("canonical fixture must exist");
        let raw: serde_json::Value =
            serde_json::from_str(&content).expect("fixture must be valid JSON");

        let sd = validate(&raw).expect("canonical fixture should validate");
        assert_eq!(sd.service.name, "task-tracker-extended");
    }
}
