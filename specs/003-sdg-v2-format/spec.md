# SDG v2 Format Specification

**Status:** Draft
**Date:** 2026-04-07
**Supersedes:** execution_layer_6pager.md section 5 (SDG v1 format)

---

## 1. Overview

SDG v2 is a convention-over-configuration redesign of the Service Definition Graph format. Users describe the minimum (domain model, computation rules, API shape) and the runtime derives everything else (events, event payloads, command schemas, projections, endpoints).

### Design Principles

1. **Machine-generated, human-editable.** SDG is produced by tooling (UI, CLI wizard, AI) and edited when needed. Verbosity is acceptable; clarity is mandatory.
2. **Flat DAG for computations.** Inspired by n8n workflow format: flat node array + separate edges array. No nesting.
3. **Three data sources.** `field` (aggregate state), `command` (input data), `context` (request context: actor, timestamp, correlation).
4. **Convention with override.** Every derived element can be explicitly overridden.
5. **Validate everything at load time.** DAG cycles, type mismatches, dangling references, missing ports — all caught before the first request.

### Top-Level Structure

```json
{
  "schema_version": "2.0.0",
  "service":       { },
  "model":         { },
  "computations":  { },
  "api":           { }
}
```

| Section | Purpose | Required |
|---------|---------|----------|
| `service` | Identity: name, description, owner | Yes |
| `model` | Domain: aggregates, fields, states, transitions | Yes |
| `computations` | Logic: DAG of typed functions (guards, validations, derived values) | No (empty = no guards) |
| `api` | Access: exposure rules, custom queries, endpoint overrides | No (defaults = expose all) |

---

## 2. `service` Section

```json
"service": {
  "name": "task-tracker-extended",
  "description": "Task management with linked tasks and user assignments",
  "owner": "platform-team"
}
```

| Field | Type | Required | Default |
|-------|------|----------|---------|
| `name` | string | Yes | — |
| `description` | string | No | `""` |
| `owner` | string | No | `"unknown"` |

---

## 3. `model` Section

### 3.1. Aggregates

```json
"model": {
  "aggregates": {
    "AggregateName": {
      "fields": { },
      "states": [ ],
      "initial_state": "...",
      "transitions": { }
    }
  }
}
```

### 3.2. Fields

```json
"fields": {
  "field_name": {
    "type": "string",
    "required": true,
    "default": "value",
    "min": 0,
    "max": 100,
    "min_length": 1,
    "max_length": 255,
    "pattern": "^[a-z]+$",
    "format": "email",
    "references": "OtherAggregate",
    "description": "Human-readable description"
  }
}
```

#### Type System

| Type | Rust mapping | JSON Schema | Notes |
|------|-------------|-------------|-------|
| `string` | `String` | `"type": "string"` | |
| `integer` | `i64` | `"type": "integer"` | |
| `float` | `f64` | `"type": "number"` | |
| `boolean` | `bool` | `"type": "boolean"` | |
| `uuid` | `Uuid` | `"type": "string", "format": "uuid"` | |
| `date` | `NaiveDate` | `"type": "string", "format": "date"` | |
| `datetime` | `DateTime<Utc>` | `"type": "string", "format": "date-time"` | |
| `json` | `serde_json::Value` | `"type": "object"` | Opaque JSON blob |
| `T[]` | `Vec<T>` | `"type": "array"` | Array suffix syntax: `"uuid[]"`, `"string[]"` |

#### Validation Properties

| Property | Applies to | Meaning |
|----------|-----------|---------|
| `required` | all | Field must be present (default: false) |
| `default` | all | Default value if absent |
| `min` / `max` | integer, float | Numeric range |
| `min_length` / `max_length` | string | String length range |
| `pattern` | string | Regex pattern |
| `format` | string | Semantic format: `email`, `uri`, `ipv4`, `ipv6` |
| `references` | uuid, uuid[] | Declares relationship to another aggregate (not enforced, informational) |

### 3.3. States

```json
"states": ["Created", "InProgress", "Done", "Cancelled"]
```

- Array of unique strings.
- **Derivation rule INIT-D1:** `initial_state` defaults to first element. Override with explicit `"initial_state"`.

### 3.4. Transitions

```json
"transitions": {
  "TransitionName": {
    "from": "State" | ["State1", "State2"],
    "to": "State" | "$same",
    "command": { "fields": { } },
    "guard": "computation_node_id",
    "auto_fields": { "field_name": "computation_node_id" },
    "event_name": "override.event.name",
    "description": "Human-readable description"
  }
}
```

