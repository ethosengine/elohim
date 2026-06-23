# Dataplane — Transport (libp2p no-overwhelm floor + iroh cutover decision) — Implementation Plan

> For agentic workers: REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Steps use checkbox (- [ ]) syntax. Working draft — NOT cite-sealed.

**Goal:** Close the structural no-overwhelm FLOOR hole (finding #3): `libp2p::connection_limits::Behaviour` is absent from BOTH production swarms (`elohim-storage` and `steward/node`), so raw `established`/`pending` connection count is uncapped beneath every app-layer defense. Cap it in both swarms with role-aware, config-driven limits. Then record the iroh dual-transport endgame as a dated decision (finding #6): commit to a blob-plane-first cutover; do NOT freeze — the complementarity spec already makes dual-stack permanent for 7 planes, so a freeze cannot stop the parity tax, only the blob-canonical cutover can.

**Architecture:** The cure is purely additive: one new behaviour field per swarm struct (`connection_limits`), one `From<Void>` event impl + one ignore-arm per swarm event loop, and disjoint `#[serde(default)]` config fields supplying role-aware defaults. `connection_limits` is the layer BELOW Plan B's per-request HTTP admission shed (already shipped on `feat`): connection_limits caps libp2p sockets before any HTTP request exists; the two are orthogonal and compose. No new shared type — `libp2p::connection_limits::{Behaviour, ConnectionLimits}` is the upstream type (`libp2p-connection-limits 0.4.0`, already resolved transitively in BOTH Cargo.lock files; the gap is that the `connection-limits` FEATURE is not enabled in either `Cargo.toml`).

**Tech Stack:** Rust (`elohim-storage` WASM-flagged crate; `steward/node` native crate), libp2p 0.54.1, serde. No new external deps (feature-flag flip only). TDD: inline `#[cfg(test)]` unit tests assert config-default sanity; the behaviour wiring is verified by compilation + the whole-crate gate (a live connection-flood is an integration/soak concern, owned by P-PROOFS' `no_overwhelm_soak`).

**Findings closed:** #3 (connection_limits absent from production swarm — CONFIRMED) and #6 (iroh Phase-status/endgame — recorded as dated decision doc).

---

## CORRECTIONS TO THE CONTRACT LEDGER (read before integration)

The ledger file-ownership map lists for P-TRANSPORT: `M elohim/elohim-storage/src/config.rs (limit fields)`. **This is WRONG.** The storage `P2PConfig` struct lives in `elohim/elohim-storage/src/p2p/mod.rs:374` (verified), NOT `config.rs`. The limit fields land on that struct + its `Default` impl (`mod.rs:434`). This means **P-TRANSPORT's storage-config edit collides with P-RECONCILE's `run()`/`P2PCommand` rewrite IN THE SAME FILE** (`p2p/mod.rs`), and RESOLUTION-B (mod.rs sequencing) must cover this edit too, not just the event-arm edit. See SEAM-DELTA at the end. (RESOLUTION-C's "config.rs 3-way merge" does NOT apply to storage limits — only the node's `config.rs` is touched there.)

---

## Canonical names (MUST match across all tasks)

| Name | Kind | Home | Shape / value |
|---|---|---|---|
| `connection_limits` | field | `ElohimStorageBehaviour` (`behaviour.rs`) | `pub connection_limits: libp2p::connection_limits::Behaviour` |
| `connection_limits` | field | `ElohimBehaviour` (`steward/node/.../transport.rs`) | `pub connection_limits: libp2p::connection_limits::Behaviour` |
| `ConnectionLimits` event | variant | each `*BehaviourEvent` enum | `ConnectionLimits(void::Void)` (`type ToSwarm = Void`; never constructed at runtime) |
| `max_established` | config field | storage `P2PConfig` (`p2p/mod.rs`) + node `P2PConfig` (`config.rs`) | `Option<u32>` `#[serde(default = "default_max_established")]` |
| `max_established_per_peer` | config field | same two structs | `Option<u32>` `#[serde(default = "default_max_established_per_peer")]` |
| `max_pending` | config field | same two structs | `Option<u32>` `#[serde(default = "default_max_pending")]` |
| `default_max_established` | fn | each crate, role-aware | storage (relay/`Both`/`Server`) → `Some(512)`; node (`Client`) → `Some(128)` |
| `default_max_established_per_peer` | fn | each crate | `Some(8)` (blunt single-peer floods) |
| `default_max_pending` | fn | each crate | `Some(64)` |
| `connection_limits_from_config` | fn | each crate's behaviour builder | maps the 3 fields → `ConnectionLimits::default().with_max_established(..).with_max_established_per_peer(..).with_max_pending_incoming(..).with_max_pending_outgoing(..)` |

**Why `Option<u32>` not `u32`:** `with_max_established(None)` = unlimited; an operator must be able to disable the cap per-role without recompiling. `#[serde(default)]` supplies the role-aware value; an explicit `null` in TOML/env yields `None` (uncapped).

---

## OWNED FILES (verbatim from the ledger file-ownership map, with the mod.rs correction)

This plan **creates or mutates ONLY**:

- M `elohim/elohim-storage/src/p2p/behaviour.rs` — add `connection_limits` field to `ElohimStorageBehaviour` + `ConnectionLimits` event variant to `ElohimStorageBehaviourEvent` + `From<Void>` impl + construct in `new()`.
- M `elohim/elohim-storage/src/p2p/mod.rs` — (a) the new behaviour-event ignore-arm in `handle_behaviour_event`; (b) the limit config fields on `P2PConfig` + `Default` impl (LEDGER SAYS config.rs — IT IS mod.rs; see correction above). **RESOLUTION-B applies: SEQUENCED AFTER P-RECONCILE's `run()`/`P2PCommand` structural rewrite.**
- M `elohim/elohim-storage/Cargo.toml` — add `"connection-limits"` to the libp2p feature list. **RESOLUTION-D: P-TRANSPORT SOLE owner of libp2p feature edits.**
- M `steward/node/src/p2p/transport.rs` — add `connection_limits` field to `ElohimBehaviour` + event variant + `From<Void>` impl + ignore-arm in the swarm loop + construct in the builder.
- M `steward/node/src/config.rs` — add the 3 limit fields to node `P2PConfig` + `Default` + `default_*` fns. **SOLE owner (RESOLUTION-C: node config is disjoint from storage).**
- M `steward/node/Cargo.toml` — add `"connection-limits"` to the libp2p feature list (RESOLUTION-D).
- C `genesis/docs/superpowers/specs/2026-06-14-iroh-cutover-decision.md` — the decision record for #6.

**Collision statement:** Aside from `elohim/elohim-storage/src/p2p/mod.rs` (shared with P-RECONCILE + P-DIAGNOSTIC under RESOLUTION-B — all three edits are SEQUENCED behind P-RECONCILE's structural rewrite; P-TRANSPORT's touches are one ignore-arm + additive config fields, mechanically rebaseable), this plan **touches no file owned by another plan**. It defines no cross-track type and consumes no actuator/reconciler/backoff interface.

---

## NEW PRIMITIVES THIS PLAN OWNS

Per the ledger single-owner table row **S14 (`connection_limits::Behaviour`, OWNER = upstream libp2p 0.54.1)**: this plan introduces NO new shared type. `libp2p::connection_limits::{Behaviour, ConnectionLimits}` is upstream. The per-crate `max_established*` / `max_pending` config fields and `default_*` fns are crate-LOCAL to each `P2PConfig` (NOT a shared `elohim-compute` type) — by ledger design ("Defaults live in each `P2PConfig`, NOT a new shared type"). No `elohim-compute/src/lib.rs` re-export block is added by this plan (RESOLUTION-E does not list P-TRANSPORT).

## CONSUMED PRIMITIVES (skip-if-present clause)

This plan **consumes none** of the cross-track shared primitives (S1–S13). It reads no `ActuationRefusal`, `Sweep`, `jittered`, or `P2PStatusInfo` field. Should a future revision want runtime-tunable limits, the skip-if-present rule would apply to P-RECONCILE's `P2PCommand`/`Sweep` surface — *"Before adding a ReconfigureLimits command, verify P-RECONCILE's `P2PCommand` enum + reply-`Result<(),ActuationRefusal>` shape already exists; consume it verbatim, do not define a parallel command"* — but **v1 ships STATIC config** (limits are a structural floor, not a tuning knob), so no consume happens now.

---

## DEPENDENCY EDGES (from the DAG)

- **P-TRANSPORT → P-RECONCILE — HARD (file-sequencing only).** Only the single `p2p/mod.rs` behaviour-event ignore-arm AND the additive `P2PConfig` field edit wait for P-RECONCILE's `run()` refactor to land (they rebase onto the rewritten file). ALL OTHER transport work (`behaviour.rs`, both `Cargo.toml`, node `transport.rs`, node `config.rs`, the iroh doc) is fully independent and runs in Wave 2 in parallel.
- **P-PROOFS → P-TRANSPORT — SOFT (inbound).** P-PROOFS' `no_overwhelm_soak` (`#[ignore]`) proves `connection_limits`; it goes red if absent. No code edge — P-PROOFS verify-onlys the config field names.
- Independent of P-ACTUATION, P-DEFENSE, P-DIAGNOSTIC, iroh.

This plan is a **Wave-2 leaf** (one inbound file-sequencing edge; no outbound hard edge).

---

## p2p-class of new entities

Per the shared grounding: the connection-limit config fields are **Cat-C node-local read-models** — no DHT entry, no table, no coordinator fn, no signal. They are boot/env operational knobs on `P2PConfig` (same class as the existing `request_timeout`, `kad_replication`). `connection_limits::Behaviour` is a pure libp2p swarm behaviour, not an elohim entity. No p2p-design-gate re-litigation needed (cite: shared-grounding "New runtime entities are Cat-C node-local read-models").

---

## Build / test commands (per-crate RUSTFLAGS + /tmp target + plain cargo test)

storage (`elohim-storage` — WASM getrandom flag REQUIRED):
```
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib p2p::config 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo build --lib 2>&1 | tail -40
```

steward/node (native — RUSTFLAGS MUST be empty):
```
cd /projects/elohim/steward/node && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/sn-test RUSTC_WRAPPER="" cargo test --lib config 2>&1 | tail -40
cd /projects/elohim/steward/node && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/sn-test RUSTC_WRAPPER="" cargo build --bins 2>&1 | tail -40
```

Final gate (both crates, fmt/clippy):
```
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo clippy --lib -- -D warnings 2>&1 | tail -40
cd /projects/elohim/elohim/elohim-storage && cargo fmt --check
cd /projects/elohim/steward/node && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/sn-test RUSTC_WRAPPER="" cargo clippy -- -D warnings 2>&1 | tail -40
cd /projects/elohim/steward/node && cargo fmt --check
```

Rules: `RUSTFLAGS='--cfg getrandom_backend="custom"'` for elohim-storage (WASM); `RUSTFLAGS=""` for steward/node (native — flag leak → `undefined __getrandom_v03_custom` at link); `RUSTC_WRAPPER=""` (sccache spawn-ENOENT); `/tmp` target dirs (fingerprint-ENOENT on pool slot); **plain `cargo test`, NEVER nextest**; never `&&`-pipe a gate exit code (use `2>&1 | tail -N`).

---

## TASK 1 — Enable the `connection-limits` libp2p feature (both crates)

Files:
- `elohim/elohim-storage/Cargo.toml:207-225` (libp2p feature list).
- `steward/node/Cargo.toml:16-32` (libp2p feature list).

The transitive `libp2p-connection-limits 0.4.0` is ALREADY in both `Cargo.lock` files (verified), but the umbrella `connection-limits` feature is not enabled, so `libp2p::connection_limits` is not in scope.

- [ ] Write the failing test FIRST — confirm the module is currently absent. Run (expect FAIL — module not found):
```
cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo build --lib 2>&1 | grep -c connection_limits
```
  Then add a throwaway `use libp2p::connection_limits::ConnectionLimits as _;` at the top of `behaviour.rs` and rebuild — expect `unresolved import libp2p::connection_limits` BEFORE the feature flip.
- [ ] Add `"connection-limits", # structural no-overwhelm floor (finding #3)` to the storage libp2p feature list (after the `"autonat",` line) and to the node list (after `"autonat",`).
- [ ] Run, expect the import resolves: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo build --lib 2>&1 | tail -20`. Remove the throwaway `use`.
- [ ] Commit:
```
git add elohim/elohim-storage/Cargo.toml steward/node/Cargo.toml
git commit -m "build(p2p): enable libp2p connection-limits feature in both swarms

Transitive dep already locked; feature was off so the module was out of
scope. Prereq for the structural no-overwhelm floor (finding #3).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 2 — Storage `P2PConfig`: limit fields + role-aware defaults (`p2p/mod.rs`)

Files:
- `elohim/elohim-storage/src/p2p/mod.rs:374` (`P2PConfig` struct) + `:434` (`Default` impl). **SEQUENCED AFTER P-RECONCILE per RESOLUTION-B** — rebase onto the rewritten file before editing.

- [ ] Write the failing test — append a `#[cfg(test)] mod connection_limit_config_tests` near the existing `P2PConfig` (or in the crate's config test module). Use the storage test command (`--lib p2p::config` filter — adjust the `mod` path so the filter matches):
```rust
#[cfg(test)]
mod connection_limit_config_tests {
    use super::*;
    #[test]
    fn storage_p2pconfig_caps_default_to_relay_sane_values() {
        let c = P2PConfig::default();
        // Storage runs as relay/Both/Server (edge pods) → higher established cap.
        assert_eq!(c.max_established, Some(512));
        assert_eq!(c.max_established_per_peer, Some(8), "blunt single-peer floods");
        assert_eq!(c.max_pending, Some(64));
    }
}
```
- [ ] Run, expect FAIL: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo test --lib connection_limit_config 2>&1 | tail -30` — expect `no field max_established on type P2PConfig`.
- [ ] Write minimal implementation — add to the `P2PConfig` struct (after `fetch_blob_parallelism`):
```rust
    /// Structural no-overwhelm floor (finding #3): max total established
    /// connections. `None` = unlimited. `with_max_established` on the libp2p
    /// connection_limits behaviour. Cat C node-local boot/env knob.
    #[serde(default = "default_max_established")]
    pub max_established: Option<u32>,
    /// Max established connections per single peer (blunts single-peer floods).
    #[serde(default = "default_max_established_per_peer")]
    pub max_established_per_peer: Option<u32>,
    /// Max pending (incoming + outgoing) connections being negotiated.
    #[serde(default = "default_max_pending")]
    pub max_pending: Option<u32>,
```
  Add the `default_*` fns above the struct (storage runs as relay/edge → higher cap):
```rust
fn default_max_established() -> Option<u32> {
    Some(512)
}
fn default_max_established_per_peer() -> Option<u32> {
    Some(8)
}
fn default_max_pending() -> Option<u32> {
    Some(64)
}
```
  Add to the `Default for P2PConfig` impl body (after `fetch_blob_parallelism: 3,`):
```rust
            max_established: default_max_established(),
            max_established_per_peer: default_max_established_per_peer(),
            max_pending: default_max_pending(),
```
- [ ] Run, expect PASS: same command.
- [ ] Commit:
```
git add elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(storage): connection-limit config fields on P2PConfig (relay defaults)

Cat C node-local boot/env knobs; serde-default role-aware values. Floor
beneath Plan B's per-request admission shed (finding #3).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 3 — Storage behaviour: `connection_limits` field + event variant + construct (`behaviour.rs`)

Files:
- `elohim/elohim-storage/src/p2p/behaviour.rs` — struct `ElohimStorageBehaviour` (field, after `gossipsub` at line 129); event enum `ElohimStorageBehaviourEvent` (variant, after `Gossipsub`); a `From<void::Void>` impl (after the gossipsub `From` impl at line 351); construct in `new()` (after gossipsub setup, before the `Self { ... }` at line 538).

The `#[derive(NetworkBehaviour)]` macro REQUIRES a `From<<Behaviour as NetworkBehaviour>::ToSwarm>` for the to-swarm enum. `connection_limits::Behaviour` has `type ToSwarm = void::Void`, so the variant carries `void::Void` and is never constructed at runtime — the `From<Void>` impl + the match arm both pattern-match the never type.

- [ ] Write the failing test — this is a wiring change with no pure unit; the FAIL is a compile error. First confirm absence: `grep -c connection_limits elohim/elohim-storage/src/p2p/behaviour.rs` → expect `0`. The behavioral failing-state is the `NetworkBehaviour` derive WITHOUT the field; after adding the field but BEFORE the `From<Void>` impl, the build fails with `the trait From<Void> is not implemented for ElohimStorageBehaviourEvent`.
- [ ] Write minimal implementation:
  - Add to `ElohimStorageBehaviour` struct (after the `gossipsub` field):
```rust
    /// Structural no-overwhelm floor (finding #3): caps raw established/pending
    /// connection count beneath every app-layer defense. Limits read from
    /// P2PConfig; `None` per field = unlimited.
    pub connection_limits: libp2p::connection_limits::Behaviour,
```
  - Add to `ElohimStorageBehaviourEvent` enum (after `Gossipsub(...)`):
```rust
    /// Connection-limits event. `type ToSwarm = void::Void`; never constructed
    /// at runtime (the behaviour denies over-limit connections internally).
    ConnectionLimits(void::Void),
```
  - Add the `From<void::Void>` impl (after the `From<gossipsub::Event>` impl):
```rust
impl From<void::Void> for ElohimStorageBehaviourEvent {
    fn from(event: void::Void) -> Self {
        Self::ConnectionLimits(event)
    }
}
```
  - In `ElohimStorageBehaviour::new()`, before the final `Self { ... }`, construct:
```rust
        // Structural no-overwhelm floor (finding #3).
        let connection_limits = libp2p::connection_limits::Behaviour::new(
            libp2p::connection_limits::ConnectionLimits::default()
                .with_max_established(config.max_established)
                .with_max_established_per_peer(config.max_established_per_peer)
                .with_max_pending_incoming(config.max_pending)
                .with_max_pending_outgoing(config.max_pending),
        );
```
  - Add `connection_limits,` to the returned `Self { ... }` field list.
  - **Trait-import note:** `with_max_established` etc. are inherent methods on `ConnectionLimits`; no extra `use`. `void::Void` is re-exported by libp2p as `libp2p::swarm::derive_prelude::Void`; if the bare `void::Void` path fails to resolve, use `libp2p::swarm::derive_prelude::Void` in the variant + impl (verify which resolves on 0.54.1 during the build).
- [ ] Run, expect PASS (compile clean): `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo build --lib 2>&1 | tail -30`. Expect a `dead_code`/unused-match warning on the new variant until Task 4 lands the arm — acceptable mid-task (final clippy gate is after Task 4).
- [ ] Commit:
```
git add elohim/elohim-storage/src/p2p/behaviour.rs
git commit -m "feat(storage): connection_limits behaviour on ElohimStorageBehaviour

Caps raw established/pending connections from P2PConfig. ToSwarm=Void
(never emitted; denies over-limit internally). Finding #3 floor.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 4 — Storage swarm loop: ignore-arm for the new event (`p2p/mod.rs`)

Files:
- `elohim/elohim-storage/src/p2p/mod.rs:3698` (`handle_behaviour_event` match). **SEQUENCED AFTER P-RECONCILE per RESOLUTION-B.**

- [ ] Write the failing test — compile-level: after Task 3 the `match event` in `handle_behaviour_event` is non-exhaustive (`ConnectionLimits` unhandled). Run, expect FAIL: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo build --lib 2>&1 | tail -30` — if a trailing `_ => {}` already absorbs it, the build is clean but the variant is silently swallowed; prefer an EXPLICIT arm so the never-type is documented.
- [ ] Write minimal implementation — add an explicit arm in `handle_behaviour_event` (before any catch-all `_ =>`; the `ShardProtocol` arm starts the match at line 3699):
```rust
            behaviour::ElohimStorageBehaviourEvent::ConnectionLimits(ev) => {
                // type ToSwarm = void::Void — unreachable; the behaviour denies
                // over-limit connections internally. Match for exhaustiveness.
                match ev {}
            }
```
- [ ] Run, expect PASS: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' CARGO_TARGET_DIR=/tmp/es-test RUSTC_WRAPPER="" cargo build --lib 2>&1 | tail -20` then clippy clean: `... cargo clippy --lib -- -D warnings 2>&1 | tail -20`.
- [ ] Commit:
```
git add elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(storage): handle ConnectionLimits event arm (never-type, exhaustive)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 5 — Node `P2PConfig`: limit fields + Client-role defaults (`steward/node/src/config.rs`)

Files:
- `steward/node/src/config.rs:101` (`P2PConfig` struct) + its `Default`/init in `Config::default` (`:218`). SOLE owner.

- [ ] Write the failing test — append to the node config test module:
```rust
#[cfg(test)]
mod connection_limit_node_config_tests {
    use super::*;
    #[test]
    fn node_p2pconfig_caps_default_to_client_role() {
        let c = P2PConfig::default();
        // Household node runs as relay Client → LOWER established cap.
        assert_eq!(c.max_established, Some(128));
        assert_eq!(c.max_established_per_peer, Some(8));
        assert_eq!(c.max_pending, Some(64));
    }
}
```
  (If node `P2PConfig` has no `#[derive(Default)]`/`Default for Config`, construct it via the existing `Config::default()` path and read `.p2p`.)
- [ ] Run, expect FAIL: `cd /projects/elohim/steward/node && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/sn-test RUSTC_WRAPPER="" cargo test --lib connection_limit_node 2>&1 | tail -30`.
- [ ] Write minimal implementation — add the same 3 `#[serde(default = "...")]` fields to node `P2PConfig`, with node-local `default_*` fns returning the CLIENT-role values (`max_established → Some(128)`, per-peer `Some(8)`, pending `Some(64)`), and wire them into the `Config::default` `P2PConfig { ... }` initializer at `:218`.
- [ ] Run, expect PASS: same command.
- [ ] Commit:
```
git add steward/node/src/config.rs
git commit -m "feat(node): connection-limit config fields on P2PConfig (client defaults)

Household nodes run as relay Client → lower established cap than the
edge/relay storage role. Finding #3 floor.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 6 — Node behaviour + swarm loop: `connection_limits` field, event, construct, arm (`transport.rs`)

Files:
- `steward/node/src/p2p/transport.rs:43` (`ElohimBehaviour` struct, field after `bitswap`); the `ElohimBehaviourEvent` enum (auto-derived by `#[derive(NetworkBehaviour)]` — the macro generates it; add the explicit ignore-arm in the swarm loop near line 199–345); construct in the `with_behaviour` closure (before `ElohimBehaviour { ... }`, after the bitswap block ~line 489).

Mirrors the storage change exactly (parity item). The node uses `Config::default()`-sourced `P2PConfig`; thread `config.p2p.max_established` etc. into the builder closure (capture the values BEFORE the closure, like `bitswap_enabled`, since the closure is `move`).

- [ ] Write the failing test — compile-level (same shape as Task 3/4). Confirm absence: `grep -c connection_limits steward/node/src/p2p/transport.rs` → `0`.
- [ ] Write minimal implementation:
  - Capture limits before the swarm builder (near `let bitswap_enabled = ...`):
```rust
    let max_established = config.p2p.max_established;
    let max_established_per_peer = config.p2p.max_established_per_peer;
    let max_pending = config.p2p.max_pending;
```
  - Add field to `ElohimBehaviour` (after `bitswap`):
```rust
    /// Structural no-overwhelm floor (finding #3). ToSwarm=Void.
    pub connection_limits: libp2p::connection_limits::Behaviour,
```
  - In the `with_behaviour` closure, before `ElohimBehaviour { ... }`:
```rust
            let connection_limits = libp2p::connection_limits::Behaviour::new(
                libp2p::connection_limits::ConnectionLimits::default()
                    .with_max_established(max_established)
                    .with_max_established_per_peer(max_established_per_peer)
                    .with_max_pending_incoming(max_pending)
                    .with_max_pending_outgoing(max_pending),
            );
```
  - Add `connection_limits,` to the returned `ElohimBehaviour { ... }`.
  - Add the ignore-arm to the swarm event loop (alongside the other `LibSwarmEvent::Behaviour(ElohimBehaviourEvent::...)` arms):
```rust
                LibSwarmEvent::Behaviour(ElohimBehaviourEvent::ConnectionLimits(ev)) => {
                    match ev {} // type ToSwarm = void::Void — unreachable.
                }
```
  - The `#[derive(NetworkBehaviour)]` macro auto-generates `ElohimBehaviourEvent::ConnectionLimits` + the `From<Void>` impl — no manual `From` needed here (the node uses the derived event enum; the storage crate hand-rolls its enum, hence storage needs the manual `From` in Task 3). Verify which the node uses during build.
- [ ] Run, expect PASS: `cd /projects/elohim/steward/node && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/sn-test RUSTC_WRAPPER="" cargo build --bins 2>&1 | tail -30` then `... cargo clippy -- -D warnings 2>&1 | tail -20`.
- [ ] Commit:
```
git add steward/node/src/p2p/transport.rs
git commit -m "feat(node): connection_limits behaviour on ElohimBehaviour (parity w/ storage)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 7 — iroh cutover decision record (`2026-06-14-iroh-cutover-decision.md`)

Files:
- C `genesis/docs/superpowers/specs/2026-06-14-iroh-cutover-decision.md`. SOLE owner. No code; no test (doc-only).

Records the #6 decision so the recurring "should we freeze iroh?" question is settled in-repo. Content (concrete, grounded in `p2p_iroh/README.md` + `2026-05-08-iroh-libp2p-complementarity.md`):

- [ ] Write the doc with these sections:
  - **Status:** Proposed (operator-approval gate noted; this is a decision RECORD, the call is operator-only).
  - **Refuted framing:** iroh is at Phases 1–10 (cutover-ready transport, `README.md:53`), NOT "Phase-2 only" (that was the stale `mod.rs:8` doc-comment). 27 iroh test files (7 `*_parity`, 6 `*_real_backend`, gossip/e2e/dual-publish), not ~20.
  - **The freeze is a no-op for the parity tax.** `2026-05-08-iroh-libp2p-complementarity.md` (Approved) lands dual-stack PERMANENT for gossip/sync/EPR/shard/view-fed/identity/trust (7 planes), iroh-canonical only for blob, dual for discovery. For 7 of 9 planes the parity tests are the *permanent* cross-stack byte-parity contract, NOT transition scaffolding. Freezing does not delete them; only the blob-canonical cutover removes a transport (the single tax-reducing move).
  - **Decision:** commit to a DATED, blob-plane-first cutover. Milestone "blob-iroh-canonical" lands `README.md:265-310` gates 1 (backend wiring, blob path only), 2 (`/api/v1/blob/{hash}` reads `IrohBlobStore`), 3 (seeder write-through to `IrohBlobStore`), 6 (CI parity soak ×1wk), 8 (10k-roundtrip p99 ≤ libp2p baseline). Gates 4/5/7/9/10 are full-cutover, NOT in the blob milestone.
  - **Owner + date:** named TODO line for operator to fill owner + target date (the gap today is "no calendar date and no owner" — make the blank explicit, do not invent one).
  - **Per-plane parity table:** 9 planes × {dual-permanent | iroh-canonical | dual-transition} drawn from the complementarity spec.
  - **Cross-link:** the no-overwhelm floor (this same plan) applies to BOTH stacks (connection_limits is libp2p-only; note that iroh QUIC has its own connection accounting — a FOLLOW-ON seam, not in scope).
- [ ] No build. Verify the doc renders: `grep -c '^#' genesis/docs/superpowers/specs/2026-06-14-iroh-cutover-decision.md` (sanity).
- [ ] Commit:
```
git add genesis/docs/superpowers/specs/2026-06-14-iroh-cutover-decision.md
git commit -m "docs(iroh): dated blob-first cutover decision record (finding #6)

Refutes 'freeze the tax': complementarity spec makes dual-stack permanent
for 7 planes, so only blob-canonical cutover reduces the tax. Records the
named gate subset + leaves owner/date as an explicit operator blank.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

## TASK 8 — Final cross-crate gate

- [ ] Run the full gate block (both crates, all four checks) from the "Build / test commands" section. Expect: storage `cargo build --lib` + `clippy --lib -D warnings` + `fmt --check` clean; node `cargo build --bins` + `clippy -D warnings` + `fmt --check` clean; the two new config-default tests green.
- [ ] If clippy flags `match ev {}` on the never-type, that is the canonical empty-match idiom and is correct; do NOT replace it with `_ => {}` (which would silently swallow a real variant if the ToSwarm type ever changes).
- [ ] No commit (gate-only); if fmt rewrote anything, `git add` the touched file + amend its task commit.

---

## // FOLLOW-ON seams (deliberately left for the integration pass)

- **Runtime-tunable limits.** v1 ships STATIC config. If limits should become a knob, route through P-RECONCILE's `P2PCommand` + `Result<(),ActuationRefusal>` reply (consume, do not define) — and through P-ACTUATION's `ScopeId::StorageAdmission`-adjacent vocab (a `ScopeId::ConnectionFloor` would be a NEW S3 variant, owned by P-ACTUATION, NOT this plan). Left as a named seam.
- **iroh/QUIC connection accounting.** `connection_limits::Behaviour` is libp2p-only. The iroh stack has its own endpoint connection accounting; a parity floor for the iroh transport is a separate item (noted in the cutover doc, NOT built here).
- **Idle-timeout asymmetry.** Storage idle timeout is 300s (`mod.rs`), node 60s (`transport.rs`) — under a flood the node sheds idle links 5× faster. This plan does NOT touch idle timeout (it has a documented genesis-#1119 rationale on the storage side). Left for soak-driven tuning under P-PROOFS.
- **Limit value tuning.** The defaults (512/128 established, 8 per-peer, 64 pending) are arbitrary-but-sane starting points. P-PROOFS' `no_overwhelm_soak` + the 6-peer alpha soak supply the evidence to set final numbers; the config field shape is the contract this plan locks.
- **Blob-plane cutover skeleton.** The `2026-06-14-iroh-cutover-decision.md` gate subset (1/2/3/6/8) is an L-sized follow-on plan, NOT in this plan (this plan only records the decision).

---

## Dispatch note

- **Isolated-worktree, subagent-driven, commit-only.** Run in a dedicated worktree off the shift branch. The integrator pushes/merges (never `git push` from here).
- **Wave 2 leaf.** Tasks 1, 3, 5, 6, 7 are fully independent and may start as soon as dispatched. Tasks 2 and 4 (storage `p2p/mod.rs`) are **SEQUENCED behind P-RECONCILE's `run()`/`P2PCommand` rewrite (RESOLUTION-B)** — do them LAST, rebasing onto the updated `p2p/mod.rs`; their edits are small (3 additive config fields + Default lines in Task 2; one ignore-arm in Task 4) and mechanically rebaseable.
- **Per-task `git add` names exact files only** (selective-stage) — a parallel thread may be active in the shared worktree.
- **No `.claude/data` writes** (runtime Rust must never touch it; this plan writes none).
