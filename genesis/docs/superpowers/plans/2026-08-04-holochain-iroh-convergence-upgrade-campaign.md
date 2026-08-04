---
title: "Holochain–iroh Convergence Upgrade Campaign — pin lift, 0.6.3 rebase, transport sovereignty, 0.7 re-genesis"
id: holochain-iroh-convergence-upgrade-campaign
status: Draft
class: substrate
domain: substrate (conductor + storage dataplane transport lineage)
sprint: proposed (multi-wave campaign; Waves 1a-1d are the first schedulable sprint)
cites:
  - conductor-leak-upstream-research-tx5-pin-verdict | Conductor anon-heap leak | sha256:ccbf95a2af47c660 | path: genesis/docs/content/elohim-protocol/history/2026-06-17-conductor-leak-upstream-research-tx5-pin-verdict.md
  - conductor-leak-jemalloc-cure-verdict | Conductor leak | sha256:049eccfdb959ebd6 | path: genesis/docs/content/elohim-protocol/history/2026-06-19-conductor-leak-jemalloc-cure-verdict.md
  - iroh-parallel-stack | 2026-05-07-iroh-parallel-stack | sha256:933a487c90b606f2 | path: genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md
  - genesis/docs/superpowers/sprints/2026-06-15-iroh-dataplane-toggle-sprint-result.md
  - substrate-trust-contract-runbook | The Substrate Trust Contract | sha256:cb76e9f0ae6bacfc | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
memory_anchors:
  - project_iroh_dataplane_actual_state
  - project_dna_hash_blind_to_coordinator_zomes
  - feedback_codex_side_delegation_queue
  - feedback_subagent_disjointness_read_write
  - feedback_multi_agent_coherence_take_leave
  - feedback_delegate_narrow_tasks_to_cheaper_tiers
---

# Holochain–iroh Convergence Upgrade Campaign

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Wave 1 lanes A-D are independently claimable (see Delegation Map); Waves 2-3 are gated on Wave 1 evidence.

**Goal:** Converge the conductor (Holochain/kitsune2) and the elohim-storage dataplane onto the modern iroh 1.0 line, catch the conductor fork up to upstream 0.6.3 maintenance, and stage the 0.7 migration as a planned re-genesis event — without losing our two live fork patches or our infrastructure sovereignty.

**Architecture:** Three cargo workspaces move independently (elohim-storage, holochain-conductor fork, doorway-service — verified separate lockfiles, no shared resolver). Wave 1 runs four disjoint lanes in parallel; Wave 2 is the transport-sovereignty design gate (self-hosted iroh-relay vs n0 public relay, SBD/coturn retirement); Wave 3 is the 0.7 migration executed as an authorized genesis-storage reset. The in-network governance upgrade path is explicitly OUT of scope here and held as a named backlog item.

**Tech Stack:** Rust (cargo, separate workspaces), iroh 1.0.x / iroh-blobs 0.103 / iroh-gossip 0.101, Holochain 0.6.3 → 0.7.0, kitsune2 0.4.1 → 0.5.0, hdk/hdi, Jenkins CI, k8s manifests under `genesis/orchestrator/manifests/`.

## Global Constraints

