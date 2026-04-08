---
phase: 02-sdg-schema-loader
plan: 03
subsystem: sdg-loader
tags: [petgraph, strsim, semantic-validation, dag, topological-sort, tdd]

# Dependency graph
requires:
  - phase: 02-sdg-schema-loader (plan 02)
    provides: Schema pass, version pass, typed ServiceDefinition, canonical fixture, error types
provides:
  - "Did you mean?" suggestion engine using strsim normalized Damerau-Levenshtein
  - Semantic validation pass (Pass 3) checking state refs, node types, context paths, implicit fields
  - DAG materialization pass (Pass 4) with petgraph DiGraph and pre-computed topological order
  - ValidatedSdg type bundling ServiceDefinition + MaterializedDag
  - Full 4-pass validation pipeline wired in strict order
affects: [aggregate-engine, api-surface, runtime]

# Tech tracking
tech-stack:
  added: [strsim, petgraph]
  patterns: [multi-pass-validation-pipeline, error-collection-per-pass, did-you-mean-suggestions]

key-files:
  created:
    - crates/sdg-loader/src/suggestions.rs
    - crates/sdg-loader/src/validation/semantic_pass.rs
    - crates/sdg-loader/src/validation/dag_pass.rs
  modified:
    - crates/sdg-loader/src/lib.rs
    - crates/sdg-loader/src/validation/mod.rs
    - specs/003-sdg-v2-format/examples/task-tracker-extended.sdg.json

key-decisions:
  - "Split validate_semantics into 3 helper functions to satisfy clippy too_many_lines lint"
  - "MaterializedDag stores petgraph DiGraph<String, String> with node IDs and port names"
  - "ValidatedSdg returned from load() bundles definition + materialized DAG"
  - "Fixed spec example literal node to include output_type per D-36"

patterns-established:
  - "Suggestion engine: suggest_similar() with 0.6 normalized Damerau-Levenshtein threshold"
  - "Validation passes: each returns Vec<SdgError>, pipeline halts on non-empty"
  - "DAG materialization: petgraph toposort for cycle detection + evaluation order"
  - "ValidatedSdg: load() result bundles definition + materialized DAG for downstream consumers"

requirements-completed: [SDG-03, SDG-05]

# Metrics
duration: 10min
completed: 2026-04-08
---

# Phase 02 Plan 03: Semantic Validation, DAG Materialization, and Pipeline Wiring Summary

**Full 4-pass validation pipeline with semantic cross-reference checks, "did you mean?" suggestions via strsim, and petgraph DAG materialization with cycle detection and topological ordering**

## Performance

- **Duration:** 10 min
- **Started:** 2026-04-08T08:16:31Z
- **Completed:** 2026-04-08T08:27:04Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments
- Suggestions module provides "did you mean?" hints using strsim for state names, node types, context paths
- Semantic pass validates 9 cross-reference check types: state refs (from/to), guard refs, auto_fields refs, node types, duplicate IDs, context paths, implicit fields, field-node names, literal output_type
- DAG materialization builds petgraph DiGraph, detects cycles via toposort, produces pre-computed evaluation order
- Full pipeline wired: schema -> version -> semantic -> DAG with halt-on-failure at each pass
- load() returns ValidatedSdg with both ServiceDefinition and MaterializedDag

## Task Commits

Each task was committed atomically:

1. **Task 1: Suggestions module and semantic validation pass**
   - `b10094e` (test: TDD RED -- 15 failing tests)
   - `d6bd204` (feat: TDD GREEN -- implementation passing all tests)
2. **Task 2: DAG materialization pass and pipeline wiring**
   - `820ae5c` (test: TDD RED -- 7 failing tests)
   - `47b7c76` (feat: TDD GREEN -- implementation + pipeline wiring)

_TDD tasks have RED + GREEN commits as required._

## Files Created/Modified
- `crates/sdg-loader/src/suggestions.rs` - "Did you mean?" engine with 0.6 similarity threshold
- `crates/sdg-loader/src/validation/semantic_pass.rs` - Pass 3: semantic cross-reference validation
- `crates/sdg-loader/src/validation/dag_pass.rs` - Pass 4: DAG materialization with petgraph + toposort
- `crates/sdg-loader/src/validation/mod.rs` - Pipeline wired with all 4 passes, ValidatedSdg type
- `crates/sdg-loader/src/lib.rs` - Re-exports for suggestions, ValidatedSdg, MaterializedDag
- `specs/003-sdg-v2-format/examples/task-tracker-extended.sdg.json` - Fixed literal node missing output_type

## Decisions Made
- Split `validate_semantics` into 3 helper functions (`validate_duplicate_node_ids`, `validate_aggregates`, `validate_computation_nodes`) to satisfy clippy `too_many_lines` lint while keeping the public API clean
- `MaterializedDag` stores `DiGraph<String, String>` where nodes are computation node IDs and edges are port names -- simple and sufficient for Phase 4 consumption
- `ValidatedSdg` struct in `validation/mod.rs` bundles `ServiceDefinition` + `MaterializedDag` as the pipeline output type

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed spec example literal node missing output_type**
- **Found during:** Task 2 (pipeline wiring)
- **Issue:** `specs/003-sdg-v2-format/examples/task-tracker-extended.sdg.json` literal node `zero` had `{ "value": 0 }` without `output_type` param, causing semantic validation failure per D-36
- **Fix:** Added `"output_type": "integer"` to the `zero` node params
- **Files modified:** specs/003-sdg-v2-format/examples/task-tracker-extended.sdg.json
- **Verification:** Canonical fixture now passes all 4 validation passes
- **Committed in:** 47b7c76 (part of Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Essential fix for D-36 compliance. The spec example was inconsistent with the validation rules. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Full 4-pass validation pipeline operational and ready for Phase 3+ consumption
- `ValidatedSdg` provides both typed definition and materialized DAG for the aggregate engine
- Pre-computed topological order ready for Phase 4 DAG interpreter

---
## Self-Check: PASSED

All created files verified present. All commit hashes verified in git log.

---
*Phase: 02-sdg-schema-loader*
*Completed: 2026-04-08*
