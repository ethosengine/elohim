# HANDOFF — Close the Gaps (paste into a fresh session, or: "read this and drive")

You are picking up an in-flight sprint on branch **`sprint/cross-pillar-cleanup`** (repo `/projects/elohim`). A long prior session shipped the replication-prioritizer epic + hub-identity substrate + the keystone producer. Your job: **keep driving the remaining gaps to working code**, subagent-driven, letting the CI pipeline + a2o do end-to-end verification. This doc is self-contained — read the two plan docs it points to for depth, then drive.

## 0. First, orient (do this before anything)
- `git config --global --add safe.directory /projects/elohim` if git reports "dubious ownership" (post-restart UID change — known).
- `git log --oneline -15` and `git status --short`. **Expect ~42 uncommitted files** — that is the **operator's in-flight `cargo fmt` sweep + agent/devfile edits + the auto-memory files** (`.claude/memory/`). NOT yours. Do **not** stash/reset/checkout/`git add -A`; stage only files you change.
- Read these durable anchors:
  - `genesis/docs/superpowers/plans/2026-05-29-close-the-gaps-sprint-kickoff.md` — the gap synthesis + sequencing + felt north-star.
  - `genesis/docs/superpowers/plans/2026-05-29-prioritizer-end-state-wire-hub-fetch.md` — the prioritizer epic plan (Wave 2 T6 deferred; per-wave dispatch notes).
  - `.claude/agents/rust-architect.md` → **"Canonical Implementation Patterns"** (wire-format evolution, read-side inline diesel vs ReconcileController-owns-writes, correct-but-dormant, cargo target-pool, shared-tree git, verification gate). The backend lane runs through `rust-architect`.
  - Memory: `project_hub_identity_cid_canonical_slug_alias`, `project_storage_tiering_placement_intelligence`, `project_dwelling_hub_replication_pattern`, `project_rea_compute_commitment_primitive`, `project_dna_changes_dont_redeploy_without_forced_reinstall`, `project_substrate_floor_elohim_ceiling`.

## 1. What's DONE (committed, locally green — do NOT redo)
Commit chain (lib suite 1304 passing, 0 failed):
`e6300665c` PeerCapacityView readers (Epic B) · `8287af29d` additive `BlobHint` gossip wire · `403b70122`/`28accea42`/`839f707cc` hub identity (schema → `hub_resolver` → `hub_capacity_service`) · `6a0f93e30` imagodei DNA `Collective`/`Membership` post-commit signals · `28d1dab7f` storage projector for those signals · `21cb7e8b3` prioritizer wired end-to-end (broadcaster hints → receive-arm scoring → bounded fetch) · `0f346db9e` `replicates-dwelling` commitment **writer** (`replicates_dwelling_service.rs`).

**Net state:** the prioritizer + pledged-vs-held bars are *correct and wired*; the commitment writer is unit-proven to activate both consumers. The substrate is "correct but starved" — it lights up the moment producers feed it and the surface renders it.

## 2. Cadence & doctrine (operate like the prior session)
- **Subagent-driven.** Dispatch `rust-architect` per backend task (`angular-architect` for Epic A UI) with full task text + the canonical-pattern reminders. For well-understood backend, do a **light** verification (the implementer self-verifies build + full `cargo test --lib` + clippy, then commits); reserve a separate `code-reviewer` dispatch for the meatiest/riskiest tasks. **Keep driving — don't pause at every gate; the operator wants momentum.**
- **Let the pipeline do end-to-end.** Do **not** run sweettest locally (slow). Local gate = build + full lib test green + clippy + targeted commit. CI sweettest + a2o verify integration; **a2o narrative is Opus work** (`feedback_a2o_narrative_is_opus_work`).
- **Build slots:** native storage → `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev`; keep `RUSTFLAGS='--cfg getrandom_backend="custom"'` for elohim-storage + DNA, `RUSTFLAGS=""` for doorway/steward. **DNA WASM lane = plain cargo, do NOT redirect `CARGO_TARGET_DIR`** (`hc dna pack` canonicalizes `./target`).
- **Pre-existing warnings are not yours:** `api/mishpat_recognition.rs:36` unused `Arc` + the EAE-module warnings (`entity.rs`/`decider.rs`/`executor.rs`/…) are the operator's in-flight work — report, don't fix; they make crate-wide `clippy -D warnings` red independent of your changes.
- **Gates/determinism in Rust; elohim `.ts` sense-and-respond. No new DHT entry types except behind a `p2p-design-gate`.** Canonical hub-id = Collective CID `collective:{action_hash}`; slug is a first-class steward-configurable alias.

