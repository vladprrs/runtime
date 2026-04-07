# Architecture Patterns: Model-Driven Event-Sourced Rust Runtime

**Domain:** Event-sourced microservice runtime with model-driven (SDG) interpretation
**Researched:** 2026-04-07
**Confidence:** HIGH (patterns well-established in Rust ecosystem; model-driven layer is novel but composed of proven primitives)

---

## System Overview

```
                        +-------------------+
                        |   SDG JSON File   |
                        |  (model artifact)  |
                        +---------+---------+
                                  |
                                  v
+------------------------------------------------------------------+
|                         runtime (binary)                          |
|  CLI entry point, wires all crates, startup orchestration        |
+------------------------------------------------------------------+
        |              |             |              |
        v              v             v              v
+------------+  +-------------+  +----------+  +-------------+
| sdg-loader |  | observabil- |  | middle-  |  | api-surface |
| Schema val,|  | ity         |  | ware     |  | axum HTTP,  |
| DAG materi-|  | OTel,tracing|  | JWT,val, |  | OpenAPI gen |
| alization  |  | struct logs |  | errors   |  |             |
+------+-----+  +-------------+  +----+-----+  +------+------+
       |                               |               |
       v                               v               v
+------+-----------------------------------------------+------+
|                    aggregate-engine                          |
|  State machine, transition guards, DAG execution,           |
|  command handling, event emission                           |
+------+----------------------------------------------+-------+
       |                                              |
       v                                              v
+------+------+                              +--------+------+
| event-store |                              | projections   |
| SQLite,     |<----- outbox (in store) ---->| Async read    |
| append-only,|                              | model builder,|
| concurrency |                              | catch-up,     |
| control     |                              | rebuild       |
+--------------+                              +---------------+
```

### Crate Dependency Graph (Strict Direction: Down Only)

```
Layer 3 (binary):    runtime
                     |  |  |  |  |  |  |
Layer 2 (HTTP):      api-surface
                     |       |      |
Layer 1 (domain):    aggregate-engine    projections    middleware
                     |       |           |              |
Layer 0 (foundation): sdg-loader    event-store    observability
                     (leaf)         (leaf)         (leaf)
```

**Rule:** Lower layers NEVER depend on higher layers. Cross-dependencies within a layer are acceptable (aggregate-engine does NOT depend on projections and vice versa).

---

## Request Lifecycle

### Write Path (Command Execution)

```
HTTP POST /tasks/create
    |
    v
[api-surface] -- Parse request, match SDG endpoint definition
    |              Route to command handler based on endpoint.aggregate + endpoint.command
    v
[middleware] -- Tower Layer pipeline (outermost to innermost):
    |           1. TraceLayer: inject span, start timer
    |           2. CorrelationIdLayer: generate/extract X-Correlation-Id
    |           3. JwtAuthLayer: validate JWT, extract claims
    |           4. ValidationLayer: validate body against SDG command schema
    v
[aggregate-engine]
    |  1. Lookup AggregateDefinition from SDG by aggregate_type
    |  2. Load all events for aggregate_id from event store
    |  3. Replay events to reconstruct AggregateState (fold)
    |  4. Find matching transition: command_type + from_states contains current_state
    |  5. Evaluate guard DAG (must return true)
    |  6. Evaluate computation DAG (produces event payload)
    v
[event-store] -- Single SQLite transaction:
    |           1. INSERT INTO events (optimistic concurrency via UNIQUE constraint)
    |           2. INSERT INTO outbox (same transaction)
    |           3. COMMIT
    v
HTTP Response 200 (command accepted, event version returned)
    |
    v (async, decoupled)
[projection-engine] -- Background tokio task:
    |  1. Poll outbox for undelivered entries
    |  2. For each entry, find subscribed ProjectionHandlers
    |  3. Apply event to each projection (SQL upsert into read model table)
    |  4. Advance checkpoint per projection
    |  5. Mark outbox entry as delivered
```

### Read Path (Query)

```
HTTP GET /tasks/{id}
    |
    v
[api-surface] -- Match SDG query endpoint, extract path params
    |
    v
[middleware] -- TraceLayer, CorrelationIdLayer, JwtAuthLayer (no validation layer for GET)
    |
    v
[projections] -- SELECT from denormalized read model table
    |
    v
HTTP Response 200 (projection data as JSON)
```

### Startup Sequence

```
1. CLI parse (clap) -- determine command: Run or Validate
2. Init observability (tracing subscriber + OTel exporter)
3. Load SDG file (serde_json::from_reader)
4. Validate SDG against JSON Schema (jsonschema crate)
5. Materialize DAGs, run static analysis (cycle detection, type checks)
6. Open SQLite database, enable WAL mode
7. Run migrations (event/outbox/checkpoint tables)
8. Construct event store (SqliteEventStore)
9. Construct aggregate engine (AggregateEngine with SDG + event store)
10. Construct projection handlers from SDG projection definitions
11. Start projection engine background task (tokio::spawn)
12. Build axum Router from SDG endpoint definitions
13. Apply Tower middleware layers
14. Bind TCP listener, start axum::serve with graceful shutdown
```

---

## Component Responsibilities and Boundaries

### 1. `sdg-loader` -- Model Loading and Validation

**Responsibility:** Parse SDG JSON, validate against JSON Schema, materialize computation DAGs in-memory, perform static analysis, and expose a typed `ServiceDefinition` consumed by all other crates.

**Depends on:** External crates only (serde, jsonschema, petgraph, thiserror). Leaf crate.

**Exposes:** `ServiceDefinition` struct tree -- the parsed, validated, immutable representation of the SDG.

**Key types:**

