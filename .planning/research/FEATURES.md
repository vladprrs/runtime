# Feature Landscape: Model-Driven Event-Sourced Runtime

**Domain:** Developer platform / event-sourced runtime interpreter
**Researched:** 2026-04-07
**Confidence:** HIGH (features derived from established ES ecosystem patterns + project 6-pager)

## Table Stakes

Features the runtime MUST have or it is unusable as an event-sourcing platform. These are non-negotiable for the MVP to demonstrate value over hand-coding ES.

### SDG Loading & Validation

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| JSON Schema validation at load time | Constitution Principle VI: deterministic validation before execution. Invalid SDG must prevent startup, never produce a degraded runtime. | Medium | jsonschema crate handles validation; the schema design itself is the complex part |
| SDG file parsing and materialization | The entire runtime is model-driven; without loading the model, nothing works | Medium | serde_json deserialization into typed Rust structs |
| Computation DAG static analysis | DAG must be validated for cycles, unreachable nodes, type mismatches before accepting | Medium | Topological sort, type checking across node edges |
| Clear, structured load-time error messages | Domain specialists (not Rust devs) write SDGs; errors must be human-readable with file location, field path, and what's wrong | Medium | Aggregate all validation errors, don't fail on first |
| Version compatibility check | SDG format will evolve; runtime must reject incompatible versions cleanly | Low | Semantic version field in SDG header |

### Event Store (Append-Only Persistence)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Append-only event streams | Fundamental ES primitive. Events are immutable facts. | Low | INSERT-only SQLite operations |
| Per-aggregate streams | Each aggregate instance gets its own ordered event stream. Standard ES pattern used by Axon, Marten, EventStoreDB. | Low | Partition by aggregate_type + aggregate_id |
| Optimistic concurrency control | Prevents lost updates under concurrent command handling. Every ES platform requires this. | Medium | expected_version check on append; reject with conflict error if stream moved |
| Event ordering guarantees | Events within a stream must be strictly ordered by version. Global ordering across streams needed for projections. | Medium | Per-stream monotonic version + global sequence number |
| Event metadata | Every event needs: event_id (UUID), aggregate_id, aggregate_type, event_type, version, timestamp, correlation_id, causation_id | Low | Standard event envelope pattern |
| JSON event payload | Constitution decision: JSON format for events | Low | serde_json serialization |
| Event store abstraction layer | Must allow future replacement of SQLite with PostgreSQL/EventStoreDB without SDG changes (Constitution Principle III) | Medium | Trait-based abstraction over storage backend |

### Aggregate Engine (State Machine + Transitions)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| State machine with named states | Aggregates have defined states (e.g., Created, InProgress, Done). Transitions move between them. Core ES+DDD pattern. | Medium | Enum-like state with allowed transitions matrix |
| Command handling and routing | Commands arrive, get routed to the correct aggregate instance, validated, and processed. Axon Framework's core loop. | Medium | Command -> aggregate_type + aggregate_id -> load -> handle |
| Event replay to rebuild state | Load aggregate by replaying all its events from the event store. This IS event sourcing. | Medium | Fold/reduce events into current state |
| Guard conditions on transitions | Transitions have preconditions (e.g., "can only complete a task that is InProgress"). Business rule enforcement. | Medium | Evaluated from SDG computation DAG |
| Event emission from transitions | Successful transitions produce domain events that get appended to the event store | Low | Transition result -> event(s) |
| Computation DAG execution (simple functions) | SDG DAG nodes: comparisons, arithmetic, boolean logic, string ops, field access. The "no-code" value proposition. | High | DAG interpreter with typed node evaluation |
| Domain error handling | Commands that violate invariants must produce structured domain errors, not panics. Separate from system errors. | Medium | Result<Events, DomainError> pattern |

### Transactional Outbox

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Atomic event store + outbox write | Events and outbox entries written in same SQLite transaction. Solves dual-write problem. | Medium | Single transaction: INSERT events + INSERT outbox entries |
| At-least-once delivery guarantee | Outbox relay must deliver all events to consumers. May duplicate, never lose. | Medium | Polling-based relay for SQLite (no WAL/CDC needed at MVP) |
| Outbox entry tracking (delivered/pending) | Must know which events have been delivered and which are pending | Low | Status column + last_delivered_position |

