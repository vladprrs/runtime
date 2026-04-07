# Phase 1: Dev Environment - Research

**Researched:** 2026-04-07
**Domain:** Rust workspace scaffolding, toolchain pinning, CI pipeline, TDD infrastructure
**Confidence:** HIGH

## Summary

Phase 1 is greenfield scaffolding: create a Cargo workspace with 8 crates (7 library + 1 binary), pin the Rust toolchain, configure quality gates (fmt, clippy), build a multi-stage Dockerfile for Dokploy CI, and establish TDD infrastructure. No runtime logic -- all crates contain only placeholder code and tests.

The environment is Ubuntu 24.04 on x86_64 with Docker 29.1.4 available but **no Rust toolchain installed locally**. All Rust compilation must happen inside Docker or after a rustup installation step. The Dockerfile itself handles Rust installation, so local Rust is not strictly required for CI, but development workflow benefits from having it.

**Primary recommendation:** Create the complete workspace skeleton with all dependency versions declared in `[workspace.dependencies]`, use `priority = -1` for clippy pedantic group lints to allow selective overrides, and use cargo-chef in the Dockerfile for dependency caching.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Pin Rust to 1.94.1 (current stable) in `rust-toolchain.toml`. This is the latest stable release; bump deliberately when needed.
- **D-02:** Cargo workspace with `crates/` directory. Each library crate maps 1:1 to a runtime component from the 6-pager architecture.
- **D-03:** 8 crates total: `runtime` (binary), `sdg-loader`, `event-store`, `aggregate-engine`, `projections`, `api-surface`, `middleware`, `observability`.
- **D-04:** Workspace-level `[workspace.dependencies]` centralizes all dependency versions to prevent drift across crates.
- **D-05:** All MVP dependencies declared upfront as a bill of materials in workspace `[workspace.dependencies]`: serde, serde_json, tokio, rusqlite, axum, opentelemetry, tracing, uuid, thiserror, jsonschema, clap, and supporting crates per CLAUDE.md tech stack.
- **D-06:** Crate-level `Cargo.toml` files reference workspace deps with `workspace = true` but only include deps relevant to that crate's domain.
- **D-07:** Multi-stage Dockerfile for Dokploy. Build stage runs all quality gates (cargo build, test, fmt --check, clippy -- -D warnings). A failing check blocks deployment.
- **D-08:** No separate CI system -- Dokploy build pipeline is the CI.
- **D-09:** `rustfmt.toml` with minimal custom config (edition 2021 defaults).
- **D-10:** Clippy with pedantic rules enabled at workspace level, deny warnings.
- **D-11:** Strict TDD with Red-Green-Refactor for all changes. Tests written before implementation.
- **D-12:** Each crate gets at least one placeholder test confirming the harness works. Per-crate test execution via `cargo test -p <crate>` for fast TDD feedback loops.
- **D-13:** Minimal scaffolding -- each library crate gets a `lib.rs` with a placeholder test. Binary crate gets a `main.rs`. No error types, traits, or stubs -- those come in later phases when they're needed.

