---
phase: 02-sdg-schema-loader
reviewed: 2026-04-08T00:00:00Z
depth: standard
files_reviewed: 26
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
  - crates/sdg-loader/fixtures/invalid_context_path.sdg.json
  - crates/sdg-loader/fixtures/invalid_dag_cycle.sdg.json
  - crates/sdg-loader/fixtures/invalid_dangling_edge.sdg.json
  - crates/sdg-loader/fixtures/invalid_duplicate_node.sdg.json
  - crates/sdg-loader/fixtures/invalid_implicit_field.sdg.json
  - crates/sdg-loader/fixtures/invalid_missing_required.sdg.json
  - crates/sdg-loader/fixtures/invalid_state_reference.sdg.json
  - crates/sdg-loader/fixtures/invalid_type_mismatch.sdg.json
  - crates/sdg-loader/fixtures/invalid_unknown_node_type.sdg.json
  - crates/sdg-loader/fixtures/invalid_version.sdg.json
  - specs/003-sdg-v2-format/examples/task-tracker-extended.sdg.json
findings:
  critical: 0
  warning: 5
  info: 4
  total: 9
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-04-08
**Depth:** standard
**Files Reviewed:** 26
**Status:** issues_found

## Summary

Reviewed the full `sdg-loader` crate: types, error definitions, all four validation passes (schema, version, semantic, DAG), the JSON Schema itself, integration tests, and all fixture files.

The overall architecture is clean and well-structured. The four-pass pipeline with early exit is correctly implemented. The JSON Schema is thorough and the test suite covers the happy path and all defined error variants.

Three categories of issues were found:

1. **Unreachable error variants** — `DagInvalidPort`, `InvalidFieldReference`, and `TypeMismatch` are defined in `error.rs` but never constructed. This will cause clippy `dead_code` warnings and creates confusion about what validation is actually enforced.
2. **Silent validation gaps** — a `context` node with no `path` param silently passes, and a `field` node with an unresolvable name silently passes. Both are logic errors in the semantic pass.
3. **Misleading fixture naming** — `invalid_type_mismatch.sdg.json` does not test a `TypeMismatch` error; it tests `SemanticError` about a missing `output_type`. This creates confusion about what the file tests.

No critical (security, crash, data loss) issues were found.

---

## Warnings

### WR-01: Dead error variants — `DagInvalidPort`, `InvalidFieldReference`, `TypeMismatch` never constructed

**File:** `crates/sdg-loader/src/error.rs:85-97` and `crates/sdg-loader/src/error.rs:130-135`

**Issue:** Three error variants are declared in `SdgError` but no code in the crate ever constructs them:
- `InvalidFieldReference` (line 85) — intended for cross-referencing field names in computation nodes
- `TypeMismatch` (line 93) — intended for type-level edge compatibility checking
- `DagInvalidPort` (line 130) — intended for validating port names on target nodes

These are matched in `pass()` (lines 148, 155) which prevents compiler dead-code warnings on that arm, but no call site ever creates these variants. Clippy `pedantic` (enabled workspace-wide) will flag these as `dead_code` warnings unless the crate is suppressing them. More importantly, the validation they represent is silently absent.

**Fix:** Either implement the missing validation passes that produce these errors, or remove the variants (and their `pass()` match arms) until the validation is implemented. If deferring to a future phase, replace the dead variants with a `// TODO(phaseN): not yet implemented` comment at minimum. Do not leave unreachable variants in an enum that is already used in production error reporting.

---

### WR-02: `context` node with missing `path` param silently passes semantic validation

**File:** `crates/sdg-loader/src/validation/semantic_pass.rs:194-205`

**Issue:** The context-path validation block is:

```rust
if node.node_type == "context" {
    if let Some(path_val) = node.params.get("path").and_then(|v| v.as_str()) {
        // validate path_val...
    }
}
```

The outer `if let Some(...)` means a `context` node that has no `path` param at all silently passes. A context node without a `path` is semantically invalid — the node cannot produce any value — but the validator emits no error. The JSON Schema does not require `path` on a context node either (the `params` object is an open schema), so this gap is not caught at the schema pass either.

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

Add a corresponding fixture `invalid_context_missing_path.sdg.json` and integration test.

---

### WR-03: `field` node name not validated against aggregate fields

**File:** `crates/sdg-loader/src/validation/semantic_pass.rs:207-217`

