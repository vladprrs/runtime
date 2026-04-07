# Phase 2: SDG Schema & Loader - Research

**Researched:** 2026-04-07
**Domain:** JSON Schema validation, serde deserialization, DAG construction, multi-pass validation pipeline
**Confidence:** HIGH

## Summary

Phase 2 implements the SDG (Service Definition Graph) loader: a multi-pass pipeline that reads a JSON file, validates it against a JSON Schema (Draft 2020-12), checks version compatibility, performs semantic validation (state references, type matching, completeness), materializes computation DAGs with petgraph, and produces a typed `ServiceDefinition` Rust struct. The phase also defines the canonical SDG JSON Schema embedded in the binary, creates a realistic task tracker example SDG, and builds 5-10 broken fixture SDGs for error path testing.

All core libraries are already declared in the workspace `Cargo.toml` and resolved at the correct versions: `jsonschema` 0.45.1, `petgraph` 0.8.3, `serde` 1.0.228, `serde_json` 1.0.149, `thiserror` 2.0.18, `insta` 1.47.2, `tempfile` 3.27.0. Two additional dependencies are recommended: `semver` 1.0.28 for SemVer version parsing/matching and `strsim` 0.11.1 for "did you mean?" suggestion generation.

**Primary recommendation:** Build a four-pass validation pipeline (schema conformance, version check, semantic validation, DAG materialization) where each pass collects all errors before reporting, passes execute strictly in order, and later passes are skipped if earlier ones fail. Use `jsonschema::draft202012::new()` for schema validation with `iter_errors()` for multi-error collection. Use `#[serde(tag = "type")]` internally-tagged enums for DAG node deserialization. Use `petgraph::algo::toposort()` which returns `Err(Cycle)` on cyclic graphs, combining cycle detection and topological ordering in one call.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Full JSON Schema covering all 6-pager sections (service, glossary, aggregates, transitions, projections, endpoints, external_dependencies, processes). MVP loader validates and parses only MVP sections. Deferred sections (TS blocks, Decision Tables, Integration Calls, BPMN processes) are schema-valid but loader logs warnings and ignores them at runtime.
- **D-02:** Schema defined inline in Rust code (no separate schema file in repo). Embedded in the binary via const or string literal. Schema is version-locked to the runtime binary.
- **D-03:** SemVer version field in SDG (`"schema_version": "1.0.0"`). Runtime checks major version match; minor/patch differences accepted.
- **D-04:** SDG file loaded from `./service.sdg.json` by default. Overridable via CLI argument (`runtime --sdg path/to/other.sdg.json`). Runtime requires the SDG file to exist -- fails fast if missing.
- **D-05:** Single monolithic schema with internal `$defs` sections (no split files with cross-file `$ref`).
- **D-06:** Realistic complexity -- Task aggregate with 3 states (Created, InProgress, Done), guard conditions (e.g., can't complete without assignee), 1-2 projections (task list, task count by status), command/query endpoints.
- **D-07:** Valid task tracker SDG plus 5-10 intentionally broken SDG fixtures, each triggering a specific validation error (missing field, DAG cycle, type mismatch, invalid state reference, version incompatibility, etc.).
- **D-08:** All test fixture SDGs live in `crates/sdg-loader/fixtures/`.
- **D-09:** Rich error messages: JSON path to error location, expected vs. found values, which validation pass caught it, and actionable suggestions when possible (e.g., "Did you mean state 'InProgress'?"). Aim for rustc-quality error messages.
- **D-10:** Collect all errors within each pass before reporting. Passes execute in strict order -- later passes do not run if an earlier pass has failures (prevents confusing cascading errors).
- **D-11:** Four validation passes in order: (1) JSON Schema conformance, (2) Version compatibility check, (3) Semantic validation (state references exist, types match across edges, completeness checks), (4) DAG cycle detection and topological sort.
- **D-12:** Dual output format -- human-readable with colorized terminal output by default, machine-readable JSON via `--json` flag.
- **D-13:** Core MVP DAG node types: field access, comparison (eq, neq, gt, lt, gte, lte), boolean logic (and, or, not), arithmetic (+, -, *, /), string operations (concat, contains, length). TS Blocks, Decision Tables, and Integration Calls defined as schema stubs only -- not implemented in loader.
- **D-14:** Phase 2 builds the petgraph `DiGraph`, runs cycle detection via `is_cyclic_directed()`, and computes topological order via `toposort()`. Phase 4 adds the interpreter that evaluates nodes.
- **D-15:** DAG nodes use tagged union with `"type"` discriminator field in JSON (e.g., `{"type": "comparison", "op": "eq", ...}`). Maps to Rust enum with `#[serde(tag = "type")]`.

### Claude's Discretion
- Exact JSON Schema structure and `$defs` organization
- Rust struct field naming and module layout within sdg-loader
- Specific broken SDG fixture scenarios (beyond the general categories listed)
- Internal error type hierarchy (`thiserror` enum design)
- Suggestion algorithm for "did you mean" hints
- Whether to use `insta` snapshot tests for validation output

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SDG-01 | JSON Schema (Draft 2020-12) defines the SDG format with all aggregate, transition, projection, and endpoint structures | jsonschema 0.45.1 `draft202012` module provides full Draft 2020-12 support with `$defs` for internal schema definitions. Schema embedded as `const &str` in Rust binary. |
| SDG-02 | SDG file validated against JSON Schema at load time; invalid files prevent startup with structured error messages | `jsonschema::draft202012::new(&schema)` creates reusable validator; `iter_errors()` collects all schema violations with `instance_path()` and `schema_path()` for location data. |
| SDG-03 | Multi-pass validation: schema conformance, DAG cycle detection, type compatibility across edges, completeness checks | Four-pass architecture per D-11. Each pass returns `Vec<SdgError>`. Pipeline halts on first failing pass. |
| SDG-04 | SDG parsed into typed Rust structs (not raw `serde_json::Value`) with `ServiceDefinition` as the root type | serde `#[derive(Deserialize)]` with `#[serde(tag = "type")]` for DAG nodes, `#[serde(rename_all = "snake_case")]` for field naming. Two-phase: schema validate raw JSON, then deserialize into typed structs. |
| SDG-05 | Computation DAG materialized with pre-computed topological order for runtime evaluation | petgraph 0.8.3 `DiGraph` with `toposort()` returning `Vec<NodeIndex>` or `Err(Cycle)`. Store both the graph and pre-computed order in the validated SDG output. |
| SDG-06 | Task tracker example SDG created as canonical test fixture and demo artifact | Fixture at `crates/sdg-loader/fixtures/valid_task_tracker.sdg.json` with 3 states, guard conditions, projections, endpoints per D-06. |
| SDG-07 | SDG version compatibility field checked at load time; incompatible versions rejected | semver 1.0.28 crate for parsing `"schema_version": "1.0.0"`. Major version must match runtime's expected major version. |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **Language:** Rust stable 1.94.1, pinned via `rust-toolchain.toml`
- **Strict TDD:** Red-Green-Refactor for all changes -- tests written before implementation
- **Clippy pedantic + deny warnings:** All new code must pass `cargo clippy --workspace -- -D warnings`
- **Formatting:** `cargo fmt --check` must pass
- **CI:** Dokploy Dockerfile runs build, test, fmt, clippy as quality gates
- **Event Sourcing only:** No CRUD (Constitution Principle II)
- **Validation at load time:** Constitution Principle VI -- deterministic validation, invalid SDG prevents startup
- **Observability built-in:** Constitution Principle V -- not opt-in
- **What NOT to use:** No actix-web, no warp, no diesel, no sqlx, no anyhow in library crates, no cqrs-es/eventually-rs, no aide, no valico, no daggy

## Standard Stack

### Core (Already in Workspace)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| jsonschema | 0.45.1 | JSON Schema Draft 2020-12 validation | Only viable Rust JSON Schema validator. 75-645x faster than alternatives. Supports iter_errors() for multi-error collection. [VERIFIED: cargo metadata] |
| petgraph | 0.8.3 | DAG data structure + algorithms | Mature graph library. DiGraph, toposort(), is_cyclic_directed(). [VERIFIED: cargo metadata] |
| serde | 1.0.228 | Serialization framework | De facto standard. `#[serde(tag = "type")]` for tagged union deserialization. [VERIFIED: cargo metadata] |
| serde_json | 1.0.149 | JSON parsing | Standard JSON handling. Parses SDG file into `serde_json::Value` for schema validation, then into typed structs. [VERIFIED: cargo metadata] |
| thiserror | 2.0.18 | Error type definitions | Derive macro for typed errors. Powers the `SdgError` hierarchy. [VERIFIED: cargo metadata] |

### New Dependencies Required

| Library | Version | Purpose | Why Needed |
|---------|---------|---------|------------|
| semver | 1.0.28 | SemVer version parsing and matching | D-03 requires SemVer version field with major version match check. `Version::parse()` + direct field comparison is cleaner than hand-rolling. [VERIFIED: cargo search] |
| strsim | 0.11.1 | String similarity for "did you mean" suggestions | D-09 requires actionable suggestions. Levenshtein distance finds closest matches to typos in state names, field names. [VERIFIED: cargo search] |

### Dev Dependencies (Already in Workspace)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| insta | 1.47.2 | Snapshot testing | Assert validation error output, parsed struct shapes, JSON Schema output. `assert_json_snapshot!` with redactions. [VERIFIED: cargo metadata] |
| tempfile | 3.27.0 | Temporary files | Create temp SDG files for file-loading tests. [VERIFIED: cargo metadata] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| semver crate | Manual string parsing | semver handles edge cases (pre-release, build metadata) correctly; manual code is fragile |
| strsim crate | No suggestions | D-09 explicitly requires "did you mean" -- strsim is 50KB, zero dependencies, battle-tested |
| petgraph toposort() | Manual Kahn's algorithm | petgraph is already a dependency, toposort() combines cycle detection + ordering in one call |

**Installation (additions to workspace Cargo.toml):**
```toml
[workspace.dependencies]
semver = { version = "1.0", features = ["serde"] }
strsim = "0.11"
```

**sdg-loader Cargo.toml additions:**
```toml
[dependencies]
semver = { workspace = true }
strsim = { workspace = true }
```

## Architecture Patterns

### Recommended Module Structure

```
crates/sdg-loader/
├── Cargo.toml
├── fixtures/                          # Test SDG fixtures (D-08)
│   ├── valid_task_tracker.sdg.json    # Canonical valid SDG (D-06)
│   ├── invalid_missing_field.sdg.json
│   ├── invalid_dag_cycle.sdg.json
│   ├── invalid_type_mismatch.sdg.json
│   ├── invalid_state_reference.sdg.json
│   ├── invalid_version.sdg.json
│   ├── invalid_duplicate_name.sdg.json
│   ├── invalid_empty_transitions.sdg.json
│   └── invalid_missing_guard_field.sdg.json
└── src/
    ├── lib.rs                         # Public API: load(), validate()
    ├── schema.rs                      # Embedded JSON Schema (const &str)
    ├── types.rs                       # ServiceDefinition, Aggregate, Transition, etc.
    ├── dag.rs                         # DagNode enum, ComputationDag, materialization
    ├── validation/
    │   ├── mod.rs                     # ValidationPipeline orchestration
    │   ├── schema_pass.rs             # Pass 1: JSON Schema conformance
    │   ├── version_pass.rs            # Pass 2: SemVer compatibility
    │   ├── semantic_pass.rs           # Pass 3: Cross-reference validation
    │   └── dag_pass.rs               # Pass 4: DAG cycle detection + toposort
    ├── error.rs                       # SdgError enum, error formatting
    └── suggestions.rs                 # "Did you mean?" string similarity
```

### Pattern 1: Multi-Pass Validation Pipeline

**What:** A sequential pipeline of validation passes that each collect all errors within their scope, with later passes only running if earlier passes succeed.
**When to use:** Always -- this is the core loading mechanism.
**Example:**
```rust
// Source: D-10, D-11 (CONTEXT.md locked decisions)
pub fn load(path: &Path) -> Result<ServiceDefinition, Vec<SdgError>> {
    // Read file
    let content = std::fs::read_to_string(path)
        .map_err(|e| vec![SdgError::FileRead { path: path.to_owned(), source: e }])?;

    // Parse JSON
    let raw: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| vec![SdgError::JsonParse { source: e }])?;

    // Pass 1: Schema conformance
    let schema_errors = validate_schema(&raw);
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    // Pass 2: Version compatibility
    let version_errors = validate_version(&raw);
    if !version_errors.is_empty() {
        return Err(version_errors);
    }

    // Deserialize into typed structs (serde)
    let definition: ServiceDefinition = serde_json::from_value(raw)
        .map_err(|e| vec![SdgError::Deserialization { source: e }])?;

    // Pass 3: Semantic validation
    let semantic_errors = validate_semantics(&definition);
    if !semantic_errors.is_empty() {
        return Err(semantic_errors);
    }

    // Pass 4: DAG materialization + cycle detection
    let dag_errors = materialize_dags(&mut definition);
    if !dag_errors.is_empty() {
        return Err(dag_errors);
    }

    Ok(definition)
}
```
[VERIFIED: pattern follows D-10, D-11 locked decisions]

### Pattern 2: Embedded JSON Schema

**What:** The JSON Schema is embedded as a `const &str` in the Rust binary, compiled into the validator at startup.
**When to use:** Schema validation pass.
**Example:**
```rust
// Source: D-02, D-05 (CONTEXT.md locked decisions)
// [VERIFIED: jsonschema::draft202012::new() API from docs.rs]
const SDG_SCHEMA: &str = include_str!("sdg_schema.json");
// OR: defined as a const string literal directly in schema.rs

use jsonschema::Validator;

fn create_validator() -> Validator {
    let schema: serde_json::Value = serde_json::from_str(SDG_SCHEMA)
        .expect("embedded schema must be valid JSON");
    jsonschema::draft202012::new(&schema)
        .expect("embedded schema must be valid Draft 2020-12")
}

fn validate_schema(instance: &serde_json::Value) -> Vec<SdgError> {
    let validator = create_validator();
    validator.iter_errors(instance)
        .map(|error| SdgError::SchemaViolation {
            instance_path: error.instance_path().to_string(),
            schema_path: error.schema_path().to_string(),
            message: error.to_string(),
        })
        .collect()
}
```

**Design note on schema embedding (D-02):** The schema can either be defined as a Rust string literal in `schema.rs` or placed as a `.json` file inside `src/` and loaded via `include_str!()`. The latter is recommended: it allows the schema to be validated independently (e.g., by JSON Schema meta-validators), is easier to read and maintain, and still compiles into the binary. Place the file at `crates/sdg-loader/src/sdg_schema.json`. [ASSUMED]

### Pattern 3: Tagged Union DAG Nodes

**What:** DAG nodes deserialized using serde's internally-tagged enum representation.
**When to use:** Parsing computation DAG definitions from SDG JSON.
**Example:**
```rust
// Source: D-15 (CONTEXT.md), serde docs (https://serde.rs/enum-representations.html)
// [VERIFIED: serde internally-tagged enum API from serde.rs]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DagNode {
    FieldAccess {
        field: String,
        #[serde(default)]
        path: Vec<String>,
    },
    Comparison {
        op: ComparisonOp,
        left: String,   // node reference
        right: String,  // node reference
    },
    BooleanLogic {
        op: BooleanOp,
        operands: Vec<String>, // node references
    },
    Arithmetic {
        op: ArithmeticOp,
        left: String,
        right: String,
    },
    StringOp {
        op: StringOperation,
        operands: Vec<String>,
    },
    Literal {
        value: serde_json::Value,
    },
    // Schema stubs -- defined but not interpreted until Phase 4+
    TsBlock {
        source: String,
        #[serde(default)]
        config: serde_json::Value,
    },
    DecisionTable {
        #[serde(default)]
        config: serde_json::Value,
    },
    IntegrationCall {
        #[serde(default)]
        config: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOp { Eq, Neq, Gt, Lt, Gte, Lte }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BooleanOp { And, Or, Not }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArithmeticOp { Add, Sub, Mul, Div }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StringOperation { Concat, Contains, Length }
```

**Limitation:** Serde's `#[serde(tag = "type")]` does not work with tuple variants. All variants must be struct or unit variants. This is fine for DAG nodes. [VERIFIED: serde docs at serde.rs/enum-representations.html]

### Pattern 4: petgraph DAG Materialization

**What:** Build a `DiGraph` from parsed DAG node definitions, detect cycles, compute topological order.
**When to use:** Pass 4 of validation pipeline.
**Example:**
```rust
// Source: petgraph 0.8.3 docs (docs.rs/petgraph)
// [VERIFIED: toposort API from petgraph source code]
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::toposort;

pub struct ComputationDag {
    pub graph: DiGraph<DagNode, EdgeType>,
    pub topo_order: Vec<NodeIndex>,
    pub node_map: HashMap<String, NodeIndex>, // node_id -> index
}

fn materialize_dag(
    nodes: &HashMap<String, DagNodeDef>,
    edges: &[(String, String)],
) -> Result<ComputationDag, Vec<SdgError>> {
    let mut graph = DiGraph::new();
    let mut node_map = HashMap::new();
    let mut errors = Vec::new();

    // Add nodes
    for (id, node_def) in nodes {
        let idx = graph.add_node(node_def.node.clone());
        node_map.insert(id.clone(), idx);
    }

    // Add edges (with validation)
    for (from, to) in edges {
        match (node_map.get(from), node_map.get(to)) {
            (Some(&src), Some(&dst)) => { graph.add_edge(src, dst, EdgeType::DataFlow); }
            _ => errors.push(SdgError::DagEdgeReference {
                from: from.clone(), to: to.clone(),
            }),
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // toposort() returns Err(Cycle) if graph has cycles
    match toposort(&graph, None) {
        Ok(order) => Ok(ComputationDag { graph, topo_order: order, node_map }),
        Err(cycle) => {
            // cycle.node_id() returns a NodeIndex involved in the cycle
            Err(vec![SdgError::DagCycle {
                node: find_node_id(&node_map, cycle.node_id()),
            }])
        }
    }
}
```

**Key insight:** `toposort()` combines cycle detection and topological ordering in a single O(|V| + |E|) pass. There is no need to call `is_cyclic_directed()` separately -- `toposort()` returns `Err(Cycle(node_id))` when a cycle exists. Use `is_cyclic_directed()` only if you need a boolean check without the ordering result. [VERIFIED: petgraph 0.8.3 source code]

### Pattern 5: Version Compatibility Check

**What:** Parse SemVer version from SDG, check major version matches runtime expectation.
**When to use:** Pass 2 of validation pipeline.
**Example:**
```rust
// Source: semver 1.0.28 docs (docs.rs/semver)
// [VERIFIED: semver crate API from docs.rs]
use semver::Version;

const SUPPORTED_MAJOR_VERSION: u64 = 1;

fn validate_version(raw: &serde_json::Value) -> Vec<SdgError> {
    let version_str = match raw.get("schema_version").and_then(|v| v.as_str()) {
        Some(v) => v,
        None => return vec![SdgError::MissingVersion],
    };

    let version = match Version::parse(version_str) {
        Ok(v) => v,
        Err(e) => return vec![SdgError::InvalidVersion {
            value: version_str.to_string(),
            reason: e.to_string(),
        }],
    };

    if version.major != SUPPORTED_MAJOR_VERSION {
        return vec![SdgError::IncompatibleVersion {
            found: version,
            expected_major: SUPPORTED_MAJOR_VERSION,
        }];
    }

    vec![]
}
```

### Anti-Patterns to Avoid

- **Validating raw `serde_json::Value` throughout:** Parse into typed structs after schema validation. Working with `Value` beyond pass 1 loses type safety and makes semantic checks brittle. [ASSUMED]
- **Running all passes regardless of failures:** D-10 explicitly requires halting on first failing pass to prevent cascading errors.
- **Using `is_cyclic_directed()` then `toposort()` separately:** Redundant. `toposort()` already detects cycles. Call it once.
- **Storing schema as external file loaded at runtime:** D-02 locks the schema to be embedded in the binary. External schema files would create a deployment coupling.
- **Catching only the first error per pass:** D-09/D-10 require collecting ALL errors within each pass. Use `iter_errors()` for schema pass, collect `Vec<SdgError>` for other passes.
- **Using `anyhow` for errors:** CLAUDE.md explicitly forbids `anyhow` in library crates. Use `thiserror` for typed error enums.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON Schema validation | Custom field-by-field validators | `jsonschema` 0.45.1 | SDG schema is complex (aggregates, transitions, DAGs, projections); hand validation would be thousands of lines and miss edge cases |
| SemVer parsing | Manual version string splitting | `semver` 1.0.28 | Pre-release, build metadata, comparison semantics are tricky |
| String similarity | Manual Levenshtein implementation | `strsim` 0.11.1 | Multiple algorithms (Levenshtein, Damerau-Levenshtein, Jaro-Winkler); edge cases in Unicode handling |
| DAG cycle detection | BFS/DFS cycle detection | `petgraph::algo::toposort` | Already a dependency; O(V+E) with correct implementation |
| Graph data structures | Adjacency list from scratch | `petgraph::DiGraph` | Mature, generic, with iterators and index types |

**Key insight:** The SDG schema is the most complex artifact in this phase. A full JSON Schema with `$defs` for aggregates, transitions, projections, endpoints, and DAG node types would be hundreds of lines. The `jsonschema` crate validates this automatically; hand-rolling it would be error-prone and incomplete.

## Common Pitfalls

### Pitfall 1: Lifetime Issues with jsonschema `iter_errors`

**What goes wrong:** `ValidationError` borrows from both the schema and the instance. Collecting errors into a `Vec` and returning them requires converting to owned data immediately.
**Why it happens:** `jsonschema`'s `iter_errors()` returns `impl Iterator<Item = ValidationError<'_>>` with lifetime ties to the input value.
**How to avoid:** Map each `ValidationError` to an owned `SdgError` (with owned `String` fields) immediately during iteration. Never try to store `ValidationError` values.
**Warning signs:** Compiler errors about lifetimes when trying to return or store `ValidationError`. [VERIFIED: jsonschema error.rs source -- ValidationError has lifetime parameter 'i tied to instance]

### Pitfall 2: Serde Tagged Enum Limitations

**What goes wrong:** Compilation error if any DAG node variant is a tuple variant.
**Why it happens:** Serde's `#[serde(tag = "type")]` only supports struct variants, newtype variants containing structs/maps, and unit variants. Tuple variants cause a compile-time error.
**How to avoid:** Make all DAG node variants struct variants with named fields. This is more readable anyway.
**Warning signs:** Compile error mentioning "tuple variant" and "internally tagged". [VERIFIED: serde docs at serde.rs/enum-representations.html]

### Pitfall 3: Schema-Deserialization Mismatch

**What goes wrong:** JSON passes schema validation but fails serde deserialization, or vice versa.
**Why it happens:** The JSON Schema and the serde struct definitions can drift apart. A field might be `required` in the schema but `Option<T>` in the struct, or the schema allows additional properties that serde rejects.
**How to avoid:** Write integration tests that validate AND deserialize every fixture. Keep schema and struct definitions in sync by treating the schema as the source of truth. Consider a snapshot test that serializes a `ServiceDefinition` and validates it against the schema.
**Warning signs:** Tests pass for schema validation but fail on deserialization, or fixtures that deserialize fine but fail schema validation. [ASSUMED]

### Pitfall 4: DAG Node ID Uniqueness Not Enforced

**What goes wrong:** Two DAG nodes with the same ID silently overwrite each other in the `HashMap`, producing an incorrect graph.
**Why it happens:** JSON objects allow duplicate keys (they're valid JSON per RFC 7159), and `serde_json` takes the last value.
**How to avoid:** Add an explicit uniqueness check in semantic validation (pass 3). Validate that all node IDs within a computation DAG are unique.
**Warning signs:** Graph has fewer nodes than expected; edges point to wrong nodes. [ASSUMED]

### Pitfall 5: JSON Schema `$defs` Scoping

**What goes wrong:** `$ref` references in the monolithic schema fail to resolve correctly.
**Why it happens:** Draft 2020-12 uses `$defs` (not `definitions`). Internal `$ref` paths must use `#/$defs/name` format. Nested `$defs` have different scoping rules than flat ones.
**How to avoid:** Keep all `$defs` at the top level of the schema (flat structure). Use `#/$defs/AggregateDefinition` style references. Test the schema itself with `jsonschema::draft202012::meta::validate()` to catch schema-level errors.
**Warning signs:** Cryptic "unknown reference" errors from jsonschema during validator construction. [VERIFIED: JSON Schema Draft 2020-12 spec uses $defs, confirmed in jsonschema crate source]

### Pitfall 6: Clippy Pedantic False Positives on Large Enums

**What goes wrong:** Clippy pedantic may flag large enum variants (e.g., `SdgError` with many variants) or suggest boxing large variants.
**Why it happens:** `clippy::large_enum_variant` fires when enum variants have significantly different sizes.
**How to avoid:** Box large variant data (e.g., `Box<String>` or structured error data in a `Box<ErrorDetail>`). Or allow the lint on specific types if the perf impact is negligible.
**Warning signs:** Clippy warnings about "large size difference between variants". [ASSUMED]

## Code Examples

### SDG JSON Structure (Task Tracker Example)

This is the shape of the canonical valid fixture per D-06, 6-pager section 5.1:

```json
{
  "schema_version": "1.0.0",
  "service": {
    "name": "task-tracker",
    "description": "A simple task management service",
    "owner": "platform-team"
  },
  "glossary": {
    "terms": {
      "task": "A unit of work that can be created, assigned, and completed"
    }
  },
  "aggregates": {
    "Task": {
      "fields": {
        "title": { "type": "string", "required": true },
        "description": { "type": "string", "required": false },
        "assignee": { "type": "string", "required": false },
        "priority": { "type": "integer", "required": false, "default": 0 }
      },
      "states": ["Created", "InProgress", "Done"],
      "initial_state": "Created"
    }
  },
  "transitions": {
    "StartTask": {
      "aggregate": "Task",
      "from": "Created",
      "to": "InProgress",
      "command": {
        "fields": {
          "assignee": { "type": "string", "required": true }
        }
      },
      "guard": { /* DAG nodes for guard condition */ },
      "events": ["TaskStarted"]
    },
    "CompleteTask": {
      "aggregate": "Task",
      "from": "InProgress",
      "to": "Done",
      "command": { "fields": {} },
      "guard": {
        "nodes": {
          "has_assignee": {
            "type": "field_access",
            "field": "assignee"
          },
          "check_not_empty": {
            "type": "comparison",
            "op": "neq",
            "left": "has_assignee",
            "right": "empty_string"
          },
          "empty_string": {
            "type": "literal",
            "value": ""
          }
        },
        "edges": [
          ["has_assignee", "check_not_empty"],
          ["empty_string", "check_not_empty"]
        ],
        "output": "check_not_empty"
      },
      "events": ["TaskCompleted"]
    }
  },
  "projections": {
    "TaskList": {
      "type": "simple",
      "source_aggregate": "Task",
      "fields": ["title", "assignee", "state"]
    },
    "TaskCountByStatus": {
      "type": "simple",
      "source_aggregate": "Task",
      "group_by": "state",
      "aggregation": "count"
    }
  },
  "endpoints": {
    "commands": {
      "POST /tasks/start": { "transition": "StartTask" },
      "POST /tasks/complete": { "transition": "CompleteTask" }
    },
    "queries": {
      "GET /tasks": { "projection": "TaskList" },
      "GET /tasks/count": { "projection": "TaskCountByStatus" }
    }
  }
}
```
[ASSUMED: Exact SDG JSON structure is Claude's discretion per CONTEXT.md. This example follows the 6-pager section 5.1 specification and D-06 requirements.]

### Error Type Hierarchy

```rust
// Source: D-09 requirements, thiserror 2.0 patterns
// [VERIFIED: thiserror derive API]
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SdgError {
    // File system errors
    #[error("Failed to read SDG file '{path}': {source}")]
    FileRead {
        path: std::path::PathBuf,
        source: std::io::Error,
    },

    // JSON parse errors
    #[error("Invalid JSON: {source}")]
    JsonParse {
        #[from]
        source: serde_json::Error,
    },

    // Pass 1: Schema violations
    #[error("[schema] {instance_path}: {message}")]
    SchemaViolation {
        instance_path: String,
        schema_path: String,
        message: String,
    },

    // Pass 2: Version errors
    #[error("Missing 'schema_version' field")]
    MissingVersion,

    #[error("Invalid version '{value}': {reason}")]
    InvalidVersion {
        value: String,
        reason: String,
    },

    #[error("Incompatible schema version {found} (expected major version {expected_major})")]
    IncompatibleVersion {
        found: semver::Version,
        expected_major: u64,
    },

    // Deserialization errors (between pass 2 and 3)
    #[error("Failed to deserialize SDG: {source}")]
    Deserialization {
        source: serde_json::Error,
    },

    // Pass 3: Semantic errors
    #[error("[semantic] {path}: state '{name}' not found in aggregate '{aggregate}'{suggestion}")]
    InvalidStateReference {
        path: String,
        name: String,
        aggregate: String,
        suggestion: String, // e.g., ". Did you mean 'InProgress'?"
    },

    #[error("[semantic] {path}: type mismatch - expected {expected}, found {found}")]
    TypeMismatch {
        path: String,
        expected: String,
        found: String,
    },

    #[error("[semantic] {path}: {message}")]
    CompletenessError {
        path: String,
        message: String,
    },

    // Pass 4: DAG errors
    #[error("[dag] Cycle detected involving node '{node}' in {context}")]
    DagCycle {
        node: String,
        context: String, // e.g., "transition 'CompleteTask' guard"
    },

    #[error("[dag] {from} -> {to}: edge references non-existent node")]
    DagEdgeReference {
        from: String,
        to: String,
    },
}
```

### Suggestion Algorithm

```rust
// Source: strsim 0.11 (https://github.com/rapidfuzz/strsim-rs)
// [VERIFIED: strsim crate API from crates.io]
use strsim::normalized_damerau_levenshtein;

/// Find the closest match from candidates, returning None if no good match exists.
fn suggest_similar(input: &str, candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .map(|c| (c, normalized_damerau_levenshtein(input, c)))
        .filter(|(_, score)| *score > 0.6) // threshold: 60% similarity
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(candidate, _)| format!(". Did you mean '{candidate}'?"))
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `jsonschema::JSONSchema::compile()` | `jsonschema::draft202012::new()` | jsonschema 0.28+ | New module-based API per draft version. Old `JSONSchema` type deprecated. [VERIFIED: jsonschema source] |
| `definitions` in JSON Schema | `$defs` in JSON Schema | Draft 2019-09 | Draft 2020-12 uses `$defs` not `definitions`. Both still work but `$defs` is standard. [VERIFIED: JSON Schema spec] |
| petgraph 0.6 API | petgraph 0.8 API | 2024 | API stable for `DiGraph`, `toposort`, `is_cyclic_directed`. No breaking changes for our use case. [VERIFIED: petgraph 0.8.3 source] |
| thiserror 1.x | thiserror 2.0 | Late 2024 | v2 is mostly compatible. `#[error]` syntax unchanged. [VERIFIED: both versions resolved in cargo metadata] |

**Deprecated/outdated:**
- `jsonschema::JSONSchema` type: use module-specific constructors (`draft202012::new()`)
- `definitions` keyword in JSON Schema: use `$defs` for Draft 2020-12

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Schema file at `src/sdg_schema.json` loaded via `include_str!()` is preferable to a Rust string literal | Architecture Patterns, Pattern 2 | Low -- either approach embeds in binary. User confirmed embedded-in-binary (D-02); file vs literal is implementation detail. |
| A2 | Exact SDG JSON structure (field names, nesting, section layout) | Code Examples | Medium -- the 6-pager describes sections but not exact JSON syntax. Implementation must define the precise schema. User gave Claude discretion on exact structure. |
| A3 | Schema-deserialization mismatch as a pitfall requiring sync tests | Common Pitfalls, Pitfall 3 | Low -- standard practice for any schema+deserialization system |
| A4 | DAG node ID uniqueness check needed in semantic pass | Common Pitfalls, Pitfall 4 | Medium -- if JSON Schema enforces unique keys via `additionalProperties` this may be redundant, but JSON technically allows duplicate keys |
| A5 | Clippy pedantic may flag large SdgError enum variants | Common Pitfalls, Pitfall 6 | Low -- easy to fix with `#[allow]` or `Box` if it happens |
| A6 | 0.6 similarity threshold for "did you mean" suggestions | Code Examples | Low -- threshold is tunable; 0.6 is a reasonable starting point |

## Open Questions

1. **Exact SDG JSON Schema structure**
   - What we know: The 6-pager (section 5.1) describes sections: service, glossary, aggregates, transitions, projections, endpoints, external_dependencies, processes. D-13 defines DAG node types.
   - What's unclear: The exact JSON nesting, field names, and value formats within each section need to be designed. The 6-pager gives section names but not JSON syntax.
   - Recommendation: This is in Claude's discretion (CONTEXT.md). Design the schema during implementation, following the example structure in this research. Use the task tracker fixture as the primary driver for schema design.

2. **Guard condition output semantics**
   - What we know: Guards are computation DAGs that evaluate to a boolean result. D-14 says Phase 2 builds the graph but Phase 4 adds interpretation.
   - What's unclear: Whether the DAG `output` node must be a boolean-producing node (comparison or boolean_logic), or if any type is valid at Phase 2.
   - Recommendation: Phase 2 should validate that the output node exists and is reachable. Type validation of output can be deferred to Phase 4 (interpreter), or checked in semantic pass if we define output type expectations per context (guards must produce boolean).

3. **Deferred section handling**
   - What we know: D-01 says deferred sections (TS blocks, Decision Tables, Integration Calls, BPMN processes) should be "schema-valid but loader logs warnings and ignores them."
   - What's unclear: What "logs warnings" means in Phase 2 context (no logging infrastructure yet -- Phase 8 sets up observability).
   - Recommendation: Use `eprintln!` or collect warnings alongside the result. Or define a `LoadResult` struct with both the `ServiceDefinition` and `Vec<SdgWarning>`. Phase 8 can retroactively wire this to tracing.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | cargo test (built-in) + insta 1.47.2 (snapshot) |
| Config file | Workspace Cargo.toml (already configured) |
| Quick run command | `cargo test -p sdg-loader` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SDG-01 | JSON Schema validates correct SDG | integration | `cargo test -p sdg-loader --test schema_validation -- --exact test_valid_schema` | No -- Wave 0 |
| SDG-02 | Invalid SDG rejected with structured errors | integration | `cargo test -p sdg-loader --test schema_validation -- --exact test_invalid_rejected` | No -- Wave 0 |
| SDG-03 | Multi-pass validation catches all error categories | integration | `cargo test -p sdg-loader -- multi_pass` | No -- Wave 0 |
| SDG-04 | Parsed into `ServiceDefinition` typed struct | unit | `cargo test -p sdg-loader -- parse_typed` | No -- Wave 0 |
| SDG-05 | DAG materialized with topological order | unit | `cargo test -p sdg-loader -- dag_materialization` | No -- Wave 0 |
| SDG-06 | Task tracker SDG loads successfully | integration | `cargo test -p sdg-loader -- task_tracker` | No -- Wave 0 |
| SDG-07 | Version incompatibility rejected | unit | `cargo test -p sdg-loader -- version_compat` | No -- Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p sdg-loader`
- **Per wave merge:** `cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo fmt --check`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `crates/sdg-loader/fixtures/` directory -- fixture SDG JSON files
- [ ] Test modules in `src/lib.rs` or `tests/` directory for integration tests
- [ ] Snapshot directory for insta (`crates/sdg-loader/src/snapshots/`)
- [ ] No framework install needed -- cargo test is built-in, insta already in dev-dependencies

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | N/A -- SDG loader is a file parser, not an auth component |
| V3 Session Management | No | N/A |
| V4 Access Control | No | N/A -- file system access control is OS-level |
| V5 Input Validation | Yes | jsonschema 0.45.1 for schema validation; serde for type-safe deserialization |
| V6 Cryptography | No | N/A |

### Known Threat Patterns for JSON Schema + File Loading

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed JSON causing DoS (deeply nested) | Denial of Service | serde_json has default recursion limits; jsonschema validates structure before deep processing |
| Path traversal in SDG file path | Tampering | CLI argument is a file path -- validate it exists and is readable; no need for path sanitization beyond `std::fs::read_to_string` |
| Schema bomb ($ref recursion) | Denial of Service | D-05 uses monolithic schema with internal $defs only -- no external $ref resolution. jsonschema handles $ref loops internally. |
| Untrusted SDG content | Tampering | SDG is loaded at startup by operator, not received from untrusted sources. Still validate fully via schema. |

## Sources

### Primary (HIGH confidence)
- jsonschema 0.45.1 source code (compiled docs at `target/doc/src/jsonschema/`) -- Draft 2020-12 API, ValidationError fields, iter_errors() pattern
- petgraph 0.8.3 source code (compiled docs at `target/doc/src/petgraph/`) -- DiGraph, toposort(), is_cyclic_directed() signatures
- serde enum representations docs (https://serde.rs/enum-representations.html) -- internally tagged enum syntax and limitations
- cargo metadata output -- exact resolved dependency versions
- Project CONTEXT.md locked decisions (D-01 through D-15)
- Project CLAUDE.md -- technology stack, constraints, what-not-to-use list

### Secondary (MEDIUM confidence)
- jsonschema GitHub README (https://github.com/Stranger6667/jsonschema) -- API examples, migration guide reference
- semver 1.0.28 docs (https://docs.rs/semver) -- Version::parse(), field access for major version comparison
- strsim 0.11.1 (https://github.com/rapidfuzz/strsim-rs) -- normalized_damerau_levenshtein for "did you mean"
- insta 1.47.2 docs (https://docs.rs/insta) -- assert_json_snapshot!, redactions, glob features
- petgraph docs.rs (https://docs.rs/petgraph/latest/petgraph/algo/fn.toposort.html) -- toposort documentation

### Tertiary (LOW confidence)
- None -- all claims verified against source code or official documentation

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all libraries verified via cargo metadata, APIs verified via compiled docs
- Architecture: HIGH -- patterns follow locked decisions from CONTEXT.md, verified against library APIs
- Pitfalls: MEDIUM -- lifetime and serde issues verified; clippy and schema-sync pitfalls are experience-based
- SDG JSON structure: MEDIUM -- follows 6-pager spec but exact JSON syntax is new design work

**Research date:** 2026-04-07
**Valid until:** 2026-05-07 (stable ecosystem, no fast-moving dependencies)
