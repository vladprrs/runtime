use crate::error::SdgError;

/// The major version this runtime supports.
pub const SUPPORTED_MAJOR_VERSION: u64 = 2;

/// Pass 2: Check schema_version field for SemVer compatibility.
/// Major version must match SUPPORTED_MAJOR_VERSION. Minor/patch differences accepted.
pub fn validate_version(_raw: &serde_json::Value) -> Vec<SdgError> {
    todo!("RED: implement version validation")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json_with_version(version: &str) -> serde_json::Value {
        serde_json::json!({ "schema_version": version })
    }

    #[test]
    fn test_matching_major_version() {
        let raw = json_with_version("2.0.0");
        let errors = validate_version(&raw);
        assert!(errors.is_empty(), "2.0.0 should pass, got: {errors:?}");
    }

    #[test]
    fn test_matching_major_different_minor() {
        let raw = json_with_version("2.1.0");
        let errors = validate_version(&raw);
        assert!(errors.is_empty(), "2.1.0 should pass, got: {errors:?}");
    }

    #[test]
    fn test_matching_major_different_patch() {
        let raw = json_with_version("2.0.1");
        let errors = validate_version(&raw);
        assert!(errors.is_empty(), "2.0.1 should pass, got: {errors:?}");
    }

    #[test]
    fn test_wrong_major_version() {
        let raw = json_with_version("1.0.0");
        let errors = validate_version(&raw);
        assert_eq!(errors.len(), 1, "wrong major should produce 1 error");
        assert!(
            matches!(&errors[0], SdgError::IncompatibleVersion { found, expected_major }
                if found == "1.0.0" && *expected_major == 2),
            "expected IncompatibleVersion, got: {:?}",
            errors[0]
        );
    }

    #[test]
    fn test_future_major_version() {
        let raw = json_with_version("3.0.0");
        let errors = validate_version(&raw);
        assert_eq!(errors.len(), 1, "future major should produce 1 error");
        assert!(
            matches!(&errors[0], SdgError::IncompatibleVersion { .. }),
            "expected IncompatibleVersion"
        );
    }

    #[test]
    fn test_missing_version_field() {
        let raw = serde_json::json!({});
        let errors = validate_version(&raw);
        assert_eq!(errors.len(), 1, "missing version should produce 1 error");
        assert!(
            matches!(&errors[0], SdgError::MissingVersion),
            "expected MissingVersion, got: {:?}",
            errors[0]
        );
    }

    #[test]
    fn test_invalid_semver_string() {
        let raw = json_with_version("not-a-version");
        let errors = validate_version(&raw);
        assert_eq!(errors.len(), 1, "invalid semver should produce 1 error");
        assert!(
            matches!(&errors[0], SdgError::InvalidVersion { value, .. } if value == "not-a-version"),
            "expected InvalidVersion, got: {:?}",
            errors[0]
        );
    }
}
