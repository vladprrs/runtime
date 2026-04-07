# Requirements: Runtime — Execution Layer

**Defined:** 2026-04-07
**Core Value:** The SDG file is the single source of truth for service behavior — changing the model and restarting is the only mechanism to alter behavior.

## v1 Requirements

Requirements for MVP release. Each maps to roadmap phases.

### Dev Environment

- [ ] **DEV-01**: Cargo workspace compiles with `cargo build --workspace` producing zero errors and zero warnings
- [ ] **DEV-02**: Rust toolchain pinned via `rust-toolchain.toml` with auto-install via rustup
- [ ] **DEV-03**: `cargo fmt --check` validates formatting with `rustfmt.toml` config
- [ ] **DEV-04**: `cargo clippy --workspace -- -D warnings` validates linting with pedantic rules
- [ ] **DEV-05**: `cargo test --workspace` executes successfully with TDD infrastructure ready
- [ ] **DEV-06**: Dokploy Dockerfile runs build, test, fmt, clippy as quality gates before deploy
- [ ] **DEV-07**: Workspace contains 8 crates (7 library + 1 binary) mapped to runtime components
- [ ] **DEV-08**: Workspace-level `[workspace.dependencies]` centralizes dependency versions

### SDG Schema & Loader

- [ ] **SDG-01**: JSON Schema (Draft 2020-12) defines the SDG format with all aggregate, transition, projection, and endpoint structures
- [ ] **SDG-02**: SDG file validated against JSON Schema at load time; invalid files prevent startup with structured error messages
- [ ] **SDG-03**: Multi-pass validation: schema conformance, DAG cycle detection, type compatibility across edges, completeness checks
- [ ] **SDG-04**: SDG parsed into typed Rust structs (not raw serde_json::Value) with `ServiceDefinition` as the root type
- [ ] **SDG-05**: Computation DAG materialized with pre-computed topological order for runtime evaluation
- [ ] **SDG-06**: Task tracker example SDG created as canonical test fixture and demo artifact
- [ ] **SDG-07**: SDG version compatibility field checked at load time; incompatible versions rejected

### Event Store

- [ ] **EVT-01**: Append-only event store on SQLite with per-aggregate streams
- [ ] **EVT-02**: Optimistic concurrency via `UNIQUE(aggregate_id, version)` database constraint
- [ ] **EVT-03**: Event metadata envelope: event_id (UUID), aggregate_id, aggregate_type, event_type, event_version, version, timestamp, correlation_id, causation_id
- [ ] **EVT-04**: Global sequence column for cross-aggregate ordering (used by projections)
- [ ] **EVT-05**: `EventStore` trait abstraction allowing future backing store replacement without SDG changes
- [ ] **EVT-06**: SQLite configured with WAL mode, busy_timeout, synchronous=NORMAL
- [ ] **EVT-07**: Single dedicated writer connection; pooled reader connections
- [ ] **EVT-08**: All SQLite operations wrapped in `spawn_blocking` to avoid blocking tokio
- [ ] **EVT-09**: Typed concurrency conflict errors (not raw SQLite errors)

### Aggregate Engine

- [ ] **AGG-01**: State machine with named states and allowed transition matrix defined by SDG
- [ ] **AGG-02**: Command routing: incoming command matched to aggregate type + instance, state loaded via event replay
- [ ] **AGG-03**: Guard conditions evaluated from SDG computation DAG before transition
- [ ] **AGG-04**: Computation DAG interpreter executes: field access, comparison, boolean logic, arithmetic, string operations
- [ ] **AGG-05**: Successful transitions emit domain events appended to event store
- [ ] **AGG-06**: Structured domain errors for invariant violations (separate from system errors)
- [ ] **AGG-07**: Given-When-Then test pattern for all aggregate tests (assert on emitted events, not internal state)

### Transactional Outbox

- [ ] **OUT-01**: Events and outbox entries written in single SQLite transaction (atomic)
- [ ] **OUT-02**: Polling-based relay delivers outbox entries to projection engine
- [ ] **OUT-03**: At-least-once delivery guarantee with delivery tracking

### Projections

- [ ] **PRJ-01**: Async projection engine processes events from outbox in background
- [ ] **PRJ-02**: Checkpoint tracking per projection (last processed position)
- [ ] **PRJ-03**: Catch-up phase: process all unprocessed events on startup
- [ ] **PRJ-04**: Projection rebuild: drop and rebuild any projection from scratch by replaying events
- [ ] **PRJ-05**: Idempotent projection handlers (safe for duplicate event delivery)
- [ ] **PRJ-06**: Projection storage in separate SQLite database from event store

### HTTP API Surface

- [ ] **API-01**: Auto-generated command endpoints (POST) from SDG transition definitions
- [ ] **API-02**: Auto-generated query endpoints (GET) from SDG projection definitions
- [ ] **API-03**: OpenAPI spec generated dynamically from live SDG, served at `/openapi.json`
- [ ] **API-04**: Request payload validated against SDG-defined command schemas
- [ ] **API-05**: Structured error responses (RFC 7807-style Problem Details)
- [ ] **API-06**: Health check endpoint at `/health`
- [ ] **API-07**: Aggregate version returned in command responses for read-after-write consistency

### Middleware

- [ ] **MID-01**: JWT authentication via Tower layer (verify tokens on incoming requests)
- [ ] **MID-02**: Correlation ID extraction from header or auto-generation, propagated through command/event chain
- [ ] **MID-03**: Request timeout enforcement
- [ ] **MID-04**: Domain-to-HTTP error status mapping (DomainError -> appropriate HTTP status)

### Observability