- **Operator decision recorded 2026-08-04:** genesis storage reset is AUTHORIZED for the 0.7 wave — we are firmly in dev, proving primitives; agent-key lineage continuity is NOT a Wave-3 blocker. (This does not relax the alpha genesis-pair atomicity rule: any forced reinstall must hit both bootstrap peers or the namespace partitions — see `project_dna_hash_blind_to_coordinator_zomes` context in CLAUDE.md "DNA changes don't redeploy by default".)
- **Dependency bumps are verified by `cargo test`, never `cargo check`** (CLAUDE.md; diesel 2.3.5→2.3.11 precedent). Echo `EXIT=$?` on its own line — piped/tailed cargo output has misreported exit codes.
- **RUSTFLAGS:** elohim-storage builds with `RUSTFLAGS='--cfg getrandom_backend="custom"'`; doorway with `RUSTFLAGS=""`. `CARGO_TARGET_DIR` = the session's cargo-target-pool slot for each workspace (never a bare `target/`).
- **Version targets (evidence-pinned 2026-08-04):** iroh `=1.0.3`, iroh-blobs `=0.103.0`, iroh-gossip `=0.101.0`; dalek chain finals `curve25519-dalek 5.0.0` + `ed25519-dalek 3.0.0` (both published 2026-07-06, no longer pre-release). Conductor rebase target: upstream tag `holochain-0.6.3` (= commit `448a36ef`). Client-crate family matching a 0.6.3 conductor: `holochain_client 0.8.3`, `holochain_types 0.6.3`, `holochain_websocket 0.6.3`, `holochain_conductor_api 0.6.3`.
- **DECISION 2026-08-04 (Lane-A execution finding): the storage iroh bump is DEFERRED to Wave 3.** The serde-world interlock (Evidence §7) makes iroh 1.0 unreachable while any crate in storage's graph pins `holochain_serialized_bytes 0.0.56` (`serde =1.0.219`); the escape window (`holochain_types 0.7.0-dev.25–.30`, hsb `0.0.57`/serde `=1.0.228`, pre-Action-restructure) is blocked by the shared `elohim/sdk/domains/*/types` crates, whose `holo_hash =0.6.0` is also consumed by five DNA zome crates building against the 0.6 hdk — bumping the shared crates would force two incompatible `ActionHash` types into zome builds. The iroh 1.0.x bump therefore executes inside Wave 3's family move (everything to 0.7 finals together). Wave 2's dual-mode dataplane enablement does NOT wait: it rides the existing `=0.92` build already shipped in the image.
- **Fork patches that MUST survive the rebase:** `dd12826` (store_slice_hash change-check — confirmed NOT fixed upstream at 0.6.3, target function byte-identical), `b477ca7` + `d0f505f` (jemalloc allocator cure + profiling feature — additive, deployment-local). `f85c2a7` is upstream's own `6923effd` cherry-picked and MUST drop out empty at 0.6.1+. `7cc927e` (ethosengine/tx5 zombie-fix vendor pin) stays as long as tx5 compiles in; it retires with the Wave-2 transport flip.
- **Commit-only discipline:** all work lands as commits on the shift/feature branch; the integrator is the single push/merge authority. Codex-claimed tasks land on their own branches and are reviewed by the orchestrating session (take/leave/reshape) before integration.
- **Layer guard:** elohim-storage's `p2p_iroh` dataplane and kitsune2's `transport_iroh` are DIFFERENT LAYERS (different crate families, config surfaces, failure modes). Operational relay-hosting experience transfers; code does not. Never argue "we already run iroh" across this seam.

---

## Evidence base (from the 2026-08-04 three-agent deep-dive)