### Claude's Discretion
- Exact rustfmt.toml settings beyond edition default
- Dockerfile caching strategy and base image choice
- Whether to include a `clippy.toml` or use workspace-level Cargo.toml clippy config
- Placeholder test content (simple assert vs. module structure test)

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DEV-01 | Cargo workspace compiles with `cargo build --workspace` producing zero errors and zero warnings | Workspace structure pattern (D-02, D-03), clippy pedantic config with priority pattern, dependency bill of materials |
| DEV-02 | Rust toolchain pinned via `rust-toolchain.toml` with auto-install via rustup | rust-toolchain.toml format verified, pin to `1.94.1` with components `rustfmt`, `clippy` |
| DEV-03 | `cargo fmt --check` validates formatting with `rustfmt.toml` config | rustfmt.toml configuration pattern with `edition = "2021"` |
| DEV-04 | `cargo clippy --workspace -- -D warnings` validates linting with pedantic rules | Workspace lint config with `priority = -1` for pedantic group, selective allows |
| DEV-05 | `cargo test --workspace` executes successfully with TDD infrastructure ready | Placeholder tests in each crate, per-crate test support via `cargo test -p <crate>` |
| DEV-06 | Dokploy Dockerfile runs build, test, fmt, clippy as quality gates before deploy | Multi-stage Dockerfile with cargo-chef for caching, quality gates in build stage |
| DEV-07 | Workspace contains 8 crates (7 library + 1 binary) mapped to runtime components | Crate layout matching 6-pager architecture, dependency mapping per crate |
| DEV-08 | Workspace-level `[workspace.dependencies]` centralizes dependency versions | All 25+ dependencies declared with verified versions, feature flags documented |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **Language:** Rust stable, pinned via `rust-toolchain.toml`
- **Edition:** 2021
- **TDD:** Strict Red-Green-Refactor for all changes
- **CI:** Dokploy build pipeline runs build, test, fmt, clippy as quality gates
- **Clippy:** Pedantic enabled at workspace level, deny warnings
- **Persistence:** Event sourcing only (no CRUD) -- Constitution Principle II
- **Observability:** Built into runtime, not opt-in -- Constitution Principle V
- **Reload:** Restart only, no hot reload -- Constitution Principle III
- **What NOT to use:** actix-web, warp, diesel, sqlx, anyhow (in library crates), cqrs-es, eventually-rs, aide, valico, daggy, sea-orm, rocket, reqwest (for now)

## Standard Stack

### Core Dependencies (Bill of Materials)

All versions verified against docs.rs on 2026-04-07.

| Library | Version | Purpose | Features | Crate(s) |
|---------|---------|---------|----------|----------|
| serde | 1.0.228 | Serialization framework | `derive` | all library crates |
| serde_json | 1.0.149 | JSON serialization | -- | all library crates |
| tokio | 1.51.0 | Async runtime | `full` (binary), `rt`, `macros` (libs) | runtime, api-surface, projections |
| rusqlite | 0.39.0 | SQLite binding | `bundled`, `serde_json`, `uuid` | event-store, projections |
| axum | 0.8.8 | HTTP framework | -- | api-surface, middleware |
| clap | 4.6.0 | CLI argument parsing | `derive` | runtime |
| thiserror | 2.0.18 | Typed error definitions | -- | all library crates |
| jsonschema | 0.45.1 | JSON Schema validation | -- | sdg-loader |
| uuid | 1.23.0 | Unique identifiers | `v4`, `v7`, `serde` | event-store, aggregate-engine |
| chrono | 0.4.44 | Date/time handling | `serde` | event-store |
| bytes | 1.11.1 | Byte buffer | -- | api-surface (transitive via axum) |

[VERIFIED: docs.rs registry, 2026-04-07]

### Observability Stack

| Library | Version | Purpose | Features |
|---------|---------|---------|----------|
| tracing | 0.1.44 | Structured logging + spans | -- |
| tracing-subscriber | 0.3.23 | Log output formatting | `env-filter`, `json`, `fmt` |
| opentelemetry | 0.31.0 | OTel API | -- |
| opentelemetry_sdk | 0.31.0 | OTel SDK | -- |
| opentelemetry-otlp | 0.31.0 | OTLP exporter | -- |
| tracing-opentelemetry | 0.32.1 | Bridge tracing to OTel | -- |

[VERIFIED: docs.rs registry, 2026-04-07]

**Note:** Use opentelemetry-otlp 0.31.0, NOT 0.31.1 (0.31.1 has a docs.rs build failure). [VERIFIED: docs.rs shows build failure for 0.31.1]

### HTTP / API Surface Stack

| Library | Version | Purpose | Features |
|---------|---------|---------|----------|
| axum-extra | 0.12.5 | Extended extractors | `typed-header`, `query`, `json-deserializer` |
| tower | 0.5.3 | Service trait + middleware | -- |
| tower-http | 0.6.8 | HTTP-specific middleware | `cors`, `compression-gzip`, `trace`, `request-id`, `timeout`, `catch-panic` |
| utoipa | 5.4.0 | OpenAPI spec generation | -- |
| utoipa-axum | 0.2.0 | Axum router integration | -- |
| utoipa-swagger-ui | 9.0.2 | Swagger UI serving | `axum` |