## 3. THE GAPS (leverage-ordered — drive top-down, confirm the product-shaped ones)

**Gap 0 — Writer-caller / bilateral dwelling-hub handshake** *(immediate activation; PRODUCT-shaped — confirm initiation with operator)*
The commitment writer exists but **nothing calls it**, so no pledges exist in alpha and the bar stays dark despite the live mechanism. Needs the flow that invokes `replicates_dwelling_service::create_replicates_dwelling_commitment` — the bilateral counter-commitment handshake (a2o `household-resiliency-handshake.feature`) and/or a seeder path. *Who/when initiates a pledge is a product decision — confirm before implementing.* This is the single change that makes the pledged bar + prioritizer light up end-to-end.

**Gap 1 — `infrastructure:system-sample` emitter (+ observation gossip)** *(remote totals)*
Nothing emits per-node capacity samples → remote-peer `total_raw_bytes` reads 0 (local works via fs-probe). Periodic probe (`system_metrics.rs` fns exist) → write `observations` row (observer_cid = local peer) → gossip via the observation plane so peers see each other. **Scout the observation-gossip plane** (`OBSERVATION_LOG_PROTOCOL_ID` / `IROH_OBSERVATION_ALPN`, `elohim/observations/<kind>` topics) before scoping the cross-peer half; local emit alone is low-value (duplicates fs-probe).

**Gap 2 — Convergence & correctness polish**
- **T6:** align live-data reads (`humans.household_id`/`stewarded_nodes.household_id`) onto the canonical Collective CID via the slug↔CID alias; converge seed commitments' `recipient_dwelling_hub_id` onto the CID where a DHT anchor exists (so hint↔commitment match is representation-stable once live Memberships project).
- **T3 `node→agent` pledge mapping:** hub-member pledges read 0 (members are device node-ids; pledges key on agent CID — `KNOWN LIMITATION` in `hub_capacity_service.rs`). Map device→agent via `peer_identity_bindings`/`humans.agent_pub_key`.

**Gap 3 — Epic A: surface the live data** *(`angular-architect`; high felt-impact, no new substrate)*
Top-level posture view (`ResilienceSnapshotView`/`NetworkPostureView`/`TopologyOverviewView` wire types exist, no consuming component); free/used is `<dl>` text → make it a **clickable bar**; peer/device cards → **links** with drill-down. Pure Angular wiring on now-live wire-types.

**Gap 4 — Remaining epics (original light-up-the-topology kickoff)**
- **Epic C** — doorway Role-2 resolver (peer-hosted EPR-apps): the big "new internet" lift; **own `p2p-design-gate`**.
- **Epic D** — account-management surface + recovery UX + post-recovery key rotation (imagodei M5).
- **Epic E** — `<elohim-context-menu>` integration into `EprLinkComponent` (primitive + stories built; zero app consumers).
- **Epic F** — delivery stats (bytes-served/who-pulled; `blob-served` aggregation + endpoint; toll economics deferred).
- **Epic G** — wisdom-input shape (grow `wisdom.rs` to see placement-gaps/resilience/inventory; sense-and-respond only; after producers).

**Gap 5 — Deploy & verify** *(operator-owned + pipeline)*
- **Alpha DNA forced-reinstall** for the new imagodei signals (DNA-hash drift; all same-namespace peers reinstall or DHT-partition).
- **CI sweettest + a2o** shake out the prioritizer epic; **author/refresh a2o narrative** (`household-resiliency-handshake`, `constitutional-ratio-enforcement`) — Opus work.

**Coded follow-ups flagged in source:** the commitment writer used the storage-direct path (`dht_anchor_hash` nullable) — DHT-first (Mishpat coordinator → projection) is a clean follow-up; upgrade `validate_typed_for_creation`'s floor check from declaration-based to pledge-backed when `replicates-commons` lands.

## 4. Suggested first move
Confirm with the operator: drive **Gap 0** (writer-caller/handshake — makes the bar light up) vs **Gap 1** (system-sample emitter) vs **Gap 3** (Epic A surface) — and whether to push for CI + run the alpha DNA reinstall first. Story-first: hang each gap off the a2o scenario (`genesis/a2o/features/` — shefa storage-stewardship + topology). Felt north-star: *a steward sees their household's real free-vs-pledged-vs-held storage, watches a peer's commitment pull a blob to where it belongs, and never types a URL.*