1. **Runtime truth:** the alpha fleet runs libp2p-only dataplane (no manifest sets `ELOHIM_TRANSPORT_BACKEND`; the old `TRANSPORT_BACKEND: "dual-stack"` in `genesis/orchestrator/manifests/edgenode/alpha.yaml:286` is wrong-name AND unparseable-value) and tx5 conductor transport (live `tx5 send error` warnings via `wss://signal.elohim.host`). `"via DualGossipPublisher"` log lines do NOT prove dual mode — the libp2p-only path wraps `DualGossipPublisher::new(Some(libp2p), None)`; the true sentinel is `"Dual: DualGossipPublisher wired into P2PNode"` (`elohim/elohim-storage/src/main.rs:~2907`).
2. **Dep chain:** Holochain 0.7.0 → kitsune2 0.5.0 → plain upstream `iroh = "1.0.0"`. Holochain 0.6.3's default `transport-iroh` feature rides `iroh-holochain 0.95.1` (a same-source republish) whose dalek chain is still pre-release-flavored. Our storage freeze rationale (pre-release dalek) dissolved 2026-07-06.
3. **0.7 Action model:** normalization (`Action { header: ActionHeader, data: ActionData }`, closed 10-variant enum, `#[repr(i64)]` discriminants) — NOT an open ontology. Our exposure: ~35-45 sites across ~12 files; integrity zomes nearly untouched (FlatOp absorbed it; 3 binding sites). TWO real risks: (a) Action serialization is the hash+signature preimage → silent cross-version validity break at `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:3757,3775` (carried-head verification); (b) `elohim/elohim-storage/src/services/holochain_humans_replayer.rs:119` decodes typed `Vec<Record>` msgpack across the conductor boundary and its `holochain_types` pin is already skewed vs the conductor.
4. **Client pins are on the wrong family TODAY:** doorway + storage pin `holochain_client 0.9.0-dev.5` / `holochain_types 0.7.0-dev.5/.11` (the unreleased 0.7 wire line) against a 0.6.0 conductor. Correct 0.6.x pairing is `0.8.3`/`0.6.3`. Pre-existing latent risk, surfaced not created by this campaign.
5. **0.6.1 breaking changes** (conductor side): transport default tx5→iroh; conductor-config gains `relay_url`/`base64_auth_material`, loses `dpki:`; `ConductorConfig` gains required `incoming_request_concurrency_limit`; Rust client `AppInfoStatus`→`AppStatus` union reshape; `get_agent_activity` gains a `GetOptions` param and returns `Vec<SignedWarrant>`. DNA-hash surface verified byte-identical 0.6.0→0.6.1 by our prior research (`2026-06-17` tx5 pin verdict addendum).
6. **Serde-world interlock (found in Lane-A execution, 2026-08-04):** iroh ≥1.0 requires `serde ^1.0.228` (via mandatory `iroh-metrics`), but the entire Holochain 0.6-line crate family — including `holo_hash =0.6.0` in the shared sdk domain crates and the 0.6.x conductor family (verified from the sweettest lock) — rides `holochain_serialized_bytes 0.0.56` which exact-pins `serde =1.0.219`. Disjoint by construction; cargo must unify hsb (and thus serde) across a workspace graph. The first serde-1.0.228 hsb is `0.0.57`, adopted by `holochain_types 0.7.0-dev.25`; the Action-model restructure lands at `dev.31` — so `dev.25–.30` is the only old-wire-shape/new-serde window, and it is unreachable for us until the DNA/zome side leaves the 0.6 family (Wave 3). Corollary: **storage can never realign DOWN to the 0.6.x client family while targeting iroh 1.0** — the two goals are serde-exclusive; the iroh convergence and the 0.7 migration are structurally the same move.
7. **Transport-sovereignty surface:** our tx5 infra = hand-rolled SBD signal server (`doorway/doorway-service/src/signal/`), dual coturn TURN pair (`genesis/orchestrator/manifests/infra/alpha-coturn-{operations,shem}.yaml`), custom kitsune2 bootstrap (`doorway/doorway-service/src/bootstrap/k2.rs` — bootstrap protocol architecturally separate from signal/relay, believed unaffected, unverified). iroh replaces STUN/TURN+SBD with QUIC hole-punching + its own relay protocol; upstream defaults to n0's PUBLIC relay — unacceptable as a silent default for our self-hosted posture; `iroh-relay` is self-hostable (open-source server crate).

---

## Wave 1 — four disjoint lanes (parallel; each independently claimable)

Read/write-set disjointness map (per `feedback_subagent_disjointness_read_write`):

| Lane | Write-set | Reads shared with | Safe in parallel with |
|---|---|---|---|
| A | `elohim/elohim-storage/**` (Cargo.toml, Cargo.lock, src fixes) | — | B, C, D |
| B | `elohim/holochain-conductor/**` (submodule, own branch) | — | A, C, D |
| C | `doorway/doorway-service/**` | — | A, B, D |
| D | read-only + one doc | reads A/B/C trees | A, B, C |

Lane A owns BOTH the iroh bump and storage's client-pin question because they share `elohim/elohim-storage/Cargo.toml` — never split those across two agents.

### Task A1: Lift the elohim-storage iroh pin to the 1.0 line

> **OUTCOME 2026-08-04: DEFERRED to Wave 3** (serde-world interlock — see Global Constraints decision + Evidence §6). Landed value: two pre-flight fixes on the rotted `p2p-iroh` test surface (`8fe94b212` EchoBackend ListDocumentsSince arm, `086537e61` stale Announce-contract test dropped) and the interlock evidence itself. One known-red parked: `seed_e2e_dual_address` (backlog `2026-08-04-dual-address-blob-router-unmanifested-blake3-400`). The steps below execute inside Wave 3's family move.

**Files:**
- Modify: `elohim/elohim-storage/Cargo.toml:272-283` (pin block + comment)
- Modify: `elohim/elohim-storage/Cargo.lock` (regenerated)
- Modify: `elohim/elohim-storage/src/p2p_iroh/**` (compile-error-driven API migration, 0.92→1.0.3 / blobs 0.94→0.103 / gossip 0.92→0.101)

