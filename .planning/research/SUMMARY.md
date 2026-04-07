# Project Research Summary

**Project:** Runtime -- Execution Layer
**Domain:** Model-driven event-sourced microservice runtime (Rust)
**Researched:** 2026-04-07
**Confidence:** HIGH

## Executive Summary

This project is a model-driven Rust runtime that interprets SDG (Service Definition Graph) JSON files as fully operational event-sourced microservices. The core value proposition is "zero-code service from a JSON model" -- a domain specialist writes JSON, and the runtime produces a working API with event sourcing, projections, observability, and authentication built in. The Rust ecosystem in 2026 is fully mature for this: axum 0.8, tokio 1.47, rusqlite 0.39, jsonschema 0.45, petgraph 0.8, and the tracing/opentelemetry stack are all production-grade, actively maintained, and have no risky gaps. The most consequential architectural decision confirmed by research is that event sourcing must be built as a custom thin layer (~300 lines of event store + ~200 lines of outbox) over rusqlite, rather than adopting any existing Rust ES framework (cqrs-es, eventually-rs, esrs), because all existing frameworks assume compile-time generic aggregate types via Rust traits -- fundamentally incompatible with runtime-interpreted, SDG-defined aggregates backed by `serde_json::Value`.

The recommended approach follows an 8-crate workspace with strict unidirectional dependency layers: foundation crates (sdg-loader, event-store, observability) have zero internal dependencies, domain crates (aggregate-engine, projections, middleware) depend only on foundation, the HTTP layer (api-surface) depends on domain, and the binary (runtime) wires everything together. This layering ensures each crate is independently testable and prevents circular dependencies. The critical path runs SDG Loader -> Event Store -> Aggregate Engine -> Projections -> API Surface, with middleware and observability as cross-cutting concerns wired in at the end. This matches the 10-step MVP decomposition from the project's 6-pager.

The two highest-risk components are the computation DAG interpreter (novel, no existing Rust library, must be scoped tightly to avoid becoming a Turing-complete language) and dynamic HTTP route generation from the SDG (atypical axum usage, programmatic utoipa OpenAPI generation rather than derive macros). Research identified 7 critical pitfalls, 8 moderate pitfalls, and 10 minor pitfalls, with the densest pitfall concentration in the Event Store phase (Step 3) -- SQLite single-writer bottleneck, optimistic concurrency correctness, blocking the tokio runtime with synchronous rusqlite calls, and event schema versioning must all be addressed from day one, not deferred. The Given-When-Then testing pattern for aggregates (asserting on emitted events, not internal state) is a mandatory discipline throughout.

## Key Findings

### Recommended Stack

The full stack is verified against docs.rs as of April 2026 with HIGH confidence across all core dependencies. No experimental or alpha crates are required.

**Core technologies:**
- **Rust stable 1.85+** (pinned in rust-toolchain.toml): Safe, compiled, zero-cost abstractions. Pin conservatively above MSRV of all deps (jsonschema requires 1.83+).
- **axum 0.8 + tower 0.5 + tower-http 0.6**: HTTP framework built on tower's Service/Layer abstraction. Composable middleware pipeline. v0.8 uses `/{param}` path syntax (breaking change from 0.7).
- **tokio 1.47**: Async runtime (LTS, supported through Sept 2026). Required by axum and the entire async ecosystem.
- **rusqlite 0.39 (bundled)**: Synchronous SQLite binding. Use `features = ["bundled", "serde_json", "uuid"]` for zero external deps and reproducible builds. Wrap with `spawn_blocking` in async context.
- **jsonschema 0.45**: JSON Schema validation (Draft 2020-12). 75-645x faster than alternatives. Used for SDG validation at load time.
- **petgraph 0.8**: DAG structural validation (cycle detection via `toposort`, reachability analysis). Used at load time only; runtime DAG evaluation uses a flat `Vec<DagNode>` with pre-computed topological order for performance.
- **utoipa 5.4 + utoipa-axum 0.2**: OpenAPI generation. Derive macros for static types, programmatic `OpenApiBuilder` for SDG-defined dynamic endpoints.
- **tracing 0.1 + opentelemetry 0.31**: Structured logging and distributed tracing. Critical version alignment: opentelemetry/opentelemetry_sdk/opentelemetry-otlp must all be 0.31; tracing-opentelemetry must be 0.32.
- **thiserror 2.0**: Typed error enums in all library crates. anyhow allowed only in the binary crate.
- **insta 1.47**: Snapshot testing for SDG validation output, event serialization, API response shapes. JSON snapshot mode with redactions.

