# Roadmap: Runtime -- Execution Layer

## Overview

This roadmap delivers a compiled Rust runtime that loads an SDG (Service Definition Graph) JSON file and becomes the microservice it describes. The journey starts with development infrastructure, builds the SDG loader and event store as foundation crates, layers on the aggregate engine and projections for full CQRS/ES behavior, exposes everything via a dynamic HTTP API with middleware, wires in observability, and culminates in a CLI validator and end-to-end integration demo using a task tracker SDG. Each phase delivers an independently testable capability following strict TDD discipline.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Dev Environment** - Cargo workspace, toolchain, CI pipeline, and TDD infrastructure
- [ ] **Phase 2: SDG Schema & Loader** - JSON Schema definition, multi-pass validation, typed Rust structs, and task tracker example SDG
- [ ] **Phase 3: Event Store** - Append-only SQLite event persistence with per-aggregate streams, optimistic concurrency, and trait abstraction
- [ ] **Phase 4: Aggregate Engine** - State machine transitions, command routing, computation DAG interpreter, and event emission
- [ ] **Phase 5: Transactional Outbox & Projections** - Atomic outbox writes, polling relay, async projection engine with catch-up and rebuild
- [ ] **Phase 6: HTTP API Surface** - Dynamic route generation from SDG, command and query endpoints, OpenAPI spec, health check
- [ ] **Phase 7: Middleware** - JWT authentication, correlation ID propagation, request timeout, domain-to-HTTP error mapping
- [ ] **Phase 8: Observability** - Structured logging, distributed tracing, OTel metrics export, runtime log level configuration
- [ ] **Phase 9: CLI & End-to-End Integration** - SDG validation command, task tracker demo lifecycle, graceful shutdown

## Phase Details

### Phase 1: Dev Environment
**Goal**: Developers have a fully functional Rust workspace with quality gates that catch issues before merge
**Depends on**: Nothing (first phase)
**Requirements**: DEV-01, DEV-02, DEV-03, DEV-04, DEV-05, DEV-06, DEV-07, DEV-08
**Success Criteria** (what must be TRUE):
  1. `cargo build --workspace` compiles all 8 crates with zero errors and zero warnings
  2. `cargo test --workspace` runs successfully with at least one test per crate
  3. `cargo fmt --check` and `cargo clippy --workspace -- -D warnings` pass cleanly
  4. Dokploy Dockerfile builds successfully, running build + test + fmt + clippy as quality gates
  5. All workspace dependencies are centralized in workspace-level `[workspace.dependencies]`
**Plans:** 2 plans

Plans:
- [x] 01-01-PLAN.md — Workspace skeleton: Rust toolchain, 8 crate scaffolds, centralized deps, quality gates
- [x] 01-02-PLAN.md — CI pipeline: Multi-stage Dockerfile with cargo-chef and quality gates for Dokploy

### Phase 2: SDG Schema & Loader
**Goal**: The runtime can load, validate, and parse an SDG file into typed Rust structures, rejecting invalid definitions at startup with clear error messages
**Depends on**: Phase 1
**Requirements**: SDG-01, SDG-02, SDG-03, SDG-04, SDG-05, SDG-06, SDG-07
**Success Criteria** (what must be TRUE):
  1. A valid task tracker SDG file loads successfully and produces a typed `ServiceDefinition` struct
  2. An invalid SDG file is rejected at load time with structured error messages identifying the exact problem location
  3. Multi-pass validation catches schema violations, DAG cycles, type mismatches across edges, and completeness gaps
  4. Computation DAGs are materialized with pre-computed topological order ready for runtime evaluation
  5. SDG files with incompatible version numbers are rejected before any other processing
**Plans:** 5 plans

Plans:
- [x] 02-01-PLAN.md — Foundation: workspace deps (semver, strsim), type definitions, error hierarchy, JSON Schema
- [x] 02-02-PLAN.md — Pipeline + fixture: task tracker SDG, validation passes 1-2, load() function
- [x] 02-03-PLAN.md — Semantic + DAG: suggestions module, semantic validation (Pass 3), DAG materialization (Pass 4), pipeline wiring
- [x] 02-04-PLAN.md — Error formatting + integration: broken fixtures, dual output format, comprehensive integration tests
- [ ] 02-05-PLAN.md — Gap closure: edge type-compatibility checking and port validation in DAG pass

### Phase 3: Event Store
**Goal**: Domain events can be persisted reliably with per-aggregate streams, optimistic concurrency, and a clean trait abstraction that hides SQLite details
**Depends on**: Phase 1
**Requirements**: EVT-01, EVT-02, EVT-03, EVT-04, EVT-05, EVT-06, EVT-07, EVT-08, EVT-09
**Success Criteria** (what must be TRUE):
  1. Events appended to an aggregate stream are retrievable in order with full metadata (event_id, aggregate_id, version, correlation_id, etc.)
  2. Concurrent appends to the same aggregate at the same version produce a typed concurrency conflict error (not a raw SQLite error)
  3. A global sequence column provides cross-aggregate event ordering for projection consumption
  4. All SQLite operations execute via `spawn_blocking` without blocking the tokio runtime under concurrent load
  5. A mock `EventStore` implementation can be written without any rusqlite dependency
**Plans**: TBD

Plans:
- [ ] 03-01: TBD

