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

## Deferred Ideas

None — discussion stayed within phase scope