### Projections (Read Model Builder)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Async projection processing | Background process reads event stream, applies projection logic, builds read models. Core CQRS pattern. | High | Async daemon with catch-up and live phases |
| Projection position tracking | Must track where each projection is in the event stream (last processed position) | Low | Checkpoint/cursor per projection |
| Single-stream projections | Project events from one aggregate stream into a read model (e.g., TaskDetail view) | Medium | Filter events by aggregate, apply handlers |
| Projection rebuild capability | Must be able to drop and rebuild any projection from scratch by replaying all events. Critical for schema evolution. | Medium | Reset checkpoint to 0, replay |
| Idempotent projection handlers | At-least-once delivery means projections may see duplicate events. Handlers must be safe to re-run. | Medium | Idempotency via position tracking or upsert semantics |

### HTTP API Surface

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Auto-generated command endpoints | SDG defines transitions; runtime auto-generates POST endpoints to invoke them. The "no-code API" value prop. | High | SDG transitions -> axum route handlers |
| Auto-generated query endpoints | SDG defines projections; runtime auto-generates GET endpoints to read them | Medium | SDG projections -> axum route handlers |
| OpenAPI spec generation | Machine-readable API documentation generated from SDG. Standard for any API platform. | Medium | Build OpenAPI JSON/YAML from SDG endpoint definitions |
| Request/response JSON validation | Incoming command payloads validated against SDG-defined schemas. Invalid requests rejected with structured errors. | Medium | jsonschema validation on request bodies |
| Structured error responses | Consistent error format: error code, message, field-level details. Not raw Rust panics. | Low | RFC 7807 Problem Details or similar |
| Content-type negotiation | Accept/Content-Type: application/json at minimum | Low | Standard axum middleware |

### Middleware Pipeline

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| JWT authentication | Verify JWT tokens on incoming requests. Standard auth mechanism. | Medium | jsonwebtoken crate, configurable JWKS/secret |
| Request validation | Validate request structure and payload before reaching command handler | Low | Part of API surface validation |
| Structured error handling | Consistent error responses across all middleware layers | Low | Error type hierarchy with HTTP status mapping |
| Correlation ID propagation | Every request gets a correlation_id that flows through command -> event -> projection for traceability | Low | Extract from header or generate, attach to context |

### Observability

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Structured logging | JSON-formatted logs with correlation_id, aggregate_id, command_type, event_type. Constitution Principle V. | Low | tracing + tracing-subscriber with JSON formatter |
| Request tracing (spans) | Trace the full lifecycle: HTTP request -> command -> aggregate load -> event append -> response | Medium | tracing spans with OpenTelemetry export |
| Basic metrics | Command throughput, latency, error rates. Event store write latency. API request rate. | Medium | opentelemetry metrics with prometheus exporter |
| Health check endpoint | Runtime must expose /health for container orchestration | Low | Simple axum endpoint returning 200 + store connectivity |

### CLI Validation

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Validate SDG file without starting runtime | "Is this SDG valid?" must be answerable without running the full server. Developer workflow essential. | Low | Reuse SDG loader validation, output errors, exit code |
| Human-readable validation output | Errors formatted for terminal consumption with colors, paths, suggestions | Low | clap CLI + colored output |

## Differentiators

Features that set this runtime apart from hand-coding event sourcing or using existing frameworks. These provide competitive advantage and justify the "model-driven" approach.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Zero-code service from JSON model** | Domain specialist writes JSON, gets a fully operational ES microservice. No Rust/Java/C# knowledge needed. This is THE differentiator. | N/A (emergent) | Emerges from table stakes features working together |
| **Model-behavior identity** | SDG diff IS behavior diff. No code to review, no drift to detect. The model is the running system. | N/A (architectural) | Guaranteed by interpretation, not code generation |
| **Automatic endpoint generation from model** | Add a transition to SDG, restart, API endpoint exists. No routing code, no controller classes. | High | Covered in table stakes, but the "automatic" aspect differentiates |
| **Built-in observability without instrumentation** | Every service gets the same metrics/traces/logs by default. No developer effort needed. | Medium | Constitution Principle V -- runtime provides, not developer |
| **Computation DAG for simple business logic** | Comparisons, field access, boolean logic -- expressed declaratively in JSON, not code. Visualizable, statically analyzable. | High | MVP scope: simple functions only. Decision tables/TS blocks are Phase 2. |
| **Projection rebuild from event stream** | Any read model can be rebuilt from scratch. Schema changes in projections are non-destructive -- just rebuild. | Medium | Standard ES capability but hidden complexity from user |
| **Single binary, zero-config deployment** | Rust binary + SQLite = no external DB setup, no runtime dependencies. Download, provide SDG, run. | Low | SQLite embedded in binary |
| **Deterministic load-time validation** | SDG errors caught at startup in seconds, not at runtime in production. Fail-fast with clear messages. | Medium | Prevents entire classes of production incidents |
| **Audit trail from the event store** | Complete history of every state change, every command, every transition. Free with ES. | Low | Event store IS the audit log; no additional infrastructure |

