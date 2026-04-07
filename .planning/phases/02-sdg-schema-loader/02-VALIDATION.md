---
phase: 2
slug: sdg-schema-loader
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-04-07
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) + insta 1.47 (snapshot testing) |
| **Config file** | `Cargo.toml` workspace config |
| **Quick run command** | `cargo test -p sdg-loader` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p sdg-loader`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 02-01-01 | 01 | 1 | SDG-01 | — | N/A | unit | `cargo test -p sdg-loader --lib schema` | ❌ W0 | ⬜ pending |
| 02-01-02 | 01 | 1 | SDG-02 | — | N/A | unit | `cargo test -p sdg-loader --lib types` | ❌ W0 | ⬜ pending |
| 02-01-03 | 01 | 1 | SDG-03 | — | N/A | unit | `cargo test -p sdg-loader --lib validation` | ❌ W0 | ⬜ pending |
| 02-01-04 | 01 | 1 | SDG-04 | — | N/A | snapshot | `cargo test -p sdg-loader --lib errors` | ❌ W0 | ⬜ pending |
| 02-01-05 | 01 | 1 | SDG-05 | — | N/A | unit | `cargo test -p sdg-loader --lib version` | ❌ W0 | ⬜ pending |
| 02-01-06 | 01 | 1 | SDG-06 | — | N/A | unit | `cargo test -p sdg-loader --lib dag` | ❌ W0 | ⬜ pending |
| 02-01-07 | 01 | 1 | SDG-07 | — | N/A | integration | `cargo test -p sdg-loader --test integration` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/sdg-loader/tests/` — integration test stubs for SDG-07
- [ ] `crates/sdg-loader/fixtures/` — task tracker SDG and broken SDG fixtures

*Existing infrastructure covers test framework (cargo test built-in) and snapshot testing (insta already in dev-dependencies).*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Colorized terminal output | SDG-04 (D-12) | Terminal color rendering not testable in CI | Run `cargo run -p runtime -- validate fixtures/invalid.sdg.json` and visually confirm colored output |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
