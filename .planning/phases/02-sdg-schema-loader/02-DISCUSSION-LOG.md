# Phase 2: SDG Schema & Loader - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-07
**Phase:** 02-sdg-schema-loader
**Areas discussed:** SDG Schema Scope, Task Tracker Example, Validation Error UX, DAG Node Types for MVP

---

## SDG Schema Scope

| Option | Description | Selected |
|--------|-------------|----------|
| Full schema, MVP validation | Define complete JSON Schema covering all 6-pager sections. Validate/parse only MVP sections. Future sections schema-valid but ignored. | ✓ |
| MVP-only schema | Define schema for only what Phases 2-9 need. Simpler now, requires expansion later. | |
| Progressive schema with extension points | MVP schema with explicit additionalProperties slots for future sections. | |

**User's choice:** Full schema, MVP validation
**Notes:** Avoids breaking schema changes when TS sandbox, Decision Tables, etc. arrive in future phases.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Warn but accept | Schema allows deferred features, loader logs warning that they'll be ignored | ✓ |
| Reject in loader | Schema allows, but semantic validation rejects SDGs using deferred features | |
| Schema forbids them | MVP schema doesn't include deferred sections at all | |

**User's choice:** Warn but accept
**Notes:** Lets users experiment with the full schema shape.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Embedded in binary | Schema as const/include_str! in sdg-loader crate. No runtime file dependency. | Initially selected |
| External file | Schema loaded from file path at startup. | Initially selected, then revised |

**User's choice:** Initially chose "External file" + "No fallback". After clarification about schema vs. SDG distinction, revised to: **Schema embedded in binary as inline Rust code.** SDG loaded from file.

| Option | Description | Selected |
|--------|-------------|----------|
| SemVer on schema version field | SDG has version field (e.g., '1.0.0'). Runtime checks major version match. | ✓ |
| Simple integer version | schema_version: 1. Runtime supports known versions. | |

**User's choice:** SemVer on schema version field

| Option | Description | Selected |
|--------|-------------|----------|
| ./service.sdg.json | Current directory, predictable name. Override via CLI arg. | ✓ |
| ./sdg/service.json | Dedicated subdirectory. | |

**User's choice:** ./service.sdg.json (default, with CLI override)

| Option | Description | Selected |
|--------|-------------|----------|
| schemas/ in repo | Separate file for human reference, embedded via include_str! at build | |
| Inline in Rust code | Schema defined directly as Rust string constant. No separate file. | ✓ |

**User's choice:** Inline in Rust code

---

## Task Tracker Example

| Option | Description | Selected |
|--------|-------------|----------|
| Realistic | 3 states, guard conditions, 1-2 projections, endpoints | ✓ |
| Minimal | 2 states, one transition, no guards | |
| Rich multi-aggregate | Task + Project, cross-references, multiple projections | |

**User's choice:** Realistic
**Notes:** Exercises most validation paths without over-scoping.

| Option | Description | Selected |
|--------|-------------|----------|
| Valid + broken fixtures | One valid + 5-10 broken SDGs for specific error testing | ✓ |
| Valid only, inline broken | Only task tracker as file; broken SDGs inline in tests | |

**User's choice:** Valid + broken fixtures

| Option | Description | Selected |
|--------|-------------|----------|
| crates/sdg-loader/fixtures/ | Co-located with loader crate tests | ✓ |
| tests/fixtures/ at repo root | Top-level shared test data | |

**User's choice:** crates/sdg-loader/fixtures/

---

## Validation Error UX

| Option | Description | Selected |
|--------|-------------|----------|
| Rich with context | JSON path, expected vs. found, pass info, suggestions. Like rustc. | ✓ |
| Structural path only | JSON path + error type + constraint. Like jsonschema default output. | |

**User's choice:** Rich with context
**Notes:** Aspiration is rustc-quality error messages.

| Option | Description | Selected |
|--------|-------------|----------|
| Collect all per pass | Each pass collects all errors. Passes in order, later passes skip if earlier fails. | ✓ |
| Fail-fast per pass | First error stops validation. | |
| Collect all across all passes | Run all passes regardless. | |

