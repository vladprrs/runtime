use std::fmt::Write as _;
use std::path::PathBuf;
use thiserror::Error;

/// Format a list of SDG errors into a human-readable report.
/// Each error includes: pass identification, JSON path, message, and suggestions.
pub fn format_errors(errors: &[SdgError]) -> String {
    if errors.is_empty() {
        return String::from("No errors found.");
    }

    let mut output = format!("SDG validation failed with {} error(s):\n\n", errors.len());

    for (i, error) in errors.iter().enumerate() {
        let _ = writeln!(output, "  {}. [pass: {}] {}", i + 1, error.pass(), error);
    }

    output
}

/// Format errors as a JSON array for machine-readable output (per D-12 research).
/// Returns a `serde_json::Value` array of error objects.
pub fn errors_to_json(errors: &[SdgError]) -> serde_json::Value {
    serde_json::Value::Array(
        errors
            .iter()
            .map(|e| {
                serde_json::json!({
                    "pass": e.pass(),
                    "message": e.to_string(),
                })
            })
            .collect(),
    )
}

/// All errors the SDG loader can produce, organized by validation pass.
#[derive(Debug, Error)]
pub enum SdgError {
    // --- File system errors ---
    #[error("Failed to read SDG file '{path}': {source}")]
    FileRead {
        path: PathBuf,
        source: std::io::Error,
    },

    // --- JSON parse errors ---
    #[error("Invalid JSON in SDG file: {message}")]
    JsonParse { message: String },

    // --- Pass 1: Schema violations ---
    #[error("[schema] {instance_path}: {message}")]
    SchemaViolation {
        instance_path: String,
        schema_path: String,
        message: String,
    },

    // --- Pass 2: Version errors ---
    #[error("Missing 'schema_version' field in SDG")]
    MissingVersion,

    #[error("Invalid schema version '{value}': {reason}")]
    InvalidVersion { value: String, reason: String },

    #[error(
        "Incompatible schema version {found} (runtime supports major version {expected_major})"
    )]
    IncompatibleVersion { found: String, expected_major: u64 },

    // --- Deserialization errors (between pass 2 and 3) ---
    #[error("Failed to deserialize SDG into typed structures: {message}")]
    Deserialization { message: String },

    // --- Pass 3: Semantic errors ---
    #[error("[semantic] {path}: state '{name}' not found in aggregate '{aggregate}'{suggestion}")]
    InvalidStateReference {
        path: String,
        name: String,
        aggregate: String,
        suggestion: String,
    },

    #[error("[semantic] {path}: field '{name}' not found in aggregate '{aggregate}'{suggestion}")]
    InvalidFieldReference {
        path: String,
        name: String,
        aggregate: String,
        suggestion: String,
    },

    #[error("[semantic] {path}: type mismatch on edge - expected {expected}, found {found}")]
    TypeMismatch {
        path: String,
        expected: String,
        found: String,
    },

    #[error("[semantic] {path}: {message}")]
    SemanticError { path: String, message: String },

    #[error("[semantic] {path}: unknown node type '{node_type}'{suggestion}")]
    UnknownNodeType {
        path: String,
        node_type: String,
        suggestion: String,
    },

    #[error("[semantic] {path}: duplicate node ID '{node_id}'")]
    DuplicateNodeId { path: String, node_id: String },

    #[error("[semantic] {path}: unknown context path '{context_path}'{suggestion}")]
    InvalidContextPath {
        path: String,
        context_path: String,
        suggestion: String,
    },

    #[error("[semantic] {path}: implicit field name '{name}' cannot be declared by user")]
    ImplicitFieldConflict { path: String, name: String },

    // --- Pass 4: DAG errors ---
    #[error("[dag] Cycle detected involving node '{node}' in computation graph")]
    DagCycle { node: String },

    #[error("[dag] Edge references non-existent node: {from} -> {to}")]
    DagEdgeReference { from: String, to: String },

    #[error("[dag] Edge targets non-existent port '{port}' on node '{node}' (type: {node_type})")]
    DagInvalidPort {
        port: String,
        node: String,
        node_type: String,
    },
}

impl SdgError {
    /// Returns which validation pass caught this error.
    pub fn pass(&self) -> &'static str {
        match self {
            Self::FileRead { .. } | Self::JsonParse { .. } => "file",
            Self::SchemaViolation { .. } => "schema",
            Self::MissingVersion
            | Self::InvalidVersion { .. }
            | Self::IncompatibleVersion { .. } => "version",
            Self::Deserialization { .. } => "deserialization",
            Self::InvalidStateReference { .. }
            | Self::InvalidFieldReference { .. }
            | Self::TypeMismatch { .. }
            | Self::SemanticError { .. }
            | Self::UnknownNodeType { .. }
            | Self::DuplicateNodeId { .. }
            | Self::InvalidContextPath { .. }
            | Self::ImplicitFieldConflict { .. } => "semantic",
            Self::DagCycle { .. } | Self::DagEdgeReference { .. } | Self::DagInvalidPort { .. } => {
                "dag"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_schema_violation() {
        let err = SdgError::SchemaViolation {
            instance_path: "/service".into(),
            schema_path: "".into(),
            message: "missing field".into(),
        };
        let display = format!("{err}");
        assert!(
            display.contains("[schema]"),
            "expected '[schema]' in: {display}"
        );
        assert!(
            display.contains("/service"),
            "expected '/service' in: {display}"
        );
    }

    #[test]
    fn test_error_pass_identification() {
        assert_eq!(SdgError::MissingVersion.pass(), "version");
        assert_eq!(SdgError::DagCycle { node: "x".into() }.pass(), "dag");
        assert_eq!(
            SdgError::SchemaViolation {
                instance_path: String::new(),
                schema_path: String::new(),
                message: String::new(),
            }
            .pass(),
            "schema"
        );
        assert_eq!(
            SdgError::InvalidStateReference {
                path: String::new(),
                name: String::new(),
                aggregate: String::new(),
                suggestion: String::new(),
            }
            .pass(),
            "semantic"
        );
    }
}