```rust
/// The fully validated, materialized service definition.
/// Immutable after construction. Shared via Arc across the entire runtime.
pub struct ServiceDefinition {
    pub service: ServiceMeta,
    pub aggregates: HashMap<AggregateName, AggregateDefinition>,
    pub projections: HashMap<ProjectionName, ProjectionDefinition>,
    pub endpoints: Vec<EndpointDefinition>,
}

pub struct AggregateDefinition {
    pub name: AggregateName,
    pub fields: Vec<FieldDefinition>,
    pub states: Vec<StateName>,
    pub initial_state: StateName,
    pub transitions: Vec<TransitionDefinition>,
}

pub struct TransitionDefinition {
    pub name: TransitionName,
    pub from_states: Vec<StateName>,
    pub to_state: StateName,
    pub command: CommandDefinition,
    pub guards: ComputationDag,     // DAG for guard conditions (returns bool)
    pub computation: ComputationDag, // DAG for producing event payload
    pub event: EventDefinition,
}

/// A node in the computation DAG -- recursive enum representing an expression tree.
pub enum DagNode {
    /// Literal constant value
    Literal(serde_json::Value),
    /// Read a field from aggregate state or command payload
    FieldAccess { source: DataSource, path: FieldPath },
    /// Built-in operation: comparison, arithmetic, boolean, string, date
    BuiltinFn { op: BuiltinOp, inputs: Vec<DagNodeRef> },
    /// Combinator: AND/OR/NOT for boolean composition
    Combinator { op: CombinatorOp, children: Vec<DagNodeRef> },
}

/// Reference to a node -- enables DAG sharing (multiple parents for one child).
/// During evaluation, results of DagNodeRef lookups are memoized.
pub type DagNodeRef = usize; // Index into a flat node array

/// The materialized computation DAG. Nodes stored in a flat Vec,
/// edges implicit via DagNodeRef indices. Validated acyclic at load time.
pub struct ComputationDag {
    pub nodes: Vec<DagNode>,
    pub output_node: DagNodeRef,
    /// Topological order computed at load time for efficient evaluation.
    pub eval_order: Vec<DagNodeRef>,
}
```

**Validation responsibilities (Constitution Principle VI):**

| Check | Mechanism | Failure Mode |
|-------|-----------|--------------|
| JSON structure | `jsonschema` crate against SDG schema | Startup fails with schema violation list |
| DAG acyclicity | `petgraph::algo::toposort` on each DAG | "Cycle detected involving node X" |
| Type compatibility | Walk DAG, check input/output types match | "Node X expects number, gets string from node Y" |
| Referential integrity | All referenced states/fields/transitions exist | "Transition references unknown state 'Archived'" |
| Unreachable nodes | Compare reachable set (from output) vs all nodes | Warning: "Node X is unreachable from output" |

**Why petgraph for validation but NOT for runtime evaluation:** `petgraph::DiGraph` is the right tool for structural validation (cycle detection via `toposort`, reachability analysis). But for runtime evaluation, the DAG is better represented as a flat `Vec<DagNode>` with pre-computed topological order. This avoids the overhead of graph traversal per request and enables simple index-based memoization. The `eval_order` field in `ComputationDag` is computed once at load time and reused for every evaluation.

**Confidence:** HIGH -- JSON Schema validation via `jsonschema` is production-grade. DAG cycle detection is a textbook algorithm. The `serde_json::Value` type handles dynamic JSON naturally.

---

### 2. `event-store` -- Event Persistence

**Responsibility:** Append-only persistence of domain events with per-aggregate streams, optimistic concurrency control, and a transactional outbox.

**Depends on:** External crates only (rusqlite, serde, uuid, chrono, thiserror). Leaf crate.

**Exposes:** `EventStore` trait, `Outbox` trait, `StoredEvent` types, `SqliteEventStore` implementation.

**Core trait design:**

```rust
/// Core event store abstraction.
/// Backing store can be swapped without changing consumers.
/// (Constitution Principle III: SQLite MVP, production DB chosen after load testing.)
///
/// Uses async_trait even though rusqlite is sync -- SQLite impl uses
/// spawn_blocking internally. This keeps the API consistent for future
/// async backends (PostgreSQL, EventStoreDB).
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Append events to an aggregate's stream.
    /// Returns the new version after appending.
    /// Fails with ConcurrencyConflict if expected_version != current version.
    async fn append(
        &self,
        aggregate_type: &str,
        aggregate_id: &AggregateId,
        events: &[NewEvent],
        expected_version: Version,
    ) -> Result<Version, EventStoreError>;

    /// Load all events for an aggregate, ordered by version ascending.
    async fn load(
        &self,
        aggregate_type: &str,
        aggregate_id: &AggregateId,
    ) -> Result<Vec<StoredEvent>, EventStoreError>;

    /// Load events starting from a specific version (for catch-up after snapshot).
    async fn load_from(
        &self,
        aggregate_type: &str,
        aggregate_id: &AggregateId,
        from_version: Version,
    ) -> Result<Vec<StoredEvent>, EventStoreError>;
}

/// Transactional outbox -- lives alongside the event store.
/// Events and outbox entries are written in the same database transaction.
#[async_trait]
pub trait Outbox: Send + Sync {
    /// Poll for undelivered outbox entries, ordered by entry_id.
    async fn poll(&self, batch_size: usize) -> Result<Vec<OutboxEntry>, EventStoreError>;

    /// Mark entries as delivered after successful projection processing.
    async fn acknowledge(&self, entry_ids: &[i64]) -> Result<(), EventStoreError>;
}

/// A persisted event with full metadata envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    pub event_id: EventId,
    pub aggregate_type: String,
    pub aggregate_id: AggregateId,
    pub version: Version,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub metadata: EventMetadata,
    pub timestamp: DateTime<Utc>,
}

/// Metadata attached to every event for tracing and audit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub correlation_id: CorrelationId,
    pub causation_id: Option<EventId>,
    pub user_id: Option<String>,
}

/// Type aliases for clarity.
pub type Version = u64;
pub type AggregateId = String;
pub type EventId = String;       // UUID v4 as string
pub type CorrelationId = String;  // UUID v4 as string
```