**Interfaces:**
- Consumes: nothing from other lanes.
- Produces: a green `p2p-iroh` feature build on iroh 1.0.x; the parity harness (`src/p2p_iroh/parity_harness.rs`) and dual-publish tests as the behavioral evidence gate. Wave 2 consumes this (dual-mode enablement rides the new pin).

- [ ] **Step 1: Baseline — record the pre-bump test surface.** Run and save output:
```bash
cd /projects/elohim/elohim/elohim-storage
export CARGO_TARGET_DIR=<session pool slot for elohim__elohim-storage>/dev
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh --tests 2>&1 | tail -30; echo EXIT=$?
```
Expected: green (this is the regression baseline; if it is NOT green, stop and report — do not bump on a red base).
- [ ] **Step 2: Bump the pins.** Replace lines 281-283 with `iroh = { version = "=1.0.3", optional = true }`, `iroh-blobs = { version = "=0.103.0", optional = true }`, `iroh-gossip = { version = "=0.101.0", optional = true }`. Rewrite the comment block (272-280): state the 2026-07-06 dalek-finals fact, the kitsune2-0.5-pins-iroh-1.0 convergence rationale, and delete the "Never bump" language.
- [ ] **Step 3: Compile-error-driven migration.** `cargo build --features p2p-iroh 2>&1 | head -60; echo EXIT=$?`. Fix errors module-by-module (`endpoint.rs`, `blob_store.rs`, `gossip.rs`, `node.rs`, the 7 ALPN protocol impls, `dual_publish/`). Known 0.9x→1.0 change classes to expect: `Endpoint` builder/discovery API renames, `NodeAddr`/`NodeId` moves into `iroh-base` re-exports, blobs `FsStore` API surface (0.94→0.103 is nine releases), gossip topic join signatures. Rule: mechanical adaptation only — if a semantic choice appears (e.g., a removed feature with two replacement paths), STOP and surface it to the orchestrator rather than choosing silently.
- [ ] **Step 4: Full behavioral gate — not check.**
```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh --tests 2>&1 | tail -30; echo EXIT=$?
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release 2>&1 | tail -5; echo EXIT=$?   # DEFAULT features too — the 2026-06-15 lesson: gated code must not break the default build
cargo test export_bindings 2>&1 | tail -5; echo EXIT=$?
```
Expected: all EXIT=0; generated TS byte-identical (sha256 diff) — the bump must not move the ts-rs surface.
- [ ] **Step 5: Commit** (path-limited): `git commit -m "feat(storage): iroh 0.92→1.0.3 pin lift — dalek chain final, converges with kitsune2 0.5 line" -- elohim/elohim-storage`

### Task A2: Storage client-pin decision (same owner, after A1)

> **OUTCOME 2026-08-04: ANSWERED by the interlock evidence, no code change.** The 0.6.x realign is structurally incompatible with the iroh-1.0 target (Evidence §6 corollary); the current `0.7.0-dev.5/.11` pins stay, documented as the pre-restructure wire-compatible family, and complete to 0.7 finals at Wave 3 (Task F2).

**Files:**
- Modify: `elohim/elohim-storage/Cargo.toml:101-102`
- Modify: `elohim/elohim-storage/src/services/holochain_humans_replayer.rs` (if decode types shift)

**Interfaces:**
- Consumes: A1's green tree.
- Produces: an explicit, tested pin decision recorded in the Cargo.toml comment — either realigned to `holochain_client 0.8.x`/`holochain_types 0.6.3` (matching the 0.6.x conductor) or deliberately held on the 0.7.0-dev line with the skew documented at the `holochain_humans_replayer.rs:119` decode site.

- [ ] **Step 1:** Attempt the realign: pin `holochain_client = "0.8.3"`, `holochain_types = "0.6.3"`; `cargo build 2>&1 | head -40; echo EXIT=$?`.
- [ ] **Step 2:** If it compiles: run the replayer + conductor_writes test modules (`cargo test replayer 2>&1 | tail -20; echo EXIT=$?`) and keep it. If the 0.6.x family conflicts with other deps (e.g., shared holochain_serialized_bytes), REVERT and instead add a load-bearing comment at both the Cargo.toml pin and `holochain_humans_replayer.rs:119` documenting the intentional 0.7-dev skew and its Wave-3 resolution. Either outcome is a valid deliverable; silent status quo is not.
- [ ] **Step 3: Commit** with the decision in the message.

