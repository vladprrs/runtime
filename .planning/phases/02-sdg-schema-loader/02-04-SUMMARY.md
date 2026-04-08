---
phase: 02-sdg-schema-loader
plan: 04
subsystem: validation
tags: [sdg, json-schema, error-formatting, integration-tests, fixtures, tdd]

# Dependency graph
requires:
  - phase: 02-sdg-schema-loader (plans 01-03)
    provides: SDG JSON Schema, types, multi-pass validation pipeline, semantic pass, DAG materialization
provides:
  - 10 broken SDG fixtures targeting specific validation error types
  - format_errors() human-readable error report function
  - errors_to_json() machine-readable JSON error output function
  - 16 integration tests verifying full load() pipeline end-to-end
affects: [03-event-store, runtime CLI validation command]

# Tech tracking
tech-stack:
  added: []
  patterns: [fixture-driven integration testing, dual error output format]

key-files:
  created:
    - crates/sdg-loader/fixtures/invalid_missing_required.sdg.json
    - crates/sdg-loader/fixtures/invalid_version.sdg.json
    - crates/sdg-loader/fixtures/invalid_state_reference.sdg.json
    - crates/sdg-loader/fixtures/invalid_duplicate_node.sdg.json
    - crates/sdg-loader/fixtures/invalid_unknown_node_type.sdg.json
    - crates/sdg-loader/fixtures/invalid_context_path.sdg.json
    - crates/sdg-loader/fixtures/invalid_implicit_field.sdg.json
    - crates/sdg-loader/fixtures/invalid_dag_cycle.sdg.json
    - crates/sdg-loader/fixtures/invalid_dangling_edge.sdg.json
    - crates/sdg-loader/fixtures/invalid_type_mismatch.sdg.json
    - crates/sdg-loader/tests/integration.rs
  modified:
    - crates/sdg-loader/src/error.rs

key-decisions:
  - "unknown_node_type fixture caught by schema pass enum constraint, not semantic pass -- adjusted test expectations to match actual pipeline behavior"
  - "error formatting uses writeln! macro per clippy pedantic to avoid extra allocations"

patterns-established:
  - "Fixture-driven integration testing: each broken fixture targets exactly one error class for isolated regression testing"
  - "Dual error output: format_errors for humans, errors_to_json for tooling/CI"

requirements-completed: [SDG-02, SDG-03, SDG-06]

# Metrics
duration: 6min
completed: 2026-04-08
---

# Phase 2, Plan 4: Error Formatting, Broken Fixtures, and Integration Tests Summary

**10 broken SDG fixtures, dual-format error output (human + JSON), and 16 end-to-end integration tests proving the full SDG loader pipeline**

## Performance

- **Duration:** 6 min
- **Started:** 2026-04-08T08:30:35Z
- **Completed:** 2026-04-08T08:37:26Z
- **Tasks:** 2/2
- **Files modified:** 12

## Accomplishments
- Created 10 broken SDG fixture files, each targeting a specific validation error type across all 4 passes
- Implemented `format_errors()` and `errors_to_json()` for dual human-readable and machine-readable error output
- Wrote 16 integration tests exercising the full `load()` pipeline end-to-end against every fixture
- Full workspace test suite passes (86 tests), clippy clean, fmt clean
- Phase 2 SDG loader is now complete: schema validation, version check, semantic validation, DAG materialization, error formatting

## Task Commits

Each task was committed atomically:

1. **Task 1: Broken SDG fixtures** - `f7903b2` (feat)
2. **Task 2: Error formatting + integration tests (TDD RED)** - `c4f0c7e` (test)
3. **Task 2: Error formatting + integration tests (TDD GREEN)** - `7e0139e` (feat)

## Files Created/Modified
- `crates/sdg-loader/fixtures/invalid_missing_required.sdg.json` - Missing service section (schema pass)
- `crates/sdg-loader/fixtures/invalid_version.sdg.json` - Wrong major version 1.0.0 (version pass)
- `crates/sdg-loader/fixtures/invalid_state_reference.sdg.json` - Misspelled state "InProgres" (semantic pass)
- `crates/sdg-loader/fixtures/invalid_duplicate_node.sdg.json` - Two nodes with same ID (semantic pass)
- `crates/sdg-loader/fixtures/invalid_unknown_node_type.sdg.json` - Invalid node type "foobar" (schema pass enum)
- `crates/sdg-loader/fixtures/invalid_context_path.sdg.json` - Invalid context path "actor.name" (semantic pass)
- `crates/sdg-loader/fixtures/invalid_implicit_field.sdg.json` - User-declared "id" field conflicts with implicit (semantic pass)
- `crates/sdg-loader/fixtures/invalid_dag_cycle.sdg.json` - Cyclic DAG: node_a -> node_b -> node_c -> node_a (dag pass)
- `crates/sdg-loader/fixtures/invalid_dangling_edge.sdg.json` - Edge from nonexistent_node (dag pass)
- `crates/sdg-loader/fixtures/invalid_type_mismatch.sdg.json` - Literal node without output_type (semantic pass)
- `crates/sdg-loader/src/error.rs` - Added format_errors() and errors_to_json() functions
- `crates/sdg-loader/tests/integration.rs` - 16 end-to-end integration tests

## Decisions Made
- The `invalid_unknown_node_type` fixture is caught by the JSON schema enum constraint (Pass 1) rather than the semantic pass (Pass 3). The integration test was adjusted to assert `SchemaViolation` errors. This is correct behavior per D-28/D-29: the schema pass catches structural errors before semantic analysis.
- Used `writeln!` macro instead of `format!` + `push_str` per clippy pedantic lint (`format_push_string`).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] unknown_node_type fixture caught at schema pass, not semantic pass**
- **Found during:** Task 2 (integration test design)
- **Issue:** Plan expected `UnknownNodeType` error from semantic pass for `invalid_unknown_node_type.sdg.json`, but the JSON Schema has an enum constraint on `ComputationNode.type` that catches "foobar" at schema level (Pass 1)
- **Fix:** Adjusted integration test to assert `SchemaViolation` errors from schema pass instead of `UnknownNodeType` from semantic pass
- **Files modified:** `crates/sdg-loader/tests/integration.rs`
- **Verification:** Test passes, correctly validating that the schema enum catches invalid node types
- **Committed in:** 7e0139e (Task 2 GREEN commit)

---

**Total deviations:** 1 auto-fixed (1 bug in plan assumptions)
**Impact on plan:** Minimal. The fixture still tests invalid node type detection; it just correctly identifies the schema pass as the catching layer rather than the semantic pass. All error types are still covered.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 2 SDG loader is complete: full 4-pass validation pipeline with typed Rust structures
- SDG files can be loaded, validated, and parsed into `ValidatedSdg` with `ServiceDefinition` + `MaterializedDag`
- 86 tests across unit and integration suites provide comprehensive regression coverage
- Ready for Phase 3 (event store) which will consume `ValidatedSdg` from the loader

## Self-Check: PASSED

- All 13 files verified present
- All 3 commits verified in git log
- 10 broken fixtures confirmed
- 16 integration tests confirmed
- format_errors() and errors_to_json() confirmed in error.rs

---
*Phase: 02-sdg-schema-loader*
*Completed: 2026-04-08*