**SQLite schema:**

```sql
CREATE TABLE events (
    event_id       TEXT PRIMARY KEY,          -- UUID v4
    aggregate_type TEXT NOT NULL,
    aggregate_id   TEXT NOT NULL,
    version        INTEGER NOT NULL,
    event_type     TEXT NOT NULL,
    payload        TEXT NOT NULL,             -- JSON string
    metadata       TEXT NOT NULL,             -- JSON string
    timestamp      TEXT NOT NULL,             -- ISO 8601
    UNIQUE(aggregate_id, version)            -- Optimistic concurrency enforcement
);

CREATE INDEX idx_events_aggregate ON events(aggregate_type, aggregate_id, version);

CREATE TABLE outbox (
    entry_id   INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id   TEXT NOT NULL REFERENCES events(event_id),
    created_at TEXT NOT NULL,
    delivered  INTEGER NOT NULL DEFAULT 0     -- 0 = pending, 1 = delivered
);

CREATE INDEX idx_outbox_pending ON outbox(delivered, entry_id)
    WHERE delivered = 0;                     -- Partial index for efficient polling
```

**Optimistic concurrency mechanism:** The `UNIQUE(aggregate_id, version)` constraint is the sole concurrency control mechanism. When appending:

1. Caller provides `expected_version` (the version of the last event they loaded).
2. New events get versions `expected_version + 1`, `expected_version + 2`, etc.
3. If another writer already claimed those version numbers, the `INSERT` fails with a SQLite `UNIQUE constraint` violation.
4. The Rust code catches `rusqlite::Error` and translates it to `EventStoreError::ConcurrencyConflict`.

This is simpler and more reliable than SELECT-check-INSERT because the database enforces the invariant atomically.

**Transactional outbox guarantee:** Events and outbox entries are written in a single SQLite transaction (`BEGIN ... COMMIT`). If the transaction succeeds, both the events and the outbox entries exist. The projection engine polls the outbox and processes events asynchronously. After successful processing, it acknowledges the entries (sets `delivered = 1`).

**SQLite connection strategy:**

- **WAL mode** enabled at connection time (`PRAGMA journal_mode=WAL`) -- allows concurrent reads during writes.
- **Two connections:** one write connection (serialized access via a Mutex or actor), one read connection (for query handlers and projection reads).
- **spawn_blocking** for all rusqlite calls -- prevents blocking the tokio event loop.

**Confidence:** HIGH -- `rusqlite` is mature (10+ years). The UNIQUE constraint for optimistic concurrency is a well-known pattern. Transactional outbox in a single database is the simplest correct approach.

---

### 3. `aggregate-engine` -- State Machine and DAG Execution

**Responsibility:** The core domain execution engine. Given an SDG aggregate definition and an event stream, it: (1) replays events to reconstruct aggregate state, (2) validates commands against the current state machine, (3) evaluates guard and computation DAGs, and (4) emits new events.

**Depends on:** `sdg-loader` (for `ServiceDefinition`, `AggregateDefinition`, `ComputationDag`), `event-store` (for `EventStore` trait, `StoredEvent`).

**Exposes:** `AggregateEngine` and `AggregateState` types.

**Critical design decision -- model-driven, NOT generic aggregates:**

This is the key architectural divergence from Rust CQRS frameworks like `cqrs-es`. In `cqrs-es`, you define concrete Rust types for each aggregate:

```rust
// cqrs-es pattern -- NOT what this project does:
struct BankAccount { balance: f64 }
impl Aggregate for BankAccount {
    type Command = BankAccountCommand;
    type Event = BankAccountEvent;
    type Error = BankAccountError;
    fn handle(&self, cmd: Self::Command) -> Result<Vec<Self::Event>, Self::Error> { ... }
    fn apply(&mut self, event: Self::Event) { ... }
}
```

This runtime CANNOT do that because aggregate types are defined in the SDG at load time, not at compile time. Instead, aggregates are fully dynamic:

```rust
/// The runtime state of a single aggregate instance.
/// Fields are dynamic (serde_json::Value) because they are defined by the SDG.
pub struct AggregateState {
    pub aggregate_id: AggregateId,
    pub aggregate_type: String,
    pub version: Version,
    pub current_state: StateName,          // State machine position
    pub fields: serde_json::Value,         // Dynamic field values (JSON object)
}

impl AggregateState {
    /// Create initial state for a new aggregate (before any events).
    pub fn initial(aggregate_def: &AggregateDefinition) -> Self {
        Self {
            aggregate_id: String::new(),
            aggregate_type: aggregate_def.name.clone(),
            version: 0,
            current_state: aggregate_def.initial_state.clone(),
            fields: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    /// Apply a single event to state. This is a pure fold operation.
    /// Events are facts -- apply MUST NOT fail (Constitution: "an event is a
    /// historical fact, it can be ignored but it should never cause an error").
    pub fn apply(&mut self, event: &StoredEvent) {
        // Merge event payload fields into aggregate fields
        if let (Some(state), Some(payload)) = (
            self.fields.as_object_mut(),
            event.payload.as_object(),
        ) {
            for (key, value) in payload {
                state.insert(key.clone(), value.clone());
            }
        }
        // Update state machine position if event carries a state transition
        if let Some(new_state) = event.payload.get("__new_state") {
            if let Some(s) = new_state.as_str() {
                self.current_state = s.to_string();
            }
        }
        self.version = event.version;
    }

    /// Reconstruct state by replaying a stream of events (left fold).
    pub fn from_events(
        aggregate_def: &AggregateDefinition,
        aggregate_id: &AggregateId,
        events: &[StoredEvent],
    ) -> Self {
        let mut state = Self::initial(aggregate_def);
        state.aggregate_id = aggregate_id.clone();
        for event in events {
            state.apply(event);
        }
        state
    }
}
```

