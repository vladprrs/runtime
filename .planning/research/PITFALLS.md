# Domain Pitfalls

**Domain:** Model-driven event-sourced Rust runtime (SDG interpreter)
**Researched:** 2026-04-07
**Overall Confidence:** HIGH (multi-source verification across event sourcing literature, Rust ecosystem docs, SQLite documentation)

---

## Critical Pitfalls

Mistakes that cause rewrites, data corruption, or architectural dead-ends. Each carries a phase mapping for when to address it.

---

### CP-1: SQLite Single-Writer Bottleneck Treated as "Just MVP"

**What goes wrong:** SQLite allows only one writer at a time, even in WAL mode. Under concurrent command processing (multiple HTTP requests hitting different aggregates simultaneously), all event appends serialize through a single database-level write lock. Developers assume "SQLite is fine for MVP" without designing the write path to account for this, then discover under moderate load that commands queue behind each other, latency spikes, and `SQLITE_BUSY` errors propagate to API clients.

**Why it happens:** WAL mode allows concurrent reads alongside a single write, which gets conflated with "concurrent writes work." The distinction is critical: multiple tokio tasks issuing `INSERT INTO events` contend on the database-level write lock, not row-level locks.

**Consequences:**
- `SQLITE_BUSY` errors returned to API clients under any concurrent load
- Aggregate commands that should be independent block each other
- The event store abstraction leaks SQLite-specific retry/backoff semantics into the aggregate engine
- WAL file grows unbounded if checkpoint starvation occurs during sustained writes with long-running read transactions

**Warning signs:**
- No `busy_timeout` configured in rusqlite connection setup
- Using multiple rusqlite connections for writes (only readers should be pooled)
- No backpressure mechanism in the command bus
- Tests only exercise single-threaded command processing

**Prevention:**
- Use a **single dedicated writer connection** for all event appends, serialized through a channel/queue (not connection pooling for writes)
- Pool multiple reader connections for aggregate replay and projection queries
- Configure `PRAGMA busy_timeout = 5000` on all connections
- Configure `PRAGMA journal_mode = WAL` and `PRAGMA synchronous = NORMAL` at connection open
- Configure `PRAGMA wal_autocheckpoint = 1000` to prevent WAL growth
- Use `tokio-rusqlite` or `spawn_blocking` to avoid blocking the tokio runtime
- Design the `EventStore` trait so the backing store abstraction does NOT expose connection-level details
- Load test with concurrent commands early

**Detection:** Monitor `tokio_runtime_worker_overflow_count` metric. Watch for latency spikes correlated with event store operations.

**Phase:** Event Store (Step 3). Must be addressed from day one, not deferred.

---

### CP-2: Optimistic Concurrency Check Missing or Incorrect

**What goes wrong:** The event store appends events without checking that the aggregate version matches the expected version. Two concurrent commands against the same aggregate both read version N, both try to write version N+1, and both succeed -- producing a corrupted event stream with duplicate version numbers and divergent state.

**Why it happens:** Developers implement the "append event" path first, then plan to "add concurrency control later." Or they check versions in application code but not at the database constraint level, leaving a race condition window.

**Consequences:**
- Silent data corruption: aggregate state becomes inconsistent and unrecoverable
- Events cannot be replayed deterministically (two events claim same position)
- Projections produce wrong results from corrupted streams
- No way to recover without manual event stream surgery

**Warning signs:**
- No `UNIQUE` constraint on `(aggregate_id, version)` in the events table
- Version check happening in Rust code before the INSERT, not as part of the INSERT/transaction
- No tests for concurrent command handling against the same aggregate
- No retry logic for version conflicts

**Prevention:**
- Add a `UNIQUE(aggregate_id, version)` constraint to the events table schema from day one
- Use a single SQL transaction: INSERT with the unique constraint violation as the concurrency check
- Return a typed `ConcurrencyConflict` error (not a generic DB error) from the event store
- Implement retry at command-handler level: reload aggregate, re-evaluate command, retry append (2-3 attempts)
- Test explicitly: spawn two tasks issuing conflicting commands to the same aggregate, assert exactly one succeeds and the other gets a conflict error

**Detection:** Two events with the same `(aggregate_id, version)` in the database. Add a startup integrity check.

**Phase:** Event Store (Step 3). Schema design decision that must be correct in the first migration.

---

### CP-3: Event Schema Locked Into V1 Without Versioning Strategy

**What goes wrong:** Events are stored as JSON blobs with no version field. When the SDG model evolves (new fields, renamed fields, restructured payloads), old events cannot be deserialized by the new code. The system either crashes on replay or silently drops data.

**Why it happens:** In early development, the schema feels stable. Versioning is "something we'll add when we need it." But by the time you need it, you have an event store full of unversioned events that you cannot distinguish from each other.

