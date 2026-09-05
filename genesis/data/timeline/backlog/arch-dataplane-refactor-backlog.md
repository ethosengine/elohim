---
id: "backlog-arch-dataplane-refactor-backlog"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "EPR dataplane refactor backlog — 16 ranked, file-grounded items (reuse, IoC seams, mod.rs decomposition, dev QoL, head-plane scale)"
slug: "arch-dataplane-refactor-backlog"
written: "2026-06-11"
author: "agentic-developer (dataplane architecture review, operator-requested)"
status: "backlog"
priority: "medium"
tags: [architecture, dataplane, refactor, p2p, ioc, dev-qol, code-reuse]
cites:
  - elohim/elohim-storage/src/p2p/mod.rs
  - elohim/elohim-storage/src/reconcile/custody.rs
  - elohim/elohim-storage/src/services/commitment_fetcher.rs
---

# EPR dataplane refactor backlog (architecture review 2026-06-11)

Produced by the 5-analyst dataplane architecture review (duplication across
the three libp2p stacks, p2p/mod.rs decomposition, crate/IoC coherence, SDK
surface, dev QoL). Items graduate to shifts individually; the sequencing
note at the bottom is load-bearing (10 -> 12 -> 15 is a strict chain).

# ARTIFACT 1 — RANKED REFACTOR BACKLOG (EPR Dataplane)

Ranked by leverage-per-risk for keeping dataplane iteration fast. **[MECH]** = safe mechanical extraction; **[DESIGN]** = design-bearing (needs a seam decision before code).

