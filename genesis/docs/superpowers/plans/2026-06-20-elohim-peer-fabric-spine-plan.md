---
title: "elohim-peer-fabric crate — the shared defense+score spine (Wave-A of the Doorway Membrane arc)"
id: elohim-peer-fabric-spine-plan
status: Draft
class: protocol-canonical
domain: D8
sprint: doorway-membrane-wave-a
cites:
  - doorway-membrane-prosocial-routing-design | the spec this plan implements (Wave-A spine = §2 elohim-peer-fabric crate) | sha256:560686edf977447a | path: genesis/docs/superpowers/specs/2026-06-20-doorway-membrane-prosocial-routing-design.md
  - elohim-facings-crate-extraction-plan | the pure-crate extraction pattern this plan follows (deps-graph boundary, no-diesel, byte-identical discipline) | sha256:d301f34b3b7e66d4 | path: genesis/docs/superpowers/plans/2026-06-19-elohim-facings-crate-extraction-plan.md
refines: genesis/docs/superpowers/specs/2026-06-20-doorway-membrane-prosocial-routing-design.md
# Single-host / DB-free: the crate is pure logic with hand-built test inputs — needs NO node/cluster.
# (No requires_env: this whole plan is buildable+testable on a dev box.)
---

# elohim-peer-fabric Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create the write-once shared pure-logic crate `elohim/elohim-peer-fabric` with two modules — `guard` (fail2ban-style admission/ban/rate-shape defense-in-depth) and `score` (capability-aware peer ranking with graceful degradation) — that both `doorway-service` and `elohim-storage` will later consume, feature-gated per node role.