**Consequences:**
- Aggregate replay fails after SDG model changes
- Cannot add required fields to events (old events don't have them)
- Forced choice between dangerous data migration (rewriting immutable events) or carrying permanent backward-compatibility baggage
- Projection rebuilds fail on historical events

**Warning signs:**
- Event JSON has no `version` or `schema_version` field
- No upcasting/migration layer between raw JSON and domain event types
- SDG changes that add/rename aggregate fields are tested only with fresh event stores
- Tests never replay events created by older SDG versions

**Prevention:**
- Every event type gets a version number from day one: store `event_type` and `event_version` as separate columns (not inside the JSON payload)
- Store event metadata (type, version, timestamp, aggregate_id, aggregate_version) separately from the payload in the events table
- Design an upcasting pipeline: `raw JSON -> upcaster chain -> current domain event struct` (even if the chain is a no-op in Phase 1)
- The **event storage format** must accommodate versioning from Phase 1, even though upcasting logic is Phase 3
- Keep fixture files of events from each SDG version in the test suite
- Test that current code can deserialize events from all prior versions

**Detection:** Any `serde_json::from_value` call that could fail on old event formats without producing a clear, actionable error.

**Phase:** Event Store schema design (Step 3), SDG Loader (Step 2) for event type derivation. Storage format is a Phase 1 decision; upcasting logic is Phase 3.

---

### CP-4: Blocking the Tokio Runtime with Synchronous SQLite Calls

**What goes wrong:** rusqlite is a synchronous library. Calling it directly from an async context (inside a `tokio::spawn` task or an axum handler) blocks the tokio worker thread, starving other tasks. Under load, all worker threads block on SQLite, and the entire runtime becomes unresponsive.

**Why it happens:** The code "works" in single-request testing. The problem only manifests under concurrent load when multiple async tasks simultaneously hit the database. With tokio's default 4-8 worker threads, a few blocked calls paralyze the server.

**Consequences:**
- Complete runtime freeze under moderate concurrent load
- Axum stops accepting new connections
- Health checks time out, orchestrator restarts the service
- Extremely difficult to diagnose: no explicit error, just "everything stops"

**Warning signs:**
- Direct `rusqlite::Connection` usage inside `async fn` handlers
- No `spawn_blocking` or `tokio-rusqlite` wrapper around database calls
- Load tests not part of CI
- No tracing spans around database operations to detect blocking

**Prevention:**
- Use `tokio-rusqlite` which runs each connection on a dedicated background thread and communicates via channels
- Alternatively, use `tokio::task::spawn_blocking` for every database call
- Never hold a rusqlite `Connection` or `Transaction` across an `.await` point
- Add tracing instrumentation to every database call to detect unexpected latency
- Include a concurrent request load test (even minimal: 20 concurrent commands) in CI
- Note: `tokio-rusqlite` uses unbounded channels; for backpressure, consider `async-rusqlite` which uses bounded channels

**Detection:** Monitor `tokio_runtime_worker_overflow_count`. Latency spikes correlating with database operations.

**Phase:** Event Store (Step 3). Architecture decision at crate design time.

---

### CP-5: Computation DAG Validated Structurally But Not Semantically

**What goes wrong:** The SDG Loader validates that the DAG is well-formed JSON matching the schema, has no cycles, and all node references resolve. But it doesn't validate semantic correctness: type mismatches between node outputs and downstream inputs, unreachable nodes, impossible guard conditions, or missing required fields in event payloads.

**Why it happens:** Structural validation (JSON Schema) is straightforward. Semantic validation requires building a type system for the DAG -- type inference across nodes, checking that a "greater_than" node receives numeric inputs, verifying that the event payload construction covers all required fields.

**Consequences:**
- Runtime errors during command processing that should have been caught at load time (violates Constitution Principle VI: "Deterministic Validation Before Execution")
- Cryptic error messages deep in DAG execution instead of clear validation errors at startup
- SDG authors don't know their model is broken until they issue the specific command that exercises the broken path
- Debugging requires tracing through the DAG execution to find the type mismatch

**Warning signs:**
- SDG validation only checks JSON Schema conformance
- No type-checking pass over the DAG node graph
- Error messages reference internal node IDs rather than SDG-level concepts
- Tests only validate that "valid" SDGs load, never that "subtly invalid" SDGs are rejected

**Prevention:**
- Implement multi-pass validation in SDG Loader:
  1. Schema validation (JSON Schema conformance)
  2. Graph validation (cycle detection via topological sort, connectivity, unreachable nodes)
  3. Type validation (input/output type compatibility across edges)
  4. Completeness validation (event payloads cover all required fields, guards reference valid state values)
- Each validation pass produces structured errors with SDG-level context (aggregate name, transition name, node path)
- Build a test suite of intentionally broken SDGs that must be rejected with specific error messages
- The CLI validation command (`sdg validate`) must run all passes

**Detection:** Any runtime panic or error inside DAG evaluation that could have been caught at load time.

**Phase:** SDG Loader (Step 2) for passes 1-2, Aggregate Engine (Step 4) for passes 3-4 (requires knowledge of supported types).

---

### CP-6: Projection Lag Ignored in API Design

**What goes wrong:** The API returns data from eventually-consistent projections without informing the client. A user creates a task (command succeeds, event stored), then immediately queries the task list (projection hasn't caught up), and gets a response that doesn't include the newly created task. Users report "data loss."

**Why it happens:** Developers test with a single request at a time. Projections catch up in milliseconds during dev. The lag is invisible in the happy path but becomes user-visible under any real load or when projections process slowly.

**Consequences:**
- Users see stale data after write operations (read-your-writes violation)
- Support tickets about "lost" data
- Frontend developers build polling/retry hacks
- Trust in the system erodes

**Warning signs:**
- API handlers return projection data immediately after command acceptance
- No mechanism for "read-after-write consistency" or causal consistency
- No projection lag metric exposed
- API responses don't include version/timestamp metadata for clients to reason about freshness

**Prevention:**
- Return the command result (aggregate version after write) in the command response, so clients can wait for projections to catch up
- Implement a "wait-for-version" query option: client passes expected minimum version, API blocks briefly (with timeout) or returns 409 if projection hasn't caught up
- For critical reads immediately after writes, offer a "sync read" path that replays from the event store (slower but consistent)
- Expose projection lag as a metric (Constitution Principle V requires this)
- Include version/timestamp in all API query responses
- Document eventual consistency behavior in auto-generated OpenAPI descriptions

**Detection:** Client reports of "just created" items not appearing in list queries.

**Phase:** Projection Engine (Step 5), API Surface (Step 6). Must be designed into the API contract, not bolted on later.

---

### CP-7: Event Store Abstraction That Leaks SQLite Semantics

**What goes wrong:** The `EventStore` trait exposes methods shaped around SQLite capabilities: outbox in same transaction, ROWID for global ordering, SQLite JSON functions in queries, UPSERT syntax. When Phase 3 introduces PostgreSQL or EventStoreDB, the trait doesn't fit and requires a rewrite of all consumers.

**Why it happens:** SQLite is convenient and its features are tempting. The trait abstraction seems like enough, but implementation details leak through the API, error types, and transaction boundaries.

**Consequences:**
- Major refactor when migrating off SQLite
- Potential data model redesign
- Projection logic depends on SQLite-specific ordering guarantees
- Test mocks must reproduce SQLite-specific behavior

**Warning signs:**
- `rusqlite::Error` appearing in the event store trait's associated types
- Trait methods that assume the outbox is in the same database
- Code using SQLite ROWID for global ordering instead of an explicit column
- Cannot write a mock `EventStore` implementation without depending on rusqlite

**Prevention:**
- Use an explicit `global_sequence` column (not SQLite ROWID) for global ordering
- Keep the `EventStore` trait in terms of domain concepts: `append(stream_id, expected_version, events) -> Result<(), AppendError>`
- The outbox is an implementation detail of the SQLite-backed store, not part of the trait
- Error types are domain-level (`ConcurrencyConflict`, `StreamNotFound`), not database-level
- Test the trait contract with a mock implementation in every consumer crate
- Document which guarantees the trait provides (ordered per-aggregate, globally ordered, at-least-once outbox delivery)

**Detection:** Can you write a mock `EventStore` for testing without depending on rusqlite? If not, the abstraction leaks.

**Phase:** Event Store (Step 3). The trait design is the first thing to implement, before any SQLite code.

---

## Moderate Pitfalls

Mistakes that cause significant rework or degraded quality but don't corrupt data.

---

### MP-1: Error Types That Don't Cross Crate Boundaries Cleanly

**What goes wrong:** Each crate defines its own error enum with `thiserror`. The binary crate (`runtime`) must handle errors from all 7 library crates. Without careful design, error types either: (a) leak internal dependency types via `#[from]` (forcing `runtime` to depend on `rusqlite`, `jsonschema`, etc.), or (b) lose context through opaque wrapping (making debugging impossible).

**Warning signs:**
- `#[from] rusqlite::Error` in a public error type
- The runtime crate's `Cargo.toml` lists dependencies that should be internal to library crates
- Error messages that say "database error" with no aggregate/command context

**Prevention:**
- Each library crate exports a `pub enum Error` with domain-meaningful variants, not raw dependency errors
- Use `#[source]` with `Box<dyn std::error::Error + Send + Sync>` for internal errors rather than `#[from]` on dependency types
- The `runtime` crate uses `thiserror` for its own error enum that wraps crate-level errors
- Never use `anyhow` in library crates -- only in the binary crate if at all
- Design error types in the first implementation of each crate, not retrofitted
- Follow the GreptimeDB pattern: separate error definition from error context enrichment

**Phase:** Every crate (Steps 2-9). Establish the pattern in Step 2 (sdg-loader) and enforce consistency.

---

### MP-2: Outbox Polling Without Backpressure or Cleanup

**What goes wrong:** The transactional outbox table grows without bound because cleanup runs infrequently or not at all. The polling query becomes slow as the table fills. Or: the outbox processor crashes after publishing events but before marking them as processed, causing duplicate delivery -- and downstream consumers have no idempotency protection.

**Warning signs:**
- Outbox table row count growing monotonically over time
- No `DELETE` or cleanup task for processed outbox entries
- No idempotency check in projection event handlers
- No test for the crash-between-publish-and-acknowledge scenario

**Prevention:**
- Write outbox entries in the same SQLite transaction as the event append (the whole point of the pattern)
- Delete processed outbox entries in small batches (not bulk `DELETE WHERE processed = true` -- causes index rebuilds and lock contention in SQLite)
- Mark entries with a processed timestamp, then delete entries older than a threshold in a background task
- All outbox consumers must be idempotent (use event ID as deduplication key)
- Monitor outbox table size and processing lag as metrics
- Test the crash-restart scenario: kill the outbox processor mid-publish, restart, verify no events are lost and duplicates are handled

**Phase:** Event Store / Outbox (Step 3), Projection Engine (Step 5).

---

### MP-3: Projection Rebuild Not Tested or Impractically Slow

**What goes wrong:** Projections are built incrementally as events arrive. Nobody tests rebuilding from scratch until it's needed (schema change, bug fix, new projection). The rebuild takes hours because it replays the entire event store, and there's no progress tracking or cancellation.

**Warning signs:**
- No test that drops a projection table and rebuilds it from events
- Projection and event store share the same SQLite database file
- No "high water mark" tracking per projection
- No progress reporting during rebuild

**Prevention:**
- Test projection rebuild in CI with a non-trivial event count (hundreds of events)
- Implement rebuild as a batched operation with progress reporting
- Design projections to be droppable and rebuildable (separate database/table that can be truncated)
- Track the "high water mark" (last processed event ID) per projection
- Do NOT share the SQLite database file between event store and projection store (rebuilds truncate projection tables; in SQLite this means write contention with event appends and WAL growth)

**Phase:** Projection Engine (Step 5). Rebuild capability must be part of initial design.

---

### MP-4: OpenAPI Spec Generated Statically Instead of From Live SDG

**What goes wrong:** The OpenAPI spec is generated at compile time or as a static file. When the SDG changes, the spec must be regenerated manually. This creates the exact "model-code drift" the project aims to eliminate. Or: the spec reflects the SDG structure but not the runtime's actual validation behavior, error responses, or authentication requirements.

**Warning signs:**
- OpenAPI spec is a checked-in JSON/YAML file, not generated at startup
- Using utoipa derive macros for SDG-defined types (they're compile-time only)
- Spec doesn't include error response schemas
- Spec doesn't document eventual consistency behavior

**Prevention:**
- Generate OpenAPI spec dynamically at runtime from the loaded SDG using utoipa's programmatic `OpenApiBuilder` API
- Use derive macros only for static types (error responses, health check endpoints)
- The spec must include: all error response schemas, auth requirements, pagination parameters, version/freshness metadata
- Serve from a well-known endpoint (`/openapi.json`)
- Test that the spec matches actual API behavior via contract tests

**Phase:** API Surface (Step 6).

---

### MP-5: Testing State Instead of Behavior in Aggregate Tests

**What goes wrong:** Tests assert on aggregate internal state (`assert_eq!(aggregate.status, "InProgress")`) rather than on emitted events. This couples tests to internal representation, makes refactoring painful, and misses the core invariant.

**Warning signs:**
- Test code accesses aggregate fields directly
- Tests break when internal state representation changes (even if behavior is unchanged)
- No Given-When-Then structure in aggregate tests
- Projection tests assert on internal data structures rather than query responses

**Prevention:**
- Use the Given-When-Then pattern: Given [past events] When [command] Then [expected new events]
- Never expose aggregate internal state in the public test API
- For error cases: Given [past events] When [command] Then [expected error]
- For idempotent commands: Given [past events] When [command] Then [no events emitted]
- For projections: Given [events] When [query] Then [expected response] (NOT "Then [internal state]")
- Build a test harness that makes this pattern ergonomic (a `TestAggregate` builder)

**Phase:** Aggregate Engine (Step 4). Establish the test pattern before writing the first aggregate test.

---

### MP-6: serde Deserialization of Events Panics Instead of Returning Errors

**What goes wrong:** Event deserialization uses `serde_json::from_value::<T>().unwrap()` or similar panicking code. When an event's JSON doesn't match the expected struct (due to schema evolution, corruption, or bugs), the entire runtime crashes on aggregate replay.

**Warning signs:**
- `.unwrap()` or `.expect()` on any `serde_json` call in the event loading path
- No test with intentionally malformed event JSON
- No `DeserializationError` variant in the event store error type

**Prevention:**
- Never use `.unwrap()` or `.expect()` on event deserialization
- Return `Result<Event, DeserializationError>` from the event store's load path
- Handle deserialization failures gracefully: log the error with event metadata (ID, type, version), skip or quarantine the event, report via observability
- Test with intentionally malformed event JSON to verify graceful handling

**Phase:** Event Store (Step 3), Aggregate Engine (Step 4).

---

### MP-7: Conflating Event Store Events with Domain Events

**What goes wrong:** Using the same event struct for persistence (stored in SQLite), domain logic (aggregate state transitions), and external publication (outbox delivery). Metadata requirements differ for each use case, leading to a bloated event type or missing information.

**Why it happens:** "It's all events." In reality, a stored event needs metadata (global sequence, timestamp, aggregate version), a domain event needs only type and payload, and a published event might need additional routing information.

**Prevention:** Separate types:
- `NewEvent` -- what the aggregate engine produces (event_type, payload)
- `StoredEvent` -- what the event store returns (event_type, payload + aggregate_id, version, global_sequence, timestamp, correlation_id, causation_id)
- `PublishedEvent` -- what projections receive (event_type, payload, aggregate_type, aggregate_id, version)

**Detection:** Single `Event` struct with optional fields that are "sometimes set."

**Phase:** Event Store design (Step 3).

---

### MP-8: Computation DAG Scope Creep Into a Language Interpreter

**What goes wrong:** The DAG starts with "simple functions" (comparison, boolean, field access) but scope creeps into loops, recursion, closures, custom types, error handling within DAG nodes. The DAG becomes a Turing-complete interpreter, defeating the purpose of the TS sandbox.

**Warning signs:**
- PRs adding new DagNode variants with conditional or recursive semantics
- Node evaluation functions growing past 50 lines
- Requests for "just one more DAG capability" to avoid Phase 2 TS blocks
- DAG evaluation producing stack overflows on malformed inputs

**Prevention:**
- Define a hard boundary: DAG nodes are pure functions with typed inputs/outputs
- MVP node types: FieldAccess, Literal, Comparison (eq, neq, gt, lt, gte, lte), BooleanLogic (and, or, not), Arithmetic (add, sub, mul, div), StringOp (concat, uppercase, lowercase)
- If a use case can't be expressed with these nodes, it's a Phase 2 TS Block use case
- No loops, no recursion, no side effects, no error-catching within DAG
- Track "DAG feature request" count as a metric for DAG expressiveness

**Phase:** SDG Schema (Step 2), Aggregate Engine (Step 4).

---

## Minor Pitfalls

Issues that cause friction, confusion, or minor bugs but are recoverable.

---

### mP-1: Event Timestamps Using System Clock Without Monotonicity

**What goes wrong:** Events get timestamps from `SystemTime::now()` which can go backward (NTP adjustments, clock skew). Event ordering appears inconsistent. Projections that sort by timestamp produce wrong results.

**Prevention:**
- Use monotonic sequence numbers (aggregate version, global_sequence) for ordering, never timestamps
- Include timestamp as metadata for human display only, not for ordering logic
- Document that event ordering is defined by `(aggregate_id, version)` and `global_sequence`, not by timestamp

**Phase:** Event Store schema design (Step 3).

---

### mP-2: Overly Chatty Events That Defeat the Audit Trail

**What goes wrong:** Every field change emits a separate event ("TaskTitleChanged", "TaskDescriptionChanged", "TaskPriorityChanged"). The event stream becomes noisy, replay is slow, and the audit trail is useless because you can't distinguish meaningful business actions from trivial edits.

**Prevention:**
- Design events around business actions, not field changes: "TaskPrioritized", "TaskReassigned" rather than "TaskFieldChanged"
- The SDG schema should encourage transition-oriented events (state machine transitions naturally produce meaningful events)
- Limit event types per aggregate (warning at > 15-20 event types)

**Phase:** SDG Schema Design (Step 2).

---

### mP-3: Missing Correlation/Causation IDs in Event Metadata

**What goes wrong:** Events don't carry correlation IDs linking them to the originating command/request. When debugging production issues, you can't trace which API request caused which events, or which events triggered which projections.

**Prevention:**
- Every event carries: `correlation_id` (originating request), `causation_id` (the direct cause), `timestamp`, `aggregate_id`, `aggregate_version`
- Propagate correlation ID from HTTP request headers through command bus to event store
- Required by Constitution Principle V (observability built into runtime)

**Phase:** Event Store schema (Step 3), Middleware (Step 8).

---

### mP-4: Clippy Pedantic Fights with Derive Macros

**What goes wrong:** `clippy::pedantic` (enabled per CLAUDE.md) produces warnings on derive-macro-generated code for `thiserror`, `serde`, and `clap`. Developers add `#[allow]` annotations everywhere or disable pedantic linting.

**Prevention:**
- Configure specific clippy allows at workspace level for known derive-macro incompatibilities
- Common allows needed: `clippy::module_name_repetitions`, `clippy::missing_errors_doc`, `clippy::must_use_candidate`
- Add to workspace `Cargo.toml` or per-crate `lib.rs` with documentation of why
- Do NOT disable pedantic entirely

**Phase:** Dev Environment (Step 1).

---

### mP-5: rusqlite Feature Flag Misconfiguration

**What goes wrong:** Missing `features = ["bundled"]` causes build failures on systems without SQLite dev headers. Missing `features = ["serde_json"]` means no direct JSON column support.

**Prevention:** Always use `rusqlite = { version = "0.39", features = ["bundled", "serde_json", "uuid"] }` in workspace dependencies.

**Phase:** Dev Environment (Step 1).

---

### mP-6: OpenTelemetry Version Mismatch

**What goes wrong:** Mixing incompatible versions of opentelemetry, opentelemetry_sdk, opentelemetry-otlp, and tracing-opentelemetry. The OTel Rust crates have strict version coupling and different versions are NOT compatible.

**Prevention:** Pin all four crates together in `[workspace.dependencies]`. Compatible set: opentelemetry 0.31, opentelemetry_sdk 0.31, opentelemetry-otlp 0.31, tracing-opentelemetry 0.32.

**Detection:** Build fails with errors mentioning `opentelemetry::trace::Tracer` trait bounds not satisfied.

**Phase:** Observability (Step 7).

---

### mP-7: Forgetting Graceful Shutdown

**What goes wrong:** Runtime exits abruptly, leaving outbox entries undelivered, SQLite WAL not checkpointed, or projections in inconsistent state.

**Prevention:** Implement `tokio::signal::ctrl_c()` handling. On shutdown: stop accepting new requests, drain in-flight requests, flush outbox, checkpoint projections, close SQLite connections.

**Phase:** Runtime CLI (Step 9).

---

### mP-8: Testing with Shared SQLite Files

**What goes wrong:** Integration tests share an SQLite file, causing test pollution and flaky tests. Tests pass individually but fail when run in parallel (`cargo test` runs tests in parallel by default).

**Prevention:** Each test creates its own temporary SQLite database via `tempfile::tempdir()`. Use `:memory:` databases for unit tests that don't need persistence across function calls.

**Phase:** Event Store (Step 3).

---

### mP-9: Storing Sensitive Data in Immutable Events

**What goes wrong:** PII (names, emails, payment info) stored directly in event payloads. Events are immutable by design -- you cannot delete or modify them. GDPR "right to erasure" becomes architecturally impossible.

**Prevention:** Use indirection: store an external reference (user_id) in events, keep PII in a separate mutable store. Or use crypto-shredding: encrypt PII in events with a per-user key, delete the key to "erase" data.

**Phase:** Event Store schema design (Step 3). Decision to make early even if implementation is deferred.

---

### mP-10: axum 0.8 Path Parameter Syntax

**What goes wrong:** Using old axum 0.7 path syntax (`/:id`) instead of 0.8 syntax (`/{id}`). Compiles fine but routes don't match.

**Prevention:** Use `/{param}` and `/{*wildcard}` syntax. Check the axum 0.8 migration guide.

**Phase:** API Surface (Step 6).

---

## Technical Debt Patterns

Patterns that accumulate silently and become expensive to fix later.

---

### TD-1: SDG JSON Parsed Into Untyped `serde_json::Value` Everywhere

**Description:** Instead of defining strong Rust types for the SDG model, the code passes `serde_json::Value` through the system and pattern-matches on keys/values at runtime. Type errors surface late, refactoring is unsafe, and IDE support is absent.

**Prevention:** Define Rust structs for every SDG concept (Aggregate, Transition, DagNode, Projection, Endpoint). Deserialize into these structs in the SDG Loader. All downstream code works with typed structs.

**Detection:** Search for `serde_json::Value` in code outside the SDG Loader crate. Any usage in aggregate-engine, projections, or api-surface is a smell.

---

### TD-2: Test Event Construction Duplicated Across Test Suites

**Description:** Each crate's test suite manually constructs test events (`json!({"type": "TaskCreated", "data": {...}})`) with slightly different structures. When the event format changes, dozens of test files break.

**Prevention:** Create a shared test utilities module (or a `dev-dependency` crate) with builder functions for test events. Use the SDG task tracker example as the canonical test domain across all crates.

**Detection:** `grep -r "TaskCreated" crates/*/tests/` returns more than 2-3 locations with inline JSON construction.

---

### TD-3: Aggregate Cache Missing From Day One

**Description:** Every command requires loading the aggregate by replaying all events from the beginning. Without caching, performance degrades linearly with event count. Snapshots are Phase 3, but a simple in-memory LRU cache is cheap and should be Phase 1.

**Prevention:** Implement an in-memory aggregate cache (LRU keyed by `aggregate_id`). After processing a command, cache the updated aggregate state with its version. On next command, check cache first, only replay events since cached version. Invalidate on version mismatch.

**Detection:** Aggregate replay latency grows linearly with event count in load tests.

---

### TD-4: Observability Added as Afterthought

**Description:** Building all components first, then trying to add tracing spans and metrics. By then, the code structure doesn't support clean instrumentation. Spans are too coarse or too fine-grained. Important context (aggregate_id, command_type) is not available where metrics are collected.

**Prevention:** Add `#[instrument]` annotations to key functions from day one. Design function signatures to include the information you'll want in spans (aggregate_id, command_type as parameters, not buried inside structs). Add the observability crate early and wire it into the bootstrap sequence.

**Detection:** Functions that take opaque structs and produce metrics/spans that lack domain context.

---

## Performance Traps

Issues that only manifest under load and are hard to reproduce in development.

---

### PT-1: Aggregate Replay Without Caching

**Trap:** Every command requires loading the aggregate by replaying all events. With 1000 events per aggregate, every command incurs 1000 deserializations + applies. For hot aggregates receiving multiple commands per second, this dominates latency.

**Detection:** Add a tracing span around aggregate replay. Alert if replay > 10ms or event count > 100.

**Mitigation:** In-memory LRU cache. Snapshots are Phase 3.

---

### PT-2: Projection Queries Without Indexes

**Trap:** Projection read models stored in SQLite tables generated from the SDG. Without appropriate indexes, queries that filter by commonly-used fields do full table scans.

**Detection:** Enable SQLite `EXPLAIN QUERY PLAN` in development/test.

**Mitigation:** The SDG projection definition should allow specifying indexed fields. At minimum, always index primary key and aggregate_id.

---

### PT-3: WAL File Growth During Projection Rebuild

**Trap:** A projection rebuild reads the entire event store (long-running read transaction) while the system continues accepting writes. The WAL file cannot be checkpointed while the long read is active, growing unbounded.

**Detection:** Monitor WAL file size. Alert if > 100MB.

**Mitigation:** Rebuild in batches with gaps (release read transaction periodically, allowing checkpoints). Or rebuild from a database copy.

---

### PT-4: Unbounded Outbox Table Growth

**Trap:** Processed outbox entries are never cleaned up. The outbox table grows with every event, slowing down the polling query that finds unprocessed entries.

**Detection:** Monitor outbox table row count.

**Mitigation:** Background task that deletes old processed entries in small batches (not bulk DELETE).

---

### PT-5: JSON Serialization/Deserialization Overhead in Hot Path

**Trap:** Every command processes the cycle: deserialize all aggregate events from JSON -> apply -> serialize new event to JSON -> store. JSON parsing is not free; with large events or many events, serialization dominates CPU time.

**Detection:** Profiling shows significant time in `serde_json::from_value` / `serde_json::to_value`.

**Mitigation:** The aggregate cache (TD-3) eliminates most deserialization. For Phase 3, consider binary event formats (protobuf/MessagePack) behind the event format abstraction.

---

## "Looks Done But Isn't" Checklist

Items that pass basic testing but will fail in production or under edge cases.

| Item | Seems Done When... | Actually Done When... |
|------|--------------------|-----------------------|
| Event Store | Events append and load correctly | Concurrent writes to same aggregate produce typed conflict errors; WAL mode configured; busy_timeout set; writer connection serialized; global_sequence is explicit column not ROWID |
| Optimistic Concurrency | Version check exists in code | Database constraint enforces uniqueness; conflict error is typed and retryable; retry logic exists in command handler; tested with concurrent tasks |
| Projection Engine | Events processed, read model updated | Projection can be rebuilt from scratch; handles duplicate events idempotently; tracks high-water mark per projection; lag metric exposed; separate DB from event store |
| SDG Validation | Valid SDGs load successfully | Invalid SDGs produce clear errors; type mismatches in DAG caught; unreachable nodes detected; missing event payload fields caught; suite of intentionally broken SDGs tested |
| API Generation | Endpoints respond to requests | OpenAPI spec generated from live SDG at startup; error responses documented; eventual consistency behavior documented; auth requirements reflected in spec |
| Outbox | Events reach projections | Outbox survives process crash; duplicate delivery handled by idempotent consumers; outbox table cleaned up in background; processing lag monitored |
| Aggregate Engine | Commands produce events | State machine transitions validated; guards evaluated correctly; DAG execution handles errors gracefully; aggregate version returned to caller for read-after-write |
| Error Handling | Errors don't panic the runtime | Errors carry full context (aggregate ID, command type, event type); errors cross crate boundaries cleanly; deserialization errors don't crash replay |
| Observability | Tracing spans exist | Correlation ID propagated end-to-end from HTTP header; projection lag metric works; aggregate replay duration tracked; SQLite blocking detected via metrics |
| TDD | Tests exist for features | Tests use Given-When-Then for aggregates; projection tests verify query responses not state; intentionally broken SDGs tested; concurrent command scenarios tested; shared test fixtures exist |
| Event Versioning | Current events serialize/deserialize | Event storage format includes type+version columns; old events can be deserialized by new code; fixture files from prior SDG versions in test suite |
| Graceful Shutdown | Process exits cleanly | In-flight requests drained; outbox flushed; WAL checkpointed; projection positions saved; all connections closed |

---

## Pitfall-to-Phase Mapping

| Phase/Step | Pitfalls to Address | Priority |
|------------|---------------------|----------|
| Step 1: Dev Environment | mP-4 (clippy pedantic), mP-5 (rusqlite features) | LOW |
| Step 2: SDG Schema & Loader | CP-5 (semantic validation passes 1-2), mP-2 (chatty events), CP-3 (versioning format in event type derivation), TD-1 (typed SDG structs), MP-8 (DAG scope) | HIGH |
| Step 3: Event Store | CP-1 (SQLite single-writer), CP-2 (optimistic concurrency), CP-3 (event versioning storage format), CP-4 (blocking tokio), CP-7 (abstraction leaks), MP-2 (outbox cleanup), MP-6 (deserialization panics), MP-7 (event type separation), mP-1 (timestamps), mP-3 (correlation IDs), mP-8 (shared test DBs), mP-9 (PII in events) | CRITICAL |
| Step 4: Aggregate Engine | CP-5 (semantic validation passes 3-4), MP-5 (test patterns), MP-8 (DAG scope boundary), TD-3 (aggregate cache), TD-2 (shared test fixtures) | HIGH |
| Step 5: Projections | CP-6 (projection lag), MP-3 (rebuild), PT-2 (indexes), PT-3 (WAL growth), PT-4 (outbox growth) | HIGH |
| Step 6: API Surface | CP-6 (freshness in API responses), MP-4 (OpenAPI from live SDG), mP-10 (axum syntax) | MEDIUM |
| Step 7: Observability | mP-6 (OTel versions), mP-3 (correlation IDs end-to-end), TD-4 (instrumentation) | MEDIUM |
| Step 8: Middleware | MP-1 (error types cross-crate) | MEDIUM |
| Step 9: CLI & Runtime | mP-7 (graceful shutdown) | MEDIUM |
| All Steps | MP-1 (error type design consistency), TD-2 (shared test fixtures) | Ongoing |

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| SDG Schema Definition | Schema too rigid, blocking iteration | Start minimal, iterate. Test with both valid and invalid SDGs. |
| SDG Loader | Validation errors that don't help the user | Aggregate ALL validation errors with JSON path context. Don't stop at first error. |
| Event Store (SQLite) | SQLITE_BUSY under concurrent load | Single writer connection, busy_timeout, spawn_blocking. Test with 20+ concurrent commands. |
| Event Store (Schema) | Storage format doesn't accommodate versioning | Include event_type and event_version as separate columns from day one. |
| Aggregate Engine | DAG interpreter scope creep | Hard boundary on node types. Defer complex logic to Phase 2 TS blocks. |
| Aggregate Engine | Unbounded replay time | In-memory LRU cache from Phase 1; snapshots deferred to Phase 3. |
| Transactional Outbox | Polling too frequently or infrequently | Start at 100ms interval. Make configurable. Monitor table size. |
| Projections | "It works in dev" but lag visible under load | Projection lag metric required; read-after-write pattern for API. |
| Projections | Rebuild not possible or impractically slow | Separate projection DB from event store DB. Batch rebuilds. |
| API Surface | OpenAPI spec drifts from actual behavior | Generate spec from SDG at runtime. Contract tests. |
| API Surface | Dynamic routing can't use utoipa derive macros | Use programmatic `OpenApiBuilder` API for SDG-derived types. |
| Middleware | JWT validation too strict for development | Provide a dev mode with optional JWT bypass. Never in production. |
| Observability | Too many spans, too much noise | Instrument at crate boundaries and key decision points. |
| CLI Validation | Different validation behavior in CLI vs runtime | Reuse EXACTLY the same validation code. CLI calls the loader. |
| End-to-end Demo | Task tracker SDG designed in isolation | Design the SDG alongside the schema. The SDG is both demo and test fixture. |
| Cross-Crate Integration | Error types leak internal dependencies | Domain-level error enums. Box<dyn Error> for wrapped internals. |
| TDD | Tests pass but don't test the right things | Given-When-Then for aggregates, query-based for projections. |

---

## Sources

- [Don't Let the Internet Dupe You, Event Sourcing is Hard](https://chriskiehl.com/article/event-sourcing-is-hard) -- Real-world pitfalls from production ES system
- [What they don't tell you about event sourcing](https://medium.com/@hugo.oliveira.rocha/what-they-dont-tell-you-about-event-sourcing-6afc23c69e9a)
- [EventSourcingDB: Common Issues](https://docs.eventsourcingdb.io/best-practices/common-issues/) -- Comprehensive pitfall catalog
- [EventSourcingDB: Testing Event-Sourced Systems](https://docs.eventsourcingdb.io/best-practices/testing-event-sourced-systems/)
- [EventSourcing Testing Patterns (Verraes)](https://verraes.net/2023/05/eventsourcing-testing-patterns/) -- Given-When-Then, anti-patterns
- [Eventual Consistency is a UX Nightmare](https://codeopinion.com/eventual-consistency-is-a-ux-nightmare/)
- [Things I wish I knew: Event Sourcing consistency](https://softwaremill.com/things-i-wish-i-knew-when-i-started-with-event-sourcing-part-2-consistency/)
- [Optimistic concurrency for pessimistic times](https://event-driven.io/en/optimistic_concurrency_for_pessimistic_times/)
- [Idempotent Command Handling](https://event-driven.io/en/idempotent_command_handling/)
- [Outbox, Inbox patterns and delivery guarantees](https://event-driven.io/en/outbox_inbox_patterns_and_delivery_guarantees_explained/)
- [SQLite WAL Mode](https://www.sqlite.org/wal.html) -- Official single-writer documentation
- [SQLite concurrent writes and "database is locked" errors](https://tenthousandmeters.com/blog/sqlite-concurrent-writes-and-database-is-locked-errors/)
- [Event Sourcing with Aggregates in Rust](https://medium.com/capital-one-tech/event-sourcing-with-aggregates-in-rust-4022af41cf67)
- [CQRS and Event Sourcing using Rust](https://doc.rust-cqrs.org/)
- [Error Handling for Large Rust Projects (GreptimeDB)](https://greptime.com/blogs/2024-05-07-error-rust)
- [tokio-rusqlite](https://docs.rs/tokio-rusqlite) -- Async wrapper pattern for rusqlite
- [Async: What is blocking? (Alice Ryhl)](https://ryhl.io/blog/async-what-is-blocking/) -- Tokio blocking pitfalls
- [Event Sourcing Pattern - Azure Architecture Center](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing)
- [Deduplication in Distributed Systems](https://www.architecture-weekly.com/p/deduplication-in-distributed-systems)
- [axum 0.8 migration](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0)
- [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust) -- Version compatibility
