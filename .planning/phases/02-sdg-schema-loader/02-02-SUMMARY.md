---
phase: 02-sdg-schema-loader
plan: 02
subsystem: sdg-loader
tags: [validation, schema, version, fixture, tdd]
dependency_graph:
  requires: [02-01]
  provides: [load, validate, validate_schema, validate_version, task-tracker-fixture]
  affects: [02-03, 02-04]
tech_stack:
  added: []
  patterns: [validation-pipeline, pass-based-validation, tdd-red-green]
key_files:
  created:
    - crates/sdg-loader/src/validation/mod.rs
    - crates/sdg-loader/src/validation/schema_pass.rs
    - crates/sdg-loader/src/validation/version_pass.rs
    - crates/sdg-loader/fixtures/valid_task_tracker.sdg.json
  modified:
    - crates/sdg-loader/src/lib.rs
decisions:
  - Validation passes share a Vec<SdgError> return type; load() and validate() return Result<ServiceDefinition, Vec<SdgError>>
  - validate() works on raw JSON without file I/O, enabling testing without temp files
  - let-else pattern used for version_pass to satisfy clippy pedantic
metrics:
  duration: 5 minutes
  completed: 2026-04-08T08:12:20Z
  tasks_completed: 2
  tasks_total: 2
  tests_added: 25
  tests_total: 35
---

# Phase 02 Plan 02: Validation Pipeline and Task Tracker Fixture Summary

Validation pipeline with schema pass (Pass 1) and version pass (Pass 2), load() entry point, and canonical task tracker fixture with 2 aggregates, 23 computation nodes, 25 edges, guards, and auto_fields.

## Tasks Completed

### Task 1: Validation pipeline with schema pass and version pass

| Commit | Type | Description |
|--------|------|-------------|
| 043148f | test | Failing tests for validation pipeline, schema pass, version pass (RED) |
| 04735e3 | feat | Implementation of schema_pass, version_pass, load(), validate() (GREEN) |

**What was built:**
- `schema_pass::validate_schema()` -- validates raw JSON against embedded SDG JSON Schema using `jsonschema` crate's `iter_errors`, maps each `ValidationError` to an owned `SdgError::SchemaViolation` to avoid lifetime issues
- `version_pass::validate_version()` -- parses `schema_version` field with `semver::Version`, checks major version matches `SUPPORTED_MAJOR_VERSION` (2), accepts minor/patch differences per D-03
- `validation::load()` -- reads file, parses JSON, runs pass 1 then pass 2, deserializes into `ServiceDefinition`
- `validation::validate()` -- same pipeline but operates on `serde_json::Value` directly (no file I/O)
- Pipeline halts on first failing pass per D-29

**Tests (15 new):**
- 3 schema_pass tests: valid SDG passes, missing service fails, multiple violations collected
- 7 version_pass tests: matching major, different minor/patch, wrong major, future major, missing field, invalid semver
- 5 integration tests: load canonical fixture, nonexistent file, schema failure prevents version check, wrong version after valid schema, validate from raw JSON

### Task 2: Canonical task tracker fixture

| Commit | Type | Description |
|--------|------|-------------|
| 227ad9c | test | Failing fixture tests (RED) |
| ac50d25 | feat | Fixture file and fixture tests passing (GREEN) |

**What was built:**
- Canonical fixture at `crates/sdg-loader/fixtures/valid_task_tracker.sdg.json`
- Based on spec example with D-36 compliance: `zero` literal node has `"output_type": "integer"`
- 2 aggregates: User (Active, Deactivated) and Task (Created, InProgress, Done, Cancelled)
- 6 user-defined Task fields: title, description, author_id, assignee_id, priority, linked_task_ids
- 23 computation nodes covering: context, field, command, lookup, lookup_many, filter, count, literal, eq, neq, contains, not, and
- 25 computation edges with named ports and variadic indexes
- 9 Task transitions with guards (can_complete, cmd_assignee_active, can_link, is_linked) and auto_fields (author_id -> actor_id)
- 3 User transitions using $same sentinel for UpdateProfile

**Tests (10 new):**
- Schema validation, deserialization, Task states, User states, Task fields (6 fields)
- Computation nodes (>= 22) and edges (>= 22), key node existence
- Complete transition guard, Create auto_fields, load from file path

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed jsonschema API: methods vs fields**
- **Found during:** Task 1 GREEN phase
- **Issue:** Plan showed `error.instance_path` and `error.schema_path` as field access, but jsonschema 0.45 exposes them as methods
- **Fix:** Changed to `error.instance_path()` and `error.schema_path()`
- **Files modified:** `crates/sdg-loader/src/validation/schema_pass.rs`
- **Commit:** 04735e3

**2. [Rule 1 - Bug] Fixed clippy pedantic: manual_let_else and SemVer doc link**
- **Found during:** Task 1 GREEN phase
- **Issue:** `match` with early return flagged as `manual_let_else` by clippy pedantic; doc comment needed `SemVer` in backticks
- **Fix:** Converted to `let...else` syntax; wrapped SemVer in backticks
- **Files modified:** `crates/sdg-loader/src/validation/version_pass.rs`
- **Commit:** 04735e3

## Known Stubs

| File | Line | Description | Resolution |
|------|------|-------------|------------|
| crates/sdg-loader/src/validation/mod.rs | 59-60 | Pass 3 and Pass 4 placeholder comments | Plan 03 wires semantic and DAG validation |

These are intentional comment placeholders, not runtime stubs. The validate() function returns Ok() after Pass 2 until Plan 03 adds the remaining passes.

## Verification

- `cargo test -p sdg-loader` -- 35 tests pass (25 new + 10 from Plan 01)
- `cargo clippy -p sdg-loader -- -D warnings` -- clean
- `cargo fmt --check` -- clean
- `load()` with canonical fixture returns `Ok(ServiceDefinition)` with correct service name and 2 aggregates
- `load()` with invalid JSON returns schema errors (not version errors)
- `validate_version` rejects major version mismatches

## Self-Check: PASSED

All 5 created/modified files verified on disk. All 4 commits found in git log. All 13 acceptance criteria matched via grep. 35 tests passing, clippy clean, fmt clean.
