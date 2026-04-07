# Runtime — Execution Layer Development Guidelines

Auto-generated from feature plans. Last updated: 2026-04-07

## Active Technologies

- Rust stable (pinned via `rust-toolchain.toml`)
- Key deps: serde, serde_json, tokio, rusqlite, axum, opentelemetry, tracing, uuid, thiserror, jsonschema, clap

## Project Structure

```text
Cargo.toml                  # Workspace root
rust-toolchain.toml         # Pinned toolchain
rustfmt.toml                # Format config
Dockerfile                  # Dokploy build + deploy
crates/
├── runtime/                # Binary crate (entry point / CLI)
├── sdg-loader/             # SDG loading & validation
├── event-store/            # Event persistence (SQLite)
├── aggregate-engine/       # State machine, transitions, DAG
├── projections/            # Read model builder
├── api-surface/            # HTTP endpoints + OpenAPI
├── middleware/              # Auth (JWT), validation, errors
└── observability/          # OTel metrics, tracing, structured logs
```

## Commands

```bash
cargo build --workspace          # Build all crates
cargo test --workspace           # Test all crates
cargo test -p <crate>            # Test single crate (TDD loop)
cargo fmt --check                # Check formatting
cargo clippy --workspace -- -D warnings  # Lint check
```

## Code Style

- Rust edition 2021, stable channel
- rustfmt with minimal custom config
- clippy pedantic enabled at workspace level
- Deny warnings in CI

## Development Discipline

- **Strict TDD**: Red-Green-Refactor for all changes
- **Dokploy CI**: Quality checks run in Dockerfile build stage
- **Coder workspaces**: Primary development environment

## Recent Changes

- 001-mvp-dev-environment: Workspace skeleton, toolchain, CI, TDD infra

<!-- MANUAL ADDITIONS START -->
<!-- MANUAL ADDITIONS END -->

<!-- GSD:project-start source:PROJECT.md -->
## Project

**Runtime — Execution Layer**

A compiled Rust runtime that loads a Service Definition Graph (SDG) file at startup and becomes the microservice it describes. No code generation — the SDG is interpreted directly, eliminating model-code drift by architectural design. Domain specialists describe *what* a service does in JSON; the runtime handles *how* it executes: event-sourced persistence, projections, HTTP API, middleware, observability.

**Core Value:** The SDG file is the single source of truth for service behavior. Changing the model and restarting the runtime is the only mechanism to alter behavior — this eliminates drift between model and implementation.

### Constraints

- **Language**: Rust stable (pinned via `rust-toolchain.toml`)
- **Persistence**: Event sourcing only — no CRUD alternative (Constitution Principle II)
- **Event format**: JSON (protobuf/Avro deferred to benchmarks)
- **Event store**: SQLite for MVP (Constitution Principle III)
- **Reload**: Restart only, no hot reload (Constitution Principle III)
- **TDD**: Strict Red-Green-Refactor for all changes (clarified 2026-04-07)
- **CI**: Dokploy build pipeline runs build, test, fmt, clippy as quality gates
- **Observability**: Built into runtime, not opt-in (Constitution Principle V)
- **Validation**: SDG validated deterministically at load time (Constitution Principle VI)
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