**Explicit exclusions with rationale:**
- No `cqrs-es` / `eventually-rs` (compile-time generics incompatible with model-driven aggregates)
- No `sqlx` / `diesel` (ORM patterns inappropriate for append-only event store)
- No `actix-web` (non-tower middleware ecosystem)
- No `anyhow` in library crates (erases error types needed for HTTP status mapping)

### Expected Features

**Must have (table stakes) -- ordered by dependency chain:**
1. SDG JSON Schema validation and loader with structured multi-pass validation errors
2. Computation DAG materialization with cycle detection and type checking
3. Append-only event store with per-aggregate streams and optimistic concurrency (UNIQUE constraint)
4. Event metadata envelope (event_id, aggregate_id, version, event_type, event_version, correlation_id, causation_id, timestamp)
5. Aggregate state machine with transitions, guard conditions, and event replay
6. Computation DAG interpreter (field access, comparison, boolean logic, arithmetic, string ops)
7. Transactional outbox (atomic event + outbox writes in single SQLite transaction)
8. Async projection engine with checkpoint tracking, catch-up, and rebuild capability
9. Auto-generated command and query HTTP endpoints from SDG definitions
10. OpenAPI spec generated dynamically from live SDG at startup
11. JWT authentication, request validation, structured error responses (RFC 7807-style)
12. Structured logging, distributed tracing, basic metrics (command throughput, latency, projection lag)
13. CLI `validate` command that runs all SDG validation passes without starting the server
14. Health check endpoint (`/health`)

**Should have (differentiators):**
- Zero-code service from JSON model (emergent from table stakes working together)
- Model-behavior identity: SDG diff IS behavior diff, no code-model drift
- Built-in observability without developer instrumentation effort
- Single binary + SQLite = zero-config deployment
- Deterministic load-time validation catching errors before runtime
- Projection rebuild from event stream for zero-migration schema evolution

**Defer (v2+):**
- TypeScript sandbox / V8 isolate (Phase 2)
- Decision tables in DAG (Phase 2)
- Integration call nodes (Phase 2)
- Saga / process manager (Phase 2)
- Multi-stream projections (Phase 2)
- Command idempotency / deduplication (Phase 2)
- On-demand snapshots (Phase 3)
- Event upcasting logic (Phase 3, but storage format must accommodate from Phase 1)
- gRPC API surface (Phase 3)
- Zeebe / BPMN bridge (Phase 3)
- Production DB swap to PostgreSQL/EventStoreDB (Phase 3)
- Hot reload (explicit non-goal per Constitution Principle III)
- CRUD persistence mode (architectural exclusion per Constitution Principle II)

### Architecture Approach

The system is an 8-crate Rust workspace organized in 4 strict dependency layers. Lower layers never depend on higher layers. The SDG is loaded once, validated through multiple passes (schema, graph, type, completeness), materialized into an immutable `Arc<ServiceDefinition>` shared across all components. All aggregate state is dynamic (`serde_json::Value`) because types are defined by the SDG at runtime, not by Rust generics at compile time. The event store uses a single dedicated writer connection (serialized through a channel) and pooled reader connections, with `spawn_blocking` for all SQLite operations to avoid blocking tokio worker threads. The projection engine runs as a background tokio task, polling the outbox with configurable intervals, processing events through SDG-defined handlers that generate SQL upserts dynamically.

