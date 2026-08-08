---
id: head-plane-trust-gradient-program-plan
title: Head-Plane Trust-Gradient Program — batched externs, corpus digest, priced verification, gate amendment (L1–L6)
status: Draft
class: protocol-canonical
topic: [dataplane, head-plane, trust-gradient, simulacra, batch-externs, corpus-digest, verification-pricing, p2p-design-gate, quiesce]
domain: D5
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/trust-as-efficiency-signal.md
sprint: row-16-pickup
cites:
  - genesis/data/timeline/backlog/arch-dataplane-refactor-backlog.md
  - trust-as-efficiency-signal | CANONICAL principle this program finally implements on the head plane — trust must measurably reduce propagation/verification overhead | sha256:40b8e3d166c935a7 | path: genesis/docs/content/elohim-protocol/architecture/trust-as-efficiency-signal.md
  - trust-compute-gradient-brainstorm | Design source: §3.2 seven-layer gradient (bulk-verify amortization), §4.2 standing-derived-never-stored, §7.2 standing-policy manifests, §10.4 stage gates (Simulacra home) | sha256:89c493c73ff6b06b | path: genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md
  - adam-slow-link-write-guard-saturation | The constraint of record: conductor write-guard saturation forbids raising sweep caps — batch round-trips at flat read-permit cost instead | sha256:556142ddd510a091 | path: genesis/docs/content/elohim-protocol/history/2026-07-20-adam-slow-link-write-guard-saturation.md
  - sdk-promise-substrate-program-plan | Owns the standing_projector T19 writer that turns the inert trust seam into live gradient behavior; also the ListDocumentsSince rollout precedent | sha256:980a1582890452b4 | path: genesis/docs/superpowers/plans/2026-07-25-sdk-promise-substrate-program-plan.md
---

# Head-Plane Trust-Gradient Program (L1–L6)