### Task B1: Conductor fork rebase onto holochain-0.6.3 (tx5 kept via feature flag)

> **OUTCOME 2026-08-04: DONE (Codex lane + review + fix wave), gitlink bump deferred pending branch push.** Rebase verified byte-identical (4 commits on `elohim-0.6.3` @ `da823fc6a`; `f85c2a7` dropped correctly; tx5 stayed 0.8.1; `dd12826` intact). Review caught two composition breaks the lane missed, fixed as `4b163f707` (+ che-devworkspaces `97916b6`): (a) 0.6.3 renamed the tx5 feature (`backend-go-pion`→`transport-tx5-backend-go-pion`) and made `transport-iroh` a DEFAULT — production builds need `--no-default-features --features sqlite-encrypted,wasmer_sys,transport-tx5-backend-go-pion,jemalloc` (`schema` verified dev-only); (b) `NetworkConfig` gained required `relay_url` + `deny_unknown_fields`, so templates carrying the never-valid `enable_mdns`/`enable_relaying` keys would hard-fail conductor startup — keys dropped, `relay_url` set to the sovereign placeholder `https://relay.elohim.host` (inert under tx5; must be live before any transport-iroh build). Gates re-run with evidence (release build EXIT=0, holochain_p2p 51/0 — `lane-b-gate-evidence.md`). Backlog's `dpki:`/`incoming_request_concurrency_limit` items closed as no-ops (absent pre-0.6.0 / optional-with-default). REMAINING: operator pushes `elohim-0.6.3` to ethosengine/holochain (CI Dockerfile fetches by branch name), THEN the monorepo gitlink bump (bumping before the push would break fresh clones on an unfetchable SHA).

**Files:**
- Modify: `elohim/holochain-conductor` (submodule — new branch `elohim-0.6.3` from our `elohim-0.6`)
- Modify: `elohim/holochain-conductor/Cargo.toml` (verify the `[patch.crates-io]` tx5 section survives; verify jemalloc features)
- Modify (monorepo): the submodule gitlink bump, committed separately after review

**Interfaces:**
- Consumes: nothing from other lanes.
- Produces: a conductor building at 0.6.3 with `transport-tx5-backend-go-pion` (or the 0.6.3-era tx5 feature name — verify against `crates/holochain_p2p/Cargo.toml` at the tag) COMPILED IN and tx5 still the runtime transport (config unchanged this wave), carrying `dd12826` + both jemalloc commits. Wave 2 consumes the now-available `transport-iroh` default.

- [ ] **Step 1:** In the submodule: `git fetch upstream --tags` (first network op — if the environment blocks it, stop and report; do NOT vendor tarballs). `git checkout -b elohim-0.6.3 elohim-0.6 && git rebase --onto holochain-0.6.3 a6d4e80`.
- [ ] **Step 2:** Verify `f85c2a7` dropped out empty (`git log --oneline holochain-0.6.3..HEAD` must show 4 commits, not 5; if rebase surfaces a conflict on it, `git rebase --skip` after confirming the upstream file already contains the fix — compare against upstream `6923effd`).
- [ ] **Step 3:** Resolve conflicts on the remaining 4. Expected-clean: `dd12826` (upstream `op_store.rs` byte-identical), jemalloc pair (additive `Cargo.toml`+`main.rs`). Likely-touched: `7cc927e` `[patch.crates-io]` block if upstream moved the tx5 crate version — keep our `ethosengine/tx5` pin matched to whatever tx5 version 0.6.3 references, or escalate if 0.6.3's tx5 family moved past 0.8.1.
- [ ] **Step 4:** Config-shape check WITHOUT flipping transport: diff `crates/holochain_conductor_api` config structs 0.6.0→0.6.3 for the new required `incoming_request_concurrency_limit` and removed `dpki:` — then patch our templates (`elohim/holochain/edgenode/conductor-config.yaml`, `genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml`, `genesis/orchestrator/manifests/doorway/{prod,alpha-b}.yaml`): remove any `dpki:` remnants, add `incoming_request_concurrency_limit` with upstream's default value (read it from the 0.6.3 source, cite the file:line in the commit message). `deny_unknown_fields` arrives in 0.7 but treat unknown keys as debt now.
- [ ] **Step 5: Gate:**
```bash
cargo build --release 2>&1 | tail -5; echo EXIT=$?
cargo test -p holochain_p2p 2>&1 | tail -20; echo EXIT=$?   # dd12826's test lives here
```
- [ ] **Step 6: Commit** the submodule branch; separately commit the monorepo gitlink + config-template changes AFTER orchestrator review.

