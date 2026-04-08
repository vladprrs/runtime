use jsonschema::Validator;

/// The embedded SDG JSON Schema (Draft 2020-12).
pub const SDG_SCHEMA_STR: &str = include_str!("sdg_schema.json");

/// Creates a JSON Schema validator from the embedded SDG schema.
///
/// # Panics
///
/// Panics if the embedded schema is invalid JSON or not a valid
/// Draft 2020-12 schema (programming error, not user error).
pub fn create_validator() -> Validator {
    let schema: serde_json::Value =
        serde_json::from_str(SDG_SCHEMA_STR).expect("embedded SDG schema must be valid JSON");
    jsonschema::draft202012::new(&schema).expect("embedded SDG schema must be valid Draft 2020-12")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_is_valid_json() {
        let result: Result<serde_json::Value, _> = serde_json::from_str(SDG_SCHEMA_STR);
        assert!(result.is_ok(), "embedded schema must be valid JSON");
    }

    #[test]
    fn test_schema_creates_validator() {
        // Must not panic
        let _validator = create_validator();
    }

    #[test]
    fn test_schema_validates_canonical_fixture() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = format!(
            "{manifest_dir}/../../specs/003-sdg-v2-format/examples/task-tracker-extended.sdg.json"
        );
        let content = std::fs::read_to_string(&fixture_path).expect("canonical fixture must exist");
        let value: serde_json::Value =
            serde_json::from_str(&content).expect("fixture must be valid JSON");

        let validator = create_validator();
        let result = validator.validate(&value);
        assert!(
            result.is_ok(),
            "canonical fixture must validate: {:?}",
            validator
                .iter_errors(&value)
                .map(|e| format!("{}: {}", e.instance_path(), e))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_schema_rejects_missing_service() {
        let invalid = serde_json::json!({
            "schema_version": "2.0.0",
            "model": { "aggregates": {} }
        });
        let validator = create_validator();
        let errors: Vec<_> = validator.iter_errors(&invalid).collect();
        assert!(
            !errors.is_empty(),
            "missing 'service' should produce validation errors"
        );
    }
}