**Major components:**
1. **sdg-loader** (Layer 0) -- Parse SDG JSON, validate against JSON Schema, materialize computation DAGs, expose typed `ServiceDefinition`. Leaf crate with no internal dependencies.
2. **event-store** (Layer 0) -- Append-only event persistence with `EventStore` and `Outbox` traits, SQLite implementation, optimistic concurrency via UNIQUE constraint, transactional outbox. Leaf crate.
3. **observability** (Layer 0) -- OTel initialization, tracing subscriber configuration, runtime metrics registration. Leaf crate.
4. **aggregate-engine** (Layer 1) -- State machine execution, command routing, guard/computation DAG evaluation, event emission. Depends on sdg-loader and event-store.
5. **projections** (Layer 1) -- Async read model builder with catch-up, live polling, checkpoint tracking, rebuild. Depends on event-store and sdg-loader.
6. **middleware** (Layer 1) -- Tower layers for JWT auth, correlation ID propagation, structured error responses. Depends on sdg-loader and observability.
7. **api-surface** (Layer 2) -- Dynamic axum router construction from SDG, OpenAPI generation, command/query handlers. Depends on aggregate-engine, projections, middleware.
8. **runtime** (Layer 3) -- Binary entry point, CLI (clap), startup orchestration, graceful shutdown. Depends on all crates.

### Critical Pitfalls

The top pitfalls, synthesized from research, ordered by severity and phase:

1. **SQLite single-writer bottleneck (CP-1, Phase: Event Store)** -- SQLite allows only one writer even in WAL mode. Use a single dedicated writer connection serialized through a channel, pool readers separately. Configure `PRAGMA busy_timeout = 5000`, `PRAGMA journal_mode = WAL`, `PRAGMA synchronous = NORMAL`. Test with 20+ concurrent commands early.

2. **Optimistic concurrency check missing or incorrect (CP-2, Phase: Event Store)** -- Without `UNIQUE(aggregate_id, version)` as a database constraint, concurrent commands against the same aggregate silently corrupt the event stream. The constraint must exist from the first migration. Application-level version checks are insufficient due to race conditions. Return typed `ConcurrencyConflict` errors with retry logic at the command handler level.

3. **Blocking tokio runtime with synchronous SQLite (CP-4, Phase: Event Store)** -- Calling rusqlite directly inside async functions blocks tokio worker threads, causing complete runtime freeze under concurrent load. Every rusqlite call must go through `tokio::task::spawn_blocking` or `tokio-rusqlite`. Never hold a `Connection` across an `.await` point.

4. **Computation DAG validated structurally but not semantically (CP-5, Phase: SDG Loader + Aggregate Engine)** -- JSON Schema conformance and cycle detection are necessary but not sufficient. Multi-pass validation must also check type compatibility across DAG edges and completeness of event payload construction. Without semantic validation, errors surface at runtime in production instead of at startup (violates Constitution Principle VI).

5. **Event schema locked into V1 without versioning (CP-3, Phase: Event Store)** -- Store `event_type` and `event_version` as separate columns from day one. Design the upcasting pipeline interface even though implementation is Phase 3. Without this, SDG model evolution becomes impossible once events exist.

6. **Projection lag ignored in API design (CP-6, Phase: Projections + API Surface)** -- Return aggregate version in command responses so clients can implement read-after-write consistency. Expose projection lag as a metric. Document eventual consistency in auto-generated OpenAPI descriptions.

7. **Event store abstraction leaking SQLite semantics (CP-7, Phase: Event Store)** -- Use explicit `global_sequence` column (not SQLite ROWID), domain-level error types (not rusqlite errors), and keep the outbox as an implementation detail of the SQLite backend, not part of the `EventStore` trait. Test: "Can you write a mock EventStore without depending on rusqlite?"

## Implications for Roadmap

Based on combined research across all four files, the 10-step MVP decomposition from the 6-pager is confirmed as well-structured. The dependency chain, architectural layering, and pitfall distribution all support this ordering. Below are the suggested phases with cross-referenced rationale.

