---
phase: 02-sdg-schema-loader
reviewed: 2026-04-08T09:20:49Z
depth: standard
files_reviewed: 27
files_reviewed_list:
  - Cargo.toml
  - crates/sdg-loader/Cargo.toml
  - crates/sdg-loader/src/error.rs
  - crates/sdg-loader/src/lib.rs
  - crates/sdg-loader/src/schema.rs
  - crates/sdg-loader/src/sdg_schema.json
  - crates/sdg-loader/src/suggestions.rs
  - crates/sdg-loader/src/types.rs
  - crates/sdg-loader/src/validation/dag_pass.rs
  - crates/sdg-loader/src/validation/mod.rs
  - crates/sdg-loader/src/validation/schema_pass.rs
  - crates/sdg-loader/src/validation/semantic_pass.rs
  - crates/sdg-loader/src/validation/version_pass.rs
  - crates/sdg-loader/tests/integration.rs
  - crates/sdg-loader/fixtures/valid_task_tracker.sdg.json
  - crates/sdg-loader/fixtures/invalid_edge_type_mismatch.sdg.json
  - crates/sdg-loader/fixtures/invalid_port_name.sdg.json
  - crates/sdg-loader/fixtures/invalid_dag_cycle.sdg.json
  - crates/sdg-loader/fixtures/invalid_dangling_edge.sdg.json
  - crates/sdg-loader/fixtures/invalid_duplicate_node.sdg.json
  - crates/sdg-loader/fixtures/invalid_context_path.sdg.json
  - crates/sdg-loader/fixtures/invalid_implicit_field.sdg.json
  - crates/sdg-loader/fixtures/invalid_missing_required.sdg.json
  - crates/sdg-loader/fixtures/invalid_state_reference.sdg.json
  - crates/sdg-loader/fixtures/invalid_type_mismatch.sdg.json
  - crates/sdg-loader/fixtures/invalid_unknown_node_type.sdg.json
  - crates/sdg-loader/fixtures/invalid_version.sdg.json
findings:
  critical: 0
  warning: 5
  info: 3
  total: 8
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-04-08T09:20:49Z
**Depth:** standard
**Files Reviewed:** 27
**Status:** issues_found

## Summary

Reviewed the complete `sdg-loader` crate: 5 library source files, 4 validation pass modules, 1 integration test file, and 13 fixtures. All 88 tests pass, clippy is clean at workspace pedantic level, and formatting is correct.

The four-pass validation pipeline (schema, version, semantic, DAG) is well-structured and correctly short-circuits on earlier-pass failures. The JSON Schema is thorough, edge type-checking in Pass 4 is comprehensive, and the test suite has good coverage of both valid and invalid inputs.

Five warnings were found. The most significant are three missing validation checks in the semantic pass that allow invalid SDGs to reach the runtime silently: (1) the `initial_state` override is never validated against the `states` list; (2) a `context` node missing its required `path` param silently passes; (3) `auto_fields` key names are never checked against actual aggregate fields. Two further warnings cover a dead error variant (`InvalidFieldReference`) and an unnecessary full-clone of the raw JSON during deserialization. Three info items cover duplicated constants, a misnamed fixture, and an infallible write suppression.

No critical issues (security, crash, data loss) were found.

## Warnings

### WR-01: `initial_state` override never validated against the `states` list

**File:** `crates/sdg-loader/src/validation/semantic_pass.rs:98-175`
**Issue:** `Aggregate.initial_state` is an optional override that a user can set to any string. The semantic pass validates every `from`/`to` state reference in transitions but has no equivalent check for `initial_state`. A user can write `"initial_state": "Typo"` in their SDG, pass all four validation passes, and the runtime will receive a `ValidatedSdg` carrying an invalid initial state. The JSON Schema does not constrain the value either — it types it as `string` only. The gap is in `validate_aggregates` which sets up `state_names` but never uses it to check `aggregate.initial_state`.
**Fix:**
```rust
// In validate_aggregates, after `let state_names: Vec<&str> = ...` (around line 119):
if let Some(initial) = &aggregate.initial_state {
    if !state_names.contains(&initial.as_str()) {
        errors.push(SdgError::InvalidStateReference {
            path: format!("{path_prefix}.initial_state"),
            name: initial.clone(),
            aggregate: agg_name.clone(),
            suggestion: suggestion_or_empty(initial, &state_names),
        });
    }
}
```
Add a fixture `invalid_initial_state.sdg.json` and a corresponding integration test.

