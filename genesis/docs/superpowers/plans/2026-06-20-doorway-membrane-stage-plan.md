---
title: "Doorway membrane policy stage — first consumer of elohim-peer-fabric guard (Wave-2 of the Doorway Membrane arc)"
id: doorway-membrane-stage-plan
status: Draft
class: protocol-canonical
domain: D8
sprint: doorway-membrane-wave-b
cites:
  - doorway-membrane-prosocial-routing-design | the arc spec this plan implements (Wave-2: §2 unit-2 membrane policy stage, the first guard consumer) | sha256:560686edf977447a | path: genesis/docs/superpowers/specs/2026-06-20-doorway-membrane-prosocial-routing-design.md
  - elohim-peer-fabric-spine-plan | the Wave-1 plan that built the guard crate this stage consumes | sha256:dd4fe05da91829de | path: genesis/docs/superpowers/plans/2026-06-20-elohim-peer-fabric-spine-plan.md
refines: genesis/docs/superpowers/specs/2026-06-20-doorway-membrane-prosocial-routing-design.md
# Single-host buildable: unit tests (derive_source, EdgeGuardStore, verdict-mapping, shadow) are DB-free.
# Execution is DISK-GATED: doorway-service is a multi-GB native build; the 85% hard ceiling denies it
# regardless of CARGO_TARGET_DIR. Needs real disk reclaim (operator), not the temp-bump trick.
---

# Doorway Membrane Policy Stage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a membrane policy stage to `doorway-service`'s `handle_request` that calls `elohim-peer-fabric`'s `guard::assess` and applies the `Verdict` (Allow / Shape / Challenge / Deny) **before any peer is touched** — the first runtime consumer of the shared `guard` crate.