**Command execution flow:**

```rust
/// The engine that processes commands against SDG-defined aggregates.
pub struct AggregateEngine {
    definition: Arc<ServiceDefinition>,
    event_store: Arc<dyn EventStore>,
}

impl AggregateEngine {
    /// Execute a command: load state, validate transition, run DAGs, emit events.
    pub async fn execute(
        &self,
        aggregate_type: &str,
        aggregate_id: &AggregateId,
        command_type: &str,
        payload: serde_json::Value,
        metadata: EventMetadata,
    ) -> Result<Vec<StoredEvent>, EngineError> {
        // 1. Look up aggregate definition from SDG
        let agg_def = self.definition.aggregates.get(aggregate_type)
            .ok_or_else(|| EngineError::UnknownAggregate(aggregate_type.into()))?;

        // 2. Load events and replay to reconstruct current state
        let events = self.event_store.load(aggregate_type, aggregate_id).await?;
        let state = AggregateState::from_events(agg_def, aggregate_id, &events);

        // 3. Find matching transition
        let transition = agg_def.transitions.iter()
            .find(|t| t.command.name == command_type
                    && t.from_states.contains(&state.current_state))
            .ok_or_else(|| EngineError::InvalidTransition {
                from: state.current_state.clone(),
                command: command_type.into(),
            })?;

        // 4. Evaluate guard DAG (must return boolean true)
        let guard_result = evaluate_dag(
            &transition.guards,
            &state.fields,
            &payload,
        )?;
        if guard_result != serde_json::Value::Bool(true) {
            return Err(EngineError::GuardFailed);
        }

        // 5. Evaluate computation DAG (produces event payload)
        let mut event_payload = evaluate_dag(
            &transition.computation,
            &state.fields,
            &payload,
        )?;

        // 6. Inject state transition into event payload
        if let Some(obj) = event_payload.as_object_mut() {
            obj.insert("__new_state".into(),
                       serde_json::Value::String(transition.to_state.clone()));
        }

        // 7. Build new event and append to store
        let new_event = NewEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: transition.event.name.clone(),
            payload: event_payload,
            metadata,
        };

        let new_version = self.event_store.append(
            aggregate_type,
            aggregate_id,
            &[new_event],
            state.version,
        ).await?;

        // Return the stored events (useful for response and testing)
        Ok(/* newly stored events */)
    }
}
```

**DAG evaluation (the computation core):**

```rust
/// Evaluate a pre-validated, pre-ordered computation DAG.
/// Walks nodes in topological order, memoizing results.
pub fn evaluate_dag(
    dag: &ComputationDag,
    aggregate_fields: &serde_json::Value,
    command_payload: &serde_json::Value,
) -> Result<serde_json::Value, DagError> {
    let mut results: Vec<Option<serde_json::Value>> = vec![None; dag.nodes.len()];

    for &node_idx in &dag.eval_order {
        let node = &dag.nodes[node_idx];
        let result = match node {
            DagNode::Literal(v) => v.clone(),

            DagNode::FieldAccess { source, path } => {
                let root = match source {
                    DataSource::Aggregate => aggregate_fields,
                    DataSource::Command => command_payload,
                };
                resolve_field_path(root, path)
                    .ok_or_else(|| DagError::FieldNotFound {
                        source: *source,
                        path: path.clone(),
                    })?
            }

            DagNode::BuiltinFn { op, inputs } => {
                let args: Vec<&serde_json::Value> = inputs.iter()
                    .map(|&ref_idx| results[ref_idx].as_ref()
                        .expect("topological order guarantees predecessor is computed"))
                    .collect();
                execute_builtin_op(*op, &args)?
            }

            DagNode::Combinator { op, children } => {
                let child_values: Vec<&serde_json::Value> = children.iter()
                    .map(|&ref_idx| results[ref_idx].as_ref()
                        .expect("topological order guarantees predecessor is computed"))
                    .collect();
                execute_combinator_op(*op, &child_values)?
            }
        };
        results[node_idx] = Some(result);
    }

    results[dag.output_node].clone()
        .ok_or(DagError::OutputNodeNotEvaluated)
}
```

**Why flat Vec + topological order instead of recursive evaluation:** The DAG may have shared nodes (diamond pattern). Recursive evaluation would re-evaluate shared sub-trees unless memoized. Using a flat `Vec` with pre-computed topological order evaluates each node exactly once, in the correct order, with O(1) lookup for results. This is both simpler and faster than recursive + HashMap memoization.

**Why NOT petgraph at runtime:** `petgraph::DiGraph` has overhead per node access (generational indices, edge lists). For DAGs of 5-50 nodes evaluated per request, a flat `Vec<DagNode>` with index-based references is faster and simpler. Petgraph is used at LOAD TIME for structural validation (cycle detection), and the validated structure is flattened into the `ComputationDag` representation.

**Confidence:** HIGH for aggregate state replay and command handling (textbook ES). MEDIUM for DAG evaluator (the flat-vec approach is sound but the builtin operation set needs careful design during implementation).

---

### 4. `projections` -- Read Model Builder

**Responsibility:** Subscribe to the event outbox, apply events to denormalized read models (separate SQLite tables), support catch-up on startup and full rebuild.

**Depends on:** `event-store` (for `Outbox` trait, `StoredEvent`), `sdg-loader` (for `ProjectionDefinition`).

**Key trait:**

```rust
/// A projection handler knows how to apply events to a read model.
/// In this runtime, handlers are interpreted from SDG definitions, not hand-coded.
pub trait ProjectionHandler: Send + Sync {
    /// Unique name for checkpointing.
    fn name(&self) -> &str;

    /// Which event types this projection subscribes to.
    fn event_types(&self) -> &[String];

    /// Apply a single event to the read model.
    fn apply(&self, event: &StoredEvent, conn: &Connection) -> Result<(), ProjectionError>;

    /// Create the read model tables (called on first run or rebuild).
    fn initialize(&self, conn: &Connection) -> Result<(), ProjectionError>;

    /// Drop the read model tables (called before rebuild).
    fn drop_tables(&self, conn: &Connection) -> Result<(), ProjectionError>;
}
```