### Task C1: Doorway client-pin realign + 0.6.1 API-reshape audit

> **OUTCOME 2026-08-04: DOCUMENTED HOLD (Codex lane, reviewed + integrated as `f49cf80b3`).** `holochain_client 0.8.3` requires `holo_hash ^0.6.3`, disjoint from `imagodei-types`' path-dep exact `=0.6.0` — the same shared-sdk `holo_hash` pin as Evidence §6, hit from the downward direction. All pin changes reverted; hold documented at the Cargo.toml pin site + the authorized `CapAccess` 0.7 breadcrumb. Resolves at Wave 3 Task F2 with the family move.

**Files:**
- Modify: `doorway/doorway-service/Cargo.toml:32-45`
- Modify: `doorway/doorway-service/src/**` (compile-driven: `AppStatus` union reshape, `get_agent_activity` signature IF used)

**Interfaces:**
- Consumes: nothing from other lanes.
- Produces: doorway pinned to the client family matching the 0.6.3 conductor (`holochain_client 0.8.3`, `holochain_types 0.6.3`, `holochain_zome_types 0.6.3`, `holochain_websocket 0.6.3`, `holochain_conductor_api 0.6.3`), all `AppInfoStatus`/"paused"/"running" matches migrated to the new `AppStatus` union.

- [ ] **Step 1:** `grep -rn "AppInfoStatus\|AppStatus\|paused\|Paused" doorway/doorway-service/src/ | head -30` — inventory the reshape surface BEFORE bumping (the 0.6.1 `AppStatus` union removed "paused"/"running", added "enabled"/nested "disabled" reasons).
- [ ] **Step 2:** Bump the five pins listed above; `RUSTFLAGS="" cargo build --release 2>&1 | head -40; echo EXIT=$?`; fix compile-driven.
- [ ] **Step 3: Gate:** `RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -20; echo EXIT=$?` and `cargo clippy -- -D warnings 2>&1 | tail -5; echo EXIT=$?`.
- [ ] **Step 4:** Note (do NOT rename yet): `CapAccess`→`CapAccessType` at `src/conductor/typed_admin.rs:18,224` is a 0.7-line change — leave it; the 0.8.3 client still uses `CapAccess`. Add a `// 0.7 migration:` breadcrumb comment only.
- [ ] **Step 5: Commit** (path-limited to `doorway/`).

### Task D1: Verification research pack (read-only; closes the three flagged unknowns)

> **OUTCOME 2026-08-04: DONE (Codex lane, reviewed take-as-is, integrated).** All three verdicts evidence-backed and independently re-derived at review: (1) dalek `digest::crypto_common` bug fixed in 5.0.0 final (commit `59305b4e`, digest 0.11); (2) bootstrap wire core-compatible — and the doc CORRECTED this task's premise: holochain 0.6.0 pins kitsune2 `0.3.0` (0.6.3 pins `0.4.1`, `default = ["transport-iroh"]`); our doorway lock rides `0.4.0-dev.2`; new auth/relay surfaces (`/authenticate`, relay registration) are Wave-2 work, absent from our k2.rs today by design; (3) config mapping actionable for E1 — `bootstrap_url→coreBootstrap.serverUrl`, `relay_url→irohTransport.relayUrl` (top-level wins; n0 public relay is the DEFAULT unless set), `signal_url`/`webrtc_config` are tx5-only. Doc: `genesis/docs/content/elohim-protocol/history/2026-08-04-holochain-iroh-dep-verification-pack.md`.

**Files:**
- Create: `genesis/docs/content/elohim-protocol/history/2026-08-04-holochain-iroh-dep-verification-pack.md`