**Handoff contract:** this plan is written for a Fable orchestrator running an
implementation sprint. Every task carries a tier assignment (Opus = design-bearing /
judgment; Sonnet = well-specified legwork), a read/write set (tasks whose write-set
intersects another's read-set must not run concurrently), and a DoD that includes the
touched tree's gate clauses. The orchestrator holds sequencing, review, and the
alpha-gated legs; workers never push (commit-only; integrator pushes).

## 1. Why (the framing that governs scope)

Alpha quiesces ~3,469 A-class content heads in ~2.5h after a deploy. The evidence pass
(backlog Row 16 addendum, 2026-08-08) closed the arithmetic: 200 heads/tick × 300s
cadence × 1 uncancellable conductor WS round-trip per head, with trust consulted
nowhere. **The operator's framing (2026-08-08): 2.5h of compute earning reach for 4k
EPRs may be roughly RIGHT for a live enforced network.** The deliverable is therefore
not "make sync fast" — it is the **trust gradient as a first-class declared axis**:

- **Simulacra stage** for dev/staging/genesis fixtures → agile development, cheap
  story-forecasting, fast CI loops;
- **Enforced stages** for the live runtime, where friction, negotiation, commitments
  and trust-building are the product — the generational, human-centric design;
- The same machinery, priced by declared stakes, never a dev hack beside the real path.

Canonical seeds this composes from (do not fork): `trust-as-efficiency-signal.md`
(CANONICAL principle), `2026-04-30-trust-compute-gradient-brainstorm.md` (§3.2 gradient,
§4.2 standing-is-derived-never-stored, §7.2 manifests, §10.4 stage gates).

## 2. Evidence corrections that bind the design

The architecture pass (rust-architect, 2026-08-08) corrected three premises — these are
binding on every task below:

1. **`trust_verification::verify_trust_context` has ZERO call sites.** The transport
   handshake is inline at `p2p/mod.rs:5721-5751` hardcoding `reach_ceiling:"public"`;
   the four helpers ignore `_hc_client` and return `Ok(vec![])`. Standing is a constant
   until `standing_projector` T19 lands its writer (`standing_projector.rs:9-13`).
   **Land the trust seam inert; never claim gradient behavior before T19.**
2. **Two unrelated `dev_mode`s.** elohim-storage's (`p2p/mod.rs:990,:1004`) is inert
   (no supplier, no consumer). doorway's `DEV_MODE` (`config.rs:89`) is LIVE,
   auth-permissive, and `true` fleet-wide. **`NetworkStage` must never derive from any
   `DEV_MODE`** — config key is `ELOHIM_NETWORK_STAKES`; durable home is the
   standing-policy manifest; unknown/absent ⇒ `Bootstrap` (fail-closed, never toward
   the cheapest stage).
3. **`WITNESS_ITEM_DELAY` is vestigial** (25ms sleep inside concurrent tasks = 5s/tick
   of pure sleep, not protection). Nothing is "replaced"; a closed-loop budget is
   **introduced**.
4. **`HcClient::call_zome` has no timeout and no cancellation** (`hc_client.rs:405`).
   A caller timeout leaves the conductor executing with nobody listening. The batch
   extern therefore carries its **own in-wasm deadline** and returns partial results.
   *Qualified by the T13 spike (2026-08-08,
   `genesis/docs/superpowers/specs/2026-08-08-conductor-call-deadline-capability-spike.md`):
   the client API already declares a timeout (`CallZomeOptions::timeout`) but enforces
   it client-side only — the deadline is a missing MESSAGE, not a missing concept; and
   most slow-call cost is DB-permit queue-wait that IS cancellable-on-drop (tokio
   semaphore permits return) — only the wasm body is non-interruptible. The in-wasm
   deadline design above stands unchanged; the qualification reshapes the upstream
   ask, not this sprint's tactic.*

**Position on the uncancellable call (operator directive 2026-08-08): this is NOT a
keystone constraint we design around forever.** A scheduling floor owned by an upstream
we don't control is itself a capture vector in a capture-resistant compute substrate.
Two-track position, pragmatic and pro-social — we need technology that delivers the
vision and will do what it takes to get there: (tactical, this sprint) the in-wasm
deadline in T1 — redesign how we use it, bounded from inside; (strategic, T13 spike)
both instruments on the table with no ideological ordering. A well-reasoned, tested PR
to the holochain org's dev branch (and/or holochain-client-rust) adding app-interface
call deadlines/cooperative cancellation as a FOUNDATIONAL capability — upstream's
idioms, generally useful, never imposing elohim's implementation — is pro-social AND
self-interested (a merged capability is fork-maintenance we stop carrying). The
conductor fork we ALREADY carry (the kitsune2 `store_slice_hash` patch ships on that
lineage, operator-gated on image build) delivers on OUR timeline regardless of upstream
review pace. T13's decision memo weighs both by delivery need; the substrate owns its
scheduling floor either way.

**Write-guard constraint** (`2026-07-20-adam-slow-link-write-guard-saturation.md`): the
head sweep is a READ path harmed by queueing behind kitsune2's write guard. Batching
collapses round-trips at flat read-permit cost. **The per-tick cap (200) and concurrency
do not rise — only per-round-trip yield.** Raising caps naively is the forbidden move.

## 3. Architecture (settled by the design pass — implement, don't re-litigate)

### L1 — Batched externs + closed-loop pacing

**Zome** (`content_store/src/lib.rs`, coordinator-only, read-only → ships via
`update_coordinators` hot-swap, NO DNA-hash move, gated `ALLOW_COORDINATOR_UPDATE`):

- `resolve_content_heads_local(BatchResolveHeadsInput) -> BatchResolveHeadsOutput`
- `resolve_canonical_elections(BatchResolveElectionsInput) -> ...Output`
- Input: `ids: Vec<String>`, `budget_ms: Option<u32>`. Output: `resolved` (attempted,
  in order; `None` = honest local absence, same contract as single-id twin),
  `unattempted` (never started — NOT failures), `elapsed_ms` (in-wasm wall time).
- Constants: `BATCH_ID_CEILING: 256`, `BATCH_BUDGET_DEFAULT_MS: 4_000`,
  `BATCH_BUDGET_CEILING_MS: 15_000`. Per-id work delegates unchanged to
  `resolve_content_head_inner(id, GetStrategy::Local)` / `select_election(...)`.
- **Deadline bound is mandatory** — verify `sys_time()` per-iteration cost first;
  fallbacks: clock-check every k=16 ids; or `BATCH_ID_CEILING=64` alone. Never ship
  unbounded.
- **NOT batched:** `validate_carried_head_record` (per-record crypto on
  caller-supplied bytes; hostile carrier would eat a batch budget; call rate already ~0).
- **Contract revision (operator review 2026-08-08, BINDING — supersedes the
  `resolved`/`unattempted` output shape above; T1 as first landed used the old shape
  and is revised before any storage caller exists):** per-item outcomes are typed and
  an admitted id's failure may NEVER discard accumulated results — no post-admission
  `?`, including the `sys_time()` calls. Output: `schema_version: u16`, `attempted:
  Vec<{id, outcome: Resolved(Option<T>) | Failed{reason: typed vocabulary, phase}}>`,
  `unattempted`, `stop_reason: Option<...>`, `elapsed_ms`. `Resolved(None)` keeps the
  single-id local-absence contract. Deterministic Refused/Unverifiable failures record
  per-item and processing CONTINUES; shared Timeout/TransportError records the current
  failure and STOPS (untouched tail → `unattempted`) — never hammer a saturated
  conductor; a DB-read-permit timeout on local `get_links` is retryable backpressure,
  never absence. Coordinator-side `content.id == requested_id` validation is mandatory
  NOW (a link to another id's valid Content produced a silent wrong-content answer);
  integrity-zome link validation is a separate hash-moving lineage change (backlogged:
  `content-store-integrity-link-validation-gap`). The failure-disposition predicate
  registers in the content_store seam registry BEFORE implementation.

**Storage:** new `services/head_batch_resolver.rs` on the `commitment_fetcher.rs`
template (trait + prod impl + `Mock*` NOT behind cfg(test), one file):
`HeadBatchResolver { resolve_heads, resolve_elections }` returning outcomes with
`resolved: Vec<(String, Answer<T>)>` (seam-contracts C4), `unattempted`,
`queue_wait: Duration` (= observed RTT − extern `elapsed_ms` — the free
conductor-pressure signal), and `unsupported: bool` (unknown-function from a
not-yet-hot-swapped peer → fall through to single-id once, log once, never retry-loop).
**Fallback rule revised with the contract revision above:** single-id fallback ONLY on
unknown function or explicit `schema_version` mismatch; per-item `Failed` outcomes map
to `Answer::Unreachable` with the typed reason (never single-id fallback); call-level
infrastructure failure returns ALL ids to pending with backoff.
Typed wrappers in `services/conductor_writes.rs` beside `call_resolve_content_head_local`
(:628), `rmp_serde::to_vec_named` encoding.

**Arm consumption:**
- Arm 2 `heal_content`: keep `resolve_pipeline`; unit changes id → batch;
  **fanout drops 8 → 2** (concurrent conductor calls must not rise). `unattempted` →
  back to `GapTracker::pending`, NEVER `mark_failed` (would burn `MAX_RETRIES` and
  poison the `MissLedger`). `heal_backoff::should_replay` partition runs before batch
  composition, unchanged. `HealPacing::attempt_timeout` (15s) stays strictly above the
  extern budget ceiling.
- Arm 4 `adopt_deferred_heads`: two-phase — batch-probe elections over the whole
  200-cap slice (2–4 RTs replace 200), then per-item declare only for
  has-election ids. `compose_adopt_slice` unchanged. Live failure split
  (`no_local_chain: 887/891`) says probes consume the slots today; the cap stays 200.
- Arms 5/6 (witness): same two-phase restructure, lower priority.

**Pacing:** delete `WITNESS_ITEM_DELAY`; add `AdaptiveBatchBudget` (AIMD) in
`p2p/reconcile_rails.rs` beside `DispatchBudget` (concurrency vs batch-size — both
stay). Pure predicate `next_size(current, queue_wait, threshold, floor, ceiling)`;
defaults floor 8, ceiling 128, threshold 2s, +16 / ×0.5. AIMD over token bucket:
no refill-rate guess; adam (WAN) converges small, matthew (LAN) large, same constants —
the shem fixture heterogeneity is preserved, not laundered. Fold `WITNESS_MAX_PER_TICK`
+ `WITNESS_SWEEP_BUDGET` into `HealPacing`. New counters:
`elohim_head_batch_{calls_total,ids_total,unattempted_total}`,
`elohim_head_batch_queue_wait_ms` (histogram — the probe that decides whether the
conductor-fork patch stays warranted), `elohim_head_batch_size` (gauge).

### L2 — Head-plane corpus digest

- Extract the fold ONCE: `digest_of_entry_lines(Vec<String>) -> String` in
  `reconcile_rails.rs`; `sync_round::corpus_digest` becomes a delegate. **Fixture test
  pins the pre-refactor sync digest byte-for-byte** (wire-visible; one byte silently
  disables the InSync shortcut fleet-wide). NOT in seam-contracts (its
  no-heavy-deps boundary test forbids `sha2`).
- `head_corpus_digest(conn, ctx)` in `db/content_diesel.rs` beside
  `list_content_anchor_inventory`, **reusing `DISTRIBUTION_SAFE_REACH`** (never
  restate the tiers). Derived on demand, never stored — invalidation is vacuous
  (sub-ms over ~3.5k rows). If profiling ever demands a cache, key =
  `(count(*), max(updated_at))` over the same relation, never a dirty flag.
- Wire: **additive optional field** on `ViewFederationRequest`
  (`head_corpus_digest: Option<String>`, `skip_serializing_if` keeps `None` bytes
  identical → dedup key unchanged) + additive `Option<bool> in_sync` on the inventory
  payload. **Never a new `ViewKind` variant** (externally tagged, no other-escape —
  pre-cure peers would fail decode). Rollout rule from `ListDocumentsSince`: responder
  ships fleet-wide one release before any sender constructs the field. Three compat
  tests: round-trip, old-bytes-on-new-struct, new-bytes-on-old-struct.
- **Amber-window honesty (T5 challenge pass):** enabling the requester is necessary
  but not sufficient to carry a digest. `head_corpus_digest_readiness` reads the exact
  distribution-safe relation in one SQL snapshot and returns `Amber{pending}` while
  any row still lacks a DHT anchor; the requester omits the optional field and an amber
  responder abstains with `in_sync: None`. Only `Ready` advertises. A matching responder
  compares before enumeration and returns an empty page plus the honest total, so the
  claimed zero-cost shortcut actually avoids page construction and wire transfer.
- **L2 before L5:** a peer whose own digest equals the one inside a signed snapshot is
  already in sync → accept-with-provenance at zero cost. The digest is the snapshot's
  self-description.

### L5 — Trust-gradient seam (`elohim/elohim-storage/src/trust/`)

Module map: `stage.rs` + `pricer.rs` (PURE — no diesel, no tokio, no clock, no env;
graduation to `crates/trust-gradient` is a `git mv` when a second runtime needs it —
named C13 successor), `snapshot.rs` + `memo.rs` (stateful), `.epr-meta` rail, `mod.rs`.

- **`NetworkStage`**: `Simulacra < Bootstrap < Coordinated < Enforced` (ordering is
  semantic, pinned by property test). `Simulacra` NEVER a default — explicit
  declaration only. `StakesProvenance` (Manifest{cid} | OperatorConfig |
  BootstrapDefault) travels with every verdict. `StakesResolver` trait;
  `ManifestStakesResolver` reads the EXISTING `ManifestRegistry` (one more field on the
  standing-policy manifest — no new entry type, registry, or manifest kind);
  `FixedStakesResolver` test double.
- **`VerificationPricer`**: `price(PricingInput) -> PricedVerification` where depth ∈
  {AcceptWithProvenance, DeltaVerify, FullChain}. Input takes TYPED
  `elohim_epr::Reach` (the `&str` path maps unknown → most-permissive via
  `reach_level_index` — a pricer must never inherit that) and
  `services::standing::Standing` (derived view; `Unknown` everywhere until T19).
  `InertPricer` = always FullChain, reason `PricerInert` — today's behavior exactly.
  **THE safety invariant: `floor != FloorClass::None` ⇒ `FullChain` at EVERY stage
  including Simulacra** — property test over
  `NetworkStage × FloorClass × Reach × Standing`. Floor classes: Constitutional,
  LocalRelationship, CounterEvidence.
- **`HeadSetSnapshot`** (signed transient, Path C, NO DHT entry type): carries
  `corpus_digest` (the L2 join), entries, `signer_agent_cid`, `trust_epoch` (named
  clock derived from the signer's attestation/citation edge set — regression REFUSES,
  C2), `edge_set_digest`. CID via `epr_codec::encode_epr_head` pattern verbatim
  (dag-cbor 0x71). Evidence-not-authority (C5): receiver re-derives everything it acts
  on. `SnapshotVerdict::Refused(SnapshotRefusal)` implements `ReasonLabel`.
- **`VerificationMemo`** — who verified, when, through which lens
  (`lens_manifest_cid`), depth, epoch, edge digest. **NO NUMERIC FIELD, EVER** (§4.2
  derived-not-stored; the .epr-meta rail exists to catch `score:` drift). Primary
  invalidation = `invalidate_for_subject` on FeedbackSignal (T19 hook); TTL eviction is
  backstop only. Bites in `authorize_reach_for_human` (`epr_service.rs:376` — the
  `familiar`+ stewards×collectives fan-out recomputed per request today).
- **IoC**: NOT via `Services` (all-concrete registry). Borrowed-context pattern per
  `AdoptContext<'a>`: `TrustGradient<'a> { stakes, pricer, memo }` +
  `TrustGradient::inert()` (Bootstrap · InertPricer · no memo = today's behavior,
  provably behavior-neutral diff). `authorize_reach_for_human` gains a
  `trust: &TrustGradient<'_>` param; both callers pass `inert()` at landing.
  Crate-wide `rg 'authorize_reach_for_human'` incl. tests before committing.
- **Prerequisite fix**: `http.rs:5884-5890` constructs a fresh `EprService` per
  request — the memo store must be process-lifetime `Arc<dyn VerificationMemoStore>`
  from `main.rs`, threaded to both call paths. Own task, not a footnote.

### Rails (seam-registry + .epr-meta)

- `elohim/elohim-storage/seam-registry.yaml`: 8 new rows (AdaptiveBatchBudget::next_size,
  digest_of_entry_lines, head_corpus_digest_in_sync, StakesResolver::stage_for,
  VerificationPricer::price, PricingReason, SnapshotRefusal, SnapshotVerdict) +
  extend `seamLocus`; `contractTests` explicit `null` + `gapNote` where absent.
  content_store zome registry: `batch_deadline_admission` row.
- New `src/trust/.epr-meta` (nested, omits `covers:`): rules `no-stored-score`,
  `stage-cannot-cheapen-the-floor`, `dev-mode-is-not-network-stage`,
  `pure-half-stays-pure` — full YAML in the architecture record (§5.3 of the design
  pass output, reproduced in the task spec for T-RAILS).
- One rule added to existing `src/.epr-meta`: `conductor-call-is-uncancellable`.

## 4. Task breakdown (sprint handoff)

Legend: tier = who does the legwork; orchestrator (Fable) reviews every diff, owns
sequencing + alpha legs. Gates per CLAUDE.md: elohim-storage =
`RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build/test/clippy/fmt` with
CARGO_TARGET_DIR at the pool slot; DNA workspace = plain cargo + sweettest via CI.
`cargo check` never verifies a task — run `cargo test`, echo `EXIT=$?`.

| ID | Task | Tier | Depends | Reads | Writes |
|----|------|------|---------|-------|--------|
| T1 | Zome batch externs + types + `batch_deadline_admission` predicate + zome unit tests + sweettest scenario. FIRST: verify `sys_time()` cost; pick bound (per-id / k=16 / ceiling-64) and record which in the commit body | **Sonnet** (spec is complete; escalate to Opus only if all three bounds fail) | — | content_store lib.rs, lamad_types | content_store lib.rs, lamad_types, sweettest |
| T2 | Hot-swap rollout: `[build:dna]` push → `ALLOW_COORDINATOR_UPDATE` path → verify externs answer on alpha before ANY storage caller lands | **Orchestrator** (+ ci-observer watches) | T1 | Jenkins, alpha probes | — |
| T3 | `HeadBatchResolver` (trait+prod+mock) + `conductor_writes` wrappers + arm 2 batch consumption (fanout 8→2) + arm 4 two-phase restructure + `AdaptiveBatchBudget` + fold `WITNESS_*` into `HealPacing` + delete `WITNESS_ITEM_DELAY` + C8 counters | **Opus** (hottest loop; `resolve_pipeline` in-order/apply-half invariants; MissLedger poisoning trap) | T1 (compiles against types), T2 (before enabling live path) | projection_reconcile.rs, head_adoption.rs, heal_backoff.rs | head_batch_resolver.rs (new), conductor_writes.rs, projection_reconcile.rs, reconcile_rails.rs, metrics.rs |
| T4 | `digest_of_entry_lines` extraction + byte-identical fixture test + `head_corpus_digest` + additive wire fields + responder + 3 compat tests | **Sonnet** | — (disjoint from T3 except reconcile_rails.rs — T3 lands first OR coordinate the one-file merge) | sync_round.rs, content_diesel.rs, elohim-views/shared.rs | reconcile_rails.rs, sync_round.rs, content_diesel.rs, view_federation.rs, elohim-views/shared.rs |
| T5 | Requester behind default-off `ELOHIM_HEAD_CORPUS_DIGEST`; amber readiness suppresses partial digests; flip only after fleet confirms responder | **Orchestrator** | T4 deployed | alpha probes | config.rs, main.rs, projection_reconcile.rs, view_federation.rs |
| T6 | `src/trust/` landed INERT: stage.rs, pricer.rs, snapshot types, `TrustGradient::inert()`, floor property test, `authorize_reach_for_human` param sweep | **Sonnet** implement; **Opus reviews the floor property test** (the safety keystone) | — (disjoint from T3/T4) | epr_service.rs, epr_kind.rs, standing.rs, manifest_registry | src/trust/* (new), epr_service.rs, http.rs (param thread) |
| T7 | Fix per-request `EprService` construction; process-lifetime memo store `Arc` from main.rs | **Sonnet** | T6 | http.rs, main.rs, epr_service.rs | http.rs, main.rs, epr_service.rs |
| T8 | Snapshot mint/verify/delta + `SnapshotSource` + `trust_epoch` C2 guard + additive wire carry | **Opus** (C2 ordering semantics, refusal taxonomy, epr_codec CID discipline) | T4 (digest join), T6 (types) | epr_codec.rs, trust/*, elohim-views | trust/snapshot.rs, view_federation.rs, elohim-views/shared.rs |
| T9 | Memo wired into `authorize_reach_for_human` + `invalidate_for_subject` hook stub (correct-but-dormant until T19) | **Sonnet** | T6, T7 | trust/memo.rs, epr_service.rs | trust/memo.rs, epr_service.rs |
| T10 | `ManifestStakesResolver` + `ELOHIM_NETWORK_STAKES` + Simulacra activation on genesis fixtures + seeder per-fixture reach field (`@requires:alpha-cluster-6peer` for the activation leg) — **trap: per-fixture opt-in, land AFTER T5 so the digest baseline is stable; a blanket seed.ts flip silently resizes the corpus L1/L2 are measured against** | **Opus** + operator gate | T5, T6 | manifest_registry, seed.ts, import.ts | trust/stage.rs, seed.ts, import CLI, manifests |
| T11 | p2p-design-gate amendment (§5 below) | **Opus** (gospel-adjacent skill surface) | evidence in this plan | SKILL.md | .claude/skills/p2p-design-gate/SKILL.md |
| T12 | seam-registry rows + `.epr-meta` rails (YAML fully specified in design record) | **Sonnet** | T3, T6 landed (rows cite real files) | design record | seam-registry.yaml ×2, src/trust/.epr-meta, src/.epr-meta |
| T13 | Conductor capability spike: fork patch on the existing lineage (delivery on our timeline) + contribution-grade upstream PR draft (general capability, upstream idioms, tested, no elohim-specific imposition — pro-social and cuts our fork maintenance if merged) + decision memo weighing both by delivery need | **Opus** (spike, design-bearing; upstream PR + fleet deploy both operator-gated) | — (independent; informs post-sprint) | conductor fork repo, hc_client.rs, upstream dev branch | upstream PR draft + fork branch + design note |

**Concurrency guidance:** {T1}, {T4}, {T6} are mutually disjoint start-points (T3/T4
share only `reconcile_rails.rs` — sequence that one file). T3 is the highest-risk diff;
give it the p2p protocol test suite between arm restructures. All work stays in
/projects/elohim; commit path-limited; one push per batch via the integrator.

**Per-task DoD:** tree's gate clauses green from a clean state (fmt, clippy -D warnings,
cargo test with echoed exit code; elohim-app untouched → no app gates), new tests listed
in the commit body, seam-registry row present for any new decision predicate, and — for
T3/T4 — the wire/fixture tests named above. A checked box is a claim; the orchestrator
verifies from evidence.

## 5. T11 spec — p2p-design-gate amendment (performance concerns)

Amend `.claude/skills/p2p-design-gate/SKILL.md` (managed surface — cite tooling if
enveloped):

1. **Split the capacity table into the two planes it conflates** (Holochain DHT entries
   vs libp2p Kad records vs local head rows), and correct the stale "~3000 before
   degradation" line with the measured reality (~3,469 A-class heads live at genesis,
   plus per-item Kad records on a 15s drain ticker).
2. **New Step 1.5 — Head-Plane Cost Budget** (between classification and addressing):
   for any Category A/A2 entity, require: (a) expected item COUNT at seed and at
   1yr; (b) the per-item recurring cost formula it joins (conductor RTs × sweep
   cadence × election candidacy × adjudication surface — cite the Row 16 numbers);
   (c) above ~500 items, a bundling justification (composite root / A2-via-link /
   corpus digest) or an explicit operator sign-off on the head-plane cost. A design
   that passes classification but adds thousands of per-item heads must say what that
   does to quiesce.
3. **New axis — Network Stakes Stage**: every entity/predicate declares which
   `NetworkStage`s it must behave under, and which of its costs are stage-priceable vs
   floor-protected (Constitutional / LocalRelationship / CounterEvidence never cheapen).
   Cross-link `trust-as-efficiency-signal.md` §6 and this plan.
4. **Honesty fix on Category A**: document the bulk-seed amber window
   (`dht_anchor_hash` NULL until the witness sweep reaches the row) so the "MUST be NOT
   NULL" contract states WHEN it holds, instead of being violated-by-design for hours.
5. Add the uncancellable-conductor-call teaching (one paragraph, pointing at the
   `.epr-meta` rail) to the anti-pattern catalog.

## 6. Measurement (falsifiable, per Row 16 agenda item 6)

- Predict gate-quiesce wall-clock as f(head count, batch size); record prediction
  before each alpha deploy leg (T2, T5, T10) and compare on the next deploy's
  `fleet-quiesce-gate` window.
- `elohim_head_batch_queue_wait_ms` histogram: AIMD must show adam converging to a
  smaller batch than matthew (heterogeneity preserved). `PTxnGuard` rate must stay
  FLAT while quiesce falls (proves we cut round-trips, not raised pressure).
- Honest ledger: quiesce improvement is NOT measurable on household-nodes (WAN
  write-guard saturation is a shem-fixture property); state alpha numbers as measured,
  CI numbers as mocked-path coverage. No gradient behavior claim before T19 lands a
  `standing_view` writer.

## 7. Out of scope (captured, not absorbed)

- The kitsune2 `store_slice_hash` write-guard fix (conductor-fork patch) — landed in
  fork, operator-gated on image build; tracked in the 2026-07-20 history doc.
- Row 16 proper (A→A2 composite-root migration of the ~3.5k corpus) — this program
  makes it cheaper to defer (digest + batching first); pickup agenda stands in the
  backlog cluster.
- `standing_projector` T19 writer — prerequisite for live gradient behavior, owned by
  the SDK-promise substrate program plan.
- Inventory cap-2000 truncation below true count (adam perpetual re-gossip lead
  hypothesis) — same cure family as L2; file under the backlog cluster if not already
  rowed.