### Phase 1: Dev Environment (Step 1)
**Rationale:** Foundation for all subsequent steps. Already in progress on branch `001-mvp-dev-environment`.
**Delivers:** Workspace skeleton, rust-toolchain.toml, CI in Dockerfile, TDD infrastructure, workspace-level dependency pinning.
**Addresses features:** None directly -- infrastructure only.
**Avoids pitfalls:** mP-4 (configure clippy pedantic allows for derive macros), mP-5 (rusqlite feature flags correct from start).

### Phase 2: SDG Schema and Loader (Step 2)
**Rationale:** Every other component depends on the parsed `ServiceDefinition`. Bad type design here cascades through the entire system. Must design the task tracker SDG alongside the schema -- the SDG is both demo artifact and test fixture.
**Delivers:** JSON Schema (Draft 2020-12), `sdg-loader` crate with multi-pass validation (schema, graph, type checks), typed Rust structs for all SDG concepts, `ComputationDag` with flat Vec + pre-computed topological order.
**Addresses features:** SDG JSON Schema validation, computation DAG materialization, structured load-time error messages, version compatibility check.
**Avoids pitfalls:** CP-5 passes 1-2 (structural + graph validation), TD-1 (typed structs, not untyped Value), MP-8 (define hard DAG scope boundary), mP-2 (transition-oriented events, not field-change events), CP-3 (event type derivation includes version field).

### Phase 3: Event Store (Step 3)
**Rationale:** The densest pitfall concentration of any phase. 12 pitfalls map to this step. Trait design must be correct before any SQLite code -- it determines how aggregate-engine and projections interact with persistence. Schema design decisions (versioning columns, global sequence, UNIQUE constraints) are irreversible once events exist.
**Delivers:** `EventStore` trait, `Outbox` trait, `SqliteEventStore` implementation, event/outbox/checkpoint SQLite tables, `StoredEvent` / `NewEvent` / `EventMetadata` types.
**Addresses features:** Append-only event streams, per-aggregate streams, optimistic concurrency, event metadata envelope, event store abstraction layer.
**Avoids pitfalls:** CP-1 (single writer connection), CP-2 (UNIQUE constraint from first migration), CP-3 (event_version column), CP-4 (spawn_blocking), CP-7 (domain-level trait, no SQLite leaks), MP-2 (outbox cleanup design), MP-6 (no unwrap on deserialization), MP-7 (separate NewEvent/StoredEvent/PublishedEvent types), mP-1 (monotonic sequence for ordering, not timestamps), mP-3 (correlation/causation IDs in schema), mP-8 (temp SQLite per test), mP-9 (PII indirection decision).

### Phase 4: Aggregate Engine (Step 4)
**Rationale:** Highest-risk component. Novel computation DAG interpreter + model-driven state machine. Building this after Event Store enables end-to-end command execution testing (load SDG -> execute command -> verify stored events) without needing HTTP. The DAG interpreter is the hardest single component and benefits from early implementation for course correction.
**Delivers:** `AggregateEngine`, `AggregateState` (dynamic, serde_json::Value-based), state machine transitions, guard/computation DAG evaluator, command routing, event emission.
**Addresses features:** Aggregate state machine, command handling/routing, event replay, computation DAG execution (simple functions), guard conditions, domain error handling.
**Avoids pitfalls:** CP-5 passes 3-4 (semantic type validation), MP-5 (Given-When-Then test pattern from first test), MP-8 (hard DAG scope: FieldAccess, Literal, Comparison, BooleanLogic, Arithmetic, StringOp -- nothing else), TD-3 (in-memory LRU aggregate cache from day one), TD-2 (shared test fixtures using task tracker SDG).

