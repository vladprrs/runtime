---
phase: 01-dev-environment
reviewed: 2026-04-07T13:30:00Z
depth: standard
files_reviewed: 22
files_reviewed_list:
  - rust-toolchain.toml
  - rustfmt.toml
  - Cargo.toml
  - Dockerfile
  - .dockerignore
  - .gitignore
  - crates/runtime/Cargo.toml
  - crates/runtime/src/main.rs
  - crates/sdg-loader/Cargo.toml
  - crates/sdg-loader/src/lib.rs
  - crates/event-store/Cargo.toml
  - crates/event-store/src/lib.rs
  - crates/aggregate-engine/Cargo.toml
  - crates/aggregate-engine/src/lib.rs
  - crates/projections/Cargo.toml
  - crates/projections/src/lib.rs
  - crates/api-surface/Cargo.toml
  - crates/api-surface/src/lib.rs
  - crates/middleware/Cargo.toml
  - crates/middleware/src/lib.rs
  - crates/observability/Cargo.toml
  - crates/observability/src/lib.rs
findings:
  critical: 0
  warning: 3
  info: 2
  total: 5
status: issues_found
---

# Phase 01: Code Review Report

**Reviewed:** 2026-04-07T13:30:00Z
**Depth:** standard
**Files Reviewed:** 22
**Status:** issues_found

## Summary

Phase 01 establishes the Cargo workspace skeleton, Rust toolchain pinning, quality gate configuration, and Dockerfile-based CI pipeline. The implementation is well-structured and follows the plan closely. All 8 crate skeletons are correctly configured with workspace dependency and lint inheritance.

No critical issues were found. Three warnings relate to Dockerfile security hardening (container running as root, unpinned base image digests, and `rust-toolchain.toml` excluded from Docker build context by the `*.md` exclusion pattern -- actually that one is fine since `.toml` is not `.md`). Two informational items note minor improvements.

The Rust source files are minimal scaffolding (placeholder tests only) and contain no logic bugs. The configuration files are correct and consistent across all crates.

## Warnings

### WR-01: Dockerfile runtime container runs as root

**File:** `Dockerfile:27-33`
**Issue:** The runtime stage does not specify a non-root user. The `ENTRYPOINT ["runtime"]` will execute as root inside the container. The threat model (T-01-09) explicitly calls this out: "Future hardening: add explicit `USER nonroot` directive." Running as root increases blast radius if the runtime binary is compromised (e.g., via a malformed SDG file triggering unexpected behavior). While this is a known accepted risk per the threat model, it should be addressed before any deployment that accepts external input.
**Fix:**
```dockerfile
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system runtime && useradd --system --gid runtime runtime
COPY --from=builder /app/target/release/runtime /usr/local/bin/runtime
USER runtime
EXPOSE 8080
ENTRYPOINT ["runtime"]
```

### WR-02: Docker base images not pinned by digest

**File:** `Dockerfile:2,9,27`
**Issue:** Base images use tag-based pinning (`rust:1.94.1-bookworm`, `debian:bookworm-slim`) which can be mutated on Docker Hub. A supply chain attack could replace the image behind a tag. The threat model (T-01-06) notes this: "Consider switching to digest-pinned images in production hardening phase." For an MVP this is acceptable, but should be tracked for hardening.
**Fix:** Pin images by digest when available:
```dockerfile
FROM rust:1.94.1-bookworm@sha256:<digest> AS planner
```
Obtain current digests with `docker pull rust:1.94.1-bookworm && docker inspect --format='{{index .RepoDigests 0}}' rust:1.94.1-bookworm`.

### WR-03: .gitignore is minimal -- missing common exclusions

**File:** `.gitignore:1`
**Issue:** The `.gitignore` only excludes `/target`. Common Rust project exclusions are missing: editor backup files (`*.swp`, `*~`, `.vscode/`, `.idea/`), OS artifacts (`.DS_Store`, `Thumbs.db`), and environment files (`*.env`, `.env`). While these do not affect correctness today, a future `.env` file with secrets could be accidentally committed since there is no gitignore rule for it.
**Fix:**
```gitignore
/target

# Editor
*.swp
*~
.vscode/
.idea/

# OS
.DS_Store
Thumbs.db

# Environment / secrets
*.env
.env
```

## Info

### IN-01: Cargo.lock should be committed for the binary crate

**File:** `Cargo.toml` (workspace root)
**Issue:** The summary mentions `Cargo.lock` was created and committed, which is correct for a workspace containing a binary crate. However, `.gitignore` does not explicitly exclude it (good) nor does it mention it. This is fine -- just confirming the Cargo.lock is being tracked, which is the correct practice for binary crates per Cargo documentation. No action needed.

### IN-02: rust-toolchain.toml is not copied before cargo-chef in Dockerfile

**File:** `Dockerfile:4-5,17`
**Issue:** The `COPY . .` on line 5 and line 17 brings in `rust-toolchain.toml`, but the `FROM rust:1.94.1-bookworm` already pins the Rust version. If these versions ever diverge, the Docker build stage would use the `FROM` tag's toolchain while `rustup` inside the container might try to install the `rust-toolchain.toml` version. Currently they match (both 1.94.1), and the `FROM` image already has 1.94.1 installed so `rust-toolchain.toml` is effectively a no-op inside Docker. This is informational only -- no current risk, but worth documenting so future toolchain version bumps update both `FROM` tags and `rust-toolchain.toml` together.

---

_Reviewed: 2026-04-07T13:30:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
