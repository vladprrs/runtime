use crate::error::SdgError;

/// Pass 1: Validate raw JSON against the embedded SDG JSON Schema.
/// Returns all schema violations found (per D-29: collect all errors within pass).
pub fn validate_schema(_raw: &serde_json::Value) -> Vec<SdgError> {
    todo!("RED: implement schema validation")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical_fixture_json() -> serde_json::Value {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = format!(
            "{manifest_dir}/../../specs/003-sdg-v2-format/examples/task-tracker-extended.sdg.json"
        );
        let content =
            std::fs::read_to_string(&fixture_path).expect("canonical fixture must exist");
        serde_json::from_str(&content).expect("fixture must be valid JSON")
    }

    #[test]
    fn test_valid_sdg_passes_schema() {
        let raw = canonical_fixture_json();
        let errors = validate_schema(&raw);
        assert!(
            errors.is_empty(),
            "valid SDG should produce no schema errors, got: {errors:?}"
        );
    }

    #[test]
    fn test_missing_service_fails_schema() {
        let raw = serde_json::json!({
            "schema_version": "2.0.0",
            "model": { "aggregates": {} }
        });
        let errors = validate_schema(&raw);
        assert!(!errors.is_empty(), "missing 'service' should fail schema validation");
        assert!(
            errors.iter().any(|e| matches!(e, SdgError::SchemaViolation { .. })),
            "errors should be SchemaViolation type"
        );
    }

    #[test]
    fn test_multiple_violations_all_collected() {
        // Empty object missing schema_version, service, and model
        let raw = serde_json::json!({});
        let errors = validate_schema(&raw);
        assert!(
            errors.len() > 1,
            "empty object should produce multiple errors, got {}",
            errors.len()
        );
        for error in &errors {
            assert!(
                matches!(error, SdgError::SchemaViolation { .. }),
                "all errors should be SchemaViolation, got: {error:?}"
            );
        }
    }
}