**Architecture:** A new stage `apply_membrane(...) -> Option<Response<BoxBody>>` inserted at the top of `handle_request`, between the wisdom gate and the inbound admission semaphore. It derives a per-source key (`agent:<agent_cid>` authed, else `ip:<client_ip>`), assesses via an in-memory `EdgeGuardStore` (implementing the crate's `GuardStore` trait, with windowing + idle eviction the crate leaves to the implementer), and short-circuits Deny/Challenge or sleeps-then-continues on Shape. Shadow-proof by position (runs before the EPR router). Reuses `admission_exempt` for the liveness/scrape/WS bypass.

**Tech Stack:** Rust (native, `RUSTFLAGS=""`), hyper, `elohim-peer-fabric` (path dep, `edge-defense` feature), prometheus metrics.

## Global Constraints

- **Native build env:** `RUSTFLAGS=""` (mandatory — the ambient Holochain WASM `getrandom_backend="custom"` flag breaks the native link). `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/frontend/doorway__doorway-service/dev` (the frontend-family pool slot for this worktree — avoids the fingerprint-ENOENT + missing-target-dir denials). `RUSTC_WRAPPER=""` if sccache misbehaves.
- **🚨 DISK GATE (execution):** the 85% hard ceiling (`genesis/agentic/pool-policy.json` `volume_hard_pct`) DENIES heavy cargo **regardless of `CARGO_TARGET_DIR`**. `doorway-service` is a multi-GB build — the temp-bump trick is NOT safe here (a cold build can exceed the headroom). Execution requires the operator to reclaim non-pool disk below 85% first. Run long cargo with `run_in_background` (>10min) so a Bash timeout can't orphan the `.cargo-lock`; **one profile per gate phase**; batch the final gate suite once.
- **`doorway-service` is its OWN crate** (NOT in the `elohim/` workspace) — it depends on elohim crates via `path` only. Run cargo from `doorway/doorway-service/`.
- **Commit discipline (shared worktree + concurrent committer):** commit path-limited (`git add <paths>` then `git commit -m "…" -- <paths>`), NEVER `git add -A`/`.`, NEVER `--amend` without re-checking HEAD, NEVER `git push`. See memory `concurrent-sessions-shared-worktree`.
- **No new DHT entry types / no DB / no HTTP-route additions** — the membrane is a request-processing stage over operational (Category-C) in-memory state only (spec §4 entity 5).
- **Source key never raw-compares identity namespaces:** `agent:<human_id>` (the JWT `claims.human_id`, the same value forwarded to storage as `X-Agent-Cid`) vs `ip:<client_ip>`; the prefix is the namespace separator (spec §1 identity-coherence).

## Locked design decisions (from Wave-2 recon — confirm before/while executing)

1. **XFF source derivation (BLOCKING — `addr.ip()` is the ingress pod IP, so it would collapse all anonymous clients to one source and ban them en masse).** Parse `X-Forwarded-For` with a configurable trusted-proxy hop count `DOORWAY_TRUSTED_PROXY_HOPS` (default `1`, the alpha single-ingress topology): take the rightmost-untrusted entry as the client IP; fall back to `addr.ip()` only when no XFF header is present (direct-dial / served-THROUGH-not-BY). Authed-only guarding is NOT an acceptable fallback (anon floods are the primary threat).
2. **Edge thresholds (BLOCKING — `GuardConfig::default()` shape=20 would shape real SPA first-loads of 20–60+ asset requests).** Use a page-load-aware edge config AND exempt static-asset paths from the membrane. **Calibrate the numbers against a measured page-load request count** (Task 6 measures it via `pnpm look` `capture.json`) before locking — do not ship a guessed threshold. Interim starting point: `{window 60s, shape 300, challenge 600, ban 1200, ban_secs 900, shape_delay_ms 250}`.
3. **Status codes:** `Deny → 403` (forbidden source, mirrors the gate-decline precedent) + `x-membrane: deny`; `Challenge → 429` (retryable rate-class) + `Retry-After` + `x-membrane: challenge`. (Crate/spec prescribe none; locked here with per-code tests.)
4. **Challenge PoW is a fast-follow** — `Verdict::Challenge` carries no payload in v1; the stage returns 429 + header. Full PoW interstitial is deferred.

---

### Task 1: Add the crate dependency (no behavior yet)

**Files:**
- Modify: `doorway/doorway-service/Cargo.toml` (near the `gate-client` path dep, ~line 150)

**Interfaces:**
- Produces: `elohim_peer_fabric::guard::{assess, Verdict, Clock, GuardStore, GuardConfig}` available in the crate.

- [ ] **Step 1: Add the path dependency**

Add under `[dependencies]`:
```toml
elohim-peer-fabric = { path = "../../elohim/elohim-peer-fabric", default-features = false, features = ["edge-defense"] }
```
(`edge-defense` gates the `guard` module in the crate's `lib.rs`; the crate is std-only — no transitive deps.)

- [ ] **Step 2: Verify it resolves + nothing else broke**

Run (DISK-GATED — needs <85%): `cd doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/frontend/doorway__doorway-service/dev cargo check`
Expected: PASS. Add a throwaway `use elohim_peer_fabric::guard::Verdict;` in `src/server/http.rs`, confirm it resolves, then remove it.

- [ ] **Step 3: Commit**
```bash
git add doorway/doorway-service/Cargo.toml
git commit -m "feat(doorway): depend on elohim-peer-fabric (edge-defense feature) for the membrane stage" -- doorway/doorway-service/Cargo.toml
```

---

### Task 2: Source-key derivation incl. X-Forwarded-For (pure logic — unit-testable without the heavy build)

**Files:**
- Create: `doorway/doorway-service/src/server/membrane.rs`
- Modify: `doorway/doorway-service/src/server/mod.rs` (add `pub mod membrane;`)
- Modify: `doorway/doorway-service/src/main.rs` (parse `DOORWAY_TRUSTED_PROXY_HOPS`)

**Interfaces:**
- Produces:
  - `pub fn client_ip_from_xff(xff: Option<&str>, peer: std::net::IpAddr, trusted_hops: usize) -> std::net::IpAddr`
  - `pub fn derive_source(authed_cid: Option<&str>, client_ip: std::net::IpAddr) -> String`

- [ ] **Step 1: Write the failing tests** (in `membrane.rs`)
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn ip(s: &str) -> IpAddr { s.parse().unwrap() }

    #[test]
    fn xff_picks_rightmost_untrusted_with_one_trusted_hop() {
        // client, ingress(trusted). hops=1 → drop 1 from the right → client.
        let got = client_ip_from_xff(Some("203.0.113.7, 10.0.0.1"), ip("10.0.0.1"), 1);
        assert_eq!(got, ip("203.0.113.7"));
    }
    #[test]
    fn xff_absent_falls_back_to_peer_addr() {
        let got = client_ip_from_xff(None, ip("198.51.100.9"), 1);
        assert_eq!(got, ip("198.51.100.9"));
    }
    #[test]
    fn xff_too_short_for_hops_falls_back_to_peer() {
        // hops=1 but only one entry → no untrusted client beyond the trusted hop → peer.
        let got = client_ip_from_xff(Some("10.0.0.1"), ip("10.0.0.1"), 1);
        assert_eq!(got, ip("10.0.0.1"));
    }
    #[test]
    fn source_prefixes_separate_namespaces() {
        assert_eq!(derive_source(Some("humanX"), ip("203.0.113.7")), "agent:humanX");
        assert_eq!(derive_source(None, ip("203.0.113.7")), "ip:203.0.113.7");
    }
}
```

- [ ] **Step 2: Run (no heavy build needed — unit test the module)**

Run: `cd doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/frontend/doorway__doorway-service/dev cargo test --lib membrane`
Expected: FAIL to compile (functions undefined).

- [ ] **Step 3: Implement** (`membrane.rs`)
```rust
//! Doorway edge membrane — the guard consumer + source-key derivation + in-memory store.
use std::net::IpAddr;

/// Resolve the real client IP behind `trusted_hops` trusted proxies (default 1 = single ingress).
/// XFF is "client, proxy1, proxy2, …"; the rightmost `trusted_hops` entries are our own proxies,
/// so the client is the entry just left of them. Falls back to the direct peer addr when XFF is
/// absent or too short (direct-dial / served-THROUGH-not-BY).
pub fn client_ip_from_xff(xff: Option<&str>, peer: IpAddr, trusted_hops: usize) -> IpAddr {
    let Some(xff) = xff else { return peer };
    let parts: Vec<&str> = xff.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    // Need at least trusted_hops trusted entries on the right PLUS one untrusted client to the left.
    if parts.len() <= trusted_hops {
        return peer;
    }
    let idx = parts.len() - trusted_hops - 1;
    parts[idx].parse::<IpAddr>().unwrap_or(peer)
}

/// The guard source key. `agent:<cid>` when authenticated (the JWT human_id), else `ip:<client>`.
/// The prefix is the namespace separator that prevents raw cross-namespace comparison.
pub fn derive_source(authed_cid: Option<&str>, client_ip: IpAddr) -> String {
    match authed_cid {
        Some(cid) => format!("agent:{cid}"),
        None => format!("ip:{client_ip}"),
    }
}
```
Add `pub mod membrane;` to `src/server/mod.rs`. In `main.rs`, parse `DOORWAY_TRUSTED_PROXY_HOPS` (default 1, mirror the `DOORWAY_MAX_INFLIGHT`→`inbound_max()` env pattern) and thread it onto `AppState` (added in Task 5) or `state.args`.

- [ ] **Step 4: Run tests** → PASS (4 tests).
- [ ] **Step 5: Commit** (`git commit -m "feat(doorway): membrane source-key derivation + X-Forwarded-For client-IP resolution" -- doorway/doorway-service/src/server/membrane.rs doorway/doorway-service/src/server/mod.rs doorway/doorway-service/src/main.rs`)

---

### Task 3: `EdgeGuardStore` + `EdgeClock` (evicting in-memory store)

**Files:**
- Modify: `doorway/doorway-service/src/server/membrane.rs`

**Interfaces:**
- Consumes: `elohim_peer_fabric::guard::{GuardStore, Clock}`.
- Produces: `pub struct EdgeGuardStore { window_secs: u64 }` impl `GuardStore`; `pub struct EdgeClock;` impl `Clock`; `EdgeGuardStore::sweep_idle(&mut self, now: u64)`.

- [ ] **Step 1: Write the failing tests** — assert: in-window prune on `record`; `count_since`/`is_banned`/`ban_until`; **idle-source eviction shrinks the map** after window + ban expiry (the crate's test `MemStore` never evicts — it must NOT be copied).
```rust
    #[test]
    fn records_prune_out_of_window_and_idle_sources_evict() {
        let mut s = EdgeGuardStore::new(60);
        s.record("ip:a", 1000);
        s.record("ip:a", 1000);
        assert_eq!(s.count_since("ip:a", 950), 2);
        // a later record prunes entries older than window (now-60):
        s.record("ip:a", 2000);
        assert_eq!(s.count_since("ip:a", 0), 1, "out-of-window hits pruned");
        // idle eviction: source with all-out-of-window hits + no live ban is dropped.
        s.sweep_idle(5000);
        assert_eq!(s.source_count(), 0, "idle source evicted (no unbounded growth)");
    }
    #[test]
    fn ban_lifecycle() {
        let mut s = EdgeGuardStore::new(60);
        s.ban_until("ip:b", 3000);
        assert!(s.is_banned("ip:b", 2999));
        assert!(!s.is_banned("ip:b", 3001));
    }
```

- [ ] **Step 2: Run** → FAIL (undefined).
- [ ] **Step 3: Implement**
```rust
use std::collections::{HashMap, VecDeque};
use elohim_peer_fabric::guard::{Clock, GuardStore};

pub struct EdgeClock;
impl Clock for EdgeClock {
    fn now_secs(&self) -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
    }
}