- [ ] **OBS-01**: Structured JSON logging with correlation_id, aggregate_id, command_type, event_type
- [ ] **OBS-02**: Distributed tracing: spans from HTTP request through command, aggregate, event store, projection
- [ ] **OBS-03**: OTel metrics: command_duration, command_total, event_store_append_duration, projection_lag_ms, api_request_duration
- [ ] **OBS-04**: OTel trace export via OTLP
- [ ] **OBS-05**: Runtime log level configuration via environment filter

### CLI & Integration

- [ ] **CLI-01**: `runtime validate <sdg-file>` command validates SDG without starting server
- [ ] **CLI-02**: Human-readable validation output with error paths and suggestions
- [ ] **CLI-03**: End-to-end demo: task tracker SDG loaded, tasks created via API, states transitioned, projections queried

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### TypeScript Sandbox (Phase 2)

- **TS-01**: Deno V8 isolate embedded in runtime
- **TS-02**: TypeScript Block as computation DAG node
- **TS-03**: Type generation from SDG for TS extensions
- **TS-04**: CPU/memory limits and network allowlist enforcement

### Decision Tables (Phase 2)

- **DT-01**: Decision Table as computation DAG node
- **DT-02**: Tabular logic evaluation for branched business rules

### Advanced Features (Phase 2-3)

- **ADV-01**: Integration Call nodes in DAG (external service calls)
- **ADV-02**: Command idempotency / deduplication
- **ADV-03**: Multi-stream projections (cross-aggregate)
- **ADV-04**: Saga / process manager patterns
- **ADV-05**: On-demand snapshots triggered by analytics
- **ADV-06**: Event upcasting for schema evolution
- **ADV-07**: gRPC API surface with proto specs
- **ADV-08**: Zeebe/BPMN Bridge for cross-service orchestration
- **ADV-09**: Production event store (PostgreSQL/EventStoreDB)

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Hot reload | Constitution Principle III: restart only, simplifies state management |
| CRUD persistence | Constitution Principle II: ES is the only mode |
| Multi-tenancy | One instance per service; Operating Layer handles tenancy |
| UI rendering | Runtime is API backend only |
| Code generation | Constitution Principle I: model-driven interpretation, not codegen |
| Rate limiting | Production hardening, not MVP scope |
| Kafka/streaming integration | Polling-based outbox sufficient for MVP |
| Inline (synchronous) projections | Async only; accept eventual consistency |
| Event store compaction/archival | Trivial volumes in MVP |
| Custom middleware plugins | Fixed middleware stack in MVP |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| DEV-01 | Phase 1 | Pending |
| DEV-02 | Phase 1 | Pending |
| DEV-03 | Phase 1 | Pending |
| DEV-04 | Phase 1 | Pending |
| DEV-05 | Phase 1 | Pending |
| DEV-06 | Phase 1 | Pending |
| DEV-07 | Phase 1 | Pending |
| DEV-08 | Phase 1 | Pending |
| SDG-01 | Phase 2 | Pending |
| SDG-02 | Phase 2 | Pending |
| SDG-03 | Phase 2 | Pending |
| SDG-04 | Phase 2 | Pending |
| SDG-05 | Phase 2 | Pending |
| SDG-06 | Phase 2 | Pending |
| SDG-07 | Phase 2 | Pending |
| EVT-01 | Phase 3 | Pending |
| EVT-02 | Phase 3 | Pending |
| EVT-03 | Phase 3 | Pending |
| EVT-04 | Phase 3 | Pending |
| EVT-05 | Phase 3 | Pending |
| EVT-06 | Phase 3 | Pending |
| EVT-07 | Phase 3 | Pending |
| EVT-08 | Phase 3 | Pending |
| EVT-09 | Phase 3 | Pending |
| AGG-01 | Phase 4 | Pending |
| AGG-02 | Phase 4 | Pending |
| AGG-03 | Phase 4 | Pending |
| AGG-04 | Phase 4 | Pending |
| AGG-05 | Phase 4 | Pending |
| AGG-06 | Phase 4 | Pending |
| AGG-07 | Phase 4 | Pending |
| OUT-01 | Phase 5 | Pending |
| OUT-02 | Phase 5 | Pending |
| OUT-03 | Phase 5 | Pending |
| PRJ-01 | Phase 5 | Pending |
| PRJ-02 | Phase 5 | Pending |
| PRJ-03 | Phase 5 | Pending |
| PRJ-04 | Phase 5 | Pending |
| PRJ-05 | Phase 5 | Pending |
| PRJ-06 | Phase 5 | Pending |
| API-01 | Phase 6 | Pending |
| API-02 | Phase 6 | Pending |
| API-03 | Phase 6 | Pending |
| API-04 | Phase 6 | Pending |
| API-05 | Phase 6 | Pending |
| API-06 | Phase 6 | Pending |
| API-07 | Phase 6 | Pending |
| MID-01 | Phase 7 | Pending |
| MID-02 | Phase 7 | Pending |
| MID-03 | Phase 7 | Pending |
| MID-04 | Phase 7 | Pending |
| OBS-01 | Phase 8 | Pending |
| OBS-02 | Phase 8 | Pending |
| OBS-03 | Phase 8 | Pending |
| OBS-04 | Phase 8 | Pending |
| OBS-05 | Phase 8 | Pending |
| CLI-01 | Phase 9 | Pending |
| CLI-02 | Phase 9 | Pending |
| CLI-03 | Phase 9 | Pending |

**Coverage:**
- v1 requirements: 57 total
- Mapped to phases: 57
- Unmapped: 0

---
*Requirements defined: 2026-04-07*
*Last updated: 2026-04-07 after initial definition*