### Phase 5: Transactional Outbox + Projections (Steps 5-6)
**Rationale:** These are tightly coupled -- the outbox bridges the event store to projections, and neither is useful without the other. Grouping them completes the full event sourcing loop: command -> event -> outbox -> projection -> query result. This is the first phase where the system demonstrates end-to-end CQRS behavior.
**Delivers:** Transactional outbox (atomic event + outbox writes), polling-based relay, projection engine with catch-up and live phases, checkpoint tracking, projection rebuild capability, SDG-driven `DynamicProjectionHandler`.
**Addresses features:** Transactional outbox, at-least-once delivery, async projection processing, projection position tracking, single-stream projections, projection rebuild, idempotent projection handlers.
**Avoids pitfalls:** CP-6 (projection lag metric + version in responses designed now), MP-2 (outbox cleanup in background), MP-3 (rebuild tested with hundreds of events), PT-2 (indexed projection tables), PT-3 (batched rebuilds to prevent WAL growth), PT-4 (outbox entry cleanup).

### Phase 6: HTTP API Surface + OpenAPI (Step 7)
**Rationale:** Makes everything accessible via HTTP. Dynamic route generation from SDG is the visible differentiator. Requires aggregate-engine and projections to be stable. OpenAPI spec must be generated programmatically from live SDG using utoipa's builder API, not derive macros.
**Delivers:** Dynamic axum router from SDG endpoint definitions, command handlers routing to aggregate engine, query handlers reading from projection tables, OpenAPI spec at `/openapi.json`, health check at `/health`, structured error responses.
**Addresses features:** Auto-generated command/query endpoints, OpenAPI spec generation, request/response JSON validation, structured error responses, content-type negotiation, health check.
**Avoids pitfalls:** CP-6 (version/timestamp in API responses, eventual consistency documented), MP-4 (OpenAPI from live SDG, not static file), mP-10 (axum 0.8 `/{param}` syntax).

### Phase 7: Middleware (Step 8)
**Rationale:** Security and correctness baseline. JWT auth can be stubbed initially but must be a real Tower layer. Request validation is per-handler (each endpoint has its own command schema from SDG). Error type hierarchy must map cleanly from domain errors to HTTP status codes.
**Delivers:** Tower layer pipeline (trace, correlation ID, JWT auth, timeout), per-handler request validation against SDG command schemas, structured `ErrorResponse` format, domain-to-HTTP error mapping.
**Addresses features:** JWT authentication, request validation, structured error handling, correlation ID propagation.
**Avoids pitfalls:** MP-1 (error types crossing crate boundaries cleanly -- establish pattern here).

### Phase 8: Observability (Step 9)
**Rationale:** Cross-cutting but can be wired in incrementally. The tracing macros (`#[instrument]`, `tracing::info!`) should be added to key functions throughout earlier phases, but the subscriber initialization, OTel exporter, and metrics registration belong here. Pin all four OTel crates together.
**Delivers:** OTel trace export via OTLP, structured JSON logging, runtime metrics (command_duration, command_total, event_store_append_duration, projection_lag_ms, api_request_duration), env-filter runtime log level configuration.
**Addresses features:** Structured logging, request tracing spans, basic metrics.
**Avoids pitfalls:** mP-6 (OTel version alignment: 0.31/0.31/0.31/0.32), TD-4 (instrument at crate boundaries, not as afterthought), mP-3 (correlation ID end-to-end verification).

### Phase 9: CLI Validation + End-to-End Demo (Step 10)
**Rationale:** Final integration. CLI validation reuses exactly the same SDG loader validation code. The task tracker SDG serves as both the demo artifact and the canonical integration test fixture. Graceful shutdown must drain in-flight requests, flush outbox, checkpoint projections, and close SQLite connections.
**Delivers:** `runtime validate` CLI command, task tracker SDG as working demo, full lifecycle integration test (create task -> transition -> query projection -> verify), graceful shutdown handling.
**Addresses features:** CLI SDG validation, human-readable validation output.
**Avoids pitfalls:** mP-7 (graceful shutdown: drain requests, flush outbox, checkpoint WAL).

### Phase Ordering Rationale