## Anti-Features

Features to deliberately NOT build in MVP. Each exclusion is intentional and justified.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| **TypeScript sandbox / V8 isolate** | Phase 2 scope. Embedding Deno adds massive complexity (V8 bindings, resource limits, typed I/O). MVP DAG covers simple functions. | Computation DAG with built-in simple functions. If a use case needs TS, it's not an MVP use case. |
| **Decision Tables in DAG** | Phase 2 scope. Tabular logic adds DAG node type complexity. Simple boolean conditions suffice for MVP. | AND/OR/NOT combinators in computation DAG |
| **Integration Call nodes** | Phase 2 scope. Calling external services from DAG requires timeout handling, circuit breaking, retry logic. | MVP services are self-contained. External integration is Phase 2. |
| **gRPC API surface** | Phase 3 scope. HTTP+JSON is sufficient for MVP validation. gRPC adds proto generation, streaming, reflection. | HTTP/JSON with OpenAPI spec |
| **Zeebe / BPMN Bridge** | Phase 3 scope. Cross-service orchestration requires Zeebe deployment, gRPC, job workers. MVP is single-service. | Single-service transitions via SDG state machine |
| **Production event store (PostgreSQL/EventStoreDB)** | Phase 3 scope. SQLite is sufficient for validation and first users. Production DB chosen after load testing per Constitution. | SQLite with abstraction layer that allows future swap |
| **On-demand snapshots** | Phase 3 scope. MVP event streams will be short. Snapshots add complexity (snapshot serialization, version management, trigger logic). | Full replay from event store. Short streams = fast replay. |
| **Event upcasting** | Phase 3 scope. MVP has no legacy events to transform. Upcasting adds versioned transformer pipeline. | SDG schema is new; no old events exist yet. |
| **Hot reload** | Explicit non-goal (Constitution Principle III). Restart-only simplifies state management enormously. | Restart on SDG change. Fast startup + SQLite = seconds. |
| **CRUD persistence mode** | Architectural exclusion (Constitution Principle II). ES is the only mode. | Event sourcing for everything. Runtime hides ES complexity from domain specialist. |
| **Multi-tenancy** | One instance per service. Multi-tenancy adds auth complexity, data isolation, noisy-neighbor concerns. | Deploy separate instances per service. Operating Layer handles multi-tenancy. |
| **UI rendering** | Runtime is an API backend. UI is a separate concern. | Consume auto-generated API from any frontend. |
| **Event streaming / Kafka integration** | Adds external dependency, consumer group management, offset tracking. MVP outbox is polling-based. | Transactional outbox with internal polling relay |
| **Command idempotency (deduplication)** | Nice-to-have but adds command ID tracking, TTL, storage. MVP commands are not idempotent-by-default. | Optimistic concurrency prevents most duplicate issues. Full idempotency is Phase 2+. |
| **Rate limiting** | Operational concern for production. MVP is for validation, not production traffic. | Not implemented. Add in production hardening phase. |
| **Saga / process manager** | Cross-aggregate coordination adds significant complexity. MVP is single-aggregate transitions. | Each aggregate manages its own state machine. Cross-aggregate is Phase 2+ (or Zeebe in Phase 3). |
| **Multi-stream projections** | Projections across multiple aggregate types add join logic, ordering challenges. | Single-stream projections only in MVP. |
| **Inline (synchronous) projections** | Synchronous projection updates during command handling add latency and coupling. | Async projections only. Accept eventual consistency for read models. |
| **Event store compaction / archival** | MVP event volumes are trivial. Compaction is a production-scale concern. | Retain all events. Address when event volume becomes a problem. |
| **Custom middleware plugins** | Extensible middleware pipeline is useful but adds plugin API design burden. | Fixed middleware stack: JWT auth, validation, errors, correlation. |