**Model-driven projections:** `ProjectionHandler` implementations are NOT hand-coded Rust. They are generated from SDG `ProjectionDefinition` at load time:

```rust
/// Interprets an SDG projection definition at runtime.
/// Generates SQL statements dynamically from the projection configuration.
pub struct DynamicProjectionHandler {
    definition: ProjectionDefinition,
    table_name: String,
    subscribed_events: Vec<String>,
}

impl ProjectionHandler for DynamicProjectionHandler {
    fn apply(&self, event: &StoredEvent, conn: &Connection) -> Result<(), ProjectionError> {
        if !self.subscribed_events.contains(&event.event_type) {
            return Ok(());
        }
        // Generate SQL upsert from projection definition + event payload
        // E.g., "INSERT OR REPLACE INTO task_list (id, title, status, updated_at) VALUES (?, ?, ?, ?)"
        let sql = self.definition.build_upsert_sql(&event.payload)?;
        conn.execute(&sql.query, rusqlite::params_from_iter(sql.params))?;
        Ok(())
    }

    fn initialize(&self, conn: &Connection) -> Result<(), ProjectionError> {
        // Generate CREATE TABLE from projection field definitions
        let ddl = self.definition.build_create_table_sql(&self.table_name);
        conn.execute_batch(&ddl)?;
        Ok(())
    }
    // ...
}
```

**Projection engine lifecycle:**

```rust
pub struct ProjectionEngine {
    handlers: Vec<Box<dyn ProjectionHandler>>,
    outbox: Arc<dyn Outbox>,
    event_store: Arc<dyn EventStore>,
    read_conn: Connection,  // Separate SQLite connection for read model writes
}

impl ProjectionEngine {
    /// Main loop: runs as a background tokio task.
    pub async fn run(&self, shutdown: tokio::sync::watch::Receiver<bool>) {
        // Phase 1: Catch-up -- for each handler, replay from last checkpoint
        for handler in &self.handlers {
            let checkpoint = self.load_checkpoint(handler.name());
            if checkpoint == 0 {
                handler.initialize(&self.read_conn).unwrap();
            }
            // Load events from checkpoint, apply each
            // Update checkpoint after each batch
        }

        // Phase 2: Live polling
        loop {
            if *shutdown.borrow() { break; }

            let entries = self.outbox.poll(100).await.unwrap();
            if entries.is_empty() {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            for entry in &entries {
                for handler in &self.handlers {
                    if handler.event_types().contains(&entry.event.event_type) {
                        handler.apply(&entry.event, &self.read_conn).unwrap();
                    }
                }
            }

            let ids: Vec<i64> = entries.iter().map(|e| e.entry_id).collect();
            self.outbox.acknowledge(&ids).await.unwrap();
            // Update checkpoints
        }
    }

    /// Rebuild a specific projection from scratch.
    pub async fn rebuild(&self, name: &str) {
        let handler = self.handlers.iter().find(|h| h.name() == name).unwrap();
        handler.drop_tables(&self.read_conn).unwrap();
        handler.initialize(&self.read_conn).unwrap();
        self.reset_checkpoint(name);
        // Load ALL events from event store (not outbox), apply each
        // This requires a method on EventStore to load all events globally
    }
}
```

**Checkpoint storage:**

```sql
CREATE TABLE projection_checkpoints (
    projection_name TEXT PRIMARY KEY,
    last_entry_id   INTEGER NOT NULL,
    updated_at      TEXT NOT NULL
);
```

**Idempotency:** If the engine crashes after applying an event but before updating the checkpoint, the event will be re-applied on restart. Projection handlers MUST be idempotent. Using `INSERT OR REPLACE` (upsert) for read model writes guarantees this.

**Confidence:** HIGH -- checkpoint-based catch-up subscriptions are the standard pattern for event-sourced projections. The approach is well-documented across multiple sources.

---

### 5. `api-surface` -- HTTP Endpoints and OpenAPI

**Responsibility:** Auto-generate axum routes from SDG endpoint definitions. Generate OpenAPI specification. Route commands to the aggregate engine and queries to projection data.

**Depends on:** `sdg-loader`, `aggregate-engine`, `projections`, `middleware`.

**Key design:**

```rust
/// Application state shared across all HTTP handlers via axum::State.
#[derive(Clone)]
pub struct AppState {
    pub definition: Arc<ServiceDefinition>,
    pub engine: Arc<AggregateEngine>,
    pub read_conn: Arc<Mutex<Connection>>,  // For projection queries
}

/// Build the full axum Router from SDG definitions.
pub fn build_router(state: AppState, middleware: MiddlewareStack) -> Router {
    let mut router = Router::new();

    for endpoint in &state.definition.endpoints {
        let handler = match endpoint.kind {
            EndpointKind::Command => build_command_handler(endpoint),
            EndpointKind::Query => build_query_handler(endpoint),
        };
        router = router.route(
            &endpoint.path,
            match endpoint.method {
                HttpMethod::Post => post(handler),
                HttpMethod::Get => get(handler),
                HttpMethod::Put => put(handler),
                HttpMethod::Delete => delete(handler),
            },
        );
    }

    // Health check and OpenAPI spec (always present)
    router = router
        .route("/health", get(health_handler))
        .route("/openapi.json", get(openapi_handler));

    // Apply shared state and middleware
    router
        .with_state(state)
        .layer(middleware.into_layer())
}
```

**Dynamic handler construction:** Each SDG endpoint produces a handler closure that captures the endpoint configuration. The handler extracts the aggregate type, command type, and path parameters from the endpoint definition, not from Rust types.