### Phase 4: Aggregate Engine
**Goal**: Commands routed to aggregates execute state machine transitions with guard conditions evaluated from the computation DAG, emitting domain events on success
**Depends on**: Phase 2, Phase 3
**Requirements**: AGG-01, AGG-02, AGG-03, AGG-04, AGG-05, AGG-06, AGG-07
**Success Criteria** (what must be TRUE):
  1. A command against the task tracker aggregate transitions state (e.g., Created to InProgress) and emits the corresponding domain event
  2. A command violating a guard condition (evaluated via DAG interpreter) is rejected with a structured domain error
  3. Aggregate state is rebuilt correctly from its event stream via replay
  4. The DAG interpreter executes field access, comparison, boolean logic, arithmetic, and string operations
  5. All aggregate tests follow the Given-When-Then pattern, asserting on emitted events rather than internal state
**Plans**: TBD

Plans:
- [ ] 04-01: TBD

### Phase 5: Transactional Outbox & Projections
**Goal**: Events flow reliably from the event store through the outbox to async projections, completing the full CQRS read-model pipeline
**Depends on**: Phase 3, Phase 4
**Requirements**: OUT-01, OUT-02, OUT-03, PRJ-01, PRJ-02, PRJ-03, PRJ-04, PRJ-05, PRJ-06
**Success Criteria** (what must be TRUE):
  1. Events and outbox entries are written atomically in a single SQLite transaction -- no event persists without its outbox entry
  2. The polling relay delivers outbox entries to the projection engine with at-least-once delivery guarantee
  3. Projections process events asynchronously, maintaining a checkpoint of the last processed position
  4. On startup, projections catch up by processing all events since their last checkpoint
  5. A projection can be dropped and rebuilt from scratch by replaying the full event stream
**Plans**: TBD

Plans:
- [ ] 05-01: TBD

### Phase 6: HTTP API Surface
**Goal**: The runtime exposes auto-generated HTTP endpoints from the SDG, making aggregates and projections accessible via a standards-compliant REST API
**Depends on**: Phase 4, Phase 5
**Requirements**: API-01, API-02, API-03, API-04, API-05, API-06, API-07
**Success Criteria** (what must be TRUE):
  1. POST endpoints for commands and GET endpoints for queries are auto-generated from SDG transition and projection definitions
  2. An OpenAPI spec reflecting the live SDG is served at `/openapi.json`
  3. Invalid request payloads are rejected with structured error responses before reaching the aggregate engine
  4. A health check endpoint responds at `/health`
  5. Command responses include the aggregate version for read-after-write consistency
**Plans**: TBD

Plans:
- [ ] 06-01: TBD

### Phase 7: Middleware
**Goal**: The HTTP pipeline enforces authentication, propagates correlation context, and maps domain errors to appropriate HTTP responses
**Depends on**: Phase 6
**Requirements**: MID-01, MID-02, MID-03, MID-04
**Success Criteria** (what must be TRUE):
  1. Requests without a valid JWT token are rejected with 401 before reaching command handlers
  2. A correlation ID is extracted from the request header (or auto-generated) and propagated through the entire command/event chain
  3. Requests exceeding the configured timeout are terminated with an appropriate error response
  4. Domain errors (invariant violations, concurrency conflicts, not-found) map to correct HTTP status codes (422, 409, 404)
**Plans**: TBD

Plans:
- [ ] 07-01: TBD

### Phase 8: Observability
**Goal**: The runtime produces structured logs, distributed traces, and metrics that make operational behavior visible without additional instrumentation effort
**Depends on**: Phase 6
**Requirements**: OBS-01, OBS-02, OBS-03, OBS-04, OBS-05
**Success Criteria** (what must be TRUE):
  1. All log output is structured JSON with correlation_id, aggregate_id, command_type, and event_type fields where applicable
  2. A trace spans from HTTP request through command handling, aggregate logic, event store append, and projection processing
  3. OTel metrics are exported via OTLP including command_duration, command_total, event_store_append_duration, projection_lag_ms, and api_request_duration
  4. Runtime log level can be changed via environment filter without recompilation
**Plans**: TBD

Plans:
- [ ] 08-01: TBD

### Phase 9: CLI & End-to-End Integration
**Goal**: The runtime ships a validate command for pre-startup SDG checking and demonstrates the full lifecycle with a task tracker from creation through state transitions to projection queries
**Depends on**: Phase 7, Phase 8
**Requirements**: CLI-01, CLI-02, CLI-03
**Success Criteria** (what must be TRUE):
  1. `runtime validate <sdg-file>` checks an SDG file against all validation passes and reports results without starting the server
  2. Validation output is human-readable with error paths and actionable suggestions for fixes
  3. The task tracker end-to-end demo works: load SDG, create tasks via API, transition states, and query projections -- all returning correct results
**Plans**: TBD

Plans:
- [ ] 09-01: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8 -> 9

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Dev Environment | 0/2 | Planning complete | - |
| 2. SDG Schema & Loader | 0/0 | Not started | - |
| 3. Event Store | 0/0 | Not started | - |
| 4. Aggregate Engine | 0/0 | Not started | - |
| 5. Transactional Outbox & Projections | 0/0 | Not started | - |
| 6. HTTP API Surface | 0/0 | Not started | - |
| 7. Middleware | 0/0 | Not started | - |
| 8. Observability | 0/0 | Not started | - |
| 9. CLI & End-to-End Integration | 0/0 | Not started | - |