| # | Item | What + Why (file-grounded) | Effort | Risk | Unblocks | Owner |
|---|------|----------------------------|--------|------|----------|-------|
| 1 | **Test fixture consolidation** [MECH] | `PeerStatusRow` builder duplicated 6× (`src/db/peer_statuses.rs:171`, `tests/render_capability_view.rs:30`, `tests/peer_status_e2e.rs:180`, `tests/peer_statuses_route.rs:54`, `tests/schema_contract.rs`, `src/db/stewarded_nodes.rs`) and the republish-epr event payload 3× (`src/services/republish_epr_validator.rs:181`, `tests/mishpat_bounds_gate_chain.rs:52`, +integration tests). Promote canonical builders into `src/test_util.rs` next to `test_pool()`; every schema drift currently means a 6-site hunt. | S | Low — test-only | Cheap schema evolution on `peer_statuses` and the validator wire contract | quality-sweep |
| 2 | **Shared length-prefix MessagePack codec crate** [MECH] | Nine near-identical `impl Codec` blocks across `elohim-storage/src/p2p/` (shard_protocol.rs:105-200, sync_protocol.rs:187-297, blob/epr/epr_atom/trust/view_federation/shamir_transport/identity_handshake) duplicate read/write framing with independently-set size limits (blob 4 KiB vs sync 16/64 MB). Extract a generic `LengthPrefixedCodec<Req,Resp>` with per-codec limits at construction; eliminates ~900 lines and centralizes the buffer-bound safety property. | M | Low-Med — wire format must stay byte-identical; covered by existing protocol tests | New protocols become ~20 lines; closes silent buffer-bound drift across 9 codecs | rust-architect shift |
| 3 | **P2PDispatcher trait (kill swarm_tx threading)** [DESIGN] | `http.rs:155,371,597-598,1068-1095` threads `P2PCommand` senders + `p2p_handle` directly into `handle_api_request`, coupling every HTTP handler (epr, blob, rea_commitments) to the swarm enum. Introduce `services/p2p_dispatcher.rs` trait with semantic methods (`publish_epr_announce`, `kad_get_providers`, `fetch_epr_atom`) registered in `Services`. | M | Med — touches the hottest routing path; enum translation is mechanical once the trait is agreed | HTTP handler tests without a live libp2p stack; prerequisite for any doorway-storage bridge | rust-architect shift |
| 4 | **BootstrapRepeater (T24) extraction** [MECH] | T24 persistent-peering spans 4 locations in `p2p/mod.rs` (init 2231-2235, learn-on-ConnectionEstablished 3574-3587, retry+backoff 2397-2442); exponential backoff state is untestable without the select! loop. Extract a `BootstrapRepeater` struct with `learn_peer()`/`redial_list()`. | S | Low — isolated state container | Unit-testable backoff; first proof-of-pattern for the loop decomposition | quality-sweep |
| 5 | **BackpressureGate centralization** [MECH] | `sync_paused` is checked in 5 of 15 select! arms (`p2p/mod.rs:2328-2391` — sync, replication, gap_dispatch, acquisition_reconcile, provide_reconcile) as scattered implicit if-checks. Wrap in a `BackpressureGate` enum with one `check()`; logic unchanged. | S | Low — behavior-preserving wrap | Composable gates (e.g. queue-depth) without re-reading the whole loop; diagnosis of "sync thrashing during bulk import" | rust-architect shift (small) |
| 6 | **PeerContextCache** [MECH] | `peer_metrics`, `identify_cache`, `trust_cache` are three of P2PNode's 154 fields (`p2p/mod.rs:462-616`, PeerMetrics at 620-644), accessed at 20+ scattered sites; steward/node (`steward/node/src/p2p/transport.rs:1-40`) has no metrics at all and would re-implement from scratch. Encapsulate as one struct now (encapsulation only); cross-crate extraction is a later option. | M | Low — encapsulation, no behavior change | Cuts god-object surface; RTT-aware peer selection reusable by custody sweep and (later) steward | rust-architect shift |
| 7 | **Dockerfile migration-layer reorder** [MECH] | `elohim-storage/Dockerfile:150-154` force-clears diesel fingerprints so any migration change invalidates the full dep build (3-5 min/cycle, 15-30 min/session during schema iteration). Split migrations into a `COPY` layer ordered so dep cache survives; document the fast path. | S | Low | Schema-iteration loop speed for every dataplane dev session | quality-sweep |
| 8 | **P2PTestBuilder harness** [MECH] | `test_util.rs:67-164` `spawn_p2p_with_peers` repeats 10+ boilerplate inserts per peer (human, peer_status, rea_commitment, mishpat_commitment) across ~40 P2P test files; `P2PTestHarness` (test_util.rs:46-50) is a stub. Builder API: `.with_peer(id, household, status)` collapses 20-line setups to 3. | M | Low — test-only | Halves new-test boilerplate; one schema-coupling point instead of 40 files | quality-sweep |
| 9 | **Logging target registry** [MECH] | 274 `tracing::` calls with ad-hoc targets in two naming conventions (`"imagodei.revocation_observed"` in signals.rs, `"recovery::transport"` in p2p/blob_fetch.rs, `"elohim_storage::ssr"` in ssr.rs); no registry, so Loki queries miss and incident response is slow. Create `src/logging.rs` const targets + a documented registry with example queries. | M | Low — string constants | Trustworthy Loki investigation of the dataplane (currently degraded — zero-results already untrustworthy per ops notes) | quality-sweep |
| 10 | **ProtocolHandler trait — Shard first** [DESIGN] | `handle_behaviour_event` is 2207 lines (`p2p/mod.rs:3698-5905`) handling 13+ protocols; testing shard routing requires a live Swarm + the 154-field node. The recipe already ships: `reconcile/custody.rs:30-42` (`LocalBlobStore` + `FetchKicker` traits, consumed at p2p/mod.rs:1984-2126). Generalize: `ProtocolHandler` trait + `ProtocolContext`, land one handler at a time (Shard ~600 lines first, then Sync/EPR/EprAtom). Subsumes the cross-service "dispatcher crate" idea — keep in-crate until two consumers exist. | M per handler | Med — critical request/response path; mitigate by landing per-protocol with full integration suite between | Unit tests for protocol routing without a Swarm; 150-line PRs instead of 2200-line merge zones | rust-architect shift |
| 11 | **CommitmentFetcher pattern coverage** [MECH] | `services/commitment_fetcher.rs` (491 lines, trait + 3 impls + tests) is the house IoC exemplar but only `economic_event_emit_service` + two routes use it; `bounds_validator.rs` and `republish_epr_validator.rs` inline their own commitment fetches. Refactor `bounds_validator::validate` to accept `Arc<dyn CommitmentFetcher>` (~50 lines) + write the pattern doc. | S | Low | Teach-by-example for every future validator; mock-friendly bounds tests | quality-sweep |
| 12 | **CommandHandler dispatch** [DESIGN] | `handle_command` (`p2p/mod.rs:2807-3546`, 739 lines, 19 variants) copy-pastes the "DHT lookup → send_request → store oneshot in pending_*_map" pattern across 5+ variants, each adding a pending-map field to P2PNode. Split into dispatch + per-command handlers; collapse the 9 pending maps toward a generic request dispatcher. | M | High-ish — touches all P2PHandle→swarm calls; sequence after #10 | New P2PCommand variants without god-object constructor edits | rust-architect shift (after #10) |
| 13 | **EprValidator trait** [DESIGN] | `elohim/epr/src/lib.rs` (814 lines) is type-pure, but validation lives inlined in `services/epr_store.rs::put_epr` + `epr_service.rs` (1600+/1400+ lines) — SDKs/bridges can't validate an EPR without importing the full service stack. Extract `EprValidator` (`validate_reach`/`validate_coupling`) backed by CommitmentFetcher + PolicyEnforcement (~300 lines moved). | M | Med — moves policy-gate logic; contract must be pinned by tests first | Offline-first validation for SDKs, bridges, doorway routes, test harnesses | rust-architect shift (light /brainstorm on trait surface) |
| 14 | **P2P commons crates: shared swarm config + kad_store move** [DESIGN] | SwarmBuilder chains diverge between `elohim-storage/src/p2p/mod.rs:1659-1679` (TCP+DNS+Relay, 300s idle) and `steward/node/src/p2p/transport.rs:412-507` (TCP+QUIC, no DNS, 60s) — drift is silent and context (the genesis #1119 timeout rationale) doesn't travel. `kad_store.rs` (sled RecordStore, 370 lines) is already shared but only via re-export from elohim-storage. Extract a `SwarmConfig`/`build_unified_swarm` crate and move kad_store out. | S+M | Med — cross-crate, two runtimes must both bake | Single source of truth for transport; third consumers (iroh-adjacent work) without import-chain breakage | backlog-only until steward arc is active |
| 15 | **TimerArm select!-loop refactor** [DESIGN] | The 15-arm select! block (`p2p/mod.rs:2197-2475`) mixes core events, 13 timers, backpressure, backoff state, and drain-modulated sync; no arm is testable in isolation. `TimerArm` trait + `ArmContext` replaces 160 lines with a dispatch loop. Highest-risk item (loop semantics, drop/lock ordering) — do **last**, after #4, #5, #10, #12 have hollowed the loop. | L | Highest — core loop semantics; needs mock clocks + invariant checks | Testable timer cadence/ordering; <3000-line mod.rs end-state | backlog-only (terminal phase) |
| 16 | **Composite-root head class — bundle immutable corpus content** [DESIGN] | Head-plane cost is linear in items: every seed item mints an A-class notarized head (~4k at genesis), each a per-id conductor round-trip + election participant + adjudication candidate + divergence surface — measured 2026-08-08: the whole corpus is 24MB/3,531 files (~6KB/item, seconds of transfer), so sync cost is count-bound, not byte-bound. Mint per-corpus content-set CID roots (dag-cbor fingerprints + DNA-notarized `epr-composite` already exist; `ListDocumentsSince{corpus_digest}` is the CRDT-plane precedent) governing A2-class sub-EPRs (derived-via-link): one election converges thousands; sub-items cannot individually diverge. Partition rule = pantry temperature on the head plane: immutable/genesis corpus → bundled roots (dozens of heads); individually-authored, governance-bearing EPRs keep their own heads (reach is earned per-item; adjudication granularity follows head granularity — a contested bundle contests the whole root). Source: integrator session 2026-08-08 (backlog/content-gap-limit-cycle-blocks-convergence evidence + operator /btw). | L | Med-High — A→A2 source-of-truth class change; MUST pass p2p-design-gate; composes with (never replaces) the shipped drain levers | Head-plane cost stops scaling with corpus size; ch11-adjacent footprint lever for corpus growth | backlog-only until corpus growth or drain-rate evidence demands it |
| 17 | **content_store root/version-listing extern — a channel's registered discipline is unreachable after its first publish** [DESIGN] | No route or extern reads a content id's ROOT/create record or lists its versions: `GET /db/content/{id}` (`elohim/elohim-storage/src/http.rs`) serves only the projected current version (`update_content` overwrites the row), `GET /db/dht/{entry_hash}` (`http.rs:11143`) is a 404 stub, and `gather_content_chain` (`elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:2864`) is `fn`-private to the coordinator zome. A release channel's `adoptionDiscipline` (registered on the channel ROOT) is therefore unreachable once a second release lands — `release-lineage-probe.ts` documents "the channel ROOT's action hash cannot be independently retrieved" (2026-09-04, rung-5 c3 shift). The packager now carries `adoptionDiscipline.channelDiscipline` forward through every manifest as the workaround (`epr-release-package.ts`, commit `6e0b89285`) rather than reading it back from the chain. Sibling of item #16 (Row 16's head-plane class) and of `content-store-update-content-targets-root-not-latest-version` (the sibling fix that made `supersedes` chain-walkable — this item is the READ-side gap that fix didn't close). | S-M | Low-Med — coordinator-only public extern (no DNA-hash move) + one hash-neutral storage route | Removes the packager workaround; lets any consumer (adoption controller, a2o probes, future governance UI) read a channel's discipline/version history without re-deriving it per-manifest | rust-architect shift (p2p-design-gate: read-only extern over an existing entry type, no new entry) |
| 18 | **`projection_reconcile.rs` decomposition — four arms, one file** [MECH] | `elohim-storage/src/p2p/projection_reconcile.rs` breached the `source-file-loc-ceiling@1` hard ceiling at **7,339 lines** (finding `c933fddb1025`), growing **~190 lines/commit** across the last ten commits (5,454 → 6,474 → 7,339) under active notary-authority drain-cure work. One file carries four reconcile arms (REA, content/Leg-4, collectives, shard-custody) + the witness arm + shared rails (`MissLedger`, `HealPacing`, `HealCircuit`, routing predicates) + a 2,232-line `mod tests`. Its whole external surface is **four call sites** (`main.rs`, `liveness_contract.rs`, `p2p/mod.rs`, `tests/schema_contract.rs`), which makes this pure internal code motion: keep `projection_reconcile.rs` as the module root at its current path (protects the lone ts-rs type `ProjectionReconcileStatus`) and add a sibling `projection_reconcile/` directory of 11 submodules. Full wave plan, per-wave gates, and the byte-identical-surface invariant live in the standalone entry [projection-reconcile-loc-ceiling-decomposition](epr:projection-reconcile-loc-ceiling-decomposition). | M (5 waves, 1 cluster/commit) | Low — plain native Rust (no zome, no DNA hash, no conductor); gated by fmt + clippy + full-crate `cargo test` + `export_bindings` sha256. Only real trap: `every_heal_arm_ends_with_update_caught_up` asserts on source-text layout of the arm outcome-struct constructions | Reviewable reconcile arms (largest submodule ~900 lines vs 7.3k); a second agent can work one arm without a whole-file merge zone; ceiling ratchets down | rust-architect shift (**BLOCKED until the in-flight drain-cure batch lands on `dev` and the file is quiet one integration cycle**) |

**Below the line (deduped, not ranked — need /brainstorm before they're backlog-ready):** views.rs codegen macro (elohim-views ↔ `elohim-storage/src/views.rs` 3-file dance — Analysis 3 F3) and `bridges/doorway-storage-api` StorageClient facade (Analysis 3 F5; depends on #3 landing first). The Dataplane SDK is deliberately excluded here — it is Artifact 2.

**Sequencing note:** items 1-9 are independently landable in any order (1, 4, 5, 7 are afternoon-sized). Items 10 → 12 → 15 are a strict dependency chain on `p2p/mod.rs`; do not run them concurrently with each other or with #6 (same file, same struct — merge-conflict zone). The custody-reconcile trait pattern (`reconcile/custody.rs`) is the proven template for 10/12/15 and should be cited in every extraction PR. Item 18 is a *different file* in the same directory, so it is concurrency-safe against 10/12/15 with one exception — both touch `p2p/mod.rs`'s `pub mod` block; land 18's waves and the 10/12/15 chain in separate commits. 18's own precondition is temporal, not structural: do not start it while the drain-cure batch is in flight on `projection_reconcile.rs` (block moves are what git merges worst).

## Row 16 pickup agenda (brainstorm handoff, 2026-08-08 integrator session)

Row 16 carries the design take; this is the deep-dive agenda for the /brainstorm
that picks it up. Second motivation beyond footprint scale: **verification
velocity** — the fleet-quiesce gate and every recording wait out per-head churn
after each deploy (45-min gate windows, multi-hour soaks, 300s sweep quanta);
fewer governing heads ⇒ faster post-deploy quiescence ⇒ faster CI loops. This
is slowing edge Dataplane Validation today.

1. Root granularity — per-corpus / per-collection / per-author-epoch; what is
   the re-declaration blast radius of one sub-item change under each?
2. Reach/governance projection — per-item earned reach under a root head: does
   the root's reach gate the bundle, or do sub-items carry reach as A2-derived
   attributes the projection enforces?
3. Adjudication semantics — can a peer contest a sub-item without contesting
   the root? What does divergence adjudication mean at root granularity?
4. Migration — how do ~4k existing A-class heads collapse into roots without a
   partition (namespace-atomic like the iroh flip? lineage from old heads)?
5. Interaction with the shipped drain levers (fan-out, no-chain backoff,
   courier widening): roll-up shrinks N, levers raise throughput per N —
   measure both on the same gauges (blocked_by, healedTotal, gate wall-clock).
6. Falsifiable velocity claim — predict gate-quiesce wall-clock as f(head
   count); test by bundling ONE corpus first and measuring the delta on the
   next deploy's gate window.
7. MANDATORY at pickup: p2p-design-gate (entry types / sync semantics change);
   check Lamad DNA entry-type headroom.

### 2026-08-08 evidence pass + operator steering (code-review brainstorm)

Two-explorer evidence pass (sweep mechanics; classification/trust audit) after the
operator asked why ~25MB quiesces in ~2.5h. The arithmetic closes: ~3,469 A-class
heads ÷ 200/tick (`WITNESS_MAX_PER_TICK`, projection_reconcile.rs:120) ÷ 300s
cadence = 100min floor, + ~20min conductor restart churn, + serial heal arms whose
budgets sum to 555s > the 300s tick (effective cadence ≥600s under saturation,
`SkipInFlight`), + gate sustain needing ≥2 heal cycles. Bytes are irrelevant.

**Named anti-pattern: flat-trust per-item head governance.** Three layers:

1. **Classification hardcodes past every stakes axis that exists.** Seeder mints
   `reach: 'public'` as a literal (genesis/seeder/src/seed.ts:817; import CLI
   `visibility: 'public'` import.ts:903; bulk route defaults omitted reach to
   `'commons'` — the WIDEST tier, http.rs:4842). The cheap tier is built and free:
   `Reach::Private/SelfScope` → `DirectOnly`, zero P2P (epr_store.rs:788),
   `is_floor_allowed()` bypasses standing (epr_kind.rs:96), excluded from
   `DISTRIBUTION_SAFE_REACH` inventory/head-record surfaces (content_diesel.rs:1663).
   Attestation-derived reach (`trust.service.ts:109-154`, `enrich-trust` CLI) is
   implemented and has no caller on the seed path.
2. **The head plane never got the O(changes) cure the CRDT plane got.**
   *(PARTIALLY STALE 2026-08-15 — the batched READ half landed: `resolve_content_heads_local`
   + `resolve_canonical_elections` externs (T1 `23a887d50`, T1b `9b3979d3b`) with the
   `head_batch_resolver` storage seam (T3a `d75bfc82d`), Background admission class.
   Still per-id: the DECLARE half (lever L8 below), and `validate_carried_head_record`
   by design.)* Original premise: `resolve_content_head_local`,
   `resolve_canonical_election`, `validate_carried_head_record` were all 1-id-per-WS-RT
   (~0.75s), while the batching pattern is already accepted at the same zome surface
   (`batch_get_content_by_ids` lib.rs:4928) and the peer plane batches
   2000/request (`ProjectionInventory`). `ListDocumentsSince{corpus_digest}`
   (sync_protocol.rs:115, sync_round.rs:131) is the shipped precedent — its module
   doc names the defect generically: "cost O(peers × corpus) instead of O(changes)."
3. **Trust is not flat in the sweep — it is absent.** `verify_trust_context` is only
   wired to transport handshake; `TrustService::handle` is a stub; trust cache is
   peer-scoped flat-TTL 3600s; `authorize_reach_for_human` recomputes
   O(stewards × collectives) per content access with no content-scoped memo; storage
   `dev_mode` is threaded through the trust seam and deliberately inert
   (p2p/mod.rs:987-1010 — the drilled socket for a network trust mode). The
   2026-04-30 trust-compute-gradient brainstorm designed the gradient
   ("bulk-verify amortization for known-good signer streams") and §3.1 admits
   "today's substrate is binary; the gradient is absent" — still true in prod.

**Operator steering recorded (2026-08-08):**
- Head-election STAYS — it is the convergence mechanism and is structurally a
  reach-earned process; verification gossip is acceptable. The smell is the
  trust-flat electorate: high-standing seed-pair heads should converge the commons
  fast; instead every peer pays unknown-signer cost per head.
- Trust verification should be a signed snapshot up-front, drilldown-verified at
  access time priced by stakes (low-stakes claim → cheap; high-stakes commons →
  expensive); content caches its trust state; subsequent peers validate only NEW
  trust edges (atomic delta), full recompute on demand — consistent with §4.2
  derived-view-never-stored-score (amortization + provenance, not authority).
- Dev/staging/genesis networks need a declared trust collapse ("simulacra") — the
  §10.4 stage-gate degenerate point, consuming the inert `dev_mode` socket — so
  fixture bootstrap doesn't pay live-network compute. Live authoring is
  local-first/private-hub first; peer-network compute curves apply at
  post/graduation, not at genesis (NO draft→graduate content path exists today;
  the gate prescribes Category B drafts, nothing implements them).

**Lever ranking (composes; each independently landable):**
- **L1 (days, coordinator-only → `update_coordinators` hot-swap, no DNA hash move):**
  batched externs `resolve_content_heads_local(Vec)` + `resolve_canonical_elections(Vec)`
  — collapses heal arm A + obey-probe from N RTs to ⌈N/page⌉. Then raise
  `WITNESS_MAX_PER_TICK`/25ms pacing (they exist to protect the conductor from
  serial RT storms batching eliminates).
- **L2 (days):** head-plane corpus digest — port `ListDocumentsSince{corpus_digest}`
  to head discovery; converged peer answers one hash, O(changes) steady state.
- **L3 (hours, seed-path):** corpus-scoped reach/provenance expression in seeder +
  import CLI (kill the two literals); flip bulk-route default off the widest tier.
- **L4 (Row 16 proper):** composite roots — `epr-composite` container already ships
  (9 paths bundle ~3,460 refs); the change is A→A2 on referents.
- **L5 (architecture):** trust-priced election gossip — high-standing peer signs the
  verified head-set snapshot (dag-cbor CID); peers accept-with-provenance, verify
  deltas only; stakes-priced drilldown; simulacra networks price verification ~0
  via the stage-gate axis. This is the §3.2 gradient finally consuming a standing
  signal on the head plane.
- **L7 (position, operator directive 2026-08-08):** `HcClient::call_zome` is
  uncancellable — NOT accepted as a keystone constraint. Tactical: in-wasm deadline
  in the batch externs. Strategic: pragmatic dual-track, no ideological ordering —
  a well-reasoned, tested PR to holochain dev adding call deadlines/cooperative
  cancellation as a foundational capability (upstream idioms, generally useful, no
  elohim implementation imposed; pro-social AND self-interested — merged = fork
  maintenance we stop carrying), alongside the fork lineage we already carry
  (kitsune2 `store_slice_hash` patch) which delivers on our timeline regardless of
  review pace. Decision memo weighs both by delivery need; the substrate owns its
  scheduling floor either way. Planned as T13 in the head-plane trust-gradient plan.
- **L8 (new lever, 2026-08-15, shift admission-throughput-to-banked-ch06):** batched
  DECLARE extern — the write half L1 never asked for. 7 per-id
  `declare_canonical_content_head` RTs in `head_adoption.rs` (contest arms,
  spin-discharge, adopt-before-author, declare_peer_head) remain 1-admission +
  1-WS-RT per candidate under the 200/tick sweeps. Not free: declares COMMIT
  (Update + links), so a batch of N commits needs first-class partial-failure
  semantics (mirror T1b `attempted/unattempted/stop_reason`), in-wasm budget
  (`conductor-call-is-uncancellable`), and the T2 rollout ordering (zome hot-swap
  verified on alpha BEFORE the storage caller lands; `FallbackLatch` covers the
  window). TRIGGER (falsifier from the 2026-08-15 shift): after sweep declares
  ride Background class + ghost-witness probes batch, matthew still gate-sheds
  with declare-attributed failures — `elohim_admission_shed_total{class="background"}`
  climbing while `content_canonical_link_minted` stalls ⇒ declare VOLUME (not
  wait class) is the residual bottleneck; take this lever.
- **L6 (gate update):** p2p-design-gate capacity model is static and count-blind —
  no dimension for conductor-RTs × sweep-cadence × election candidacy (the costs
  that bind); conflates Holochain DHT + Kad plane under one "~3000 entries" number
  (exceeded at seed time: ~3,469 heads); Category A's `dht_anchor_hash NOT NULL`
  contract is violated-by-design for hours by bulk seed. Gate needs a head-plane
  cost question at classification time and a network-stakes/stage axis.

---