## Feature Dependencies

```
SDG Schema & Loader ─────────────────────────────────────────────────┐
    |                                                                 |
    v                                                                 v
Event Store (SQLite) ──────> Aggregate Engine ──────> HTTP API Surface
    |                            |                         |
    |                            v                         v
    |                     Computation DAG            OpenAPI Generation
    |                            |
    v                            v
Transactional Outbox ────> Projections
    |
    v
[Consumers - internal only for MVP]

Middleware ──────> HTTP API Surface (wraps it)
Observability ──> Everything (cross-cutting)
CLI Validation ──> SDG Schema & Loader (reuses validation)
```

**Critical path:** SDG Loader -> Event Store -> Aggregate Engine -> API Surface

Dependency details:
- **SDG Loader** is foundation: every other component reads SDG definitions
- **Event Store** must exist before Aggregate Engine can persist events
- **Aggregate Engine** must exist before API Surface can route commands
- **Transactional Outbox** depends on Event Store (same transaction)
- **Projections** depend on Outbox (receive events from it) and SDG (define projection shape)
- **API Surface** depends on SDG (endpoint definitions), Aggregate Engine (command handling), Projections (query handling)
- **Middleware** wraps API Surface (decorates handlers)
- **Observability** is cross-cutting, wired into every component
- **CLI Validation** reuses SDG Loader's validation logic

## MVP Recommendation

### Prioritize (in dependency order):

1. **SDG Schema & JSON Schema Validation + Loader** -- Foundation. Everything reads from this. Define the task tracker SDG early; it drives all other development.
2. **Event Store (SQLite)** -- Core persistence. Simple to implement, enables TDD of aggregate engine against real storage.
3. **Aggregate Engine (state machine + transitions + DAG)** -- The runtime's brain. Takes the most effort. Computation DAG interpreter is the hardest single component.
4. **Transactional Outbox** -- Bridges event store to projections. Relatively small if built alongside event store.
5. **Simple Projections** -- Demonstrates the CQRS read side. Proves the full event flow works end-to-end.
6. **HTTP API Surface + OpenAPI** -- Makes everything accessible. Auto-generation from SDG is the visible differentiator.
7. **Middleware (JWT, validation, errors)** -- Production-readiness signals. JWT can be stubbed initially.
8. **Observability (OTel, tracing, logging)** -- Cross-cutting but can be wired in incrementally.
9. **CLI Validation** -- Developer workflow tool. Reuses SDG loader, small incremental effort.

### Defer explicitly:
- **TypeScript sandbox**: Phase 2. MVP computation DAG handles simple logic without V8.
- **Decision Tables**: Phase 2. Boolean combinators are sufficient for task tracker demo.
- **Integration Calls**: Phase 2. MVP services are self-contained.
- **Snapshots**: Phase 3. MVP event streams are short.
- **Event upcasting**: Phase 3. No legacy events in a greenfield.
- **gRPC**: Phase 3. HTTP/JSON sufficient for validation.
- **Zeebe Bridge**: Phase 3. Single-service scope in MVP.

## Feature Prioritization Matrix