**Architecture:** Pure-logic crate following the `elohim-facings` extraction pattern (deps minimal, **no diesel** — the dependency graph IS the boundary). All I/O is behind traits (`GuardStore`, `Clock`, `OperationalView`) that the consuming runtimes implement; v1 ships the logic + DB-free unit tests with hand-built inputs. No HTTP, no DB, no DHT — zero new DHT entry types (per the spec's P2P-gate verdict).

**Tech Stack:** Rust (native, `RUSTFLAGS=""`), std-only deps, `cargo` workspace member.

## Global Constraints

- **Native cargo env (every cargo command in this plan):** `RUSTFLAGS=""` (the ambient WASM `getrandom` flag breaks native linking), `RUSTC_WRAPPER=""` (sccache returns null bytes on the clippy-driver `--print` probe), `CARGO_TARGET_DIR=/tmp/peer-fabric-target` (the `/projects` pool slot throws fingerprint ENOENT on cold builds). The crate is tiny (std-only) so builds are light, but set these anyway.
- **PVC ceiling:** heavy cargo is DENIED at the 85% PVC hard ceiling by `.claude/hooks/cargo-disk-guard.py` and `FORCE_HEAVY_GATES` does NOT bypass it. This crate's build is small; if blocked anyway, free non-pool space — do not fight the hook.
- **Crate deps:** `std` ONLY for v1. Do NOT add `diesel`, `elohim-storage`, `serde`, `chrono`, `tokio`, or any I/O crate. Minimal deps is the entire point — absence of `diesel` is what enforces the purity boundary.
- **Commit discipline (shared worktree):** commit-only — the integrator pushes; never `git push`. Multiple sessions share this worktree: commit path-limited (`git commit -m "…" -- <explicit paths>`), and NEVER `git commit --amend` without first confirming `git rev-parse --short HEAD` is still your last commit (HEAD may have moved to a concurrent session's commit — see memory `concurrent-sessions-shared-worktree`).
- **No new DHT entry types / no DB / no HTTP** in this crate — it is pure logic only.
- **Crate home:** `elohim/elohim-peer-fabric` (sibling to `elohim/elohim-facings`, `elohim/elohim-views`), NOT under `crates/` (which holds client SDKs).

---

### Task 1: Scaffold the pure crate + prove the purity boundary

**Files:**
- Create: `elohim/elohim-peer-fabric/Cargo.toml`
- Create: `elohim/elohim-peer-fabric/src/lib.rs`
- Modify: root `Cargo.toml` (add the workspace member)

**Interfaces:**
- Produces: the crate `elohim-peer-fabric` with `pub mod guard;` and `pub mod score;` (empty modules for now), and cargo features `edge-defense`, `serve-routing`, `peer-defense`, `identity-routing`.

- [ ] **Step 1: Create `elohim/elohim-peer-fabric/Cargo.toml`**

```toml
[package]
name = "elohim-peer-fabric"
version = "0.1.0"
edition = "2021"
description = "Shared pure-logic peer-traffic fabric: defense-in-depth (guard) + capability-aware peer ranking (score). No I/O, no diesel — the dependency graph is the purity boundary."

[features]
default = []
# Node-role gates — a consuming runtime enables what its role drives.
edge-defense = []      # doorway: guard at the membrane
peer-defense = []      # storage: guard on directly-exposed surfaces
serve-routing = []     # storage: score.rank at serve time
identity-routing = []  # doorway: score-assisted conductor routing (fast-follow)

[dependencies]
# std ONLY. Adding diesel/serde/tokio/etc. breaks the purity boundary — see the plan's Global Constraints.
```

- [ ] **Step 2: Create `elohim/elohim-peer-fabric/src/lib.rs`**

```rust
//! elohim-peer-fabric — the write-once shared peer-traffic spine.
//!
//! Two pure-logic modules consumed by BOTH `doorway-service` and `elohim-storage`,
//! feature-gated per node role:
//!   - [`guard`]: fail2ban-style admission / ban / rate-shape / challenge (defense-in-depth).
//!   - [`score`]: capability-aware peer ranking with graceful degradation.
//!
//! The crate has NO I/O: all state/time/data access is behind traits the runtimes implement.
//! The absence of `diesel` from `Cargo.toml` IS the purity boundary — impure code won't compile here.
//! See genesis/docs/superpowers/specs/2026-06-20-doorway-membrane-prosocial-routing-design.md.

pub mod guard;
pub mod score;
```

- [ ] **Step 3: Create empty module files**

```rust
// elohim/elohim-peer-fabric/src/guard.rs
//! Defense-in-depth: per-source admission verdict over a pluggable store + clock.
```

```rust
// elohim/elohim-peer-fabric/src/score.rs
//! Capability-aware peer ranking — a pure composer over operational signals.
```

- [ ] **Step 4: Add the workspace member to root `Cargo.toml`**

Find the `members = [ … ]` array and add `"elohim/elohim-peer-fabric"` (keep alphabetical/grouped near `elohim/elohim-facings`). If the root uses glob members (`"elohim/*"`), no edit is needed — verify with Step 5.

- [ ] **Step 5: Verify the empty crate compiles**

Run: `RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/peer-fabric-target cargo check -p elohim-peer-fabric`
Expected: PASS (compiles clean, no warnings).

- [ ] **Step 6: Prove the purity boundary (the real enforcement)**

Temporarily add `use diesel::prelude::*;` to the top of `src/guard.rs`, then:
Run: `RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/peer-fabric-target cargo check -p elohim-peer-fabric`
Expected: FAIL with `error[E0432]: unresolved import diesel` (the dependency graph rejects impure code).
Then REMOVE the `use diesel` line and re-run Step 5 to confirm PASS.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-peer-fabric/Cargo.toml elohim/elohim-peer-fabric/src/lib.rs \
        elohim/elohim-peer-fabric/src/guard.rs elohim/elohim-peer-fabric/src/score.rs Cargo.toml
git commit -m "feat(peer-fabric): scaffold the pure-logic shared crate (guard + score modules, role features)" \
  -- elohim/elohim-peer-fabric/Cargo.toml elohim/elohim-peer-fabric/src/lib.rs \
     elohim/elohim-peer-fabric/src/guard.rs elohim/elohim-peer-fabric/src/score.rs Cargo.toml
```

---

### Task 2: `guard` — the defense-in-depth verdict engine

**Files:**
- Modify: `elohim/elohim-peer-fabric/src/guard.rs`

**Interfaces:**
- Produces:
  - `pub enum Verdict { Allow, Shape { delay_ms: u64 }, Challenge, Deny }`
  - `pub trait Clock { fn now_secs(&self) -> u64; }`
  - `pub trait GuardStore { fn record(&mut self, source: &str, ts_secs: u64); fn count_since(&self, source: &str, since_secs: u64) -> u32; fn is_banned(&self, source: &str, now_secs: u64) -> bool; fn ban_until(&mut self, source: &str, until_secs: u64); }`
  - `pub struct GuardConfig { pub window_secs: u64, pub shape_threshold: u32, pub challenge_threshold: u32, pub ban_threshold: u32, pub ban_secs: u64, pub shape_delay_ms: u64 }` (+ `Default`)
  - `pub fn assess<S: GuardStore, C: Clock>(store: &mut S, clock: &C, cfg: &GuardConfig, source: &str) -> Verdict`

- [ ] **Step 1: Write the failing tests**

Append to `src/guard.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct FixedClock(u64);
    impl Clock for FixedClock { fn now_secs(&self) -> u64 { self.0 } }

    #[derive(Default)]
    struct MemStore { hits: HashMap<String, Vec<u64>>, bans: HashMap<String, u64> }
    impl GuardStore for MemStore {
        fn record(&mut self, source: &str, ts: u64) { self.hits.entry(source.into()).or_default().push(ts); }
        fn count_since(&self, source: &str, since: u64) -> u32 {
            self.hits.get(source).map_or(0, |v| v.iter().filter(|&&t| t >= since).count() as u32)
        }
        fn is_banned(&self, source: &str, now: u64) -> bool {
            self.bans.get(source).map_or(false, |&until| until > now)
        }
        fn ban_until(&mut self, source: &str, until: u64) { self.bans.insert(source.into(), until); }
    }

    fn cfg() -> GuardConfig {
        GuardConfig { window_secs: 60, shape_threshold: 3, challenge_threshold: 6, ban_threshold: 10, ban_secs: 300, shape_delay_ms: 250 }
    }

    #[test]
    fn first_request_is_allowed() {
        let mut s = MemStore::default();
        assert_eq!(assess(&mut s, &FixedClock(1000), &cfg(), "ip:1.2.3.4"), Verdict::Allow);
    }

    #[test]
    fn crossing_shape_threshold_shapes_then_challenges_then_bans() {
        let mut s = MemStore::default();
        let clk = FixedClock(1000);
        let c = cfg();
        // Pre-load 4 hits in-window → next assess sees 5 → Shape (>=shape, <challenge).
        for _ in 0..4 { s.record("src", 1000); }
        assert!(matches!(assess(&mut s, &clk, &c, "src"), Verdict::Shape { .. }));
        // Push to challenge band.
        for _ in 0..3 { s.record("src", 1000); }
        assert_eq!(assess(&mut s, &clk, &c, "src"), Verdict::Challenge);
        // Push past ban threshold → Deny + future ban set.
        for _ in 0..5 { s.record("src", 1000); }
        assert_eq!(assess(&mut s, &clk, &c, "src"), Verdict::Deny);
        assert!(s.is_banned("src", 1000));
    }

    #[test]
    fn banned_source_is_denied_without_recording() {
        let mut s = MemStore::default();
        s.ban_until("bad", 2000);
        assert_eq!(assess(&mut s, &FixedClock(1500), &cfg(), "bad"), Verdict::Deny);
        assert_eq!(s.count_since("bad", 0), 0, "a banned source must not be recorded (no unbounded growth)");
    }

    #[test]
    fn ban_expires_and_traffic_resumes() {
        let mut s = MemStore::default();
        s.ban_until("x", 2000);
        assert_eq!(assess(&mut s, &FixedClock(2001), &cfg(), "x"), Verdict::Allow);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/peer-fabric-target cargo test -p elohim-peer-fabric guard`
Expected: FAIL to compile — `Verdict`, `assess`, etc. not defined.

- [ ] **Step 3: Write the minimal implementation**

Prepend to `src/guard.rs` (above the `#[cfg(test)]` block):

```rust
/// The membrane policy verdict for one inbound request from `source`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    Shape { delay_ms: u64 },
    Challenge,
    Deny,
}

/// Monotonic-ish wall clock in whole seconds (the runtime supplies it; tests inject a fixed value).
pub trait Clock {
    fn now_secs(&self) -> u64;
}

/// Pluggable defense state. Storage backs this with SQLite; doorway with an in-memory/edge store.
pub trait GuardStore {
    fn record(&mut self, source: &str, ts_secs: u64);
    fn count_since(&self, source: &str, since_secs: u64) -> u32;
    fn is_banned(&self, source: &str, now_secs: u64) -> bool;
    fn ban_until(&mut self, source: &str, until_secs: u64);
}

/// Thresholds for the sliding-window rate response. `shape <= challenge <= ban`.
#[derive(Debug, Clone)]
pub struct GuardConfig {
    pub window_secs: u64,
    pub shape_threshold: u32,
    pub challenge_threshold: u32,
    pub ban_threshold: u32,
    pub ban_secs: u64,
    pub shape_delay_ms: u64,
}

impl Default for GuardConfig {
    fn default() -> Self {
        Self { window_secs: 60, shape_threshold: 20, challenge_threshold: 60, ban_threshold: 200, ban_secs: 900, shape_delay_ms: 250 }
    }
}

/// Assess one request from `source`. Banned sources are denied WITHOUT recording (no unbounded growth).
/// Otherwise record the hit, count the in-window rate, and escalate: Allow → Shape → Challenge → Deny(+ban).
pub fn assess<S: GuardStore, C: Clock>(store: &mut S, clock: &C, cfg: &GuardConfig, source: &str) -> Verdict {
    let now = clock.now_secs();
    if store.is_banned(source, now) {
        return Verdict::Deny;
    }
    store.record(source, now);
    let since = now.saturating_sub(cfg.window_secs);
    let count = store.count_since(source, since);
    if count > cfg.ban_threshold {
        store.ban_until(source, now + cfg.ban_secs);
        Verdict::Deny
    } else if count > cfg.challenge_threshold {
        Verdict::Challenge
    } else if count > cfg.shape_threshold {
        Verdict::Shape { delay_ms: cfg.shape_delay_ms }
    } else {
        Verdict::Allow
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/peer-fabric-target cargo test -p elohim-peer-fabric guard`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-peer-fabric/src/guard.rs
git commit -m "feat(peer-fabric): guard — sliding-window admission verdict (allow/shape/challenge/deny + ban)" \
  -- elohim/elohim-peer-fabric/src/guard.rs
```

---

### Task 3: `score` — capability-aware peer ranking

**Files:**
- Modify: `elohim/elohim-peer-fabric/src/score.rs`

**Interfaces:**
- Produces:
  - `pub struct Candidate { pub agent_cid: String, pub capability_level: u8, pub current_load: f64, pub attested_rtt_ms: Option<u32>, pub household_id: String, pub bonded: bool, pub delivery_score: f64 }`
  - `pub struct ScoredPeer { pub agent_cid: String, pub score: f64 }`
  - `pub fn rank(candidates: &[Candidate], min_capability: u8) -> Vec<ScoredPeer>`

- [ ] **Step 1: Write the failing tests**

Append to `src/score.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn cand(cid: &str, cap: u8, load: f64, rtt: Option<u32>, hh: &str, bonded: bool, delivery: f64) -> Candidate {
        Candidate { agent_cid: cid.into(), capability_level: cap, current_load: load, attested_rtt_ms: rtt, household_id: hh.into(), bonded, delivery_score: delivery }
    }

    #[test]
    fn capability_floor_filters_out_incapable_peers() {
        let cs = vec![cand("low", 1, 0.1, Some(10), "h1", true, 1.0), cand("ok", 5, 0.1, Some(10), "h2", true, 1.0)];
        let r = rank(&cs, 3);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].agent_cid, "ok");
    }

    #[test]
    fn more_headroom_ranks_higher() {
        let cs = vec![cand("busy", 5, 0.9, Some(10), "h1", true, 1.0), cand("idle", 5, 0.1, Some(10), "h2", true, 1.0)];
        let r = rank(&cs, 0);
        assert_eq!(r[0].agent_cid, "idle", "the less-loaded peer should rank first");
    }

    #[test]
    fn saturated_peers_are_excluded_so_caller_can_shed() {
        let cs = vec![cand("full", 5, 1.0, Some(10), "h1", true, 1.0)];
        assert!(rank(&cs, 0).is_empty(), "load>=1.0 means no headroom → not a candidate → caller sheds");
    }

    #[test]
    fn unknown_rtt_degrades_gracefully_not_crashes() {
        let cs = vec![cand("nort", 5, 0.1, None, "h1", true, 0.5)];
        let r = rank(&cs, 0);
        assert_eq!(r.len(), 1, "a peer with no attested RTT is still rankable (neutral rtt factor)");
    }

    #[test]
    fn lower_rtt_ranks_higher_when_all_else_equal() {
        let cs = vec![cand("far", 5, 0.1, Some(300), "h1", true, 1.0), cand("near", 5, 0.1, Some(10), "h2", true, 1.0)];
        let r = rank(&cs, 0);
        assert_eq!(r[0].agent_cid, "near");
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/peer-fabric-target cargo test -p elohim-peer-fabric score`
Expected: FAIL to compile — `Candidate`, `rank`, etc. not defined.

- [ ] **Step 3: Write the minimal implementation**

Prepend to `src/score.rs`:

```rust
/// One candidate peer for serve-routing, composed from operational signals
/// (NodeRegistration capability, NodeHeartbeat load, HealthAttestation RTT, Mishpat bond, delivery history).
#[derive(Debug, Clone)]
pub struct Candidate {
    pub agent_cid: String,
    pub capability_level: u8,
    pub current_load: f64,          // 0.0..=1.0 (NodeHeartbeat.current_load)
    pub attested_rtt_ms: Option<u32>, // HealthAttestation.response_time_ms; None = not yet attested
    pub household_id: String,        // fault-domain key
    pub bonded: bool,                // backed by a replicates-* / delegates-compute commitment
    pub delivery_score: f64,         // 0.0..=1.0 decaying delivery-success (advertise-then-drop decays it)
}

/// A ranked peer. Higher `score` = preferred.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredPeer {
    pub agent_cid: String,
    pub score: f64,
}

/// Rank candidates that meet `min_capability` AND have headroom (`current_load < 1.0`), best-first.
/// Score = headroom*0.4 + rtt_factor*0.3 + delivery*0.2 + bond*0.1. Unknown RTT → neutral 0.5 (graceful
/// degradation: a not-yet-attested peer is rankable, not crashed and not unfairly penalized). An empty
/// result means "no peer has headroom" → the caller sheds (503), never fans out.
pub fn rank(candidates: &[Candidate], min_capability: u8) -> Vec<ScoredPeer> {
    let mut scored: Vec<ScoredPeer> = candidates
        .iter()
        .filter(|c| c.capability_level >= min_capability && c.current_load < 1.0)
        .map(|c| {
            let headroom = (1.0 - c.current_load).clamp(0.0, 1.0);
            let rtt_factor = match c.attested_rtt_ms {
                Some(ms) => 1.0 / (1.0 + ms as f64 / 100.0), // 0ms→1.0, 100ms→0.5, 300ms→0.25
                None => 0.5,
            };
            let bond_factor = if c.bonded { 1.0 } else { 0.5 };
            let delivery = c.delivery_score.clamp(0.0, 1.0);
            let score = headroom * 0.4 + rtt_factor * 0.3 + delivery * 0.2 + bond_factor * 0.1;
            ScoredPeer { agent_cid: c.agent_cid.clone(), score }
        })
        .collect();
    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scored
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/peer-fabric-target cargo test -p elohim-peer-fabric score`
Expected: PASS (5 tests).

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-peer-fabric/src/score.rs
git commit -m "feat(peer-fabric): score — capability/headroom/rtt/bond/delivery ranking with graceful degradation" \
  -- elohim/elohim-peer-fabric/src/score.rs
```

---

### Task 4: `score` — fault-domain-diverse selection

**Files:**
- Modify: `elohim/elohim-peer-fabric/src/score.rs`

**Interfaces:**
- Consumes: `rank()`, `Candidate`, `ScoredPeer` (Task 3).
- Produces: `pub fn select_diverse(candidates: &[Candidate], min_capability: u8, n: usize) -> Vec<ScoredPeer>`

- [ ] **Step 1: Write the failing tests**

Append inside the existing `mod tests` in `src/score.rs` (before its closing brace):

```rust
    #[test]
    fn diverse_selection_spreads_across_households() {
        // Two best-scored peers share household h1; the 3rd is in h2. Selecting 2 must include h2.
        let cs = vec![
            cand("h1a", 5, 0.05, Some(10), "h1", true, 1.0), // best
            cand("h1b", 5, 0.10, Some(10), "h1", true, 1.0), // 2nd best, same household
            cand("h2a", 5, 0.20, Some(10), "h2", true, 1.0), // 3rd best, different household
        ];
        let picked = select_diverse(&cs, 0, 2);
        let cids: Vec<&str> = picked.iter().map(|p| p.agent_cid.as_str()).collect();
        assert_eq!(cids[0], "h1a", "still takes the single best");
        assert_eq!(cids[1], "h2a", "second pick crosses to a new fault domain, not the same-household runner-up");
    }

    #[test]
    fn diverse_selection_falls_back_when_no_new_household() {
        // Only one household available — diversity can't be satisfied, so fill from best remaining.
        let cs = vec![cand("a", 5, 0.05, Some(10), "h1", true, 1.0), cand("b", 5, 0.10, Some(10), "h1", true, 1.0)];
        let picked = select_diverse(&cs, 0, 2);
        assert_eq!(picked.len(), 2, "no new household → fall back to best remaining rather than under-fill");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/peer-fabric-target cargo test -p elohim-peer-fabric score::tests::diverse`
Expected: FAIL to compile — `select_diverse` not defined.

- [ ] **Step 3: Write the minimal implementation**

Add to `src/score.rs` (after `rank`):

```rust
/// Pick up to `n` peers, greedily preferring the highest score while spreading across `household_id`
/// fault domains. When no unseen household remains, fall back to the best remaining peer (never under-fill
/// for diversity's sake — resilience beats purity once domains are exhausted). Needs the household key, so
/// it re-walks `candidates` rather than taking `rank()`'s output alone.
pub fn select_diverse(candidates: &[Candidate], min_capability: u8, n: usize) -> Vec<ScoredPeer> {
    let ranked = rank(candidates, min_capability);
    // Map agent_cid -> household for the diversity check.
    let household = |cid: &str| candidates.iter().find(|c| c.agent_cid == cid).map(|c| c.household_id.as_str());
    let mut seen_households: Vec<String> = Vec::new();
    let mut picked: Vec<ScoredPeer> = Vec::new();
    // Pass 1: greedily take peers from not-yet-seen households.
    for p in &ranked {
        if picked.len() >= n { break; }
        if let Some(hh) = household(&p.agent_cid) {
            if !seen_households.iter().any(|s| s == hh) {
                seen_households.push(hh.to_string());
                picked.push(p.clone());
            }
        }
    }
    // Pass 2: backfill from the best remaining (households exhausted) until n.
    if picked.len() < n {
        for p in &ranked {
            if picked.len() >= n { break; }
            if !picked.iter().any(|q| q.agent_cid == p.agent_cid) {
                picked.push(p.clone());
            }
        }
    }
    picked
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/peer-fabric-target cargo test -p elohim-peer-fabric score`
Expected: PASS (7 tests total).

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-peer-fabric/src/score.rs
git commit -m "feat(peer-fabric): score — fault-domain-diverse selection with best-remaining fallback" \
  -- elohim/elohim-peer-fabric/src/score.rs
```

---

### Task 5: Crate docs + final gates

**Files:**
- Create: `elohim/elohim-peer-fabric/CLAUDE.md`

**Interfaces:**
- Consumes: the whole crate (Tasks 1–4).
- Produces: nothing new — this task is the gate + the consumer-facing contract doc.

- [ ] **Step 1: Write the crate CLAUDE.md**

```markdown
# elohim-peer-fabric — shared peer-traffic spine (pure logic)

Write-once defense + ranking logic consumed by BOTH `doorway-service` and `elohim-storage`, feature-gated
per node role. Pattern mirrors `elohim-facings`: **pure logic, no diesel** — the dependency graph is the
boundary (a `use diesel;` here won't compile; that compile-failure IS the enforcement).

- `guard`: `assess(store, clock, cfg, source) -> Verdict` (Allow/Shape/Challenge/Deny + ban). Runtimes
  implement `GuardStore` (SQLite for storage; in-memory/edge for doorway) + `Clock`.
- `score`: `rank(candidates, min_capability)` and `select_diverse(..)` — capability×headroom×attested-RTT×
  delivery×bond ranking with graceful degradation (unknown RTT → neutral; all-saturated → empty ⇒ caller sheds).

**Features (node role):** `edge-defense` (doorway guard), `peer-defense` (storage guard), `serve-routing`
(storage score), `identity-routing` (doorway conductor-axis, fast-follow).

Spec: `genesis/docs/superpowers/specs/2026-06-20-doorway-membrane-prosocial-routing-design.md`.
Do NOT add I/O deps here (no diesel/serde/tokio) — keep it pure.
```

- [ ] **Step 2: Run fmt + clippy (the crate must be clean)**

Run: `RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/peer-fabric-target cargo fmt -p elohim-peer-fabric --check`
Expected: PASS (no diff). If it fails, run without `--check` to fix, then re-run.

Run: `RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/peer-fabric-target cargo clippy -p elohim-peer-fabric --all-targets -- -D warnings`
Expected: PASS (zero warnings).

- [ ] **Step 3: Run the full crate test suite**

Run: `RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/peer-fabric-target cargo test -p elohim-peer-fabric`
Expected: PASS (9 tests: 4 guard + 5 score; the 2 diversity tests are within score's 7).

- [ ] **Step 4: Confirm the purity boundary one more time (no accidental impure dep crept in)**

Run: `RUSTFLAGS="" RUSTC_WRAPPER="" CARGO_TARGET_DIR=/tmp/peer-fabric-target cargo tree -p elohim-peer-fabric`
Expected: the tree shows `elohim-peer-fabric v0.1.0` with NO `diesel`, `tokio`, `elohim-storage`, or other I/O crate. If any appears, a dep leaked — remove it.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-peer-fabric/CLAUDE.md
git commit -m "docs(peer-fabric): add crate CLAUDE.md (purity boundary + consumer contract)" \
  -- elohim/elohim-peer-fabric/CLAUDE.md
```

---

## Out of scope (follow-on plans — do NOT build here)

- **Doorway membrane policy stage** (consumes `guard`): a new stage in `handle_request` + an in-memory `GuardStore` + an `is_service_path`-style unit test. Separate plan.
- **Storage serve-routing** (consumes `score`): extend `services/distribution_view.rs` to call `rank`/`select_diverse`; a SQLite `OperationalView`/`GuardStore`. Separate plan.
- **Self-heal loop, recognition epoch rollups, the toll/`bridges/fiat` layer, cross-WAN/DNS legs** — later waves / design-only per the spec's §8.

## Self-Review

- **Spec coverage:** this plan implements the spec's §2 `elohim-peer-fabric` crate (`guard` + `score`) and the §4·4 (shared defense state, as the in-memory test store) — the buildable-now spine. It deliberately does NOT cover the doorway/storage integration, self-heal loop, recognition, or toll layer (flagged out-of-scope as follow-on plans, matching §8's wave split). No spec requirement in this slice is unimplemented.
- **Placeholder scan:** every code step contains complete, compiling Rust; every command has expected output. No TBD/TODO.
- **Type consistency:** `Verdict`, `GuardStore`, `Clock`, `GuardConfig`, `assess` (Task 2) and `Candidate`, `ScoredPeer`, `rank`, `select_diverse` (Tasks 3–4) are used consistently; `select_diverse` consumes `rank` and the `Candidate.household_id` field defined in Task 3.