- **Strict dependency chain confirmed by architecture research:** sdg-loader and event-store are Layer 0 (no internal deps), aggregate-engine and projections are Layer 1 (depend on Layer 0), api-surface is Layer 2, runtime is Layer 3. Building in this order means each phase has its dependencies already built and tested.
- **Pitfall density drives rigor:** Step 3 (Event Store) has 12 mapped pitfalls -- the most of any phase. It must be built carefully with full awareness of these traps. Step 4 (Aggregate Engine) is highest-risk for scope creep (DAG interpreter).
- **Outbox and projections grouped:** Research across FEATURES.md and ARCHITECTURE.md confirms they are tightly coupled (outbox bridges events to projections). Neither is demonstrable alone.
- **Observability late but not absent:** `tracing` macros are zero-cost if no subscriber exists. Functions can be instrumented with `#[instrument]` from Phase 2 onward; the subscriber wiring happens in Phase 8. This avoids blocking domain work on OTel configuration.
- **CLI reuses, not reimplements:** CLI validation calls the exact same `sdg_loader::load_and_validate()` function. Building it last is trivial and ensures the validation path is identical.

### Research Flags

**Phases likely needing deeper research during planning:**
- **Phase 2 (SDG Schema + Loader):** The SDG JSON Schema itself needs design work -- this is domain-specific and not covered by general ES research. The task tracker example SDG should be designed first as a forcing function.
- **Phase 4 (Aggregate Engine / DAG):** Highest risk. The computation DAG evaluator's builtin operation set (what BuiltinOp variants to implement) needs careful scoping. Consider a research spike or proof-of-concept before full implementation.
- **Phase 6 (API Surface):** Dynamic route generation in axum and programmatic OpenAPI generation via utoipa's builder API are atypical patterns with less documentation. A small proof-of-concept would de-risk this phase.

**Phases with standard patterns (skip research):**
- **Phase 3 (Event Store):** Well-documented patterns across multiple sources. The pitfalls are well-known; prevention strategies are clear. Implement carefully, but no novel research needed.
- **Phase 5 (Outbox + Projections):** Transactional outbox and checkpoint-based catch-up projections are standard ES patterns documented by multiple authoritative sources (microservices.io, Marten, EventStoreDB).
- **Phase 7 (Middleware):** Tower layer composition and JWT validation are thoroughly documented in axum/tower ecosystem.
- **Phase 8 (Observability):** tracing + opentelemetry integration is well-documented. Pin versions and follow examples.
- **Phase 9 (CLI + Integration):** Trivial -- reuses existing loader. No research needed.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All versions verified against docs.rs as of April 2026. No alpha or experimental crates. Compatible version sets documented for OTel. |
| Features | HIGH | Table stakes derived from established ES ecosystem (Axon, Marten, EventStoreDB) and ratified project 6-pager/constitution. Anti-features explicitly justified. |
| Architecture | HIGH | 8-crate workspace with strict layering. Patterns proven in Rust ecosystem (axum state, tower layers, thiserror cross-crate). Model-driven approach is novel but composed of well-understood primitives. |
| Pitfalls | HIGH | 25 pitfalls identified across 4 severity levels from 20+ authoritative sources (event sourcing literature, SQLite docs, Rust ecosystem guides, production post-mortems). Phase mapping is comprehensive. |
| Computation DAG | MEDIUM | The flat-Vec + topological-order evaluation approach is sound, but the BuiltinOp set and type system need design work during implementation. No existing Rust library for this exact use case. |
| Dynamic OpenAPI | MEDIUM | utoipa's programmatic builder API is less documented than its derive macros. Proof-of-concept recommended before Phase 6. |

**Overall confidence:** HIGH

### Gaps to Address