| Feature | Table Stakes? | Differentiator? | Complexity | MVP Step | Risk Level |
|---------|:------------:|:---------------:|:----------:|:--------:|:----------:|
| SDG JSON Schema + validation | Yes | Yes (deterministic) | Medium | 2 | Low |
| SDG Loader + DAG materialization | Yes | Yes (model-driven) | Medium | 2 | Medium |
| Event Store (SQLite, append-only) | Yes | No (standard ES) | Medium | 3 | Low |
| Per-aggregate streams | Yes | No (standard ES) | Low | 3 | Low |
| Optimistic concurrency | Yes | No (standard ES) | Medium | 3 | Low |
| Event store abstraction trait | Yes | No (enables swap) | Medium | 3 | Low |
| Aggregate state machine | Yes | No (standard ES) | Medium | 4 | Medium |
| Command handling + routing | Yes | Yes (auto from SDG) | Medium | 4 | Medium |
| Event replay | Yes | No (standard ES) | Medium | 4 | Low |
| Computation DAG (simple functions) | Yes | Yes (no-code logic) | High | 4 | High |
| Guard conditions | Yes | No (standard) | Medium | 4 | Low |
| Transactional outbox | Yes | No (standard pattern) | Medium | 5 | Medium |
| At-least-once delivery | Yes | No (standard) | Medium | 5 | Medium |
| Async projection processing | Yes | No (standard CQRS) | High | 6 | High |
| Projection position tracking | Yes | No (standard) | Low | 6 | Low |
| Projection rebuild | Yes | Yes (zero-migration) | Medium | 6 | Medium |
| Auto-generated command endpoints | Yes | Yes (key differentiator) | High | 7 | High |
| Auto-generated query endpoints | Yes | Yes (key differentiator) | Medium | 7 | Medium |
| OpenAPI spec generation | Yes | Yes (documentation-free) | Medium | 7 | Medium |
| Request validation | Yes | No (standard) | Medium | 7 | Low |
| Structured error responses | Yes | No (standard) | Low | 7-8 | Low |
| JWT authentication | Yes | No (standard) | Medium | 8 | Low |
| Correlation ID propagation | Yes | No (standard) | Low | 8 | Low |
| Structured logging | Yes | Yes (built-in) | Low | 9 | Low |
| Request tracing | Yes | Yes (built-in) | Medium | 9 | Medium |
| Basic metrics | Yes | Yes (built-in) | Medium | 9 | Medium |
| Health check | Yes | No (standard) | Low | 9 | Low |
| CLI SDG validation | Yes | Yes (dev workflow) | Low | 10 | Low |

### Risk Assessment

**High-risk features** (most likely to cause delays or require rework):

1. **Computation DAG interpreter** (Step 4) -- The most novel component. No existing Rust library to lean on. Must define node types, type system, evaluation semantics. This is where "simple" could spiral into scope creep.
   - *Mitigation:* Start with minimal node types (field access, comparison, boolean logic). Add arithmetic and string ops only when task tracker demo requires them.

2. **Auto-generated API endpoints** (Step 7) -- Mapping SDG definitions to axum handlers dynamically at runtime. Not typical Rust pattern (Rust prefers compile-time routing).
   - *Mitigation:* Use axum's Router::new() with dynamic route registration. Build a route factory that reads SDG and creates handlers.

3. **Async projection daemon** (Step 6) -- Background task that must handle catch-up, live processing, error recovery, and checkpoint management without blocking command handling.
   - *Mitigation:* Start with simple polling loop. Catch-up first, then live subscription. No need for sophisticated backpressure in MVP.

## Sources

- [Axon Framework - DDD, CQRS and Event Sourcing](https://www.axoniq.io/framework)
- [Axon Framework 5 - Dynamic Consistency Boundary](https://www.axoniq.io/blog/dcb-in-af-5)
- [Marten Event Store](https://martendb.io/events/)
- [Marten Projections](https://martendb.io/events/projections/)
- [Kurrent (EventStoreDB) - Event-native data platform](https://www.eventstore.com/)
- [Live projections for read models - Kurrent](https://www.kurrent.io/blog/live-projections-for-read-models-with-event-sourcing-and-cqrs)
- [Event Sourcing Pattern - Azure Architecture Center](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing)
- [Idempotent Command Handling - Event-Driven.io](https://event-driven.io/en/idempotent_command_handling/)
- [Common Issues - EventSourcingDB](https://docs.eventsourcingdb.io/best-practices/common-issues/)
- [Transactional Outbox Pattern - microservices.io](https://microservices.io/patterns/data/transactional-outbox.html)
- [Outbox, Inbox patterns and delivery guarantees - Event-Driven.io](https://event-driven.io/en/outbox_inbox_patterns_and_delivery_guarantees_explained/)
- [Projections in Event Sourcing - CodeOpinion](https://codeopinion.com/projections-in-event-sourcing-build-any-model-you-want/)
- [Guide to Projections and Read Models - Event-Driven.io](https://event-driven.io/en/projections_and_read_models_in_event_driven_architecture/)
- [Event Sourcing Fails: 5 Real-World Lessons](https://kitemetric.com/blogs/event-sourcing-fails-5-real-world-lessons)
- [Don't Let the Internet Dupe You, Event Sourcing is Hard](https://chriskiehl.com/article/event-sourcing-is-hard)
- [OpenTelemetry Best Practices - Better Stack](https://betterstack.com/community/guides/observability/opentelemetry-best-practices/)