[VERIFIED: docs.rs registry, 2026-04-07]

### Authentication

| Library | Version | Purpose | Features |
|---------|---------|---------|----------|
| jsonwebtoken | 10.3.0 | JWT decode/verify | `aws_lc_rs` (default) |

[VERIFIED: docs.rs registry, 2026-04-07]

### Graph Processing

| Library | Version | Purpose | Features |
|---------|---------|---------|----------|
| petgraph | 0.8.3 | DAG data structure + algorithms | -- |

[VERIFIED: docs.rs registry, 2026-04-07]

### Schema Migrations

| Library | Version | Purpose | Features |
|---------|---------|---------|----------|
| rusqlite_migration | 2.5.0 | SQLite schema migrations | -- |

[VERIFIED: docs.rs registry, 2026-04-07]

### Dev Dependencies (Testing)

| Library | Version | Purpose | Features |
|---------|---------|---------|----------|
| insta | 1.47.2 | Snapshot testing | `json`, `redactions` |
| tempfile | 3.27.0 | Temporary files/dirs | -- |
| tokio-test | 0.4.5 | Async test utilities | -- |
| assert_matches | 1.5.0 | Pattern matching asserts | -- |

[VERIFIED: docs.rs registry, 2026-04-07]

### Crate-to-Dependency Mapping

This maps which dependencies each crate should declare in Phase 1. Dependencies are declared but NOT used in code -- this is a bill of materials.

| Crate | Dependencies | Dev Dependencies |
|-------|-------------|------------------|
| **runtime** (bin) | clap, tokio, tracing, tracing-subscriber, sdg-loader, event-store, aggregate-engine, projections, api-surface, middleware, observability | -- |
| **sdg-loader** | serde, serde_json, jsonschema, thiserror, petgraph | insta, tempfile |
| **event-store** | serde, serde_json, rusqlite, rusqlite_migration, uuid, chrono, thiserror, tokio | insta, tempfile, assert_matches |
| **aggregate-engine** | serde, serde_json, thiserror, uuid | insta, assert_matches |
| **projections** | serde, serde_json, rusqlite, thiserror, tokio | insta, tempfile |
| **api-surface** | axum, axum-extra, serde, serde_json, tower, tower-http, utoipa, utoipa-axum, utoipa-swagger-ui, tokio, thiserror | insta, tokio-test |
| **middleware** | axum, tower, jsonwebtoken, thiserror, tracing, uuid | tokio-test |
| **observability** | tracing, tracing-subscriber, opentelemetry, opentelemetry_sdk, opentelemetry-otlp, tracing-opentelemetry, thiserror | -- |

[ASSUMED] -- The exact per-crate dependency mapping is based on the 6-pager architecture component boundaries. Later phases may adjust as actual code is written.

## Architecture Patterns

### Recommended Project Structure

```
Cargo.toml                   # Workspace root with [workspace.dependencies]
rust-toolchain.toml          # Pin Rust 1.94.1, components: rustfmt, clippy
rustfmt.toml                 # edition = "2021"
.dockerignore                # Exclude target/, .git/, docs, etc.
Dockerfile                   # Multi-stage: chef-planner -> chef-cook -> build -> runtime
crates/
├── runtime/                 # Binary crate (entry point / CLI)
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── sdg-loader/              # SDG loading & JSON Schema validation
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
├── event-store/             # Append-only event persistence (SQLite)
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
├── aggregate-engine/        # State machine, transitions, DAG
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
├── projections/             # Read model builder from event stream
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
├── api-surface/             # HTTP endpoint generation + OpenAPI
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
├── middleware/               # Auth (JWT), validation, errors
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
└── observability/           # OTel metrics, tracing, structured logs
    ├── Cargo.toml
    └── src/
        └── lib.rs
```

### Pattern 1: Workspace Root Cargo.toml

**What:** Centralized workspace configuration with dependency versions, lint config, and member list.
**When to use:** Always -- this is the single point of truth for the workspace.