**Issue:** The field-node validation only checks for an empty `name` string:

```rust
if node.node_type == "field" {
    if let Some(field_name) = node.params.get("name").and_then(|v| v.as_str()) {
        if field_name.is_empty() {
            errors.push(SdgError::SemanticError { ... "empty 'name' param" });
        }
    }
}
```

It does not verify that the named field actually exists on any aggregate. A `field` node with `"name": "nonexistent_field"` will silently pass all four validation passes and only fail at runtime. This is the same class of problem as a guard referencing a nonexistent computation node (which is correctly caught). Additionally, a missing `name` param is not flagged at all — only an empty string is checked.

**Fix:** Cross-reference the field name against the set of all aggregate field names (including implicit fields from `IMPLICIT_FIELDS`) and report `SemanticError` for unknown names. At minimum, also check for the absent `name` param case:

```rust
if node.node_type == "field" {
    match node.params.get("name").and_then(|v| v.as_str()) {
        None => {
            errors.push(SdgError::SemanticError {
                path: node_path.clone(),
                message: "field node is missing required 'name' param".to_string(),
            });
        }
        Some("") => {
            errors.push(SdgError::SemanticError {
                path: node_path.clone(),
                message: "field node has empty 'name' param".to_string(),
            });
        }
        Some(field_name) => {
            let all_field_names: HashSet<&str> = definition
                .model
                .aggregates
                .values()
                .flat_map(|agg| agg.fields.keys().map(String::as_str))
                .chain(IMPLICIT_FIELDS.iter().map(|(name, _)| *name))
                .collect();
            if !all_field_names.contains(field_name) {
                errors.push(SdgError::SemanticError {
                    path: node_path.clone(),
                    message: format!(
                        "field node references unknown field '{}'{}",
                        field_name,
                        suggestion_or_empty(
                            field_name,
                            &all_field_names.into_iter().collect::<Vec<_>>(),
                        )
                    ),
                });
            }
        }
    }
}
```

Note: `validate_computation_nodes` currently only receives `&ServiceDefinition`, so it already has access to `definition.model.aggregates`.

---

### WR-04: `validate` clones entire raw JSON value unnecessarily

**File:** `crates/sdg-loader/src/validation/mod.rs:66`

**Issue:**

```rust
let definition: ServiceDefinition = serde_json::from_value(raw.clone()).map_err(|e| { ... })?;
```

`serde_json::from_value` requires an owned `Value`. Since `raw` is `&serde_json::Value`, a full clone is made. For the startup-time `load()` path this is acceptable, but `validate()` is a public API that could be called with an already-owned value. The clone duplicates the entire JSON document (potentially large) just to feed `serde_json::from_value`.

**Fix:** Change the `validate` signature to accept ownership of the raw value:

```rust
pub fn validate(raw: serde_json::Value) -> Result<ValidatedSdg, Vec<SdgError>>
```

Update `load()` to pass ownership:
```rust
let raw: serde_json::Value = serde_json::from_str(&content)...?;
validate(raw)
```

This eliminates the clone and makes the ownership model explicit. Update call sites in tests — `validate(&raw)` becomes `validate(raw)` (tests that need to reuse `raw` can clone before the call explicitly).

---

### WR-05: `auto_fields` target field names not validated against aggregate fields

**File:** `crates/sdg-loader/src/validation/semantic_pass.rs:163-174`

**Issue:** `auto_fields` validation checks that the value (computation node ID) exists, but not that the key (the aggregate field name being populated) is a real field on the aggregate:

```rust
for (field_name, node_id) in &transition.auto_fields {
    if !node_ids.contains(&node_id.as_str()) {
        // reports error for unknown node_id
    }
    // field_name is never validated
}
```

A typo in an `auto_fields` key such as `"autor_id": "actor_id"` passes validation silently and will cause a runtime failure. This is directly analogous to WR-03 and represents a missing cross-reference check.

**Fix:** After confirming `node_id` is valid, also verify `field_name` exists in `aggregate.fields` (or `IMPLICIT_FIELDS`):

```rust
if !aggregate.fields.contains_key(field_name)
    && !IMPLICIT_FIELDS.iter().any(|(n, _)| *n == field_name.as_str())
{
    errors.push(SdgError::InvalidFieldReference {
        path: format!("{trans_path}.auto_fields.{field_name}"),
        name: field_name.clone(),
        aggregate: agg_name.clone(),
        suggestion: suggestion_or_empty(
            field_name,
            &aggregate.fields.keys().map(String::as_str).collect::<Vec<_>>(),
        ),
    });
}
```