---

### WR-02: `context` node with missing `path` param silently passes semantic validation

**File:** `crates/sdg-loader/src/validation/semantic_pass.rs:194-205`
**Issue:** The context-path validation is wrapped in `if let Some(path_val) = node.params.get("path")...`, so a `context` node with no `path` param at all produces no error. A `context` node without a `path` cannot produce any value at runtime — it is semantically invalid — yet it passes all four validation passes. The JSON Schema does not require `path` on a context node's `params` object either (params is an open schema), so the schema pass does not catch this either.
**Fix:**
```rust
if node.node_type == "context" {
    match node.params.get("path").and_then(|v| v.as_str()) {
        Some(path_val) => {
            let valid_paths: Vec<&str> = VALID_CONTEXT_PATHS.iter().map(|(p, _)| *p).collect();
            if !valid_paths.contains(&path_val) {
                errors.push(SdgError::InvalidContextPath {
                    path: node_path.clone(),
                    context_path: path_val.to_string(),
                    suggestion: suggestion_or_empty(path_val, &valid_paths),
                });
            }
        }
        None => {
            errors.push(SdgError::SemanticError {
                path: node_path.clone(),
                message: "context node is missing required 'path' param".to_string(),
            });
        }
    }
}
```

---

### WR-03: `auto_fields` key names never validated against aggregate field definitions

**File:** `crates/sdg-loader/src/validation/semantic_pass.rs:163-174`
**Issue:** `auto_fields` validation confirms the map value (a computation node ID) exists, but never validates the map key (the aggregate field name to populate). A typo such as `"autor_id": "actor_id"` passes all four validation passes and will cause a silent no-op at runtime. This is directly analogous to guard validation (which correctly catches non-existent node IDs) but is missing for field names.

Additionally, the `SdgError::InvalidFieldReference` variant (defined in `error.rs:85`) is never constructed anywhere in the codebase. This warning and the fix below address both gaps simultaneously.

**Fix:** After the existing node-ID check, add a field-name check:
```rust
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
    // NEW: validate the key is a real aggregate field
    let field_names: Vec<&str> = aggregate.fields.keys().map(String::as_str)
        .chain(IMPLICIT_FIELDS.iter().map(|(n, _)| *n))
        .collect();
    if !field_names.contains(&field_name.as_str()) {
        errors.push(SdgError::InvalidFieldReference {
            path: format!("{trans_path}.auto_fields.{field_name}"),
            name: field_name.clone(),
            aggregate: agg_name.clone(),
            suggestion: suggestion_or_empty(field_name, &field_names),
        });
    }
}
```

---

### WR-04: `validate()` clones entire raw JSON value for deserialization

**File:** `crates/sdg-loader/src/validation/mod.rs:66`
**Issue:** `serde_json::from_value(raw.clone())` clones the full `serde_json::Value` tree because `from_value` requires ownership while `validate` holds `raw` by shared reference (`&serde_json::Value`). For a large SDG this duplicates the entire document in memory. The public `validate` entry point is also less ergonomic than it could be for callers who already own a `Value`.
**Fix:** Change `validate` to take ownership:
```rust
pub fn validate(raw: serde_json::Value) -> Result<ValidatedSdg, Vec<SdgError>> {
    let schema_errors = schema_pass::validate_schema(&raw);
    // ...
    let definition: ServiceDefinition = serde_json::from_value(raw).map_err(|e| { ... })?;
    // ...
}
```
Update `load()` to pass the locally-owned `raw` directly. Tests that call `validate(&raw)` must be updated to `validate(raw)` (clone explicitly before the call only where the test needs `raw` afterward — currently none do).

---