pub struct EdgeGuardStore {
    window_secs: u64,
    hits: HashMap<String, VecDeque<u64>>,
    bans: HashMap<String, u64>,
}
impl EdgeGuardStore {
    pub fn new(window_secs: u64) -> Self { Self { window_secs, hits: HashMap::new(), bans: HashMap::new() } }
    pub fn source_count(&self) -> usize { self.hits.len() }
    /// Drop sources whose hits are all out-of-window AND whose ban (if any) has expired.
    pub fn sweep_idle(&mut self, now: u64) {
        let cutoff = now.saturating_sub(self.window_secs);
        self.hits.retain(|src, dq| {
            while dq.front().is_some_and(|&t| t < cutoff) { dq.pop_front(); }
            !dq.is_empty() || self.bans.get(src).is_some_and(|&u| u > now)
        });
        self.bans.retain(|_, &mut until| until > now);
    }
}
impl GuardStore for EdgeGuardStore {
    fn record(&mut self, source: &str, ts_secs: u64) {
        let cutoff = ts_secs.saturating_sub(self.window_secs);
        let dq = self.hits.entry(source.to_string()).or_default();
        while dq.front().is_some_and(|&t| t < cutoff) { dq.pop_front(); }
        dq.push_back(ts_secs);
    }
    fn count_since(&self, source: &str, since_secs: u64) -> u32 {
        self.hits.get(source).map_or(0, |dq| dq.iter().filter(|&&t| t >= since_secs).count() as u32)
    }
    fn is_banned(&self, source: &str, now_secs: u64) -> bool {
        self.bans.get(source).is_some_and(|&until| until > now_secs)
    }
    fn ban_until(&mut self, source: &str, until_secs: u64) { self.bans.insert(source.to_string(), until_secs); }
}
```
- [ ] **Step 4: Run tests** → PASS.
- [ ] **Step 5: Commit** (path-limited to `membrane.rs`).

---

### Task 4: `apply_membrane` stage + verdict→response mapping

**Files:**
- Modify: `doorway/doorway-service/src/server/membrane.rs` (the stage)
- Modify: `doorway/doorway-service/src/server/http.rs` (verdict response builders if not inlined)

**Interfaces:**
- Consumes: `guard::assess`, `EdgeGuardStore`, `EdgeClock`, `derive_source`, `resolve_agent_cid_from_request` (`http.rs:958`), `admission_exempt` (`http.rs:3942`), `to_boxed` (`http.rs:3887`).
- Produces: `pub async fn apply_membrane(state: &AppState, peer: SocketAddr, req: &Request<Incoming>, path: &str, is_upgrade: bool) -> Option<Response<BoxBody>>`.

- [ ] **Step 1: Write the failing tests** — verdict→response mapping over a hand-driven `EdgeGuardStore` + the edge `GuardConfig`: a fresh source → `Allow` (None-equivalent); a source pre-driven past `ban_threshold` → `Deny` → `Some(403)` with `x-membrane: deny`; past `challenge_threshold` → `Challenge` → `Some(429)` with `Retry-After` + `x-membrane: challenge`; past `shape_threshold` → `Shape` (falls through after delay). Assert the lock is dropped before the `Shape` sleep (structural — assert `Shape` path returns None and does not hold the guard across await; verify by code review + a test that two concurrent Shape assessments don't deadlock).

- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement** the stage:
```rust
pub async fn apply_membrane(
    state: &AppState, peer: std::net::SocketAddr, req: &Request<Incoming>, path: &str, is_upgrade: bool,
) -> Option<Response<BoxBody>> {
    if admission_exempt(path, is_upgrade) || is_static_asset(path) { return None; } // bypass: liveness/scrape/WS + static assets (Task 6)
    let authed = resolve_agent_cid_from_request(state, req);
    let xff = req.headers().get("x-forwarded-for").and_then(|v| v.to_str().ok());
    let client_ip = client_ip_from_xff(xff, peer.ip(), state.trusted_proxy_hops);
    let source = derive_source(authed.as_deref(), client_ip);
    let verdict = {
        let mut g = state.membrane_guard.lock().unwrap_or_else(|e| e.into_inner()); // fail-open on poison
        elohim_peer_fabric::guard::assess(&mut *g, &EdgeClock, &state.membrane_cfg, &source)
    }; // lock dropped here, before any await
    match verdict {
        elohim_peer_fabric::guard::Verdict::Allow => { metrics::inc_membrane_verdict("allow"); None }
        elohim_peer_fabric::guard::Verdict::Shape { delay_ms } => {
            metrics::inc_membrane_verdict("shape");
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            None
        }
        elohim_peer_fabric::guard::Verdict::Challenge => {
            metrics::inc_membrane_verdict("challenge");
            Some(to_boxed(Response::builder().status(StatusCode::TOO_MANY_REQUESTS)
                .header("retry-after", "5").header("x-membrane", "challenge").body(empty_body()).unwrap()))
        }
        elohim_peer_fabric::guard::Verdict::Deny => {
            metrics::inc_membrane_verdict("deny");
            Some(to_boxed(Response::builder().status(StatusCode::FORBIDDEN)
                .header("x-membrane", "deny").body(empty_body()).unwrap()))
        }
    }
}
```
(Mirror `catching_up_response` `http.rs:3962` for the response-builder body/`to_boxed` idiom; `metrics::inc_membrane_verdict` lands in Task 7 — stub it as a no-op fn first or land Task 7 before this task's gate.)

- [ ] **Step 4: Run tests** → PASS. **Step 5: Commit** path-limited.

---

### Task 5: Wire into `handle_request` + `AppState`

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` (AppState field `:116`/`:291`; the `is_upgrade` move + stage call at `:2283`; the 4 ctor sites `:492,592,707,825`)