This would also put the `InvalidFieldReference` error variant (currently dead — see WR-01) to use.

---

## Info

### IN-01: Fixture `invalid_type_mismatch.sdg.json` is misnamed

**File:** `crates/sdg-loader/fixtures/invalid_type_mismatch.sdg.json`

**Issue:** The fixture contains a `literal` node missing its `output_type` param. The corresponding integration test (`tests/integration.rs:117`) is correctly named `test_invalid_literal_missing_output_type`. The fixture name implies it tests the `TypeMismatch` error variant, but it tests `SemanticError { message: "...output_type..." }`. This creates confusion for future contributors trying to understand what each fixture tests.

**Fix:** Rename to `invalid_literal_missing_output_type.sdg.json` and update the reference in `tests/integration.rs:118`. Once the `TypeMismatch` variant (WR-01) is implemented, create a proper `invalid_type_mismatch.sdg.json` for that scenario.

---

### IN-02: `valid_task_tracker.sdg.json` duplicates the canonical spec fixture

**File:** `crates/sdg-loader/fixtures/valid_task_tracker.sdg.json`

**Issue:** This fixture is byte-for-byte identical to `specs/003-sdg-v2-format/examples/task-tracker-extended.sdg.json`. The `validation/mod.rs` tests use both paths as if they are distinct documents. Tests in `schema.rs` and `validation/mod.rs` use the canonical spec path directly; tests in `tests/integration.rs` use the loader fixture. As the spec evolves, these two files can diverge silently — the spec gets updated but the loader fixture does not, causing tests to pass on a stale fixture.

**Fix:** Have the loader fixture path point to the canonical spec example (or symlink it), or add a CI check that asserts the two files are identical. The duplicate should not exist as a copied file.

---

### IN-03: Dead `let _ =` discards infallible `writeln!` result

**File:** `crates/sdg-loader/src/error.rs:15`

**Issue:**

```rust
let _ = writeln!(output, "  {}. [pass: {}] {}", i + 1, error.pass(), error);
```

`writeln!` on a `String` (via `std::fmt::Write`) returns `fmt::Result` which is always `Ok(())` — writing to a `String` cannot fail. The `let _ =` suppression is unnecessary. Clippy pedantic may flag this as `unused_must_use` or it may pass because `let _` is an explicit discard. Either way the code is misleading — it suggests the write could fail.

**Fix:** Use the `write!`/`writeln!` directly without capturing the result, or use `push_str`:

```rust
writeln!(output, "  {}. [pass: {}] {}", i + 1, error.pass(), error).unwrap();
// or simply:
output.push_str(&format!("  {}. [pass: {}] {}\n", i + 1, error.pass(), error));
```

The `.unwrap()` form is idiomatic for infallible writes and makes the intent explicit.

---

### IN-04: `schema_pass` creates a new validator on every call

**File:** `crates/sdg-loader/src/validation/schema_pass.rs:7`

**Issue:**

```rust
pub fn validate_schema(raw: &serde_json::Value) -> Vec<SdgError> {
    let validator = create_validator();
    // ...
}
```

`create_validator()` parses the embedded schema JSON and compiles a `jsonschema::Validator` on every call. For the production startup path (`load()` called once), this is acceptable. However, the test suite calls `validate_schema` directly in multiple unit tests, each creating a new validator. The `jsonschema` crate documentation recommends creating a validator once and reusing it.

**Fix:** Consider using `std::sync::OnceLock` to cache the compiled validator:

```rust
use std::sync::OnceLock;

static VALIDATOR: OnceLock<jsonschema::Validator> = OnceLock::new();

fn get_validator() -> &'static jsonschema::Validator {
    VALIDATOR.get_or_init(|| {
        let schema: serde_json::Value =
            serde_json::from_str(SDG_SCHEMA_STR).expect("embedded SDG schema must be valid JSON");
        jsonschema::draft202012::new(&schema)
            .expect("embedded SDG schema must be valid Draft 2020-12")
    })
}
```

This is not a correctness issue for single-threaded startup, but matters for test isolation and potential future multi-call patterns.

---

_Reviewed: 2026-04-08_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
