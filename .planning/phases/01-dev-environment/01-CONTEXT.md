# Phase 1: Dev Environment - Context

**Gathered:** 2026-04-07
**Status:** Ready for planning

<domain>
## Phase Boundary

Cargo workspace skeleton with 8 crates (7 library + 1 binary), pinned Rust toolchain, Dokploy CI pipeline with quality gates, and TDD infrastructure. No runtime logic — this is pure project scaffolding. All crates compile with zero errors/warnings, and quality gates (build, test, fmt, clippy) pass from day one.

</domain>

<decisions>
## Implementation Decisions

### Toolchain Version
- **D-01:** Pin Rust to 1.94.1 (current stable) in `rust-toolchain.toml`. This is the latest stable release; bump deliberately when needed.

### Workspace Structure
- **D-02:** Cargo workspace with `crates/` directory. Each library crate maps 1:1 to a runtime component from the 6-pager architecture.
- **D-03:** 8 crates total: `runtime` (binary), `sdg-loader`, `event-store`, `aggregate-engine`, `projections`, `api-surface`, `middleware`, `observability`.
- **D-04:** Workspace-level `[workspace.dependencies]` centralizes all dependency versions to prevent drift across crates.

### Initial Dependencies
- **D-05:** All MVP dependencies declared upfront as a bill of materials in workspace `[workspace.dependencies]`: serde, serde_json, tokio, rusqlite, axum, opentelemetry, tracing, uuid, thiserror, jsonschema, clap, and supporting crates per CLAUDE.md tech stack.
- **D-06:** Crate-level `Cargo.toml` files reference workspace deps with `workspace = true` but only include deps relevant to that crate's domain.

### CI Pipeline
- **D-07:** Multi-stage Dockerfile for Dokploy. Build stage runs all quality gates (cargo build, test, fmt --check, clippy -- -D warnings). A failing check blocks deployment.
- **D-08:** No separate CI system — Dokploy build pipeline is the CI.

### Code Quality
- **D-09:** `rustfmt.toml` with minimal custom config (edition 2021 defaults).
- **D-10:** Clippy with pedantic rules enabled at workspace level, deny warnings.

### TDD Infrastructure
- **D-11:** Strict TDD with Red-Green-Refactor for all changes. Tests written before implementation.
- **D-12:** Each crate gets at least one placeholder test confirming the harness works. Per-crate test execution via `cargo test -p <crate>` for fast TDD feedback loops.

### Crate Scaffolding
- **D-13:** Minimal scaffolding — each library crate gets a `lib.rs` with a placeholder test. Binary crate gets a `main.rs`. No error types, traits, or stubs — those come in later phases when they're needed.

### Claude's Discretion
- Exact rustfmt.toml settings beyond edition default
- Dockerfile caching strategy and base image choice
- Whether to include a `clippy.toml` or use workspace-level Cargo.toml clippy config
- Placeholder test content (simple assert vs. module structure test)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Architecture
- `execution_layer_6pager.md` — Ratified 6-pager with all architectural decisions, component boundaries, and SDK design
- `.specify/memory/constitution.md` — 7 principles governing all implementation decisions

### Existing Spec Work (supplementary)
- `specs/001-mvp-dev-environment/spec.md` — Detailed user stories, acceptance scenarios, and functional requirements for Phase 1
- `specs/001-mvp-dev-environment/plan.md` — Implementation plan with project structure, dependency list, and phase breakdown
- `specs/001-mvp-dev-environment/research.md` — Phase 0 research output

### Requirements
- `.planning/REQUIREMENTS.md` §Dev Environment — Requirements DEV-01 through DEV-08

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- None — greenfield project. No Rust files exist yet.

### Established Patterns
- None yet. Phase 1 establishes the foundational patterns (workspace layout, dependency management, quality gates).

### Integration Points
- Dokploy deployment infrastructure already configured for this repo
- Coder workspaces serve as the development environment

</code_context>

<specifics>
## Specific Ideas

- Workspace structure from existing spec plan.md should be followed (crates/ directory layout with 8 named crates)
- Dependencies and versions defined in CLAUDE.md tech stack section should be used as the bill of materials
- Existing spec user stories provide detailed acceptance scenarios to validate against

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-dev-environment*
*Context gathered: 2026-04-07*