## Recommended Stack
### Core Framework
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| Rust stable | 1.85+ (pin in rust-toolchain.toml) | Language | Compiled, safe, zero-cost abstractions. Pin to a specific stable version (e.g., 1.85.0) to ensure reproducible builds in Dokploy CI. Current stable is 1.94.1 but pin conservatively above MSRV of all deps (1.83 for jsonschema being the highest). | HIGH |
| tokio | 1.47 | Async runtime | The async runtime for Rust. LTS release supported through Sept 2026. Axum, tower, and the entire ecosystem depend on it. Use `features = ["full"]` for the binary crate, `features = ["rt", "macros"]` for library crates that need `#[tokio::test]`. | HIGH |
| axum | 0.8 | HTTP framework | Most popular Rust web framework (surpassed Actix-web in 2023 survey). Built on tower/hyper by the tokio team. No proprietary middleware -- uses tower::Service, giving access to the entire tower-http ecosystem. v0.8 uses `/{param}` syntax. | HIGH |
| serde | 1.0 | Serialization framework | De facto standard. No alternatives worth considering. Use `features = ["derive"]`. | HIGH |
| serde_json | 1.0 | JSON serialization | Standard JSON handling for Rust. Used for SDG parsing, event payloads, API request/response bodies, projection data. | HIGH |
| rusqlite | 0.39 | SQLite binding | Ergonomic SQLite wrapper. Use `features = ["bundled"]` to compile SQLite from source (zero external deps, reproducible builds). Also enable `features = ["serde_json"]` for direct JSON column support and `features = ["uuid"]` for UUID type mapping. | HIGH |
| clap | 4.6 | CLI argument parsing | Industry standard for Rust CLIs. Use `features = ["derive"]` for declarative argument definitions. Powers the `runtime validate` SDG validation command. | HIGH |
### Observability Stack
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| tracing | 0.1.44 | Structured logging + spans | The Rust ecosystem's standard instrumentation library. Structured, span-based, composable. All axum/tokio/tower middleware integrates natively. Use `#[instrument]` attribute macro extensively. | HIGH |
| tracing-subscriber | 0.3.23 | Log output formatting | Subscriber implementation for tracing. Use `features = ["env-filter", "json", "fmt"]` for runtime-configurable log levels and JSON structured output. | HIGH |
| opentelemetry | 0.31 | OTel API | OpenTelemetry API layer for metrics and traces. Pin to 0.31 -- the OTel Rust crates have strict version coupling. | HIGH |
| opentelemetry_sdk | 0.31 | OTel SDK | SDK implementation. Must match opentelemetry version exactly. | HIGH |
| opentelemetry-otlp | 0.31 | OTLP exporter | Exports traces/metrics via OTLP protocol to collectors (Jaeger, Grafana, etc.). Use 0.31.0 (0.31.1 has docs.rs build failure but may work -- verify before adopting). | MEDIUM |
| tracing-opentelemetry | 0.32 | Bridge tracing to OTel | Bridges tracing spans into OpenTelemetry traces. Version 0.32.x is compatible with opentelemetry 0.31.x (one version ahead pattern). | HIGH |
### Error Handling
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| thiserror | 2.0 | Typed error definitions | Derive macro for custom error types. Use in every library crate for domain-specific errors (SdgValidationError, EventStoreError, etc.). v2 is current stable. | HIGH |
### JSON Schema Validation
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| jsonschema | 0.45 | SDG JSON Schema validation | High-performance JSON Schema validator. Supports Draft 2020-12, 2019-09, 7, 6, 4. Use Draft 2020-12 for the SDG schema (latest standard, best features). 75-645x faster than alternatives (valico, jsonschema_valid). Requires Rust 1.83+. | HIGH |
- `valico`: Unmaintained, dramatically slower, no modern draft support.
- `jsonschema_valid`: Minimal feature set, slower.
- Manual validation: Not viable for a JSON Schema as complex as the SDG.
### Graph Processing (Computation DAG)
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| petgraph | 0.8 | DAG data structure + algorithms | Mature graph library for Rust. Provides `DiGraph` for directed graphs, `toposort()` for topological ordering, cycle detection via `is_cyclic_directed()`. Use for SDG computation DAG materialization and execution ordering. | HIGH |
- `daggy`: Built on petgraph but last meaningful update was Feb 2025. Adds a thin DAG-specific API but restricts flexibility. petgraph directly gives you everything daggy does plus more algorithms.
- Custom implementation: Unnecessary. petgraph is battle-tested and covers all needed operations (topological sort, cycle detection, node/edge iteration, subgraph extraction).
### HTTP / API Surface
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| axum | 0.8 | HTTP routing + handlers | (See Core Framework above) | HIGH |
| axum-extra | 0.12 | Extended extractors | TypedHeader for JWT Bearer extraction, Query for typed query params, erased-json for dynamic JSON responses. Use `features = ["typed-header", "query", "json-deserializer"]`. | HIGH |
| tower | 0.5 | Service trait + middleware | The middleware abstraction that axum is built on. Use `ServiceBuilder` to compose middleware layers (auth, validation, tracing, timeout). | HIGH |
| tower-http | 0.6 | HTTP-specific middleware | CORS, compression, request tracing, request-id, timeout, catch-panic. Use `features = ["cors", "compression-gzip", "trace", "request-id", "timeout", "catch-panic"]`. | HIGH |
| utoipa | 5.4 | OpenAPI spec generation | Compile-time OpenAPI generation via derive macros. `#[derive(ToSchema)]` on types, `#[utoipa::path]` on handlers. Code-first approach -- spec is always in sync with code. | HIGH |
| utoipa-axum | 0.2 | Axum router integration | Auto-registers routes and generates OpenAPI paths from axum handlers. Eliminates manual route-to-spec mapping. | HIGH |
| utoipa-swagger-ui | 9.0 | Swagger UI serving | Serves Swagger UI at `/swagger-ui/` for API exploration. Dev-mode only (disable in production via feature flag). Use `features = ["axum"]`. | HIGH |
- `utoipa` over `aide`: utoipa has 5x the downloads, more active maintenance, better axum integration via utoipa-axum. aide 0.16 is still in alpha.
- The SDG defines endpoints declaratively. The runtime generates axum handlers at startup, then uses utoipa to produce the OpenAPI spec. This is NOT a typical derive-at-compile-time pattern -- the OpenAPI spec must be built dynamically from the SDG at runtime. Use utoipa's programmatic API (`utoipa::openapi::OpenApiBuilder`) rather than derive macros for the dynamic parts.
### Authentication
| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| jsonwebtoken | 10.3 | JWT decode/verify | Mature JWT library. v10 requires choosing a crypto backend. Use `features = ["aws_lc_rs"]` (default, fastest) or `features = ["rust_crypto"]` (pure Rust, no C deps). Validates exp, nbf automatically. Configure iss, aud, sub validation in the Validation struct. | HIGH |
### Supporting Libraries
| Library | Version | Purpose | When to Use | Confidence |
|---------|---------|---------|-------------|------------|
| uuid | 1.23 | Unique identifiers | Aggregate IDs, event IDs, correlation IDs. Use `features = ["v4", "v7", "serde"]`. Prefer UUIDv7 for event IDs (time-ordered, sortable). UUIDv4 for aggregate IDs. | HIGH |
| chrono | 0.4 | Date/time handling | Event timestamps, projection timestamps. Use `features = ["serde"]`. | HIGH |
| bytes | 1.0 | Byte buffer | Efficient byte handling for HTTP bodies and serialization. Already pulled in transitively by axum/hyper. | HIGH |
| rusqlite_migration | 2.5 | SQLite schema migrations | Manages event store and projection store schema evolution. Embeds migrations in the binary. Runs at startup before SDG loading. | HIGH |
### Development & Testing
| Library | Version | Purpose | When to Use | Confidence |
|---------|---------|---------|-------------|------------|
| insta | 1.47 | Snapshot testing | Test SDG parsing output, event serialization, API response shapes, error messages. JSON snapshot mode (`assert_json_snapshot!`) is especially valuable for testing SDG validation and event payloads. Use `features = ["json", "redactions"]`. | HIGH |
| tempfile | 3.27 | Temporary files/dirs | Test fixtures for SDG files, SQLite databases in tests. Creates temp dirs that auto-clean. | HIGH |
| tokio-test | 0.4 | Async test utilities | `assert_pending!`, `assert_ready!` for testing async code. Minimal but useful. | HIGH |
| cargo-insta | (CLI tool) | Snapshot review | Interactive snapshot review workflow: `cargo insta test`, `cargo insta review`. Install via `cargo install cargo-insta`. | HIGH |
| assert_matches | 1.5 | Pattern matching asserts | `assert_matches!(result, Err(SdgError::ValidationFailed { .. }))` -- cleaner than manual match blocks. Stable since 2021. | HIGH |
## Event Sourcing: Custom Implementation (NOT a Framework)
### Rationale
### What to Build
- **events table**: `aggregate_type TEXT, aggregate_id TEXT, version INTEGER, event_type TEXT, payload JSON, metadata JSON, created_at TEXT, PRIMARY KEY (aggregate_type, aggregate_id, version)`
- **outbox table**: `id INTEGER PRIMARY KEY, event_id INTEGER, published BOOLEAN, created_at TEXT`
- **Optimistic concurrency**: INSERT with expected version, handle UNIQUE constraint violation
- **Per-aggregate streams**: Query by (aggregate_type, aggregate_id) ordered by version
- **Transactional outbox**: Event + outbox entry in single SQLite transaction
## Version Pinning Strategy
# Core
# Validation
# Graph
# OpenAPI
# Auth
# Observability
# Utilities
# Testing (dev-dependencies)
## What NOT to Use
| Technology | Why Not |
|------------|---------|
| **actix-web** | Proprietary middleware system, not tower-compatible. Axum is the ecosystem standard. |
| **warp** | Minimal maintenance, filter-based API is awkward for complex routing. |
| **diesel** | ORM for relational models. This project uses event sourcing, not ORM patterns. |
| **sqlx** | Async PostgreSQL/MySQL driver. This project uses SQLite via rusqlite (sync, simpler, bundled). sqlx adds complexity for no benefit here. |
| **anyhow (in library crates)** | Erases error types. Library crates need typed errors for proper cross-crate error handling. Only acceptable in the binary crate. |
| **cqrs-es / eventually-rs** | See "Event Sourcing" section above. Framework overhead without value for dynamic model-driven aggregates. |
| **aide** | OpenAPI generation alternative to utoipa. Still in alpha (0.16.0-alpha.3), less ecosystem support. |
| **valico** | JSON Schema validator. Unmaintained, dramatically slower than jsonschema crate. |
| **daggy** | DAG library on top of petgraph. Thin wrapper that restricts rather than helps. Use petgraph directly. |
| **sea-orm / sqlx** | ORM/query builders. Inappropriate for event sourcing append-only patterns. |
| **rocket** | Macro-heavy web framework. Slower adoption, non-tower middleware. |
| **reqwest (for now)** | HTTP client. Not needed in MVP. Phase 2+ may need it for integration calls. |
## Alternatives Considered
| Category | Recommended | Alternative | Why Not Alternative |
|----------|-------------|-------------|---------------------|
| Web framework | axum 0.8 | actix-web 4 | Non-tower middleware, proprietary ecosystem |
| Async runtime | tokio 1.47 | async-std | Tokio is the ecosystem; axum requires it |
| SQLite binding | rusqlite 0.39 | sqlite (sqlx) | rusqlite is sync (simpler for event store), bundles SQLite, no connection pool needed |
| JSON Schema | jsonschema 0.45 | valico | 75-645x slower, no modern draft support, unmaintained |
| OpenAPI | utoipa 5.4 | aide 0.15 | aide is less mature, alpha releases, smaller community |
| Graph | petgraph 0.8 | daggy 0.9 | daggy is a thin petgraph wrapper; use petgraph directly |
| Error handling | thiserror 2.0 | snafu | thiserror is simpler, more widely adopted, sufficient for this project |
| JWT | jsonwebtoken 10.3 | jwt-simple | jsonwebtoken has larger community, more active maintenance |
| Snapshots | insta 1.47 | None | Dominant snapshot testing crate, no real competitor |
## Key Integration Notes
### axum + tower Middleware Stack
### rusqlite in Async Context
### Dynamic OpenAPI from SDG
## Sources
- [axum 0.8.0 announcement](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0) -- axum 0.8 release notes
- [axum GitHub](https://github.com/tokio-rs/axum) -- axum source and docs
- [tokio crates.io](https://crates.io/crates/tokio) -- tokio LTS versions 1.47 and 1.51
- [rusqlite docs.rs](https://docs.rs/crate/rusqlite/latest) -- rusqlite 0.39.0
- [jsonschema GitHub](https://github.com/Stranger6667/jsonschema) -- jsonschema performance benchmarks
- [jsonschema docs.rs](https://docs.rs/crate/jsonschema/latest) -- jsonschema 0.45.1
- [petgraph docs.rs](https://docs.rs/crate/petgraph/latest) -- petgraph 0.8.3
- [utoipa docs.rs](https://docs.rs/crate/utoipa/latest) -- utoipa 5.4.0
- [utoipa-axum docs.rs](https://docs.rs/crate/utoipa-axum/latest) -- utoipa-axum 0.2.0
- [opentelemetry docs.rs](https://docs.rs/crate/opentelemetry/latest) -- opentelemetry 0.31.0
- [tracing-opentelemetry docs.rs](https://docs.rs/crate/tracing-opentelemetry/latest) -- tracing-opentelemetry 0.32.1
- [jsonwebtoken GitHub](https://github.com/Keats/jsonwebtoken) -- jsonwebtoken v10
- [insta docs.rs](https://docs.rs/crate/insta/latest) -- insta 1.47.2
- [cqrs-es docs.rs](https://docs.rs/crate/cqrs-es/latest) -- cqrs-es 0.5.0 (evaluated and rejected)
- [tower docs.rs](https://docs.rs/crate/tower/latest) -- tower 0.5.3
- [tower-http docs.rs](https://docs.rs/crate/tower-http/latest) -- tower-http 0.6.8
- [thiserror crates.io](https://crates.io/crates/thiserror) -- thiserror 2.0
- [rusqlite_migration docs.rs](https://docs.rs/crate/rusqlite_migration/latest) -- rusqlite_migration 2.5.0
- [Rust releases](https://releases.rs/) -- Rust 1.94.1 stable
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

| Skill | Description | Path |
|-------|-------------|------|
| "speckit-analyze" | "Perform a non-destructive cross-artifact consistency and quality analysis across spec.md, plan.md, and tasks.md after task generation." | `.claude/skills/speckit-analyze/SKILL.md` |
| "speckit-checklist" | "Generate a custom checklist for the current feature based on user requirements." | `.claude/skills/speckit-checklist/SKILL.md` |
| "speckit-clarify" | "Identify underspecified areas in the current feature spec by asking up to 5 highly targeted clarification questions and encoding answers back into the spec." | `.claude/skills/speckit-clarify/SKILL.md` |
| "speckit-constitution" | "Create or update the project constitution from interactive or provided principle inputs, ensuring all dependent templates stay in sync." | `.claude/skills/speckit-constitution/SKILL.md` |
| "speckit-implement" | "Execute the implementation plan by processing and executing all tasks defined in tasks.md" | `.claude/skills/speckit-implement/SKILL.md` |
| "speckit-plan" | "Execute the implementation planning workflow using the plan template to generate design artifacts." | `.claude/skills/speckit-plan/SKILL.md` |
| "speckit-specify" | "Create or update the feature specification from a natural language feature description." | `.claude/skills/speckit-specify/SKILL.md` |
| "speckit-tasks" | "Generate an actionable, dependency-ordered tasks.md for the feature based on available design artifacts." | `.claude/skills/speckit-tasks/SKILL.md` |
| "speckit-taskstoissues" | "Convert existing tasks into actionable, dependency-ordered GitHub issues for the feature based on available design artifacts." | `.claude/skills/speckit-taskstoissues/SKILL.md` |
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