| Field | Required | Description |
|-------|----------|-------------|
| `from` | Yes | Source state(s). String or array of strings. |
| `to` | Yes | Target state. String or `"$same"` (remain in current state). |
| `command` | No | Command payload fields. Derived if absent (see derivation rules). |
| `guard` | No | ID of a computation node (must output `boolean`). |
| `auto_fields` | No | Map of `event_field_name` → `computation_node_id`. Auto-populated from computation outputs. |
| `event_name` | No | Override derived event name. |
| `description` | No | Human-readable description. |

#### Sentinel: `$same`

When `"to": "$same"`, the transition does not change state. Used for in-place modifications (edit fields, change priority, link/unlink). The runtime keeps the aggregate in whatever state it was in when the command arrives.

---

## 4. `computations` Section

The computation graph is a flat DAG of typed function nodes connected by explicit edges. Inspired by [n8n workflow format](https://docs.n8n.io/).

### 4.1. Structure

```json
"computations": {
  "nodes": [
    { "id": "unique_id", "type": "function_type", "params": { } }
  ],
  "edges": [
    { "from": "source_node_id", "to": "target_node_id", "port": "input_port_name" }
  ]
}
```

- **Nodes** are a flat array. Each has a unique `id`, a `type` from the function catalog, and type-specific `params`.
- **Edges** are a flat array. Each connects one node's output to another node's named input port. Optionally includes `"index"` for variadic ports (e.g., `and` with N inputs).
- **No nesting.** All nodes exist at the same level. Topology is fully described by edges.

### 4.2. Load-Time Validation

The runtime validates the computation graph at startup:

1. **Parse** all nodes and edges
2. **Resolve** edge references (all `from`/`to` IDs must exist in nodes)
3. **Build** petgraph DiGraph
4. **Detect cycles** via `toposort()` — reject if cyclic
5. **Type-check** every edge: source output type must be compatible with target port's expected input type
6. **Verify guards** referenced by transitions: node must exist and output `boolean`
7. **Verify auto_fields** referenced by transitions: node must exist and output type must match field type

### 4.3. Function Catalog

#### Data Access (Leaf Nodes — No Inputs)

| Type | Params | Output | Description |
|------|--------|--------|-------------|
| `field` | `name: string` | `T` (field type) | Read aggregate field |
| `command` | `name: string` | `T` (command field type) | Read command payload field |
| `context` | `path: string` | `T` | Read request context (`actor.id`, `actor.email`, `timestamp`, `correlation_id`) |
| `literal` | `value: any` | `T` (inferred) | Constant value |

#### Lookup (Cross-Aggregate Queries)

| Type | Ports | Params | Output | Description |
|------|-------|--------|--------|-------------|
| `lookup` | `id: uuid` | `aggregate: string, pick: string` | `T` | Read one aggregate's field from projection |
| `lookup_many` | `ids: uuid[]` | `aggregate: string, pick: string` | `T[]` | Batch read by array of IDs |

#### Collection Operations

| Type | Ports | Params | Output | Description |
|------|-------|--------|--------|-------------|
| `map` | `items: T[]`, `apply: fn` | — | `U[]` | Transform each element |
| `filter` | `items: T[]` | `in: T[]` or `not_in: T[]` or `eq: T` or `neq: T` | `T[]` | Keep matching elements |
| `count` | `items: T[]` | — | `integer` | Count elements |
| `sum` | `items: number[]` | — | `number` | Sum values |
| `min` | `items: T[]` | — | `T` | Minimum value |
| `max` | `items: T[]` | — | `T` | Maximum value |
| `any` | `items: boolean[]` | — | `boolean` | At least one true |
| `all` | `items: boolean[]` | — | `boolean` | All true |
| `contains` | `collection: T[], item: T` | — | `boolean` | Item exists in collection |
| `length` | `value: T[] \| string` | — | `integer` | Length of collection or string |

#### Comparison

| Type | Ports | Output | Description |
|------|-------|--------|-------------|
| `eq` | `left: T, right: T` | `boolean` | Equal |
| `neq` | `left: T, right: T` | `boolean` | Not equal |
| `gt` | `left: T, right: T` | `boolean` | Greater than |
| `lt` | `left: T, right: T` | `boolean` | Less than |
| `gte` | `left: T, right: T` | `boolean` | Greater than or equal |
| `lte` | `left: T, right: T` | `boolean` | Less than or equal |
| `in` | `value: T, set: T[]` | `boolean` | Value in set |
| `not_in` | `value: T, set: T[]` | `boolean` | Value not in set |

For `eq` and `neq`: if only one port is connected, the other comes from `params.right` (or `params.left`). This allows `{ "type": "neq", "params": { "right": "" } }` with only the `left` port wired.

#### Logic

| Type | Ports | Output | Description |
|------|-------|--------|-------------|
| `and` | `in[0..N]: boolean` (variadic, indexed) | `boolean` | All inputs true |
| `or` | `in[0..N]: boolean` (variadic, indexed) | `boolean` | At least one input true |
| `not` | `value: boolean` | `boolean` | Negate |

#### Arithmetic

| Type | Ports | Output | Description |
|------|-------|--------|-------------|
| `add` | `left: number, right: number` | `number` | Addition |
| `sub` | `left: number, right: number` | `number` | Subtraction |
| `mul` | `left: number, right: number` | `number` | Multiplication |
| `div` | `left: number, right: number` | `number` | Division |

#### String

| Type | Ports | Output | Description |
|------|-------|--------|-------------|
| `concat` | `left: string, right: string` | `string` | Concatenation |
| `str_contains` | `haystack: string, needle: string` | `boolean` | Substring check |
| `str_len` | `value: string` | `integer` | String length |

### 4.4. Edge Format

```json
{ "from": "source_id", "to": "target_id", "port": "port_name" }
{ "from": "source_id", "to": "target_id", "port": "in", "index": 0 }
```

| Field | Required | Description |
|-------|----------|-------------|
| `from` | Yes | Source node ID (output flows from here) |
| `to` | Yes | Target node ID (input arrives here) |
| `port` | Yes | Named input port on the target node |
| `index` | No | For variadic ports (`and`, `or`): which slot (0, 1, 2...) |

### 4.5. Execution Model

When evaluating a guard:
1. Find the guard node ID
2. Walk backwards through edges to find all reachable nodes (the subgraph)
3. Topologically sort the subgraph
4. Evaluate nodes in order, passing outputs through edges
5. Return the guard node's boolean output

**Lazy evaluation:** Only the subgraph reachable from the requested output node is evaluated. Unrelated computation nodes are not executed.

### 4.6. Node Reuse

A single node can have multiple outgoing edges. Example: `actor_id` feeds into:
- `actor_exists` (lookup if user is active)
- `is_assignee` (compare with assignee)
- `auto_fields` in Create transition (write to event)

The value is computed once and shared.

---

## 5. `api` Section

```json
"api": {
  "expose": "all",
  "base_path": "/api",
  "protocol": "http",
  "overrides": { },
  "custom_queries": { }
}
```

| Field | Default | Description |
|-------|---------|-------------|
| `expose` | `"all"` | `"all"` / `"none"` / `"explicit"` — which aggregates get endpoints |
| `base_path` | `"/api"` | URL prefix for all generated endpoints |
| `protocol` | `"http"` | `"http"` / `"grpc"` / `"both"` |
| `overrides` | `{}` | Per-endpoint customization |
| `custom_queries` | `{}` | Non-default projections |

### 5.1. Endpoint Overrides

```json
"overrides": {
  "Task.Complete": {
    "path": "/tasks/{id}/finish",
    "auth": "required"
  }
}
```

### 5.2. Custom Queries

```json
"custom_queries": {
  "tasks_by_assignee": {
    "source": "Task",
    "filter_by": "assignee_id",
    "fields": ["title", "priority", "state", "created_at"]
  },
  "task_count_by_status": {
    "source": "Task",
    "group_by": "state",
    "aggregation": { "count": "count" }
  }
}
```

---

## 6. Derivation Rules

### 6.1. Events

**Rule:** Event name = `{AggregateName}.{TransitionName}`

| Transition | Derived Event |
|-----------|---------------|
| Task.Create | `Task.Create` |
| Task.Start | `Task.Start` |
| User.Register | `User.Register` |

**Override:** `"event_name": "TaskStarted"` on the transition.

No linguistic transformation (no past-tense). Dot-notation is language-neutral, greppable, and unambiguous.

### 6.2. Event Payloads

| Transition Type | Derived Payload |
|----------------|----------------|
| Initial (from = initial_state) | All aggregate fields from command (+ auto_fields) |
| Non-initial with command | Command fields (+ auto_fields) |
| Non-initial without command | Empty (+ auto_fields) |

### 6.3. Command Schemas

| Transition Type | Derived Command Fields |
|----------------|----------------------|
| Initial (from = initial_state), no explicit command | All aggregate fields (except `derived` and auto_fields targets) |
| Non-initial, no explicit command | Empty (pure state change, only aggregate_id is implicit) |
| Any with explicit `command.fields` | As specified |

### 6.4. Projections

| Rule | Name | Content |
|------|------|---------|
| List | `{aggregate}_list` | All fields + `id`, `state`, `created_at`, `updated_at` |
| Detail | `{aggregate}_detail` | Same as list + `version` |
| Count by state | `{aggregate}_count_by_state` | Count grouped by state (only if >1 state) |

### 6.5. Endpoints

| Rule | Method | Path | Source |
|------|--------|------|--------|
| Create | POST | `{base}/{plural}` | Initial transition |
| Command | POST | `{base}/{plural}/{id}/{transition}` | Non-initial transition |
| List | GET | `{base}/{plural}` | List projection |
| Detail | GET | `{base}/{plural}/{id}` | Detail projection |
| Custom | GET | `{base}/{plural}/{query_name}` | Custom query |

**Pluralization:** Simple `s` suffix. Override via `api.overrides`.

### 6.6. Initial State

First element of the `states` array. Override with explicit `"initial_state"`.

---

## 7. Request Context

The `context` node type accesses the request context injected by the middleware pipeline (JWT, headers, etc.).

### Available Context Paths

| Path | Type | Source |
|------|------|--------|
| `actor.id` | uuid | JWT `sub` claim |
| `actor.email` | string | JWT `email` claim |
| `actor.roles` | string[] | JWT `roles` claim |
| `timestamp` | datetime | Request timestamp |
| `correlation_id` | uuid | `X-Correlation-ID` header or auto-generated |

### Usage in Computations

```json
{ "id": "actor_id", "type": "context", "params": { "path": "actor.id" } }
```

### Usage in Auto-Fields

```json
"Create": {
  "from": "Created", "to": "Created",
  "auto_fields": {
    "author_id": "actor_id",
    "created_by": "actor_id"
  }
}
```

---

## 8. Comparison: v1 vs v2

| Aspect | v1 (6-pager) | v2 (this spec) |
|--------|-------------|----------------|
| Top-level sections | 8 | 4 |
| Transitions location | Separate top-level map | Nested in aggregate |
| Guard format | Inline DAG nodes/edges/output | Reference to computation node ID |
| Computation DAG | Per-transition, embedded | Service-level, flat nodes + edges |
| Events | Manual naming | Derived: `Aggregate.Transition` |
| Event payloads | Implicit | Derived from command fields |
| Projections | Manual definition | Auto-generated defaults |
| Endpoints | Manual wiring | Convention-based derivation |
| Expression sugar | None | String expressions compiled to DAG nodes |
| Cross-aggregate | Not supported | `lookup` / `lookup_many` via projections |
| Request context | Not supported | `context` node type |
| Auto-populated fields | Not supported | `auto_fields` on transitions |
| Field relationships | Not supported | `references` on field definition |
| Same-state transitions | Not supported | `"to": "$same"` |
| Array types | Not supported | `T[]` suffix syntax |
| Field validation | Type only | `min`, `max`, `pattern`, `format`, etc. |

### Size Comparison (Task Tracker)

| Format | Lines | Reduction |
|--------|-------|-----------|
| v1 (6-pager style) | ~80 | — |
| v2 minimal (no computations) | ~25 | 69% |
| v2 with full computations | ~120 | -50% (larger) |

v2 with computations is larger in absolute terms but the computation graph replaces what would be scattered inline guards, custom validation code, and manual wiring in v1. The separation of concerns (model vs computation vs API) makes each section independently understandable.

---

## 9. Full Example: Extended Task Tracker

See companion file: `specs/003-sdg-v2-format/examples/task-tracker-extended.sdg.json`

---

## 10. Open Questions

### 10.1. Expression Sugar Layer

Simple guards can be written as inline strings:

```json
"guard": "assignee != ''"
```

The loader compiles these to anonymous DAG nodes. This is syntactic sugar, not a separate mechanism. **Decision needed:** support in Phase 2 MVP or defer?

**Recommendation:** Defer. Build the DAG engine first. Sugar layer is a loader convenience that can be added without changing the runtime.

### 10.2. Consistency Model for Lookups

`lookup` and `lookup_many` read from projections (eventually consistent). Should the SDG declare consistency requirements?

```json
{ "id": "linked_states", "type": "lookup_many", "params": { "aggregate": "Task", "pick": "state", "consistency": "eventual" } }
```

**Recommendation:** Default to `eventual`. Add `"consistency": "strong"` as future option (reads from event stream directly).

### 10.3. Filter Predicates

Current design uses `params` for filter conditions (`in`, `not_in`, `eq`, `neq`). Complex predicates (e.g., "filter where field > threshold AND field < limit") would need a sub-computation.

**Recommendation:** For MVP, support simple filter params. Complex filters use `map` (apply a boolean computation to each element) + `filter` (keep truthy results).

### 10.4. Map with Apply

The `map` node needs an `apply` port that references a computation sub-function. How to express "for each element, run this computation"?

**Recommendation:** Defer `map` + `apply` to post-MVP. `lookup_many` covers the primary use case (map IDs to aggregate fields). Other map operations can wait.