**OpenAPI generation:** Build the OpenAPI spec programmatically from SDG definitions at startup. Each endpoint's command schema becomes a request body schema, each projection's field list becomes a response schema. Serve at `/openapi.json`.

**Confidence:** HIGH -- axum's routing, state management, and handler patterns are mature.

---

### 6. `middleware` -- Tower Layer Pipeline

**Responsibility:** Compose Tower layers for JWT authentication, request body validation, structured error responses, and correlation ID propagation.

**Depends on:** `sdg-loader` (for command schemas), `observability` (for correlation ID context).

**Key design -- Tower Layer composition:**

```rust
/// Compose the full middleware stack.
/// Applied to the axum Router via .layer().
pub fn build_middleware_stack(
    jwt_secret: String,
    definition: Arc<ServiceDefinition>,
) -> ServiceBuilder<...> {
    ServiceBuilder::new()
        // Outermost layer: request/response tracing
        .layer(TraceLayer::new_for_http()
            .make_span_with(|req: &Request| {
                tracing::info_span!("http_request",
                    method = %req.method(),
                    uri = %req.uri(),
                    correlation_id = tracing::field::Empty,
                )
            }))
        // Correlation ID: extract from header or generate
        .layer(CorrelationIdLayer::new())
        // JWT authentication
        .layer(JwtAuthLayer::new(jwt_secret))
        // Request timeout
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
}
```

Each middleware is a Tower `Layer` wrapping the inner `Service`. The "onion skin" model:

```
Request  -->  Trace --> CorrelationId --> JWT --> Timeout --> Handler
Response <--  Trace <-- CorrelationId <-- JWT <-- Timeout <-- Handler
```

**Request validation:** Rather than a global Tower layer, validation is done per-handler because each endpoint has its own command schema. The command handler extracts the schema from the SDG and validates the JSON body before passing it to the aggregate engine.

**Structured error responses:**

```rust
/// Unified error response format for all API errors.
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error_code: String,        // Machine-readable: "CONCURRENCY_CONFLICT"
    pub message: String,           // Human-readable description
    pub details: Option<Value>,    // Optional structured details
    pub correlation_id: String,    // From request context
}
```

**Error mapping from domain to HTTP:**

| Domain Error | HTTP Status | Error Code |
|-------------|-------------|------------|
| `UnknownAggregate` | 404 | `UNKNOWN_AGGREGATE` |
| `InvalidTransition` | 409 | `INVALID_TRANSITION` |
| `GuardFailed` | 422 | `GUARD_FAILED` |
| `ConcurrencyConflict` | 409 | `CONCURRENCY_CONFLICT` |
| `ValidationFailed` | 400 | `VALIDATION_FAILED` |
| `AuthenticationFailed` | 401 | `AUTHENTICATION_FAILED` |
| Any internal error | 500 | `INTERNAL_ERROR` |

**Confidence:** HIGH -- Tower Service/Layer composition is the standard axum middleware pattern.

---

### 7. `observability` -- Metrics, Tracing, Logging

**Responsibility:** Initialize the OpenTelemetry stack, configure tracing subscribers, register runtime metrics. Provides utilities consumed by all other crates.

**Depends on:** External crates only (tracing, opentelemetry, tracing-opentelemetry). Leaf crate.

**Key design:**

```rust
/// Initialize the full observability stack. Called once at startup.
pub fn init(config: &ObservabilityConfig) -> Result<OtelGuard, ObservabilityError> {
    // 1. Build OTLP exporter for traces
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(/* ... */)
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    // 2. Build tracing subscriber with multiple layers
    tracing_subscriber::registry()
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(tracing_subscriber::fmt::layer()
            .json()                          // Structured JSON logs
            .with_target(true)
            .with_span_events(FmtSpan::CLOSE))
        .with(EnvFilter::from_default_env()) // RUST_LOG control
        .init();

    // 3. Return guard that flushes on drop (graceful shutdown)
    Ok(OtelGuard { /* ... */ })
}

/// Pre-defined runtime metrics.
/// Registered once, used across crates via clone.
pub struct RuntimeMetrics {
    pub command_duration: Histogram<f64>,
    pub command_total: Counter<u64>,
    pub event_store_append_duration: Histogram<f64>,
    pub projection_lag_ms: Histogram<f64>,
    pub api_request_duration: Histogram<f64>,
}
```

**Cross-crate usage:** Other crates add `tracing` as a dependency and use `tracing::info!`, `tracing::error!`, etc. The subscriber is initialized once by `runtime` at startup. No crate needs to depend on `observability` to emit logs -- they only need `tracing`. The `observability` crate is consumed only by `runtime` for initialization.

**Confidence:** HIGH -- `tracing` + `opentelemetry` is the standard Rust observability stack.

---

### 8. `runtime` -- Binary Entry Point

**Responsibility:** CLI parsing, configuration, wiring all crates together, startup orchestration, graceful shutdown.

**Depends on:** ALL other crates.

**Key design:**

```rust
#[derive(Parser)]
enum Cli {
    /// Run the runtime with an SDG file
    Run {
        #[arg(long)]
        sdg: PathBuf,
        #[arg(long, default_value = "3000")]
        port: u16,
        #[arg(long, default_value = "runtime.db")]
        db: PathBuf,
    },
    /// Validate an SDG file without starting the runtime
    Validate {
        #[arg(long)]
        sdg: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli {
        Cli::Run { sdg, port, db } => {
            let _guard = observability::init(&config)?;
            let definition = Arc::new(sdg_loader::load_and_validate(&sdg)?);
            let store = Arc::new(SqliteEventStore::new(&db).await?);
            let engine = Arc::new(AggregateEngine::new(definition.clone(), store.clone()));
            let proj_engine = ProjectionEngine::new(definition.clone(), store.clone());
            let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
            tokio::spawn(proj_engine.run(shutdown_rx));
            let state = AppState { definition, engine, /* ... */ };
            let middleware = build_middleware_stack(jwt_secret, state.definition.clone());
            let router = build_router(state, middleware);
            let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
            axum::serve(listener, router)
                .with_graceful_shutdown(shutdown_signal(shutdown_tx))
                .await?;
            Ok(())
        }
        Cli::Validate { sdg } => {
            match sdg_loader::load_and_validate(&sdg) {
                Ok(def) => {
                    println!("Valid. {} aggregate(s), {} projection(s), {} endpoint(s).",
                        def.aggregates.len(), def.projections.len(), def.endpoints.len());
                    Ok(())
                }
                Err(e) => {
                    eprintln!("Validation failed:\n{e}");
                    std::process::exit(1);
                }
            }
        }
    }
}
```

