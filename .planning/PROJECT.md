# Runtime — Execution Layer

## What This Is

A compiled Rust runtime that loads a Service Definition Graph (SDG) file at startup and becomes the microservice it describes. No code generation — the SDG is interpreted directly, eliminating model-code drift by architectural design. Domain specialists describe *what* a service does in JSON; the runtime handles *how* it executes: event-sourced persistence, projections, HTTP API, middleware, observability.

## Core Value

The SDG file is the single source of truth for service behavior. Changing the model and restarting the runtime is the only mechanism to alter behavior — this eliminates drift between model and implementation.

## Requirements

### Validated

(None yet — ship to validate)

### Active

- [ ] SDG JSON Schema defined and validated at load time
- [ ] SDG loader parses and materializes computation DAG from JSON
- [ ] Append-only event store on SQLite with per-aggregate streams and optimistic concurrency
- [ ] Aggregate engine executes state machine transitions and computation DAG
- [ ] Transactional outbox guarantees event delivery from store to consumers
- [ ] Projection engine builds async read models from event streams
- [ ] Auto-generated HTTP endpoints from SDG with OpenAPI spec
- [ ] Middleware pipeline: JWT auth, request validation, structured errors
- [ ] Built-in observability: OTel metrics, distributed tracing, structured logging
- [ ] CLI command validates SDG files against schema before startup
- [ ] End-to-end demo: task tracker SDG loaded, tasks created, states transitioned, projections queried via API
- [ ] Cargo workspace with 8 crates, pinned toolchain, Dokploy CI, TDD infrastructure

### Out of Scope

- TypeScript sandbox / Deno V8 isolate — Phase 2
- Decision Tables in DAG — Phase 2
- Integration Call nodes — Phase 2
- Zeebe / BPMN Bridge — Phase 3
- gRPC API surface — Phase 3
- Production event store (PostgreSQL/EventStoreDB) — Phase 3
- On-demand snapshots — Phase 3
- Event upcasting — Phase 3
- Hot reload — explicit non-goal (restart only)
- CRUD persistence — architectural exclusion (ES only)
- Multi-tenancy — one instance per service
- UI rendering — not in scope
- Real deployment — Dokploy CI only for now

## Context

- **Architecture source**: `execution_layer_6pager.md` — ratified 6-pager with all architectural decisions
- **Constitution**: `.specify/memory/constitution.md` — 7 principles governing all implementation
- **Existing work**: Branch `001-mvp-dev-environment` has workspace skeleton (8 crates, toolchain, Dockerfile, TDD infra) — not yet merged to main
- **Existing specs**: `specs/001-mvp-dev-environment/` contains spec, plan, research for Step 1
- **Example domain**: Task tracker (Task with states: Created, InProgress, Done) for testing and demos
- **MVP decomposition**: 10 steps from dev environment through CLI validation, each independently testable
- **Development on Coder workspaces**, CI via Dokploy build pipeline

## Constraints

- **Language**: Rust stable (pinned via `rust-toolchain.toml`)
- **Persistence**: Event sourcing only — no CRUD alternative (Constitution Principle II)
- **Event format**: JSON (protobuf/Avro deferred to benchmarks)
- **Event store**: SQLite for MVP (Constitution Principle III)
- **Reload**: Restart only, no hot reload (Constitution Principle III)
- **TDD**: Strict Red-Green-Refactor for all changes (clarified 2026-04-07)
- **CI**: Dokploy build pipeline runs build, test, fmt, clippy as quality gates
- **Observability**: Built into runtime, not opt-in (Constitution Principle V)
- **Validation**: SDG validated deterministically at load time (Constitution Principle VI)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| MVP scope only (10 steps) | TS sandbox, Zeebe, production DB are Phase 2-3 | -- Pending |
| Task tracker as example SDG | Simple domain with clear states/transitions for testing | -- Pending |
| SDG schema from scratch | No existing schema — define JSON Schema as part of Step 2 | -- Pending |
| Dokploy CI only, no deployment | Quality gates matter now; real deploy later | -- Pending |
| Step 1 not yet merged | Include in roadmap as pending, not validated | -- Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? -> Move to Out of Scope with reason
2. Requirements validated? -> Move to Validated with phase reference
3. New requirements emerged? -> Add to Active
4. Decisions to log? -> Add to Key Decisions
5. "What This Is" still accurate? -> Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-04-07 after initialization*