### WR-05: `IMPLICIT_FIELDS` and `VALID_CONTEXT_PATHS` duplicated verbatim across two modules

**File:** `crates/sdg-loader/src/validation/semantic_pass.rs:8-23` and `crates/sdg-loader/src/validation/dag_pass.rs:10-25`
**Issue:** Both constants are declared identically in both files. The semantic pass uses them for validation; the dag pass uses them for output-type resolution. Adding a new implicit field or context path requires updating two locations; a partial update silently produces inconsistent behavior between passes (semantic pass accepts a path that the dag pass cannot resolve a type for).
**Fix:** Move both constants to a shared location, such as a new `crates/sdg-loader/src/catalog.rs` module:
```rust
// crates/sdg-loader/src/catalog.rs
pub const IMPLICIT_FIELDS: &[(&str, &str)] = &[
    ("id", "uuid"),
    ("state", "string"),
    ("created_at", "datetime"),
    ("updated_at", "datetime"),
    ("version", "integer"),
];

pub const VALID_CONTEXT_PATHS: &[(&str, &str)] = &[
    ("actor.id", "uuid"),
    ("actor.email", "string"),
    ("actor.roles", "string[]"),
    ("timestamp", "datetime"),
    ("correlation_id", "uuid"),
];
```
Import from both validation pass modules: `use crate::catalog::{IMPLICIT_FIELDS, VALID_CONTEXT_PATHS};`

---

## Info

### IN-01: Fixture `invalid_type_mismatch.sdg.json` is misnamed

**File:** `crates/sdg-loader/fixtures/invalid_type_mismatch.sdg.json`
**Issue:** The fixture contains a `literal` node missing its `output_type` param. The integration test that loads it (`tests/integration.rs:117`) is correctly named `test_invalid_literal_missing_output_type` and asserts a `SemanticError` about `output_type`, not a `TypeMismatch`. The filename implies it tests the `TypeMismatch` error variant, which is now implemented for edge-level type conflicts. This naming conflict will confuse future contributors adding real type-mismatch fixtures.
**Fix:** Rename to `invalid_literal_missing_output_type.sdg.json` and update the reference at `tests/integration.rs:118`.

---

### IN-02: Dead `let _ =` suppression on an infallible write

**File:** `crates/sdg-loader/src/error.rs:15`
**Issue:**
```rust
let _ = writeln!(output, "  {}. [pass: {}] {}", i + 1, error.pass(), error);
```
`writeln!` on a `String` via `std::fmt::Write` returns `fmt::Result` which is always `Ok(())` — writing to a heap-allocated `String` cannot fail. The `let _ =` suppresses a lint for an error that can never occur, which misleads readers into thinking the write could fail.
**Fix:**
```rust
writeln!(output, "  {}. [pass: {}] {}", i + 1, error.pass(), error)
    .expect("writing to String is infallible");
```
Or use `let () = writeln!(...).unwrap();` — either form makes the infallibility explicit.

---

### IN-03: `schema_pass::validate_schema` recompiles the JSON Schema validator on every call

**File:** `crates/sdg-loader/src/validation/schema_pass.rs:6-7`
**Issue:** `validate_schema` calls `create_validator()` on every invocation. `create_validator()` parses the embedded schema string and compiles it into a `jsonschema::Validator` each time. For the production startup path this is acceptable (called once), but the test suite calls `validate_schema` directly in multiple unit tests. The `jsonschema` documentation recommends compiling the validator once and reusing it.
**Fix:** Cache the compiled validator using `OnceLock`:
```rust
use std::sync::OnceLock;

static VALIDATOR: OnceLock<jsonschema::Validator> = OnceLock::new();

fn get_validator() -> &'static jsonschema::Validator {
    VALIDATOR.get_or_init(create_validator)
}

pub fn validate_schema(raw: &serde_json::Value) -> Vec<SdgError> {
    get_validator()
        .iter_errors(raw)
        .map(|error| SdgError::SchemaViolation { ... })
        .collect()
}
```

---

_Reviewed: 2026-04-08T09:20:49Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