**Note on anyhow:** The `runtime` binary crate is the ONLY place where `anyhow` is used. All library crates use `thiserror` with specific error enums. The binary crate's `main` function uses `anyhow::Result` as the top-level error type, which collects and reports errors from all library crates.

---

## Error Handling Across Crate Boundaries

**Pattern: thiserror in library crates, anyhow in binary crate.**

Each library crate defines its own error enum:

```rust
// event-store/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum EventStoreError {
    #[error("concurrency conflict: expected version {expected}, found {actual}")]
    ConcurrencyConflict { expected: Version, actual: Version },
    #[error("aggregate not found: {aggregate_type}/{aggregate_id}")]
    AggregateNotFound { aggregate_type: String, aggregate_id: String },
    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),
}

// aggregate-engine/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("unknown aggregate type: {0}")]
    UnknownAggregate(String),
    #[error("invalid transition from state '{from}' via command '{command}'")]
    InvalidTransition { from: String, command: String },
    #[error("guard condition failed")]
    GuardFailed,
    #[error("DAG evaluation error: {0}")]
    Dag(#[from] DagError),
    #[error("event store error: {0}")]
    Store(#[from] EventStoreError),
}

// sdg-loader/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum LoaderError {
    #[error("schema validation failed: {0}")]
    SchemaValidation(String),
    #[error("DAG cycle detected involving node {0}")]
    CyclicDag(String),
    #[error("type mismatch: node {node} expects {expected}, got {actual}")]
    TypeMismatch { node: String, expected: String, actual: String },
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}
```

**Cross-crate propagation:** Use `#[from]` for automatic conversion. `EngineError` wraps `EventStoreError` and `DagError` via `#[from]`, so the `?` operator works naturally across crate boundaries without manual mapping.

**HTTP error mapping:** Implemented in the `middleware` crate via axum's `IntoResponse` trait:

```rust
impl IntoResponse for EngineError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            EngineError::UnknownAggregate(_) =>
                (StatusCode::NOT_FOUND, "UNKNOWN_AGGREGATE"),
            EngineError::InvalidTransition { .. } =>
                (StatusCode::CONFLICT, "INVALID_TRANSITION"),
            EngineError::GuardFailed =>
                (StatusCode::UNPROCESSABLE_ENTITY, "GUARD_FAILED"),
            EngineError::Store(EventStoreError::ConcurrencyConflict { .. }) =>
                (StatusCode::CONFLICT, "CONCURRENCY_CONFLICT"),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR"),
        };
        let body = ErrorResponse {
            error_code: code.into(),
            message: self.to_string(),
            details: None,
            correlation_id: get_correlation_id(), // from request context
        };
        (status, Json(body)).into_response()
    }
}
```

**Confidence:** HIGH -- `thiserror` + `#[from]` + `IntoResponse` is the established Rust pattern.

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Compile-Time Generic Aggregates

**What:** Defining `Aggregate<C, E>` with compile-time `Command` and `Event` types (the `cqrs-es` approach).

**Why bad:** This runtime interprets models dynamically. Compile-time generics would require code generation or macros to create types from the SDG, violating Constitution Principle I.

**Instead:** `serde_json::Value` for all dynamic data. Type safety comes from SDG schema validation at load time, not from the Rust type system at compile time.

### Anti-Pattern 2: Event Store with Update/Delete Operations

**What:** Building the event store with generic CRUD operations.

**Why bad:** Events are immutable facts. Providing update/delete operations undermines the audit trail guarantee (Constitution Principle II).

**Instead:** Only `append` and `load` operations. No mutations. No deletions. Ever.

### Anti-Pattern 3: Synchronous Projection Updates

**What:** Updating projections inside the command handling transaction.

**Why bad:** Couples write and read paths. A slow projection blocks command processing. Adding new projections requires touching the write path.

**Instead:** Async projections via outbox polling. Accept eventual consistency. The 6-pager allows "sync read (replay from event store) for critical queries" as a special case, but this is a read-path optimization, not a write-path coupling.

### Anti-Pattern 4: anyhow in Library Crates

**What:** Using `anyhow::Error` in public APIs of library crates.

**Why bad:** Erases error types. Consumers cannot match on specific variants. Makes error recovery and HTTP status code mapping impossible.

**Instead:** `thiserror` in every library crate. `anyhow` only in the `runtime` binary crate.

### Anti-Pattern 5: Global Mutable State / Singletons

**What:** Static mutable variables for SDG model, DB connections, or configuration.

**Why bad:** Untestable. Race conditions. Cannot run parallel tests.

**Instead:** Pass state via function parameters, `Arc<T>`, and axum's `State` extractor. Test by constructing isolated instances.

### Anti-Pattern 6: Fat Event Store Trait

**What:** Putting command handling, aggregate replay, snapshotting, and projection management into the event store trait.

**Why bad:** Violates single responsibility. Makes backend swapping require reimplementing business logic.

**Instead:** Event store trait has only `append`, `load`, `load_from`. Aggregate replay is in `aggregate-engine`. Projection processing is in `projections`. Snapshots (Phase 3) will be a separate concern.

---

## Build Order Rationale

