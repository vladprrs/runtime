# Phase 2: SDG Schema & Loader - Context

**Gathered:** 2026-04-07 (revised for SDG v2)
**Status:** Ready for planning

<domain>
## Phase Boundary

Define the SDG v2 JSON Schema (Draft 2020-12), build a multi-pass validator, parse SDG files into typed Rust structs with `ServiceDefinition` as root, build the n8n-inspired flat computation DAG (nodes + edges) with type-checked ports and pre-computed topological order, implement derivation rules for events/projections/endpoints, and create an extended task tracker example SDG (2 aggregates, cross-aggregate lookups, ownership guard) as canonical test fixture. Rejecting invalid definitions at startup with clear, rich error messages.

</domain>

<decisions>
## Implementation Decisions

### SDG v2 Structure
- **D-01:** SDG has 4 top-level sections: `service`, `model`, `computations`, `api`. Replaces the 8-section v1 format.
- **D-02:** Schema defined as a JSON file embedded in the binary via `include_str!()`. Schema is version-locked to the runtime binary. Single monolithic schema with internal `$defs`.
- **D-03:** SemVer version field (`"schema_version": "2.0.0"`). Runtime checks major version match; minor/patch differences accepted.
- **D-04:** SDG file loaded from `./service.sdg.json` by default. Overridable via CLI argument. Fails fast if missing.

### Model Section
- **D-05:** Transitions are nested inside their aggregate (not a separate top-level map). Eliminates the need for `"aggregate": "Task"` on every transition.
- **D-06:** `"to": "$same"` sentinel for same-state transitions (edit fields, change priority). Runtime keeps aggregate in current state.
- **D-07:** `"from"` accepts arrays for multi-source transitions: `"from": ["Created", "InProgress"]`.
- **D-08:** `initial_state` defaults to first element of `states` array. Overridable with explicit field.
- **D-09:** Array types via suffix syntax: `"type": "uuid[]"`, `"type": "string[]"`. Maps to `Vec<T>` in Rust.
- **D-10:** Field validation properties: `min`, `max`, `min_length`, `max_length`, `pattern`, `format`. Map to JSON Schema keywords.
- **D-11:** `"references": "OtherAggregate"` on fields — declarative relationship (informational, not enforced). Used for lookup validation and OpenAPI generation.

### Computation DAG (n8n-inspired)
- **D-12:** Computations section has `nodes[]` (flat array) and `edges[]` (flat array). Inspired by n8n workflow format. No nesting — topology is fully described by edges.
- **D-13:** Each node: `{ "id": string, "type": string, "params": {} }`. ID is unique within the service. Type is from the function catalog.
- **D-14:** Each edge: `{ "from": string, "to": string, "port": string, "index?": integer }`. Port is a named input on the target node. Index for variadic ports (and/or).
- **D-15:** Three leaf node types (data sources): `field` (aggregate state), `command` (command payload), `context` (request context: actor.id, timestamp, correlation_id).
- **D-16:** `lookup` and `lookup_many` nodes read from projections (eventually consistent). Takes `aggregate` + `pick` params, wired via `id`/`ids` port.
- **D-17:** Collection operations: `filter` (with `in`/`not_in`/`eq`/`neq` params), `count`, `sum`, `min`, `max`, `any`, `all`, `contains`, `length`.
- **D-18:** Comparison: `eq`, `neq`, `gt`, `lt`, `gte`, `lte`. If only one port wired, the other from `params.right`/`params.left`.
- **D-19:** Logic: `and`/`or` are variadic (indexed `in` ports), `not` takes single `value` port.
- **D-20:** Arithmetic: `add`, `sub`, `mul`, `div`. String: `concat`, `str_contains`, `str_len`.

### Guards and Auto-Fields
- **D-21:** Transition `guard` references a computation node ID (must output boolean). Not an inline expression — all logic lives in the computation graph.
- **D-22:** `auto_fields` on transitions: `{ "field_name": "computation_node_id" }`. Automatically populates event fields from computation outputs (e.g., `author_id` from `actor_id` context node).

### Derivation Rules
- **D-23:** Event names derived as `{AggregateName}.{TransitionName}` (dot-notation, no linguistic transformation). Overridable with `"event_name"` on transition.
- **D-24:** Event payloads derived from command fields + auto_fields. Initial transitions inherit all aggregate fields.
- **D-25:** Command schemas: initial transition inherits aggregate fields; non-initial without explicit command = empty (pure state change).
- **D-26:** Default projections: `{agg}_list` (all fields + id, state, timestamps), `{agg}_detail` (+version), `{agg}_count_by_state` (if >1 state).
- **D-27:** Endpoints: `POST /{base}/{plural}` (create), `POST /{base}/{plural}/{id}/{transition}` (command), `GET /{base}/{plural}` (list), `GET /{base}/{plural}/{id}` (detail). Pluralization = simple `s` suffix.

