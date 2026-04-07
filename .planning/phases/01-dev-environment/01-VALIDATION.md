---
phase: 1
slug: dev-environment
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-07
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (built-in Rust test harness) |
| **Config file** | Cargo.toml (workspace root) |
| **Quick run command** | `cargo test -p <crate>` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~5 seconds (skeleton crates) |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p <crate>` for affected crate
- **After every plan wave:** Run `cargo test --workspace && cargo fmt --check && cargo clippy --workspace -- -D warnings`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01 | 1 | DEV-01, DEV-07, DEV-08 | — | N/A | build | `cargo build --workspace` | ❌ W0 | ⬜ pending |
| 01-01-02 | 01 | 1 | DEV-02 | — | N/A | build | `rustup show` | ❌ W0 | ⬜ pending |
| 01-01-03 | 01 | 1 | DEV-03 | — | N/A | lint | `cargo fmt --check` | ❌ W0 | ⬜ pending |
| 01-01-04 | 01 | 1 | DEV-04 | — | N/A | lint | `cargo clippy --workspace -- -D warnings` | ❌ W0 | ⬜ pending |
| 01-01-05 | 01 | 1 | DEV-05 | — | N/A | unit | `cargo test --workspace` | ❌ W0 | ⬜ pending |
| 01-01-06 | 01 | 1 | DEV-06 | — | N/A | integration | `docker build .` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `rust-toolchain.toml` — pins Rust 1.94.1
- [ ] `Cargo.toml` — workspace root with 8 member crates
- [ ] `rustfmt.toml` — format configuration
- [ ] Each crate `src/lib.rs` or `src/main.rs` — with at least one placeholder test

*Existing infrastructure covers framework needs — cargo test is built into Rust.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Dokploy build succeeds | DEV-06 | Requires Dokploy infrastructure | Push to deploy branch, verify Dokploy logs show all 4 quality gates pass |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