```toml
# Source: Cargo reference + clippy configuration docs
[workspace]
resolver = "2"
members = [
    "crates/runtime",
    "crates/sdg-loader",
    "crates/event-store",
    "crates/aggregate-engine",
    "crates/projections",
    "crates/api-surface",
    "crates/middleware",
    "crates/observability",
]

[workspace.lints.clippy]
pedantic = { level = "deny", priority = -1 }
# Selective allows for pedantic lints that cause more noise than value:
module_name_repetitions = "allow"
must_use_candidate = "allow"
missing_errors_doc = "allow"
missing_panics_doc = "allow"

[workspace.lints.rust]
unsafe_code = "deny"

[workspace.dependencies]
# Core
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.51", features = ["full"] }
# ... (all dependencies listed in Standard Stack section)
```

[VERIFIED: clippy priority syntax confirmed via rust-lang/rust-clippy docs] [CITED: https://rust.code-maven.com/simple-case-of-pedantic-lints]

### Pattern 2: Member Crate Cargo.toml

**What:** Each crate inherits workspace dependencies and lints.
**When to use:** Every member crate.

```toml
# Source: Cargo workspace inheritance docs
[package]
name = "sdg-loader"
version = "0.1.0"
edition = "2021"

[lints]
workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
jsonschema = { workspace = true }
thiserror = { workspace = true }
petgraph = { workspace = true }

[dev-dependencies]
insta = { workspace = true }
tempfile = { workspace = true }
```

[VERIFIED: Cargo workspace inheritance is stable since Rust 1.64] [CITED: https://doc.rust-lang.org/cargo/reference/workspaces.html]

### Pattern 3: rust-toolchain.toml

**What:** Pin exact Rust version with required components.
**When to use:** Repository root.

```toml
# Source: rustup docs
[toolchain]
channel = "1.94.1"
components = ["rustfmt", "clippy"]
```

[VERIFIED: rust-toolchain.toml format from rustup book] [CITED: https://rust-lang.github.io/rustup/overrides.html]

### Pattern 4: rustfmt.toml (Minimal)

**What:** Minimal formatting config, edition 2021 defaults.
**When to use:** Repository root.

```toml
edition = "2021"
```

[VERIFIED: rustfmt edition config] [CITED: https://rust-lang.github.io/rustfmt/]

### Pattern 5: Multi-Stage Dockerfile with cargo-chef

**What:** Dependency caching via cargo-chef, quality gates in build stage, minimal runtime image.
**When to use:** Dokploy CI/deploy pipeline.

```dockerfile
# Stage 1: Chef planner -- compute dependency recipe
FROM rust:1.94.1-bookworm AS planner
RUN cargo install cargo-chef --version ^0.1
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Chef cook -- cache dependencies
FROM rust:1.94.1-bookworm AS builder
RUN cargo install cargo-chef --version ^0.1
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source and run quality gates
COPY . .
RUN cargo fmt --check
RUN cargo clippy --workspace -- -D warnings
RUN cargo test --workspace
RUN cargo build --release --workspace

# Stage 3: Runtime -- minimal image
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/runtime /usr/local/bin/runtime
EXPOSE 8080
ENTRYPOINT ["runtime"]
```

[VERIFIED: cargo-chef 0.1.77 supports workspaces] [CITED: https://github.com/LukeMathWalker/cargo-chef]

### Pattern 6: .dockerignore

**What:** Exclude build artifacts and non-source files from Docker context.
**When to use:** Repository root, alongside Dockerfile.

```
target/
.git/
.planning/
.specify/
.claude/
specs/
*.md
!crates/**/README.md
Dockerfile
.dockerignore
.gitignore
```

[ASSUMED] -- Standard pattern for Rust Docker projects adapted to this project's directory structure.

### Anti-Patterns to Avoid

- **Copying `target/` into Docker context:** Adds gigabytes, invalidates cache. Always exclude via `.dockerignore`.
- **Using `channel = "stable"` in rust-toolchain.toml:** Loses reproducibility. Pin to exact version `1.94.1` per D-01.
- **Declaring dependencies without `workspace = true`:** Creates version drift across crates. All deps must go through `[workspace.dependencies]`.
- **Using `anyhow` in library crates:** Erases error types, prevents proper cross-crate error handling. Use `thiserror` for typed errors. `anyhow` is only acceptable in the binary crate.
- **Adding trait definitions or stubs in Phase 1:** Per D-13, no error types, traits, or stubs -- those come in later phases.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Dependency caching in Docker | Layer tricks with dummy Cargo.toml | cargo-chef 0.1.77 | Handles workspace complexities, recipe-based invalidation |
| JSON Schema validation | Manual validation logic | jsonschema 0.45.1 | 75-645x faster than alternatives, supports Draft 2020-12 |
| CLI argument parsing | Manual arg parsing | clap 4.6.0 with `derive` | Automatic help generation, validation, subcommand support |
| Error types | Manual `impl Error` | thiserror 2.0.18 | Derive macro eliminates boilerplate, proper `From` implementations |
| Lint configuration | Per-file `#[allow]` attributes | Workspace `[lints]` in Cargo.toml | Centralized, consistent, discoverable |
| SQLite bindings | Raw FFI calls | rusqlite 0.39.0 with `bundled` | Safe wrapper, bundles SQLite (zero system dependency) |

**Key insight:** Phase 1 declares dependencies but does not use them. The bill of materials ensures that when Phase 2+ begins implementation, the correct versions are already locked in `Cargo.lock`.

## Common Pitfalls

### Pitfall 1: Clippy Pedantic Priority Ordering

**What goes wrong:** Setting `pedantic = "deny"` in `[workspace.lints.clippy]` without `priority = -1` causes individual `"allow"` overrides to not take effect. Cargo passes flags to clippy with allow lints before deny lints alphabetically, so pedantic overrides the allows.
**Why it happens:** Cargo's lint flag ordering is alphabetical by default. Without explicit priority, group-level denies override individual allows.
**How to avoid:** Always use `pedantic = { level = "deny", priority = -1 }` so the group is processed first and individual allows override it.
**Warning signs:** Clippy errors on lints you thought you allowed (e.g., `module_name_repetitions`).

[VERIFIED: documented in rust-clippy issues and community guides] [CITED: https://rust.code-maven.com/simple-case-of-pedantic-lints]

### Pitfall 2: Unused Dependency Warnings

**What goes wrong:** Declaring dependencies in a crate's `Cargo.toml` but not using them in code triggers `unused_crate_dependencies` warnings (if that lint is enabled) or clutters `cargo tree` output.
**Why it happens:** Phase 1 declares dependencies as a bill of materials before they are used in code.
**How to avoid:** For Phase 1 specifically, only declare dependencies that will be used in the placeholder code (e.g., `serde` for a struct definition, or nothing at all for a truly empty lib.rs). Alternatively, declare them but keep the crate code minimal enough that clippy does not flag them. In practice, empty `lib.rs` files with only a test module will NOT trigger unused dependency warnings because the dependencies are declared but clippy's `unused_crate_dependencies` is NOT part of `pedantic` -- it requires explicit opt-in.
**Warning signs:** Compilation warnings about unused dependencies.

[VERIFIED: `unused_crate_dependencies` is a `restriction` lint, not `pedantic`] [CITED: https://doc.rust-lang.org/stable/clippy/lints.html]

### Pitfall 3: Workspace Resolver Version

**What goes wrong:** Not specifying `resolver = "2"` in the workspace `Cargo.toml` defaults to resolver 1, which has different feature unification behavior.
**Why it happens:** Edition 2021+ defaults to resolver 2 for packages, but workspaces require explicit opt-in.
**How to avoid:** Always include `resolver = "2"` in `[workspace]`.
**Warning signs:** Unexpected feature activation in workspace members.

[VERIFIED: Cargo documentation] [CITED: https://doc.rust-lang.org/cargo/reference/resolver.html]

### Pitfall 4: Dockerfile Quality Gate Ordering

**What goes wrong:** Running `cargo build` before `cargo fmt --check` and `cargo clippy` wastes time -- if formatting or lint checks fail, the long build was for nothing.
**Why it happens:** Instinct to put build first.
**How to avoid:** Run fast checks first: fmt -> clippy -> test -> build. Fmt is nearly instant, clippy catches issues before a full build, tests validate logic, and build produces the release binary last.
**Warning signs:** Slow CI feedback when formatting is wrong.

[ASSUMED] -- Standard CI optimization pattern.

### Pitfall 5: Missing .dockerignore

**What goes wrong:** Docker sends the entire repo as build context, including `.git/` (potentially hundreds of MB) and `target/` (potentially GB).
**Why it happens:** `.dockerignore` is easy to forget.
**How to avoid:** Create `.dockerignore` as one of the first files.
**Warning signs:** Extremely slow `docker build` context transfer.

[VERIFIED: Docker documentation] [CITED: https://docs.docker.com/build/concepts/context/#dockerignore-files]

### Pitfall 6: opentelemetry-otlp 0.31.1 Build Failure

**What goes wrong:** Pinning to `0.31.1` (latest point release) causes build issues.
**Why it happens:** docs.rs shows a build failure for 0.31.1; the issue may affect downstream builds.
**How to avoid:** Pin to `0.31.0` explicitly rather than using `0.31` semver range.
**Warning signs:** Compilation errors in opentelemetry-otlp.

[VERIFIED: docs.rs build status page for opentelemetry-otlp 0.31.1]

## Code Examples

### Placeholder Library Crate (lib.rs)

```rust
// Source: Standard Rust library crate pattern
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
```

### Placeholder Binary Crate (main.rs)

```rust
// Source: Standard Rust binary crate pattern
fn main() {
    println!("Runtime — Execution Layer");
}
```

### Workspace Cargo.toml (Complete Example)

```toml
[workspace]
resolver = "2"
members = [
    "crates/runtime",
    "crates/sdg-loader",
    "crates/event-store",
    "crates/aggregate-engine",
    "crates/projections",
    "crates/api-surface",
    "crates/middleware",
    "crates/observability",
]

[workspace.lints.clippy]
pedantic = { level = "deny", priority = -1 }
module_name_repetitions = "allow"
must_use_candidate = "allow"
missing_errors_doc = "allow"
missing_panics_doc = "allow"

[workspace.lints.rust]
unsafe_code = "deny"

[workspace.dependencies]
# Core
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.51", features = ["full"] }
rusqlite = { version = "0.39", features = ["bundled", "serde_json", "uuid"] }
axum = "0.8"
clap = { version = "4.6", features = ["derive"] }
thiserror = "2.0"
jsonschema = "0.45"
uuid = { version = "1.23", features = ["v4", "v7", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
bytes = "1"

# Graph
petgraph = "0.8"

# HTTP / API Surface
axum-extra = { version = "0.12", features = ["typed-header", "query", "json-deserializer"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "compression-gzip", "trace", "request-id", "timeout", "catch-panic"] }
utoipa = "5.4"
utoipa-axum = "0.2"
utoipa-swagger-ui = { version = "9.0", features = ["axum"] }

# Auth
jsonwebtoken = { version = "10.3", features = ["aws_lc_rs"] }

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json", "fmt"] }
opentelemetry = "0.31"
opentelemetry_sdk = "0.31"
opentelemetry-otlp = "=0.31.0"
tracing-opentelemetry = "0.32"

# Migrations
rusqlite_migration = "2.5"

# Dev/Test
insta = { version = "1.47", features = ["json", "redactions"] }
tempfile = "3.27"
tokio-test = "0.4"
assert_matches = "1.5"
```

**Note:** `opentelemetry-otlp = "=0.31.0"` uses exact version pinning to avoid the broken 0.31.1 release.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `edition = "2021"` | `edition = "2024"` available | Rust 1.85.0 (Feb 2025) | Project uses 2021 per locked decision D-09. 2024 is available but not adopted. |
| `clippy::pedantic` via CLI flags | `[workspace.lints.clippy]` in Cargo.toml | Cargo 1.74 (stable) | Workspace-level lint config is the modern approach; no need for `clippy.toml` or CLI flags |
| Manual Docker layer caching | cargo-chef | 2020+ | cargo-chef handles workspace complexities automatically |
| tokio 1.47 (LTS) | tokio 1.51 (current LTS) | March 2027 LTS | 1.51 is the newest LTS; 1.47 LTS supported until Sept 2026 |
| `resolver = "1"` (default for workspaces) | `resolver = "2"` | Edition 2021 | Always specify resolver 2 explicitly in workspace root |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Per-crate dependency mapping matches 6-pager component boundaries | Standard Stack / Crate-to-Dependency Mapping | Deps may need to be added/removed as code is written in later phases -- low risk, easily fixable |
| A2 | .dockerignore pattern covers all non-source directories in this project | Architecture Patterns / Pattern 6 | Missing exclusions would slow Docker builds but not break them |
| A3 | Quality gate ordering (fmt -> clippy -> test -> build) is optimal for Dokploy | Common Pitfalls / Pitfall 4 | Wrong ordering wastes CI time but does not affect correctness |
| A4 | `module_name_repetitions`, `must_use_candidate`, `missing_errors_doc`, `missing_panics_doc` are the right pedantic lints to allow | Architecture Patterns / Pattern 1 | Additional allows may be needed as code is written -- easily added later |

**If this table is empty:** All claims in this research were verified or cited -- no user confirmation needed.

## Open Questions

1. **Local Rust installation**
   - What we know: No Rust toolchain is installed on the Coder workspace. Docker is available (29.1.4).
   - What's unclear: Whether the implementation agent will need Rust installed locally to run `cargo` commands during development, or if all compilation happens in Docker.
   - Recommendation: Install Rust locally via `rustup` as part of the phase -- the `rust-toolchain.toml` file will ensure the correct version. Local Rust is essential for TDD workflow (`cargo test -p <crate>` must be fast).

2. **Tokio version: 1.47 LTS vs 1.51 LTS**
   - What we know: CLAUDE.md recommends tokio 1.47 (LTS until Sept 2026). Current latest is 1.51.0 (LTS until March 2027).
   - What's unclear: Whether to pin to the CLAUDE.md-recommended 1.47 or use the newer 1.51.
   - Recommendation: Use tokio 1.51 -- it is the current LTS release, has longer support, and all ecosystem crates are compatible. The CLAUDE.md version was written before 1.51 was released.

3. **Pedantic lint allows list**
   - What we know: Clippy pedantic includes many lints. Some common ones cause excessive noise in early development.
   - What's unclear: Exactly which pedantic lints will be noisy with placeholder code.
   - Recommendation: Start with a small allow list (`module_name_repetitions`, `must_use_candidate`, `missing_errors_doc`, `missing_panics_doc`) and add more as needed during implementation.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Docker | Dockerfile build / Dokploy CI | Yes | 29.1.4 | -- |
| Rust toolchain | cargo build/test/fmt/clippy | No | -- | Install via rustup (rust-toolchain.toml auto-installs) |
| rustup | Toolchain management | No | -- | curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh |
| git | Version control | Yes | -- | -- |
| curl | Download rustup | Yes | -- | -- |

**Missing dependencies with no fallback:**
- None (all can be installed)

**Missing dependencies with fallback:**
- Rust/rustup: Not installed but installable via curl. The rust-toolchain.toml will then auto-install the correct version.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + `#[cfg(test)]` |
| Config file | None needed (built into cargo) |
| Quick run command | `cargo test -p <crate>` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements to Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DEV-01 | Workspace compiles zero errors/warnings | build | `cargo build --workspace 2>&1; echo $?` | N/A (build check) |
| DEV-02 | Toolchain pinned, auto-installs | manual | Verify `rust-toolchain.toml` exists with `1.94.1` | N/A (file check) |
| DEV-03 | Format check passes | lint | `cargo fmt --check` | N/A (lint check) |
| DEV-04 | Clippy pedantic passes | lint | `cargo clippy --workspace -- -D warnings` | N/A (lint check) |
| DEV-05 | Test suite runs successfully | unit | `cargo test --workspace` | Wave 0: placeholder tests in each crate |
| DEV-06 | Dockerfile runs quality gates | integration | `docker build -t runtime-test .` | Wave 0: Dockerfile |
| DEV-07 | 8 crates exist | manual | Verify directory structure | N/A (structure check) |
| DEV-08 | Workspace deps centralized | manual | Verify `[workspace.dependencies]` in root Cargo.toml | N/A (config check) |

### Sampling Rate

- **Per task commit:** `cargo test --workspace && cargo fmt --check && cargo clippy --workspace -- -D warnings`
- **Per wave merge:** `cargo test --workspace && cargo fmt --check && cargo clippy --workspace -- -D warnings`
- **Phase gate:** Full suite green + `docker build` succeeds

### Wave 0 Gaps

- [ ] Rust toolchain installation (rustup + rust-toolchain.toml auto-install)
- [ ] Placeholder test in each of 8 crates
- [ ] Dockerfile creation and validation

## Sources

### Primary (HIGH confidence)

- [docs.rs/tokio/1.51.0](https://docs.rs/crate/tokio/latest) -- tokio 1.51.0 verified
- [docs.rs/axum/0.8.8](https://docs.rs/crate/axum/latest) -- axum 0.8.8 verified
- [docs.rs/rusqlite/0.39.0](https://docs.rs/crate/rusqlite/latest) -- rusqlite 0.39.0 verified
- [docs.rs/jsonschema/0.45.1](https://docs.rs/crate/jsonschema/latest) -- jsonschema 0.45.1 verified
- [docs.rs/serde/1.0.228](https://docs.rs/crate/serde/latest) -- serde 1.0.228 verified
- [docs.rs/thiserror/2.0.18](https://docs.rs/crate/thiserror/latest) -- thiserror 2.0.18 verified
- [docs.rs/clap/4.6.0](https://docs.rs/crate/clap/latest) -- clap 4.6.0 verified
- [docs.rs/opentelemetry/0.31.0](https://docs.rs/crate/opentelemetry/latest) -- OTel 0.31.0 verified
- [docs.rs/tracing/0.1.44](https://docs.rs/crate/tracing/latest) -- tracing 0.1.44 verified
- [docs.rs/uuid/1.23.0](https://docs.rs/crate/uuid/latest) -- uuid 1.23.0 verified
- [docs.rs/cargo-chef/0.1.77](https://docs.rs/crate/cargo-chef/latest) -- cargo-chef 0.1.77 verified
- [rustup overrides](https://rust-lang.github.io/rustup/overrides.html) -- rust-toolchain.toml format
- [Rust releases](https://releases.rs/) -- Rust 1.94.1 stable confirmed
- [Docker Hub rust](https://hub.docker.com/_/rust) -- rust:1.94.1-bookworm image available
- [cargo-chef GitHub](https://github.com/LukeMathWalker/cargo-chef) -- workspace support confirmed

### Secondary (MEDIUM confidence)

- [clippy pedantic priority](https://rust.code-maven.com/simple-case-of-pedantic-lints) -- priority = -1 pattern verified
- [coreyja clippy workspace](https://coreyja.com/til/clippy-pedantic-workspace) -- workspace lint inheritance pattern
- [Depot.dev Dockerfile guide](https://depot.dev/blog/rust-dockerfile-best-practices) -- cargo-chef Docker pattern
- [rustfmt configuration](https://rust-lang.github.io/rustfmt/) -- rustfmt.toml options

### Tertiary (LOW confidence)

- None -- all findings verified with primary or secondary sources.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all versions verified against docs.rs registry on 2026-04-07
- Architecture: HIGH -- patterns are well-established Rust conventions with locked decisions from CONTEXT.md
- Pitfalls: HIGH -- clippy priority issue verified against official clippy issue tracker and community docs
- Dockerfile: MEDIUM -- cargo-chef pattern verified but exact Dokploy integration may need adjustment

**Research date:** 2026-04-07
**Valid until:** 2026-05-07 (30 days -- stable domain, no fast-moving dependencies)