### Validation Passes
- **D-28:** Four validation passes in strict order: (1) JSON Schema conformance, (2) Version compatibility, (3) Semantic validation (state references, field references, computation node types exist, etc.), (4) Computation graph validation (build petgraph DiGraph, detect cycles via toposort, type-check all edges — output type of source must match expected input type of target port).
- **D-29:** Collect all errors within each pass before reporting. Later passes do not run if earlier pass fails.
- **D-30:** Rich error messages: JSON path to error location, expected vs found values, which pass caught it, actionable suggestions (e.g., "Did you mean state 'InProgress'?"). Use `strsim` for suggestions.

### Task Tracker Example (Extended)
- **D-31:** Two aggregates: User (Active/Deactivated) and Task (Created/InProgress/Done/Cancelled).
- **D-32:** Task has: title, description, author_id (references User), assignee_id (references User), priority (1-5), linked_task_ids (uuid[], references Task).
- **D-33:** Computation graph with: actor context, cross-aggregate lookups (User.state, Task.state), collection operations (filter, count), ownership check (actor_id == assignee_id), linked tasks guard (all linked done).
- **D-34:** Canonical fixture at `crates/sdg-loader/fixtures/valid_task_tracker.sdg.json`. Plus 8-10 broken fixtures each triggering specific validation errors.

### Expression Sugar Layer
- **D-35:** Deferred to post-MVP. Phase 2 builds the DAG engine only. String expression shorthand (e.g., `"guard": "assignee != ''"` compiled to DAG nodes) is a future convenience layer.

### Claude's Discretion
- Exact JSON Schema `$defs` organization and naming
- Rust struct field naming and module layout within sdg-loader
- Specific broken SDG fixture scenarios (beyond general categories listed)
- Internal error type hierarchy (thiserror enum design)
- Suggestion algorithm for "did you mean" hints
- Whether to use insta snapshot tests for validation output
- Derivation rule implementation details (pluralization edge cases, etc.)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### SDG v2 Format (PRIMARY SOURCE)
- `specs/003-sdg-v2-format/spec.md` — Complete SDG v2 specification: structure, computation DAG, function catalog, derivation rules, type system
- `specs/003-sdg-v2-format/examples/task-tracker-extended.sdg.json` — Canonical example SDG with 2 aggregates, computation graph, cross-aggregate lookups

### Architecture & History
- `execution_layer_6pager.md` §1-4 — Runtime architecture (still valid), SDG v1 format (§5 SUPERSEDED by v2 spec)
- `.specify/memory/constitution.md` — 7 governing principles (especially Principle VI: deterministic validation at load time)

### Requirements
- `.planning/REQUIREMENTS.md` §SDG Schema & Loader — Requirements SDG-01 through SDG-07

### Design Research
- `.planning/research/sdg-minimal-design.md` — Convention-over-configuration analysis, derivation rules rationale, service type modeling
- `.planning/research/n8n-workflow-format.md` — n8n format analysis, flat nodes + edges pattern, design lessons

### Prior Phase Context
- `.planning/phases/01-dev-environment/01-CONTEXT.md` — Workspace structure, dependency versions, TDD discipline

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/sdg-loader/` crate scaffolded with dependencies: serde, serde_json, jsonschema, thiserror, petgraph (already in Cargo.toml)
- Dev-dependencies ready: insta (snapshot testing), tempfile (temp file fixtures)
- Workspace-level dependency versioning eliminates version drift

### New Dependencies Needed
- `semver` 1.0 — SemVer version parsing (D-03)
- `strsim` 0.11 — "Did you mean?" suggestions (D-30)

### Established Patterns
- Clippy pedantic + deny warnings enforced at workspace level
- Strict TDD: tests written before implementation (Red-Green-Refactor)
- Workspace `[workspace.dependencies]` for all version pins

### Integration Points
- `crates/runtime/` binary crate will depend on sdg-loader to load SDG at startup
- `crates/aggregate-engine/` (Phase 4) consumes ServiceDefinition + evaluates computation DAG
- `crates/api-surface/` (Phase 6) reads endpoint definitions + runs derivation rules
- `crates/event-store/` (Phase 3) needs aggregate type definitions from SDG

</code_context>

<specifics>
## Specific Ideas

- SDG v2 is machine-generated, human-editable. Verbosity is acceptable; clarity is mandatory.
- Computation DAG directly inspired by n8n workflow format (flat nodes + separate edges)
- Three data sources in DAG: field (aggregate state), command (input), context (actor/request)
- `lookup`/`lookup_many` reads from projections (eventually consistent) — the computation graph does NOT do synchronous cross-aggregate reads from event store
- Error quality aspiration: "like rustc" — rich, helpful, with suggestions
- Task tracker example must exercise: cross-aggregate lookups, collection operations, ownership checks, multi-source transitions, $same sentinel, auto_fields

</specifics>

<deferred>
## Deferred Ideas

- Expression sugar layer ("assignee != ''" → DAG nodes) — post-MVP
- `map` + `apply` (transform each element via sub-computation) — post-MVP
- `"consistency": "strong"` for lookups (read from event stream) — post-MVP
- Composite projections spanning multiple aggregates — post-MVP
- gRPC protocol support — post-MVP

</deferred>

---

*Phase: 02-sdg-schema-loader*
*Context gathered: 2026-04-07 (revised for SDG v2)*
