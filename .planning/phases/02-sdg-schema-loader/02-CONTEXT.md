# Phase 2: SDG Schema & Loader - Context

**Gathered:** 2026-04-07
**Status:** Ready for planning

<domain>
## Phase Boundary

Define the SDG JSON Schema (Draft 2020-12), build a multi-pass validator, parse SDG files into typed Rust structs with `ServiceDefinition` as root, materialize computation DAGs via petgraph with pre-computed topological order, and create a realistic task tracker example SDG as canonical test fixture. Rejecting invalid definitions at startup with clear, rich error messages.

</domain>

<decisions>
## Implementation Decisions

### SDG Schema Scope
- **D-01:** Full JSON Schema covering all 6-pager sections (service, glossary, aggregates, transitions, projections, endpoints, external_dependencies, processes). MVP loader validates and parses only MVP sections. Deferred sections (TS blocks, Decision Tables, Integration Calls, BPMN processes) are schema-valid but loader logs warnings and ignores them at runtime.
- **D-02:** Schema defined inline in Rust code (no separate schema file in repo). Embedded in the binary via const or string literal. Schema is version-locked to the runtime binary.
- **D-03:** SemVer version field in SDG (`"schema_version": "1.0.0"`). Runtime checks major version match; minor/patch differences accepted.
- **D-04:** SDG file loaded from `./service.sdg.json` by default. Overridable via CLI argument (`runtime --sdg path/to/other.sdg.json`). Runtime requires the SDG file to exist — fails fast if missing.
- **D-05:** Single monolithic schema with internal `$defs` sections (no split files with cross-file `$ref`).

### Task Tracker Example
- **D-06:** Realistic complexity — Task aggregate with 3 states (Created, InProgress, Done), guard conditions (e.g., can't complete without assignee), 1-2 projections (task list, task count by status), command/query endpoints.
- **D-07:** Valid task tracker SDG plus 5-10 intentionally broken SDG fixtures, each triggering a specific validation error (missing field, DAG cycle, type mismatch, invalid state reference, version incompatibility, etc.).
- **D-08:** All test fixture SDGs live in `crates/sdg-loader/fixtures/`.

### Validation Error UX
- **D-09:** Rich error messages: JSON path to error location, expected vs. found values, which validation pass caught it, and actionable suggestions when possible (e.g., "Did you mean state 'InProgress'?"). Aim for rustc-quality error messages.
- **D-10:** Collect all errors within each pass before reporting. Passes execute in strict order — later passes do not run if an earlier pass has failures (prevents confusing cascading errors).
- **D-11:** Four validation passes in order: (1) JSON Schema conformance, (2) Version compatibility check, (3) Semantic validation (state references exist, types match across edges, completeness checks), (4) DAG cycle detection and topological sort.
- **D-12:** Dual output format — human-readable with colorized terminal output by default, machine-readable JSON via `--json` flag.

### DAG Node Types for MVP
- **D-13:** Core MVP DAG node types: field access, comparison (eq, neq, gt, lt, gte, lte), boolean logic (and, or, not), arithmetic (+, -, *, /), string operations (concat, contains, length). TS Blocks, Decision Tables, and Integration Calls defined as schema stubs only — not implemented in loader.
- **D-14:** Phase 2 builds the petgraph `DiGraph`, runs cycle detection via `is_cyclic_directed()`, and computes topological order via `toposort()`. Phase 4 adds the interpreter that evaluates nodes.
- **D-15:** DAG nodes use tagged union with `"type"` discriminator field in JSON (e.g., `{"type": "comparison", "op": "eq", ...}`). Maps to Rust enum with `#[serde(tag = "type")]`.

### Claude's Discretion
- Exact JSON Schema structure and `$defs` organization
- Rust struct field naming and module layout within sdg-loader
- Specific broken SDG fixture scenarios (beyond the general categories listed)
- Internal error type hierarchy (`thiserror` enum design)
- Suggestion algorithm for "did you mean" hints
- Whether to use `insta` snapshot tests for validation output

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Architecture & SDG Format
- `execution_layer_6pager.md` §5 — SDG format definition: all sections (service, aggregates, transitions, projections, endpoints, etc.), DAG node types, computation model, TypeScript boundary rules
- `execution_layer_6pager.md` §3 — Runtime architecture: SDG Loader component responsibilities, request lifecycle

### Governing Principles
- `.specify/memory/constitution.md` — 7 principles governing all implementation (especially Principle VI: deterministic validation at load time)

### Requirements
- `.planning/REQUIREMENTS.md` §SDG Schema & Loader — Requirements SDG-01 through SDG-07 with acceptance criteria

### Prior Phase Context
- `.planning/phases/01-dev-environment/01-CONTEXT.md` — Workspace structure, dependency versions, TDD discipline

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/sdg-loader/` crate scaffolded with dependencies: serde, serde_json, jsonschema, thiserror, petgraph (already in Cargo.toml)
- Dev-dependencies ready: insta (snapshot testing), tempfile (temp file fixtures)
- Workspace-level dependency versioning eliminates version drift

### Established Patterns
- Clippy pedantic + deny warnings enforced at workspace level — all new code must pass
- Strict TDD: tests written before implementation (Red-Green-Refactor)
- Workspace `[workspace.dependencies]` for all version pins

### Integration Points
- `crates/runtime/` binary crate will depend on `sdg-loader` to load SDG at startup
- `crates/aggregate-engine/` (Phase 4) will consume the materialized DAG and `ServiceDefinition` struct
- `crates/api-surface/` (Phase 6) will read endpoint definitions from parsed SDG
- `crates/event-store/` (Phase 3) needs aggregate type definitions from SDG

</code_context>

<specifics>
## Specific Ideas

- User explicitly revised schema storage: initially chose external file, then after understanding schema vs. SDG distinction, chose embedded-in-binary approach — schema is tied to the runtime version
- Error quality aspiration: "like rustc" — rich, helpful, with suggestions
- Task tracker example should be realistic enough to exercise most validation paths

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 02-sdg-schema-loader*
*Context gathered: 2026-04-07*