**Interfaces:**
- Consumes: reads all trees; writes only the one doc.
- Produces: written verdicts (with URLs/file:line citations) on: (1) whether `curve25519-dalek 5.0.0` final fixed the `digest::crypto_common` published-source path mismatch (read the dalek changelog/commits between `5.0.0-pre.1` and `5.0.0`); (2) whether the kitsune2 bootstrap wire protocol changed 0.4.0-dev→0.4.1 in any way that touches `doorway/doorway-service/src/bootstrap/k2.rs` (diff the kitsune2 bootstrap crate between the versions holochain 0.6.0 and 0.6.3 consume); (3) the precise `signal_url` vs `relay_url` semantics when `transport-iroh` is active in 0.6.3 (read `kitsune2_transport_iroh` 0.4.1 config source — which fields are live, which are tx5-only). Each verdict: CONFIRMED / REFUTED / STILL-UNKNOWN with evidence — no guesses.

- [ ] Steps: fetch + read the three source surfaces; write the pack; commit.

**Wave-1 exit gate (orchestrator, not delegable):** all four lanes reviewed take/leave/reshape; A1's parity harness output read (not just exit code); B1's dropped-commit verification confirmed; the monorepo gitlink for B1 committed only after A1+B1 both green (they meet in CI's edge pipeline — `elohim-storage` and the conductor ship in the same image family, see `elohim/holochain/Jenkinsfile`).

---

## Wave 2 — transport sovereignty flip (design gate first; ~1 session design + 1-2 sessions execution)

Not started until Wave 1 lands. The flip decision is operator-ratified (it changes what infrastructure exists), the design work is orchestrator+rust-architect.

- [ ] **Task E1 (design doc, Opus-assisted):** self-hosted `iroh-relay` deployment design — manifest under `genesis/orchestrator/manifests/infra/`, relay URL + `base64_auth_material` wiring into the conductor config templates, explicit decision record rejecting the n0 public-relay default (sovereignty stance), retirement plan for the SBD signal server (`doorway/doorway-service/src/signal/` — dead code once no conductor speaks SBD) and the coturn pair (verify nothing else consumes STUN/TURN first: grep manifests + doorway for coturn/stun references). Include rollback: tx5 feature stays compiled for one full wave; flip is per-env config, alpha first.
- [ ] **Task E2 (execution):** flip `transport-iroh` default on the rebased conductor in alpha config; deploy; watch the substrate trust-contract probes (per-seam smoke, `conductor-diagnostics`, canonical-head propagation line). @requires:alpha-cluster-6peer for the full 6-peer soak; the M/J/J household mesh proves the mechanism first (`feedback_household_nodes_is_the_stable_floor`).
- [ ] **Task E3 (dataplane dual enablement — the separate layer):** add `ELOHIM_TRANSPORT_BACKEND: "dual"` to `genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml` env block; DELETE the dead `TRANSPORT_BACKEND: "dual-stack"` block from `genesis/orchestrator/manifests/edgenode/alpha.yaml:284-287`; verify via the true sentinel log line + `/p2p/status` `irohNodeId`. Rides the existing `=0.92` build already shipped in the image (decision 2026-08-04: does NOT wait for the iroh 1.0 bump, which deferred to Wave 3). This closes the "STILL OPEN: live 2-node dual-delivery check" from `project_iroh_dataplane_actual_state`.

## Wave 3 — Holochain 0.7 migration as an authorized re-genesis event

Gated on Waves 1-2 stable. Scope is known-small from the 2026-08-04 impact survey; the enabling decision (genesis reset OK) is already made.

- [ ] **Task F1:** Action-model sweep — ~23 coordinator match arms (`Action::X(y)` → `ActionData::X(y)` + `.data`/`.header` paths; heaviest: `imagodei/src/lib.rs:4727-4744` collapses to `action.action_type()`, `content_store/src/lib.rs:12356` collapses to `action.entry_hash()`), 3 integrity-zome binding sites (`infrastructure_integrity/src/lib.rs:281,294`, `imagodei_integrity/src/lib.rs:1107` — `TypedAction<D>` field paths; sweettest coverage required since these define DHT validity), doorway `CapAccess`→`CapAccessType` (2 lines), storage replayer test-support rewrite (`holochain_humans_replayer.rs:398-410` → `Action { header, data }` shape, drop `weight`).
- [ ] **Task F2:** client crates to the 0.9/0.7 FINAL family (not -dev) across doorway + storage in lockstep with the conductor bump; the `Vec<Record>` msgpack decode at `holochain_humans_replayer.rs:119` is the canary test. **This is also where the deferred iroh 1.0.x bump executes** (A1's steps): the family move brings `holo_hash 0.7.0` / `hsb 0.0.57` / `serde =1.0.228` everywhere at once — sdk domain crates, zomes (on hdk 0.7), storage — dissolving the serde interlock; bump iroh/iroh-blobs/iroh-gossip in the same wave and run A1's four gates.
- [ ] **Task F3:** the re-genesis itself: `ALLOW_DNA_REINSTALL` on BOTH alpha genesis-pair peers atomically, seed replay via the established pipeline, carried-head verification (`content_store/src/lib.rs:3728-3788`) re-anchored against post-reset hashes. The hash-preimage break is neutralized BY the reset — old notarized hashes are not carried across.
- [ ] **Task F4:** conductor-config `deny_unknown_fields` cleanup + DB naming change absorbed by the reset (0.7 has no migration path; we don't need one).

## Held — named backlog item (NOT this campaign)

**In-network governance upgrade path:** the network itself enforcing revert/upgrade paths from inside (governance-ratified DNA migration, not operator-side reinstall flags). Upstream's rails are now visible — 0.6.2 `InitProperties` (DB-stored, init-readable, cleared-after-init) + `MigrationTarget` on `CloseChain`/`OpenChain` — and Wave 3's re-genesis is the LAST time we should need an out-of-band reset; the governance path is what replaces it. Mint as `genesis/data/timeline/backlog/governance-native-dna-upgrade-path.md` citing this plan and the 0.6.2/0.7.0 upstream primitives. Design belongs to a future p2p-design-gate + earned-reach ceremony pass (`project_earned_reach_governance_pr_ceremony_vision`); it is a capability of the peer-native layer, not a k8s concern (`feedback_k8s_is_not_the_architecture`).

---

## Delegation Map (orchestration tiers + Codex queue)

Orchestrating session (top tier) keeps: wave sequencing, lane-merge review (take/leave/reshape per `feedback_multi_agent_coherence_take_leave`), the Wave-2 sovereignty design, all operator-facing decisions, final composition review as work arrives.

| Task | Tier / owner | Why |
|---|---|---|
| A1 iroh pin lift | **Sonnet** (session subagent) w/ **Opus review** of the API-migration diff | Compile-driven mechanical work with a crisp behavioral gate (parity harness); the 9-release blobs jump warrants a second set of eyes |
| A2 storage client pins | Same Sonnet agent (same write-set as A1 — never split) | Decision is binary with both outcomes specified |
| B1 fork rebase | **Codex-delegable** (or Sonnet) | Fully specified: base/target commits named, per-commit conflict expectations written, gate commands given; submodule = disjoint write-set; evidence checkable (4-commit log, green gate) |
| C1 doorway pins | **Codex-delegable** (or Sonnet) | Disjoint tree, compile-driven, gate commands given |
| D1 research pack | **Codex-delegable** (or Haiku/Sonnet) | Read-only, write-set = one new doc, verdicts falsifiable |
| E1 relay design | Orchestrator + **rust-architect (Opus)** | Architecture + sovereignty judgment |
| E2/E3 flips | Sonnet execution after operator ratification | Config mechanics with named probes |
| F1-F4 0.7 sweep | Opus lead (integrity-zome sites) + Sonnet (coordinator mechanical arms), fresh impact re-survey first | DHT-validity surface deserves the strong model; the survey is 4 weeks stale by then |

**Codex claiming protocol** (per `feedback_codex_side_delegation_queue`): B1, C1, D1 get backlog entries in `genesis/data/timeline/backlog/` (one file each, task text copied from this plan verbatim, DoD = the task's gate steps) so any agent — Claude, Codex, Gemini — can claim during CI waits. Claimed work lands on its own branch, commit-only; the orchestrating session reviews before the gitlink/merge (done = composes, not compiles).

## Campaign Definition of Done

- Wave 1: all four lanes green under their own gates + orchestrator composition review; edge image builds with the rebased conductor + bumped storage together.
- Wave 2: alpha conductors on iroh transport through OUR relay; SBD/coturn retirement committed (manifests coherent — cluster reconciliation is the operator's); dataplane dual-mode sentinel line live on the fleet; tx5 vendor patch retired.
- Wave 3: 0.7 conductor + realigned finals + re-genesis complete; carried-head verification green post-reset.
- Held item minted with cites; `project_iroh_dataplane_actual_state` memory updated at each wave boundary.