**User's choice:** Collect all per pass

| Option | Description | Selected |
|--------|-------------|----------|
| 4 passes: schema → version → semantic → DAG | Full pass pipeline | ✓ |
| 3 passes: schema → semantic → DAG | Version folded into schema pass | |

**User's choice:** 4 passes

| Option | Description | Selected |
|--------|-------------|----------|
| Both, selectable | Human-readable default + --json flag | ✓ |
| Human-readable only | Terminal text only | |

**User's choice:** Both, selectable

---

## DAG Node Types for MVP

| Option | Description | Selected |
|--------|-------------|----------|
| Core MVP set | Field access, comparison, boolean, arithmetic, string ops. Deferred types as stubs. | ✓ |
| Minimal | Field access, comparison, boolean only. | |
| Full minus deferred | Everything except TS/DT/Integration. Includes date math, combinators. | |

**User's choice:** Core MVP set

| Option | Description | Selected |
|--------|-------------|----------|
| Build DAG in Phase 2 | petgraph DiGraph, cycle detection, topo sort in Phase 2. Interpreter in Phase 4. | ✓ |
| Parse only, defer DAG | Typed structs only. petgraph in Phase 4. | |

**User's choice:** Build DAG in Phase 2

| Option | Description | Selected |
|--------|-------------|----------|
| Tagged union with 'type' field | {"type": "comparison", ...}. Maps to Rust enum with serde tag. | ✓ |
| Implicit by structure | Type inferred from present fields. | |

**User's choice:** Tagged union with 'type' field

---

## Claude's Discretion

- JSON Schema `$defs` organization
- Rust struct/module layout
- Specific broken fixture scenarios
- Error type hierarchy design
- Suggestion algorithm for hints
- Snapshot testing strategy

---

## Update Session: Spec Alignment Review (2026-04-07)

**Trigger:** User requested reading `specs/003-sdg-v2-format/spec.md` and fixing CONTEXT.md
**Mode:** discuss (update — spec alignment review)

### 1. Literal — 4th Leaf Node Type

| Question | Options | Selected |
|----------|---------|----------|
| How is the literal node's type determined for edge type-checking? | Infer from value / Explicit type param | **Explicit type param** |

**Correction:** D-15 changed from "Three leaf node types" to "Four leaf node types" (added `literal`). New D-36 requires `output_type` param.
**Rationale:** Explicit type avoids ambiguity for edge cases (null, empty arrays).

### 2. Implicit Aggregate Fields

| Question | Options | Selected |
|----------|---------|----------|
| Which fields are implicit for every aggregate? | 5 fields as in spec / Only id and state / Claude decides | **5 fields as in spec** |

**New decision:** D-37 — `id` (uuid), `state` (string), `created_at` (datetime), `updated_at` (datetime), `version` (integer).
**Rationale:** Spec §6.4 derives projections with all 5. Task tracker accesses `id` via field-node.

### 3. API Section Parsing Scope

| Question | Options | Selected |
|----------|---------|----------|
| How deep should loader parse api section in Phase 2? | Full parsing + semantic validation / Parse without gRPC / Minimal (types only) | **Minimal (types only)** |

**New decision:** D-38 — Parse into typed structs, JSON Schema validation only. Semantic validation deferred to Phase 6.
**Rationale:** Phase 2 focuses on model + computations. Phase 6 owns endpoint generation.

### 4. Context Path Validation

| Question | Options | Selected |
|----------|---------|----------|
| How to validate context-nodes in computation DAG? | Strict validation (error) / Soft validation (warning) | **Strict validation** |

**New decision:** D-39 — Unknown context paths are errors in Pass 3. Type from fixed table enables edge type-checking.
**Rationale:** Fixed set means unknown paths are always bugs.

### Corrections to Existing Decisions

| Decision | Original | Updated |
|----------|----------|---------|
| D-15 | Three leaf node types: field, command, context | Four leaf node types: field, command, context, literal |

## Deferred Ideas

None — discussion stayed within phase scope
