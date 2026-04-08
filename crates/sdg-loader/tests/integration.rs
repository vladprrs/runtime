use std::path::PathBuf;

use sdg_loader::{load, SdgError};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

// === VALID FIXTURE ===

#[test]
fn test_valid_task_tracker_loads_successfully() {
    let result = load(&fixture_path("valid_task_tracker.sdg.json"));
    let validated = result.expect("canonical fixture should load successfully");
    assert_eq!(validated.definition.service.name, "task-tracker-extended");
    assert_eq!(validated.definition.model.aggregates.len(), 2);
    assert!(validated.definition.model.aggregates.contains_key("User"));
    assert!(validated.definition.model.aggregates.contains_key("Task"));
    assert!(
        !validated.dag.topo_order.is_empty(),
        "DAG should have topological order"
    );
}

// === PASS 1: SCHEMA ERRORS ===

#[test]
fn test_invalid_missing_required_caught_by_schema_pass() {
    let result = load(&fixture_path("invalid_missing_required.sdg.json"));
    let errors = result.expect_err("should fail schema validation");
    assert!(!errors.is_empty());
    assert!(
        errors.iter().all(|e| e.pass() == "schema"),
        "all errors should be from schema pass, got: {errors:?}"
    );
    // Should mention missing "service" field
    let has_service_error = errors.iter().any(|e| {
        matches!(e, SdgError::SchemaViolation { message, .. } if message.contains("service") || message.contains("required"))
    });
    assert!(
        has_service_error,
        "should mention missing service: {errors:?}"
    );
}

// === PASS 2: VERSION ERRORS ===

#[test]
fn test_invalid_version_caught_by_version_pass() {
    let result = load(&fixture_path("invalid_version.sdg.json"));
    let errors = result.expect_err("should fail version check");
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].pass(), "version");
    assert!(matches!(&errors[0], SdgError::IncompatibleVersion { .. }));
}

// === PASS 3: SEMANTIC ERRORS ===

#[test]
fn test_invalid_state_reference_with_suggestion() {
    let result = load(&fixture_path("invalid_state_reference.sdg.json"));
    let errors = result.expect_err("should fail semantic validation");
    assert!(
        errors.iter().all(|e| e.pass() == "semantic"),
        "all errors should be from semantic pass, got: {errors:?}"
    );
    let has_state_error = errors.iter().any(|e| {
        matches!(e, SdgError::InvalidStateReference { suggestion, .. } if suggestion.contains("InProgress"))
    });
    assert!(
        has_state_error,
        "should suggest InProgress: {errors:?}"
    );
}

#[test]
fn test_invalid_duplicate_node() {
    let result = load(&fixture_path("invalid_duplicate_node.sdg.json"));
    let errors = result.expect_err("should fail semantic validation");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SdgError::DuplicateNodeId { .. })));
}

#[test]
fn test_invalid_unknown_node_type() {
    // Note: the JSON schema has an enum for node types, so "foobar" is
    // caught by schema validation (Pass 1), not semantic validation (Pass 3).
    let result = load(&fixture_path("invalid_unknown_node_type.sdg.json"));
    let errors = result.expect_err("should fail validation");
    assert!(!errors.is_empty());
    // Schema pass catches this because the enum constraint rejects "foobar"
    assert!(
        errors.iter().all(|e| e.pass() == "schema"),
        "unknown node type should be caught by schema pass due to enum constraint, got: {errors:?}"
    );
}

#[test]
fn test_invalid_context_path() {
    let result = load(&fixture_path("invalid_context_path.sdg.json"));
    let errors = result.expect_err("should fail semantic validation");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SdgError::InvalidContextPath { .. })));
}

#[test]
fn test_invalid_implicit_field() {
    let result = load(&fixture_path("invalid_implicit_field.sdg.json"));
    let errors = result.expect_err("should fail semantic validation");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SdgError::ImplicitFieldConflict { .. })));
}

#[test]
fn test_invalid_literal_missing_output_type() {
    let result = load(&fixture_path("invalid_type_mismatch.sdg.json"));
    let errors = result.expect_err("should fail semantic validation");
    assert!(errors.iter().any(|e| {
        matches!(e, SdgError::SemanticError { message, .. } if message.contains("output_type"))
    }));
}

// === PASS 4: DAG ERRORS ===

#[test]
fn test_invalid_dag_cycle() {
    let result = load(&fixture_path("invalid_dag_cycle.sdg.json"));
    let errors = result.expect_err("should fail DAG validation");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SdgError::DagCycle { .. })));
}

#[test]
fn test_invalid_dangling_edge() {
    let result = load(&fixture_path("invalid_dangling_edge.sdg.json"));
    let errors = result.expect_err("should fail DAG validation");
    assert!(errors
        .iter()
        .any(|e| matches!(e, SdgError::DagEdgeReference { .. })));
}

// === ERROR FORMATTING ===

#[test]
fn test_format_errors_includes_count_and_pass() {
    let result = load(&fixture_path("invalid_missing_required.sdg.json"));
    let errors = result.expect_err("should fail");
    let formatted = sdg_loader::error::format_errors(&errors);
    assert!(
        formatted.contains("error(s)"),
        "should include error count, got: {formatted}"
    );
    assert!(
        formatted.contains("[pass: schema]"),
        "should include pass name, got: {formatted}"
    );
}

#[test]
fn test_format_errors_empty_list() {
    let formatted = sdg_loader::error::format_errors(&[]);
    assert_eq!(formatted, "No errors found.");
}

#[test]
fn test_errors_to_json_produces_array() {
    let result = load(&fixture_path("invalid_version.sdg.json"));
    let errors = result.expect_err("should fail");
    let json = sdg_loader::error::errors_to_json(&errors);
    assert!(json.is_array());
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["pass"], "version");
}

#[test]
fn test_errors_to_json_includes_message() {
    let result = load(&fixture_path("invalid_state_reference.sdg.json"));
    let errors = result.expect_err("should fail");
    let json = sdg_loader::error::errors_to_json(&errors);
    let arr = json.as_array().unwrap();
    assert!(!arr.is_empty());
    for entry in arr {
        assert!(entry.get("pass").is_some(), "each entry should have pass");
        assert!(
            entry.get("message").is_some(),
            "each entry should have message"
        );
    }
}

// === PASS ORDERING: EARLIER PASS FAILURES BLOCK LATER PASSES ===

#[test]
fn test_schema_failure_blocks_semantic_pass() {
    // The missing_required fixture fails at schema, so semantic pass should not run
    let result = load(&fixture_path("invalid_missing_required.sdg.json"));
    let errors = result.expect_err("should fail");
    // All errors should be schema-level; no semantic or dag errors
    for error in &errors {
        assert_eq!(
            error.pass(),
            "schema",
            "only schema errors expected when schema fails, got: {error}"
        );
    }
}