**Interfaces:**
- Consumes: `apply_membrane`, `EdgeGuardStore`, `GuardConfig`.
- Produces: `AppState.membrane_guard: Arc<std::sync::Mutex<EdgeGuardStore>>`, `AppState.membrane_cfg: GuardConfig`, `AppState.trusted_proxy_hops: usize`.

- [ ] **Step 1: Write the failing test** — the shadow-proofness test (spec §9): for a source pre-driven past `ban_threshold`, `assess` reaches `Deny` for BOTH `/api/v1/content/x` (`is_service_path==true`) AND `/lamad` (`is_service_path==false`); assert `admission_exempt(p,false)==false` for both, and `admission_exempt("/metrics",false)==true` + an upgrade is exempt (the bypass class is NOT guarded). Co-locate with the `is_service_path`/`admission_exempt` tests (`http.rs:1682`, `:4708`).
- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement:**
  - Add the 3 fields to `AppState` (`:116`, near `inbound_semaphore` `:291`).
  - Init them at all FOUR ctor sites (`:492,592,707,825`): `membrane_guard: Arc::new(std::sync::Mutex::new(EdgeGuardStore::new(cfg.window_secs)))`, `membrane_cfg: <edge cfg from Task 6>`, `trusted_proxy_hops: <from env>`.
  - Move `let is_upgrade = …` from `:2290` up to `~:2283` (above the insertion).
  - Insert after the wisdom-gate return (`:2281`): `if let Some(r) = membrane::apply_membrane(&state, addr, &req, &path, is_upgrade).await { return Ok(r); }`.