### Dependency-Driven Build Layers

```
Layer 0 (no internal deps):  sdg-loader, event-store, observability
Layer 1 (depends on L0):     aggregate-engine, projections, middleware
Layer 2 (depends on L1):     api-surface
Layer 3 (depends on all):    runtime
```

### Recommended Implementation Order

| Order | Crate | Why This Sequence |
|-------|-------|-------------------|
| 1 | `sdg-loader` | Its types (`ServiceDefinition`, `AggregateDefinition`, `ComputationDag`, `DagNode`) are consumed by nearly every other crate. Getting these types right is the critical foundation. Bad type design here cascades everywhere. |
| 2 | `event-store` | The other foundational abstraction. The `EventStore` trait shape determines how `aggregate-engine` and `projections` work. Both the trait and the SQLite implementation should be built and tested here. |
| 3 | `aggregate-engine` | The highest-risk component (novel DAG evaluation + model-driven state machine). Building this third enables end-to-end command execution testing (load SDG -> execute command -> verify stored events) without needing HTTP. |
| 4 | `projections` | Completes the write-then-read cycle. Once this works, the full event sourcing loop is testable: command -> event -> projection -> query result. |
| 5 | `observability` | Leaf crate, but lower priority than domain logic. Structured logging via `tracing` macros can be added to any crate at any time; the subscriber just needs initialization before the first log. |
| 6 | `middleware` | JWT auth, validation, error formatting. Needed before HTTP but after the domain crates are stable. |
| 7 | `api-surface` | HTTP layer. Depends on engine, projections, and middleware. Dynamic route building from SDG. |
| 8 | `runtime` | Wire everything together. CLI, startup sequence, graceful shutdown. The final integration point. |

**This order matches the 10-step MVP decomposition:** Step 2 = SDG Schema & Loader, Step 3 = Event Store, Step 4 = Aggregate Engine, Step 5 = Outbox (in event-store), Step 6 = Projections, Step 7 = API Surface, Step 8 = Middleware, Step 9 = Observability, Step 10 = CLI (in runtime).

---

## Scalability Considerations

| Concern | MVP (single user) | 100s of users | 10K+ users (Phase 3) |
|---------|-------------------|---------------|----------------------|
| Event store | SQLite single file, WAL mode | SQLite still viable; WAL handles concurrent reads | Swap to PostgreSQL/EventStoreDB via trait abstraction |
| Aggregate replay | Replay all events per command | Consider in-memory aggregate cache (LRU) | On-demand snapshots (DWH-triggered, per 6-pager) |
| Projection lag | Poll outbox every 100ms | Reduce poll interval, increase batch size | Push-based notification from event store |
| HTTP concurrency | Tokio runtime, single instance | Same (tokio handles thousands of concurrent connections) | Horizontal scaling with load balancer |
| DAG evaluation | Sequential, per-request | Same (DAGs are small, < 50 nodes) | Parallel branch execution for complex DAGs |
| SQLite file size | KB-MB | MB-GB; consider VACUUM and WAL checkpoint tuning | Not suitable; migrate to production DB |

**Key scalability lever:** The `EventStore` trait abstraction. Swapping from `SqliteEventStore` to `PostgresEventStore` changes zero application code, zero SDG definitions, and zero other crate implementations. This is the primary scaling path, by design.

---

## Sources

- [CQRS and Event Sourcing using Rust (cqrs-es docs)](https://doc.rust-cqrs.org/) -- Aggregate trait pattern (adapted for model-driven approach) -- HIGH confidence
- [cqrs-es Aggregate trait definition](https://doc.rust-cqrs.org/intro_add_aggregate.html) -- handle/apply method design -- HIGH confidence
- [How to Build Event-Sourced Apps with CQRS in Rust (2026)](https://oneuptime.com/blog/post/2026-01-25-event-sourcing-cqrs-rust/view) -- Event store, projection, repository patterns -- MEDIUM confidence
- [Eventually crate docs](https://docs.rs/eventually) -- EventStore trait, Subscription, Projection abstractions -- HIGH confidence
- [sqlite-es crate](https://docs.rs/sqlite-es) -- SQLite event store implementation reference -- MEDIUM confidence
- [Dagrs DAG framework](https://github.com/dagrs-dev/dagrs) -- DAG execution patterns, async task graphs -- MEDIUM confidence
- [Petgraph toposort](https://docs.rs/petgraph/latest/petgraph/algo/fn.toposort.html) -- Cycle detection, topological ordering -- HIGH confidence
- [Axum middleware docs](https://docs.rs/axum/latest/axum/middleware/index.html) -- Tower Layer/Service composition -- HIGH confidence
- [Tower Service trait](https://docs.rs/tower/latest/tower/trait.Service.html) -- Middleware chain pattern -- HIGH confidence
- [Tower blog post (Tokio)](https://tokio.rs/blog/2021-05-14-inventing-the-service-trait) -- Service trait design rationale -- HIGH confidence
- [Axum State extractor](https://docs.rs/axum/latest/axum/extract/struct.State.html) -- AppState pattern -- HIGH confidence
- [Transactional Outbox Pattern](https://microservices.io/patterns/data/transactional-outbox.html) -- Outbox design -- HIGH confidence
- [Oxide Outbox (Rust)](https://github.com/Vancoola/oxide-outbox) -- Rust outbox implementation reference -- LOW confidence (less mature)
- [thiserror crate](https://crates.io/crates/thiserror) -- Cross-crate error derivation -- HIGH confidence
- [Kurrent: Live projections](https://www.kurrent.io/blog/live-projections-for-read-models-with-event-sourcing-and-cqrs) -- Catch-up subscription, rebuild patterns -- MEDIUM confidence
- [Event-Driven.io: Projections guide](https://event-driven.io/en/projections_and_read_models_in_event_driven_architecture/) -- Projection patterns -- MEDIUM confidence