- **SDG schema design:** The JSON Schema for the SDG format itself is not covered by any research file -- it is domain-specific and must be designed during Phase 2. The task tracker example SDG should be created first as a design driver.
- **Computation DAG type system:** How DAG nodes express and check type compatibility across edges (e.g., "this comparison node expects numeric inputs") needs design during Phase 4. Research identified the need but not the solution.
- **utoipa programmatic API:** Documentation for building OpenAPI specs dynamically (not via derive macros) is sparse. A proof-of-concept should precede Phase 6 implementation.
- **SQLite connection model under concurrent load:** The single-writer + pooled-readers pattern is well-documented in theory, but the specific `tokio::task::spawn_blocking` vs `tokio-rusqlite` decision should be validated with a load test during Phase 3.
- **Event store global ordering:** Use an explicit `global_sequence` column (not SQLite ROWID) for ordering events across aggregates. The column exists; the question is whether it should be an auto-increment integer or a separate counter.
- **Aggregate cache eviction:** In-memory LRU cache for aggregate state is recommended from Phase 4, but cache size, eviction policy, and invalidation-on-version-mismatch need design during implementation.

## Sources

### Primary (HIGH confidence)
- [axum 0.8.0 announcement](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0) -- axum 0.8 breaking changes, path syntax
- [axum GitHub](https://github.com/tokio-rs/axum) -- framework patterns, state extractor
- [tokio crates.io](https://crates.io/crates/tokio) -- LTS versions 1.47/1.51
- [rusqlite docs.rs](https://docs.rs/crate/rusqlite/latest) -- rusqlite 0.39.0 features
- [jsonschema GitHub](https://github.com/Stranger6667/jsonschema) -- performance benchmarks (75-645x faster)
- [petgraph docs.rs](https://docs.rs/crate/petgraph/latest) -- toposort, cycle detection
- [utoipa docs.rs](https://docs.rs/crate/utoipa/latest) -- OpenAPI generation
- [opentelemetry docs.rs](https://docs.rs/crate/opentelemetry/latest) -- version compatibility
- [tracing-opentelemetry docs.rs](https://docs.rs/crate/tracing-opentelemetry/latest) -- bridge version alignment
- [Tower Service trait](https://docs.rs/tower/latest/tower/trait.Service.html) -- middleware pattern
- [SQLite WAL Mode](https://www.sqlite.org/wal.html) -- single-writer concurrency model
- [Transactional Outbox Pattern](https://microservices.io/patterns/data/transactional-outbox.html) -- outbox design
- [cqrs-es docs](https://doc.rust-cqrs.org/) -- aggregate trait pattern (evaluated and rejected)
- [thiserror crates.io](https://crates.io/crates/thiserror) -- error handling pattern
- [Event Sourcing Pattern - Azure Architecture Center](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing)

### Secondary (MEDIUM confidence)
- [Axon Framework](https://www.axoniq.io/framework) -- DDD/CQRS/ES reference implementation
- [Marten Event Store](https://martendb.io/events/) -- projection and event store patterns
- [Kurrent (EventStoreDB)](https://www.eventstore.com/) -- live projections, catch-up subscriptions
- [Event-Driven.io](https://event-driven.io/) -- outbox patterns, projections guide, idempotent command handling
- [EventSourcingDB Common Issues](https://docs.eventsourcingdb.io/best-practices/common-issues/) -- pitfall catalog
- [GreptimeDB Error Handling](https://greptime.com/blogs/2024-05-07-error-rust) -- cross-crate error patterns
- [How to Build Event-Sourced Apps with CQRS in Rust](https://oneuptime.com/blog/post/2026-01-25-event-sourcing-cqrs-rust/view)
- [Async: What is blocking? (Alice Ryhl)](https://ryhl.io/blog/async-what-is-blocking/) -- tokio blocking pitfalls
- [SQLite concurrent writes](https://tenthousandmeters.com/blog/sqlite-concurrent-writes-and-database-is-locked-errors/)

### Tertiary (LOW confidence)
- [Oxide Outbox](https://github.com/Vancoola/oxide-outbox) -- Rust outbox implementation (less mature)
- [Don't Let the Internet Dupe You, Event Sourcing is Hard](https://chriskiehl.com/article/event-sourcing-is-hard) -- anecdotal but valuable production lessons

---
*Research completed: 2026-04-07*
*Ready for roadmap: yes*