- [ ] **Step 4: Run** the shadow test + the existing `handle_request`/`admission_exempt` tests → all green. **Step 5: Commit** path-limited to `http.rs`.

---

### Task 6: Edge `GuardConfig` + env + page-load calibration + static-asset exemption

**Files:**
- Modify: `doorway/doorway-service/src/main.rs` (env parse), `src/server/membrane.rs` (`is_static_asset`, edge cfg builder)

- [ ] **Step 1: Calibrate** — render a real first page-load and COUNT requests (don't guess the threshold):
  Run from `genesis/a2o`: `pnpm look https://doorway-alpha.elohim.host` → read `reports/look/<slug>/capture.json` and count distinct requests in the first window. Set `shape_threshold` comfortably above that count (× a navigation-burst margin).
- [ ] **Step 2: Write tests** — `is_static_asset(".../main.js")==true`, `.css/.woff2/.png` true, `/api/v1/...`/`/lamad` false; edge `GuardConfig` parses from `DOORWAY_MEMBRANE_{WINDOW_SECS,SHAPE,CHALLENGE,BAN,BAN_SECS,SHAPE_DELAY_MS}` with the calibrated defaults (≠ crate default).
- [ ] **Step 3: Implement** `is_static_asset` (extension + SPA-bootstrap-asset check) and the env-driven edge `GuardConfig` builder (mirror `inbound_max()` `main.rs:64`). Wire the cfg into the ctors (Task 5 fields).
- [ ] **Step 4: Run** → PASS. **Step 5: Commit** path-limited.

---

### Task 7: Guard metrics on `/metrics`

**Files:**
- Modify: `doorway/doorway-service/src/metrics.rs` (`:84` vec, `:194` gauge, `:226` register, `:313` helpers)

- [ ] **Step 1: Write tests** — `inc_membrane_verdict("deny")` increments `doorway_membrane_verdict_total{verdict="deny"}` (delta assertion, the `metrics.rs:336` convention); `set_membrane_bans_active(n)` sets `doorway_membrane_bans_active`.
- [ ] **Step 2: Run** → FAIL.
- [ ] **Step 3: Implement** (mirror `CONDUCTOR_RECONNECT_TOTAL: IntCounterVec` `metrics.rs:84` + `INBOUND_MAX_INFLIGHT: IntGauge` `metrics.rs:194`): `doorway_membrane_verdict_total` (`IntCounterVec`, label `verdict`), `doorway_membrane_bans_active` (`IntGauge`); register both in the `Once` block (after `:226`); add helpers `inc_membrane_verdict(&str)` + `set_membrane_bans_active(i64)` near `inc_admission_shed()` (`:313`). Update the bans gauge from the idle sweep (Task 3) / verdict site.
- [ ] **Step 4: Run** → PASS. **Step 5: Commit** path-limited.

---

### Final gate (batch once, after Tasks 5–7 — amortize the disk-gated build)

- [ ] Run (DISK-GATED, `run_in_background`): `cd doorway/doorway-service && RUSTFLAGS="" CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/frontend/doorway__doorway-service/dev sh -c 'cargo build --release && cargo test --lib --bins && cargo clippy -- -D warnings && cargo fmt --check'`
- [ ] Expected: clean build, all membrane tests + existing tests green, clippy 0, fmt clean.

## Out of scope (later waves)

- Storage **serve-routing** (consumes `score`), the **self-heal loop** + recognition epoch-rollups, the **toll/`bridges/fiat`** layer, and the cross-WAN/DNS legs — each its own spec-gap → plan → sprint (spec §8).
- Full PoW **Challenge interstitial** (v1 = 429 + header).

## Self-Review

- **Spec coverage:** implements spec §2 unit 2 (membrane policy stage), §4 entity 5 (operational, no DHT entity), §5 read-path membrane step, §6 fail-open posture (lock-poison `into_inner`), §9 the `is_service_path`-style shadow test. The XFF/threshold/asset-exemption work is net-new (recon-surfaced, spec-consistent), flagged as locked decisions.
- **Placeholder scan:** every code step is complete; integration steps give exact line anchors + the code to insert + the pattern to mirror (the implementer reads the surrounding context of the 4803-line `http.rs`). Task 6's threshold numbers are deliberately calibration-gated (measure-then-lock), not guessed.
- **Type consistency:** `EdgeGuardStore`/`EdgeClock` impl the crate's `GuardStore`/`Clock`; `apply_membrane` returns `Option<Response<BoxBody>>` (mirrors `apply_gate_check`); `derive_source`/`client_ip_from_xff` signatures are consistent across Tasks 2/4; metrics helpers consistent across Tasks 4/7.
- **Scope:** one consumer (doorway/guard) only; storage/score and the rest are out-of-scope.
