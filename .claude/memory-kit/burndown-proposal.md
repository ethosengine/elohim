# Burndown Proposal — Operator-Gated Memory/Docs Cleanup

> **SAFETY:** This is a *proposal only*. Nothing below has been mutated. Every existing doc under
> `genesis/docs/`, `.claude/archive/`, `.claude/memory/` is untouched. The single write performed by
> the assembling agent is *this file*. All A–D actions are operator-gated and execute LATER, on approval.

---

## Summary

- **Junk drawers (3 dirs, 77 files):** `2026-05-14` (8) + `2026-05-15` (56) + `2026-06-01` (13). Triage verdict: **4 curated-history records** (all from `2026-05-15`) + **73 bodies-to-git**. `2026-05-14` and `2026-06-01` decompose to *zero* new records — their lessons already live at destination surfaces; only inline pointers + git retention are needed.
- **Pile dead-arch (~27 docs across 8 clusters):** compact to **8 curated-history records** (1 per cluster) + ~25 bodies-to-git. (Two clusters contribute a still-live successor that stays in the pile; several "landed-truth" items are already code/manifest/test surfaces — no new seed needed.)
- **Pile dupe-merges (7 clusters, ~21 docs):** compact to **7 canonical surfaces** — 4 *new/amended architecture seeds*, 3 *merge-into-existing* (memory entry or in-place spec) — + ~18 bodies-to-git. Several "excluded" docs are genuinely live and stay in the pile.
- **Museum growth:** `history/` goes from 4 records (~13 KB) → **~16 records** (4 junk + 8 dead-arch + existing 4; ~45–55 KB) — still tiny, all pure-additive.
- **Pile shrink:** the `genesis/docs/{plans,superpowers/{plans,specs,notes}}` pile drops by **~73 junk bodies + ~43 pile bodies ≈ 116 docs retired to git**, while canonical surfaces gain ~12 curated records + 7 seeds.
- **Headline ratio:** pile→canonical moves from **~6.2× toward ~1.5–2×** once the ~116 retired bodies leave the live pile and the ~19 new canonical artifacts (12 history records + 7 seeds) land. The pile stops being a junk-dominated surface; what remains in `plans/specs/` is overwhelmingly *genuinely-live* work.

**Confidence:** High across the board. The few "held / verify-before-claiming-stable" items are flagged in §Flags; none block A–C (pure adds + pointers). Only §D removals carry recoverability risk, and every body is git-recoverable at its live path.

---

## SAFEST-FIRST EXECUTION ORDER

Execute in this order; each phase is independently revertible and the risk rises monotonically.

- **(A) ADD curated-history records** — *ZERO RISK, pure adds.* New files under `genesis/docs/content/elohim-protocol/history/`. Nothing existing is touched. Add the matching INDEX.md rows.
- **(B) CREATE / amend canonical seeds** — *additive.* 4 new-or-amended architecture seeds + 3 merge-into-existing (memory entry / in-place spec compaction). The in-place spec trims are the only edits-to-existing here; they remove *now-landed how-to sections* whose truth is on disk — recoverable from git.
- **(C) PLANT inline pointers** — *small edits to live canonical docs.* One-to-three-line watch-out/paths-not-taken pointers in the live successor docs a future planner will actually land on. No content removed.
- **(D) RETIRE bodies to git** — *the only removals, gated LAST.* `git rm` the superseded bodies + the three junk-drawer dirs. Every body is recoverable at its live path (verified). Approve in bulk per-cluster or veto specific files.

Rule: **do not start (D) until (A)–(C) have landed**, so no pointer ever dangles and no lesson is in-flight when a body leaves the tree.

---

## A. Curated history records to ADD (ready to apply)

> 12 records total: **4 from junk-drawer `2026-05-15`**, **8 from pile dead-arch**. Each is a new file in
> `genesis/docs/content/elohim-protocol/history/` + an INDEX.md row. (`2026-05-14` and `2026-06-01` junk drawers
> produce ZERO records — pointers only; see §C.)

### A.1 — Junk-drawer `2026-05-15` (4 records)

The `2026-05-15` triage proposal already drafted full record text into
`/projects/elohim/.claude/archive/2026-05-15/JUNK-DRAWER-TRIAGE-PROPOSAL.md`. Apply those 4 verbatim. Headlines:

1. **`2026-06-02-rno-reference-implementation-positioning.md`** — "elohim is the missing protocol substrate" frame + two empirical R&O findings (1000-char DHT cap proves content can't live on the DHT; no-DNA-upgrade-path = network-reset-per-breaking-change). *Plant-pointer:* INDEX.md row. *Confidence: HIGH.*
2. **`2026-06-02-deploy-is-not-a-graph-node.md`** — the 2026-04-27 incident (`edfe5c57` one-line manifest change rebuilt the whole matrix); 3 root causes; two-track fix (tactical `DEPLOY_ONLY`, strategic brit-attestation). *Plant-pointer:* INDEX.md + inline near orchestrator strategy notes. *Confidence: HIGH.*
3. **`2026-06-02-seed-row-shape-satisfies-view-sql-predicates.md`** — seeded rows with wrong `action`/`provider` silently filtered by view SQL → dark surfaces; recurs across topology-substrate + seeder-registry. Distinct from the two existing seed memories. *Confidence: HIGH.*
4. **`2026-06-02-request-correlation-path-not-taken.md`** — heavyweight X-Request-ID/X-Served-By cross-runtime tracing never committed; need met by `pnpm look` headless capture + sprint-report aggregation. *Confidence: HIGH.*

> The `2026-05-15` proposal doc holds the full body text + the 3 inline-pointer plant locations + the clustered
> body-to-git list. Treat that doc as the authoritative source for these 4 records; it is itself an operator-gated artifact (the only write that pass performed).

### A.2 — Pile dead-arch (8 records, full text inline)

**Record A.2.1 — `2026-06-02-epr-foundation-landed-by-waves.md`** *(sibling of `2026-06-01-dht-is-a-notary-not-a-byte-store.md`; cross-link both in frontmatter)*

> ## History/ADR: Landing the EPR substrate by waves — audit-as-truth, stubs that hold the seam
>
> **One-sentence lesson:** A large multi-phase Rust substrate (the EPR codec → storage → federation → manifest-resolver → gradient-signal stack) was delivered NOT by executing one plan box-by-box, but by a wave-sequenced fleet of sub-plans whose *real* state was reconciled by a periodic Wave-0 audit against live code — so plan checkboxes are tracking-debt, never ground truth.
>
> **What was built/attempted (the arc):** Over 2026-04-21 → 2026-05-16 the EPR (Elohim Protocol Record) foundation shipped in phases: Phase 1 codec (`elohim/epr/` crate — 12 modules: cbor/cid/envelope/proof/reach/signature/validation + `@elohim/epr` TS package, cross-language parity via committed fixtures); Phase 2A storage foundation (`db/epr_atoms.rs`, 6 REST routes, view schemas, contract tests); Phase 2C libp2p federation (`/elohim/epr-atom/1.0.0` protocol, golden vectors); Phase 2B identity+projector+signal; Phase 3 manifest-resolver (standing-aware code paths returning `Standing::Unknown` until lit); Phase 3.5 trust-compute gradient (`FeedbackSignal`/`AttentionTending`/`CollectiveFilterPattern`, sealed-against-self predecessor records, +3 DNA entry types); Phase 4 projector controller (`db/projection_events.rs`); and the Wave-2 runtime tail (`record_predecessor` on the Announce path, IntegrityNotify KeyRotation/KeyRevocation arms in `p2p/recovery_rotation.rs`).
>
> **What superseded it / where it went:** The whole foundation was a *substrate handoff*. It is now LIVE code (`elohim/epr/`, `elohim-storage/src/db/epr_atoms.rs` + `projection_events.rs`, `p2p/epr_atom_protocol.rs` + `feedback_signal.rs` + `recovery_rotation.rs`) and its successor — the **Graph-Native Projection Substrate** sprint (2026-05-16, design + plan in-tree) — explicitly builds on these phases ('this spec is one of [the master spec's] phases, Phase 3.7+4 folded'). The remaining `@wip` a2o scenarios were dispositioned (lift / defer-with-evidence) and the deferrals routed to graph-native.
>
> **WHY we turned (the three load-bearing patterns to keep):** (1) **Audit-as-truth over checkbox-as-truth** — three plans (codec/2A/2C) still show 0/117, 0/82, 10/64 ticked yet are confirmed LANDED by the Wave-0 audit and the live crate; on a multi-agent wave fleet, a periodic re-audit against code is the reconciliation controller, and unticked boxes are NOT a signal of incomplete work. (2) **Stubs that hold the seam** — Phase 2A shipped `FederatedEprStore` as a `LocalEprStore`-delegating stub and Phase 3 shipped `Standing`-typed signatures returning `Unknown`; the route/function contract froze early so later phases (2C federation, 3.5 gradient) swapped the implementation at construction time without touching callers. (3) **Parallel-shapes-then-reconcile** — the legacy `EprHead` (~500B) and the generalized `Envelope` were kept deliberately parallel through 2A/2C rather than force-merged; reconciliation was scheduled as its own concern, not smuggled into delivery.
>
> **Watch-out for future planners:** When you inherit a 'LANDED' substrate, trust the audit + the live module list, not the plan's checkboxes — and do not 're-open' a phase because its plan looks unfinished. Conversely, when YOU run a wave fleet: write the trait/signature seam first (it's what lets stubs land safely), and schedule an explicit audit step to convert checkbox-fiction into recorded truth, because no one will tick boxes for code confirmed retroactively.

*Plant-pointer:* INDEX.md row + inline one-liner in `genesis/docs/superpowers/specs/2026-05-16-graph-native-projection-substrate-design.md` under its `Depends on:` block. Cross-link frontmatter with the existing `2026-06-01-dht-is-a-notary-not-a-byte-store.md` (that one owns substrate-PLACEMENT; this owns phase-EXECUTION). *Confidence: HIGH.* **Pre-step:** confirm `2026-06-01-dht-is-a-notary-not-a-byte-store.md`'s `distills:` pointer notes the BATCH-C-PIVOT body now lives in git, before retiring that body.

---

**Record A.2.2 — `2026-06-02-light-up-topology-operational-visibility-arc.md`**

> **Light Up the Topology / Graph (2026-05-01 → 2026-05-29) — operational-visibility arc, LANDED & evolved.** We set out to make the invisible P2P substrate *felt*: prove 'this network actually works' from the surface. The arc began as two sibling sprint designs on 2026-05-01 — **Light Up the Graph** (substrate orchestration: wire the trust-compute primitives into the live runtime — lift the two `aunt_and_rage_bait` mocks by landing the real reach-earning gate + Vouch primitive + LibP2P sink/gossip adapters) and **Light Up the Topology** (six operational views — distribution badge, my-cluster, peer-topology, reciprocity, doorway-dashboard — over one substrate query layer, plus two demo-critical substrate fixes: on-connect replication kick and GET-time blob peer-fallback). The original topology design proposed a bespoke `cluster_view`/`peer_topology_view`/`reciprocity_view` service trio fronted by a hand-rolled `/elohim/view-federation/1.0.0` libp2p request-response codec. **The turn:** on 2026-05-16 the graph-native projection substrate (CozoDB + async-graphql Apollo-Federation-v2 subgraph) landed in tree, which made the bespoke federation codec and per-view HTTP endpoints *the wrong shape*. The **2026-05-19 synthesis** re-baselined the whole effort onto GraphQL `Viewer.{hub,peers,reciprocity}` resolvers wrapping the existing Diesel-backed services (decision: wrap, don't reproject to Cozo yet — preserves the M1 substrate work, lands in days), retired the now-obsolete 2026-05-07 M1 plan, and added the qahal social-lens reframe (slide-45 'After the Feed'). The **2026-05-20 plan** landed three concrete surfaces (compute triptych free/used/stewarded, doorway-dashboard app-shell wiring, resilience tooltip + hub-abstract placement-gaps). By the **2026-05-29 kickoff** a 3-scout file-level sweep confirmed Epic A topology is LIVE (`/shefa/cluster`, `/shefa/peers`, `device-tile`, `ResilienceView`, progressive `●◐○` glyphs) and reframed the *remaining* gap as connective tissue + committed-accounting readers (PeerCapacity stubs returning 0, prioritizer dead-code) — work that then proceeded on later sprints (commit `e6300665c` lands the readers). **What superseded what:** GraphQL Viewer resolvers superseded the bespoke view-federation codec + per-view HTTP routes (codec retained as transitional fallback, never retired); the graph-native substrate superseded the hand-rolled service-federation shape; the 2026-05-19 synthesis superseded the 2026-05-07 M1 plan. **Why we turned:** building a one-off libp2p federation codec for read-views was premature substrate when a general graph-projection layer was already landing — collapse the six surfaces into one subgraph rather than six endpoints. **Watch-out for future planners:** (1) the original topology design's three pre-flight gaps were real and load-bearing — no AgentPeerBinding *seeder* existed, the `peer_identity_bindings` projection table dropped `device_archetype`/`superseded_by` columns the wire view exposed, and the `rea_projection` table the badge/distribution view queried *did not exist at all* (the module of that name is a signal-handler, not a backing store). Any 'live-aggregate over existing tables' plan MUST run the pre-flight table-and-writer grep first — 'projection module exists' ≠ 'projection table exists'. (2) Doorway did NOT forward agent identity through the registry-routed proxy (`storage_proxy::forward_to_storage` forwards only Content-Type/Authorization/X-Observation-Id; `extract_agent_key` reads an `X-Agent-Id` header nothing injects) — and `agent_pub_key` (Holochain pubkey, in the JWT) is a *different identifier* than the imagodei `agent_cid` on AgentPeerBinding. Steward `/me` federation can't resolve bindings until that proxy-inject + identifier-kind decision lands. (3) 'Hub is the abstraction, not household' — keep the substrate hub-kind-agnostic (`dwelling|collective|computed` resolve in UI labels only). (4) The reach-earning gate gates author-side compose ONLY; gating the receive path is meaningless once an EPR has arrived (project standing as evidence, don't block).

*Plant-pointer:* INDEX.md row + inline 'paths-not-taken' watch-out in `genesis/docs/superpowers/specs/2026-05-29-durability-topology-felt-resilience.md` (the live successor vision spec) + a secondary one-liner in `2026-05-16-graph-native-projection-substrate-design.md`. *Confidence: HIGH.* **Held items → §Flags (canonical-seed candidates: REA blob-projection action conventions, standing-policy debitWeights, PlacementGapKind/HubSummary enums; a2o @wip greenness).**

---

**Record A.2.3 — `2026-06-02-archetype-primary-a2o-taxonomy-not-executed.md`**

> ## Archetype-primary a2o taxonomy migration — proposed, never executed, superseded by the cites:/coverage-spine approach
>
> **Dated arc:** 2026-05-22 (Sprint 0.5 archaeology proposed) → 2026-06-01 (coherence-substrate-design supersedes the migration mechanism) → 2026-06-02 (curated to history; migration confirmed abandoned, 0 of 10 decisions landed).
>
> Sprint 0.5 ran a 10.3K-word archaeology pass over the 76 a2o .feature files, mapped each to the gospel-tier collective-archetype catalog (Tier-0 household/faith-community/life-group/wisdom-commons + cross-cutting + infrastructure), and proposed (a) a new archetype-primary directory taxonomy (`features/{archetypes,cross-cutting,infrastructure}/`), (b) a 76-row file-by-file migration plan, (c) a `@archetype:/@cc:/@inf:/@tier:` tag rename, and (d) a 10-decision digest the operator confirm-all'd. WHY WE TURNED: none of it was executed and the mechanism was superseded. Verify-gate evidence (2026-06-02): no `archetypes/` directory exists; the corpus GREW 76→90 under the original mixed-axis taxonomy; zero `@archetype:/@cc:/@inf:` tags were adopted; the named dispositions (split relationship-idempotency, graduate the 3 smoke files, the MVP-BLOCKING witness-rewrite of collective-governance) never landed — collective-governance still carries 62 vote/proposal mentions and 0 witness mentions. The replacement is the 2026-06-01 coherence-substrate-design, which achieves the SAME archetype-traceability goal WITHOUT physical file moves: frontmatter `cites:` edges + a coverage-spine walker + `@epic:value_scanner` scaffolding + `pending`-vs-`regressed` status, treating the corpus as an edge-annotated graph rather than a directory to restructure. WATCH-OUT for future planners: (1) Do NOT re-propose a bulk a2o directory restructure — taxonomy-by-relocation was tried and abandoned; express archetype axis as tags/`cites:` edges over the existing tree, per coherence-substrate-design. (2) The archaeology's standing diagnostic remains TRUE and re-derivable: faith-community, life-group, and wisdom-commons have ZERO primary scenarios — that Tier-0 authoring gap is the live finding worth carrying, and it cannot be harvested from the value-scanner corpus (categorically absent there). (3) The 'a2o is human experience not dev bugs' instinct that flagged 3 smoke files for graduation is sound and already canonical in memory — apply it at authoring time, not via a migration sprint. (4) The value-scanner content audit is a SEPARATE, still-live document — do not sweep it up with this; its 'keep-in-place + index' recommendation IS the current reality.

*Plant-pointer:* INDEX.md row + **primary** inline watch-out in `genesis/docs/superpowers/specs/2026-06-01-coherence-substrate-design.md` (near the coverage-spine / `@epic:value_scanner` scaffolding, ~lines 86-88/203-206/231) + **secondary** annotation in `genesis/docs/plans/2026-05-21-qahal-mvp-roadmap.md` (Sprint 0.5 rows ~line 73/80). *Confidence: HIGH.* **`value-scanner-content-audit` (2026-05-22) is STILL-LIVE → §Flags, leave in pile.**

---

**Record A.2.4 — `2026-06-02-doorway-dispatch-registry-fallback-and-vocabulary.md`**

> ## Doorway dispatch: registry-as-universal-fallback + storage vocabulary (2026-04-28 → 2026-04-30)
>
> **What was built.** The recurring `/blob/<hash>` thumbnail regression was traced to a hand-maintained prefix guard in the doorway dispatch tail (`p.starts_with("/api/v1/") || p.starts_with("/account/")` at `http.rs:1504`). Every time elohim-storage's manifest added a new top-level path family (`blob_proxy → /blob/`, `stream_proxy → /stream/`), those requests skipped the RouteRegistry entirely and fell into the SPA bootstrap, so thumbnails rendered as HTML. The fix extracted the dispatch decision into a unit-testable `classify_dispatch` helper returning a `Disposition` enum, replaced the prefix-gated arm + GET catch-all + default 404 with a single wildcard arm delegating to it, and pinned the contract with four `dispatch_classification_tests`. Two days later a vocabulary-cleanup sprint piggybacked: it migrated ~11 TypeScript callers off legacy `/store/<hash>` and `/api/blob/<hash>` to canonical `/blob/<hash>`, deleted the dead legacy dispatch arms, and introduced the `quilt`/`pantry`/`stock`/`draw`/`RS(N,K)` design register (rejecting `weave` for a Moss `@theweave/api` identifier collision, and `lattice` for the holonic-governance collision).
>
> **What superseded it / where the truth now lives.** Both plans landed (`8022ff361`, `51adf5118`, `4c1580133`). The routing truth is the live `classify_dispatch` + `Disposition` in `doorway/doorway-service/src/server/http.rs` and its green contract tests — verified behavior remembered AS tests, not a parked plan. `classify_dispatch` has since become load-bearing: extended for SSR (`classify_dispatch_returns_ssr_route_when_render_spec_set`) and the `root_app_slug` arm retired when `ROOT_APP_SLUG` gave way to urlPath=/ projection (`2ac1431fa`). The vocabulary truth lives in `genesis/graphos/vocabulary.md` (Status: Active reference).
>
> **Why we turned.** A dispatch that enumerates path-prefixes is a standing regression vector: it silently misclassifies every future manifest path family the day it ships, surfacing weeks later as a user-visible content bug (the Jan/Mar/Apr `/blob` whipsaw). Making the registry the unconditional fallback honors the `doorway/CLAUDE.md` contract — 'adding a new endpoint to elohim-storage automatically makes it routable through doorway, no doorway code change' — converting an open-ended bug class into a closed one. The vocabulary turn recognized that `blob/store/s3` framing is hostile to the protocol's actual shape (peers stewarding shards in pantries).
>
> **Watch-out for future planners.** (1) Never reintroduce a path-prefix check in the wildcard dispatch arm — the registry already knows its own prefixes; a prefix guard there is the exact regression this work eliminated. (2) `Disposition::RegistryUnhandled` 404s today; `BlobProxy`/`StreamProxy`/`ZomeCall`/`AgentProxy` target dispatch is the intended extension point and must plug in there, not via a new top-level arm. (3) Vocabulary boundary rule is load-bearing: `quilt`/`pantry`/`stock`/`draw` apply to design language, signals/events, and NEW invented identifiers — NOT to wire-level HTTP routes (`/blob/{hash}`), content addresses (`sha256-{hex}`), or existing Rust types (`BlobStore`); renaming those breaks external legibility. (4) `weave` is permanently off the table (Moss collision); `lattice` is taken by holonic governance.

*Plant-pointer:* INDEX.md row + inline one-liner in `doorway/doorway-service/src/server/CLAUDE.md` at the dispatch-contract section (exact spot the regression recurs). *Confidence: HIGH.*

---

**Record A.2.5 — `2026-06-02-rno-cross-wave-guidance-graduate-into-not-from.md`**

> ### R&O Lessons → Cross-Wave Guidance (2026-04-21, retired 2026-06)
>
> **What it was.** A short strategic-frame + design-principles companion to a 4-doc R&O-lessons planning set. It decomposed 9 R&O-inspired sub-projects into a 5-wave arc (DNA-manifest hygiene + sweettest → release-discipline + feature-flags → Gate B → hREA/VF-GraphQL → Gate C → Tauri/Launcher/Moss distribution → Wave-5 graduation-narrative-as-destination) with retrospective brainstorm gates between strategic pivots.
>
> **The load-bearing frame (preserved because it is still true).** 'We are not graduating R&O. We are preparing elohim to be worthy of being graduated into.' Build elohim's own coherence; coordination hApps graduate INTO it, never the reverse. Credibility is cumulative (release/test/vocabulary/economic-interop discipline are substrate, not 'just process'). The graduation narrative is the LAST act — it describes a bridge that already exists; reject any attempt to draft it before the foundation lands.
>
> **What superseded it, and WHY we turned.** The frame and every cross-cutting principle outlived the plan. (1) The cooperative-substrate stance was lifted verbatim into the live Wave 3 VF/hREA interop canon (`architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md` §48) and is cited by the live qahal-collective-membership spec. (2) All six design principles (stewardship-not-ownership, observed-not-flagged, sense-and-respond split, schema-first-is-IoC, measures-in-Jenkins, doorway-manifest-driven) are canonized as standalone memory entries and honored by default. (3) The waves became their own living surfaces: sweettest integration-layer spec, the VF/hREA interop design, the records-lifecycle canon, and the R&O application archetype. The doc's three companion plans were already retired in ceremony run-6 (`dfa4c7121`); this one was simply missed — it is the cleanup of the last orphan.
>
> **Watch-out for future planners.** When R&O (or any external coordination hApp) graduation comes up again: do NOT work backwards from the external hApp's needs — that inverts the dependency the whole frame guards against. The deliverable is elohim's internal coherence + credible release cadence + hREA-intelligible economics; absorption, if it ever happens, happens because the flywheel made elohim the obvious landing zone. Cooperate with Lynn Foster / Bob Haugen's VF-GraphQL / hREA work as first-class upstream, never subsume by default.

*Plant-pointer:* INDEX.md row + inline curated note in `genesis/docs/content/elohim-protocol/architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md` (replace its dangling reference to the retired guidance doc). **Repoint the 3 dead references in `genesis/docs/superpowers/specs/2026-05-19-qahal-collective-membership-dht-design.md` (lines 9, 29, 206) to the same Wave 3 interop canon.** *Confidence: HIGH.*

---

**Record A.2.6 — `2026-06-02-experience-story-discernment-gate-relocate-supersede-land.md`**

> ## Experience-Story Discernment Gate — relocated-then-superseded plan stub (2026-04-18 → landed)
>
> **What was attempted.** A plan to ship the v1 mechanical 'discernment gate' for experience-story EPRs: the seam that reads a2o pipeline moments (one persona × one scenario × one run) and mints discerned story-point attestations carrying a seven-valence value function (`progress`/`discovery`/`regression`/`validation`/`witness`/`refinement`/`confirmation`) rather than signed numeric story-points. The first plan proposed the gate as a pure TypeScript function in elohim-library.
>
> **The turn (why).** During Batch F the architecture was course-corrected: discernment is a first-class protocol primitive of `@elohim/elohim-agent` (a Rust `Gate` trait invoked by the SDK), NOT an app-layer `.ts` concern. The `.ts` surface is legitimately only the sense-and-respond layer; the gate's evaluation/registry/constitutional-reasoning coupling lives in Rust, with the specific ruleset declared in the app manifest. The TS module (`80fe6c70`..`0ecef2ec`) was reverted (`dfadce0b`); the schemas, contentType registrations, `experience-attestation` signal, and regenerated manifest types survived. The genesis-side plan was first *relocated* to `rakia/docs/plans/`, leaving an 18-line cross-ref stub in genesis — then the rakia body itself was marked SUPERSEDED and folded into the elohim-agent gate-interface plan, shipping as that plan's Phase 3.
>
> **Where it landed (living surface).** Seven-valence gate is real, mechanical-from-day-one Rust: `elohim/elohim-agent/gate-client/src/dag/` (discernment_gate.rs, seven_valence_rules.rs, executors/mechanical_ruleset.rs, attestation.rs, reach_aggregation.rs) driven by a CID-addressed rules artifact `src/dag/rules/seven_valence_v1.json`, with a 14-case integration suite. App vocabulary in lamad manifest content-types (`experience-story`/`experience-moment`) + `experience-attestation` signal; avodah components consume experience-stories. Authoritative data-model remains in the still-live `architecture/2026-04-18-experience-story-epr-design.md`.
>
> **Watch-out for future planners.** (1) Do NOT re-draft this as a TypeScript gate — that path was tried and reverted; gates are Rust primitives in elohim-agent, rulesets are manifest-declared, `.ts` is sense-and-respond only. (2) Rule ordering is load-bearing: rule 3 (`@validates-failure-mode` → validation) must NOT shadow rule 2 — a failure-mode scenario with a prior-passed attestation is deliberately classified by rule 2 as discovery/regression. (3) Steady-state (rule 7) mints nothing — silence is the baseline signal; the v1 ruleset never mints `confirmation`. (4) Sub-projects A (matthew peer persistence), C (doorway moment export/re-upload), D (inter-pipeline diff → shefa valueflow) and sophisticated discernment were explicitly deferred; the `attest()` output left room for an optional `linkedComputeContributionHashes` field so the compute↔evidence loop can close later without a schema break.

*Plant-pointer:* INDEX.md row + inline status note near the top of `genesis/docs/content/elohim-protocol/architecture/2026-04-18-experience-story-epr-design.md`. *Confidence: HIGH (landing is structural-verified; CI-green HELD — see §Flags).* **rakia submodule plan is out of this tree's scope — do not touch.**

---

**Record A.2.7 — `2026-06-02-attestation-consolidation-phase2a-dedup.md`**

> ### Attestation Consolidation (Phase 2a substrate dedup) — LANDED 2026-05-15 (`34fcf1070`)
>
> **What was built.** A 7-stage (A–G) cross-DNA sprint that collapsed 18+ attestation-shaped DHT entry types across four DNAs (imagodei, lamad, mishpat, infrastructure) into a single reused `Content` entry discriminated by `content_type: "attestation:<subtype>"`, with the subtype vocabulary declared in pillar manifests rather than minted as bespoke entry types. M-of-N social-threshold patterns decomposed into a parent `governance-action:<kind>` Content entry plus child vote-attestations (with an operational tally projection); Shamir share material decoupled off the DHT onto a libp2p request-response transport. Net effect: ~−20 DHT entry types reclaimed; 22 legacy per-type projection tables → 2 unified + 1 derived tally; 25+ legacy attestation routes → 8 unified routes; a discriminator-chain validator (`attestation_validator.rs`, floors F1/F5/F7/F8 live) gates issuance.
>
> **What superseded the plan body / why we turned.** Truth migrated into living surfaces and the plan checkboxes became the only thing left: the migrations exist (`2026-05-12-100000_attestations`, `_100100_governance_actions`, `_100200_governance_action_tally`, `_100300_drop_legacy_attestation_tables`), all four pillar manifests carry `attestation:` declarations, the Stage-G Shamir transport landed (`p2p/shamir_transport.rs` + `recovery/share_assembler.rs`), and the EPR foundation post-attestation audit re-verified the merge with 0 orphaned tasks and 0 stale-route hits. A verified consolidation is best remembered as green migrations + live validators + manifest declarations. The plan also explicitly SUPERSEDED the Tiered-Quilt Wave-0 'Attestation dedupe' direction.
>
> **Watch-out for future planners.** (1) Mid-sprint reality-drift forced a 'verify-before-remove' re-org of Stage C: the original 'vestigial' removal claim was WRONG for `CustodianCommitment` (14 live callers in shard replication) and `ContentSuccession` (live callers in versioning) — never grep-and-delete an entry type by name; confirm caller count first. (2) B.9 (imagodei) was a full-replacement bridge but B.10 (mishpat + infrastructure) was ADDITIVE (writes both locally and via cross-DNA call) — bridge-conversion strategy was not uniform across DNAs. (3) The unified `attestations`/`governance_actions` tables are Category-A projections (source of truth = Holochain DHT, `dht_anchor_hash NOT NULL`); `governance_action_tally` is Category-C operational (no anchor, rebuildable by replaying the attestation signal stream grouped by parent). Don't treat the tally as notarized. (4) A pre-launch HARD cutover with no back-compat shim was the chosen path — acceptable only because it preceded launch. (5) Recovery's remaining migration onto this substrate is owned by a SEPARATE live plan (`2026-05-15-recovery-m4-completion-shamir-optional-plan.md`); retiring this body orphans nothing.

*Plant-pointer:* INDEX.md row + inline watch-out near the 7-stage migration-plan section of the **still-live canonical** `genesis/docs/content/elohim-protocol/architecture/2026-05-11-attestation-consolidation-design.md`. *Confidence: HIGH.*

---

**Record A.2.8 — `2026-06-02-conductor-agent-info-substrate-gossip-step-zero.md`** *(dupe-cluster origin, but its landed-and-done shape makes a curated record the right artifact; see §B note)*

> ## Conductor agent-info substrate gossip ("step zero")
>
> Phase-1 per-human doorway routing split the alpha cluster's kitsune2 signaling mesh along the doorway-A / doorway-B boundary (each Holochain conductor registers at exactly one `signal_url`; kitsune2 has no multi-bootstrap schema). To keep the two halves discoverable, every `elohim-storage` pod publishes its embedded conductor's own `AgentInfoSigned` JSON strings (`admin_ws.agent_info(None)`, filtered to self via `list_cell_ids`) on the gossip topic `elohim/conductor/agent-info/v1` over the existing `DualGossipPublisher`; every other pod subscribes (libp2p edge → bounded mpsc → rate-limited worker → batched `admin_ws.add_agent_info`) and injects into its conductor's peer cache. The substrate is pure transport — the conductor stays authoritative for signature-verify + dedup; the kitsune2 signal URL is registration-only, so a warmed peer cache lets outbound gossip reach across both signal servers without the signal servers federating. The payload (`ConductorAgentInfo`) is Category-C operational: never stored, reconstructed by the next 60s heartbeat; no DHT entry type, no diesel table, no HTTP route. LANDED and merged to dev (`c88febe93`; module `elohim/elohim-storage/src/p2p/conductor_agent_info_gossip.rs`, 14 unit tests, dual-publish catalog row #13, byte-parity test, a2o feature), behind `ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP` (default false) pending operator matthew+adam soak.
>
> **Watch-out:** step-zero is *necessary but not sufficient* for cross-doorway EPR delivery — it propagates DHT entries cross-mesh but does not project them on remote pods (Holochain `post_commit` fires local-only), so the sibling gaps F1 (remote-receive projection visibility) + F2 (Jenkins seeds one doorway) + F4 (blob reachability) must close too; those are tracked in `genesis/docs/superpowers/plans/2026-05-29-substrate-shakeout-epr-delivery-sprint.md`. Explicitly deferred to Phase 12+: cold-start with signal_url down (substrate WebRTC signaling), multi-URL agent_info fallback, session.doorway_url single-pin refactor, substrate-native signaler, iroh subscriber-side wiring (behind Plan 4 Task 8). The publisher own-key filter uses a substring match against kitsune2 v2 agent_info JSON — re-confirm against a live `agent_info()` return on any `holochain_client` major bump.

*Plant-pointer:* INDEX.md row + update memory `project_multi_doorway_human_registration` (conductor peer-cache layer now substrate-warmed; first of three federation-wiring-audit layers closed) + adjacent to `elohim/elohim-storage/src/p2p_iroh/dual_publish/CATALOG.md` row #13. *Confidence: HIGH (code merged; runtime soak HELD — see §Flags).*

---

## B. Canonical seeds to create (ready to apply)

> 7 dupe clusters → 7 canonical surfaces. **4 are architecture seeds** (new or amended under `genesis/docs/architecture/`
> or `…/elohim-protocol/architecture/`); **3 merge into an existing surface** (memory entry / in-place spec). Full seed
> text lives in the cluster proposals; homes + compacted_from below. NOTE: cluster "Conductor agent-info gossip" is
> rendered as a curated-history record (A.2.8) rather than a standalone seed because it is closed-and-merged, not
> in-flight — its `seed_home` recommendation (architecture note + memory pointer) is satisfied by the record + the
> memory update in §C.

### B.1 — Sweettest DNA-Level Integration Test Layer *(NEW architecture seed)*
- **Home:** new short seed under `genesis/docs/content/elohim-protocol/architecture/` (e.g. `testing-layers` / `sweettest` note), the single linkable source CLAUDE.md's CI table + rno-guidance point at.
- **Compacted from:** `superpowers/specs/2026-04-22-sweettest-integration-layer-design.md`, `superpowers/plans/2026-04-22-sweettest-integration-layer-plan.md`, `superpowers/specs/2026-05-24-sweettest-stage-efficiency-design.md`.
- **Seed:** Sweettest = the DNA-level integration layer (in-process conductors, real DHT/validation/cross-agent propagation against our packed DNAs); hybrid coverage (mechanical floor via `zome-sweettest-sync` + `sweettest-check` push gate, plus per-Wave focal scenarios); crate at `elohim/holochain/tests/sweettest/` (28 files, `//! @dna-scope:` markers); Jenkins `nextest archive` + 4 shards + `build-nextest-filter.sh`; full-suite fall-through when CHANGED_PATHS empty. Non-goals settled (does not replace A2O or zome unit tests; Rust-native; cucumber-rs parked).
- **Watch-outs (carry into seed):** `CARGO_TARGET_DIR=/cargo-target` MUST stay scoped to the sweettest `sh` block (pod-wide broke `hc dna pack`); sccache stays DISABLED (spawn-ENOENT ~1.7%, NOT a cache-substrate problem — RCA in `feedback_sccache_spawn_enoent_rca.md`); `@dna-scope` defaults to always-run; compose-with the 4-test quarantine; parked: Gherkin-driving, per-DNA Jenkinsfiles. **W4b sccache 0.14 downgrade = only un-landed thread → one-line followup pointer, not a kept body.** Single-DNA ~10-15min target HELD (not demonstrated).
- **Excluded (leave in pile):** `superpowers/sprints/2026-05-24-sweettest-stage-efficiency-w1-w2-w3-w5.md` (landing-record; its own pass), `rno-lessons-cross-wave-guidance.md`, and the ~6 specs that merely *mention* sweettest. *Confidence: HIGH.*

### B.2 — Doorway SSR Runtime *(NEW architecture seed)*
- **Home:** new `genesis/docs/architecture/doorway-ssr-runtime.md` (sibling to `rea-compute-commitment-primitive.md`, `cradle-to-grave-capability-gradient.md`); cross-link the access-tier canon at `…/architecture/2026-05-23-doorway-access-tier-patterns.md`.
- **Compacted from:** `superpowers/specs/2026-05-07-doorway-ssr-runtime-design.md`, `superpowers/plans/2026-05-07-doorway-ssr-runtime.md`, `superpowers/specs/2026-05-08-ssr-capability-design.md`.
- **Seed:** doorway server-renders content routes (Rust+V8, `elohim-render` crate embedding `deno_core` behind a `Renderer` trait; manifest-driven `render: "angular-ssr"`; CSR shell always the fallback floor); SSR is an honest advertised compute capability (`renderCapability` profile, `GET /admin/capability`, projected into `PeerStatusView` as Category-C); auth context threaded through the V8 fetch shim (or CSR fallback with `x-ssr-skipped`); audit tuple shaped for future REA settlement.
- **Watch-outs:** Angular-19 global-shape shim covers exactly what Angular touches; `sophia-element`/UMD must NOT run server-side (placeholder via `CUSTOM_ELEMENTS_SCHEMA`); V8 cold-start real (wall-time 2s→15s→60s; pod memory bump + startupProbe); capability claims stratified (default stay-Tier-2); authenticated requests must NEVER silently downgrade to anonymous renders.
- **Excluded (leave in pile):** `superpowers/specs/2026-05-20-capability-profile-element-contract-design.md` (DISTINCT — viewer-side CapabilityProfile, its own cluster B.5). *Confidence: HIGH. **DEPLOY env-held → §Flags.***

### B.3 — Peer OAuth Portal / Session Bridge *(MERGE into existing memory entry)*
- **Home:** MERGE INTO `.claude/memory/project_peer_native_account_canonical_surface.md` (already cites the portal design spec + holds the graduation-based-supersession shape). Tighten its "What's still open" — the items "how graduation happens", "how doorway detects returning peer-steward", "libp2p↔browser handoff" are now ANSWERED by `trustMode`-discovered-from-`/auth/me`. Add `supersedes`/`landed_as` line → a2o scenarios + elohim-imagodei elements package. Plant a one-line curated-history pointer in the auth/portal area.
- **Compacted from:** `superpowers/specs/2026-05-25-peer-oauth-portal-design.md`, `superpowers/notes/2026-05-25-peer-oauth-portal-substrate-audit.md`, `superpowers/plans/2026-05-25-peer-oauth-portal-plan.md`.
- **Watch-outs:** NO THIRD PORTAL (doorway `auth_routes.rs` is substrate, Angular components are wrappers); `trustMode` MUST stay discovered-from-`/auth/me`, never config; explored-and-deferred items (recovery-launcher Lit primitives, hosted→native migration UI, dynamic OAuth-client registration, cross-doorway PortalHost lookup) are deliberate fast-follow not gaps; anti-crypto-bro framing is a values constraint.
- **Excluded (leave in pile — genuinely LIVE, not landed):** `superpowers/specs/2026-05-28-session-bridge-design.md` + `superpowers/plans/2026-05-28-session-bridge-implementation.md` (net-new `crates/session-bridge/` substrate, verified NOT landed; shares only "graduation" vocabulary). *Confidence: HIGH.*

### B.4 — Doorway Hub Edge / Stewardship Chain *(AMEND existing architecture seed)*
- **Home:** AMEND `genesis/docs/content/elohim-protocol/architecture/2026-05-02-elohim-hub-boundaries-design.md` (the hub-edge spec already names it as its extension target). Add the doorway/hub edge section: doorway-vs-hub responsibility split, four reach-earning surfaces, HouseholdHub→DwellingHub rename + CollectiveHub attitude framing, "DDoS = unearned reach". Fold the three-tier stewardship-chain truth in as the standing/custody-succession instance. **Carry the hub-edge spec's 23 `memory_anchors` into the seed frontmatter** (INDEX.md frontmatter-glue contract) or memory↔doc edges break.
- **Compacted from:** `superpowers/specs/2026-05-08-doorway-hub-edge-design.md`, `plans/2026-05-19-doorway-stewardship-chain-design.md`.
- **Watch-outs:** two docs = ONE topic at TWO layers (architecture boundary vs landed standing-succession slice) — seed must state the chain IS the "who has standing to operate the doorway" instance, or the link is lost. **Stewardship-chain WIRING is HELD, not done** (substrate landed: operate-doorway action, schemas, `auth/operator.rs`, `DoorwayOperatorBindingView`; task-#16 wiring NOT in DNA — imagodei Attestation kinds, `verify_custody_chain`, `request_kind` for transitions absent). Hyperscaler-fronting is a constitutional constraint (operator opt-in, never the protocol's DDoS answer; household-scale-implementable). Reach ladders differ across the two docs — note both, don't silently merge. HouseholdHub = retired synonym for DwellingHub.
- **Excluded (leave in pile):** `plans/2026-05-19-topology-resilience-qahal-synthesis.md` (predecessor, recently touched), the hub-boundaries seed itself (amendment target not a body), and ~5 doorway-adjacent siblings. *Confidence: HIGH. **Wiring HELD → §Flags + §D conditional.***

### B.5 — Capability Profile + Element Contract *(AMEND existing architecture seed)*
- **Home:** FOLD into `genesis/docs/architecture/cradle-to-grave-capability-gradient.md` (its line 13 already names "the capability-profile element contract" as a gradient-row implementer). Add a `§ Rendering-layer realization` subsection: two render-time observables (Capability Profile + Content Certainty), `capabilityContract` CEM block + analyzer, three all-or-nothing precondition gates, ProtocolOmniComponent + EprNavContextView (Category-C). Do NOT create a parallel elohim-elements architecture doc.
- **Compacted from:** `superpowers/specs/2026-05-20-capability-profile-element-contract-design.md`, `superpowers/plans/2026-05-20-capability-profile-element-contract-plan.md`, `superpowers/plans/2026-05-20-protocol-omni-component.md`.
- **Watch-outs:** Sabbath-as-default inversion (`stimulus: still`, `textuality: textual`; motion opt-up + OS-capped) — a planner WILL want to flip it; preserve. Three precondition gates are all-or-nothing (no "designed-but-not-yet-a11y" cell). Lens narrowing is a one-way downward ratchet. **Deferred sub-projects #3-#7 are real & unbuilt** — #3 steward-lock persistence IS a real DHT entry and MUST re-invoke p2p-design-gate. Protocol vocabulary preserved verbatim across locales. EprNavContextView Category-C (p2p-gate flag pre-cleared).
- **Excluded (leave in pile):** `2026-05-08-ssr-capability-design.md` (DISTINCT — doorway SSR capability, cluster B.2), and the live `protocol-omnibar` Angular component (separate, predates/coexists). *Confidence: HIGH.*

### B.6 — App-Manifest Staged-Intents & Graduation Vocabulary *(MERGE — compact-in-place spec)*
- **Home:** keep `superpowers/specs/2026-05-28-app-manifest-staged-intents-design.md` as the single canonical home, **compacted in place**: prepend a status banner (substrate LANDED / feature HELD on session-bridge), strip now-landed §6/§7/§11/§12 (truth = on-disk schema + codegen + tests), keep §0–§5/§9–§10/§13–§15. NOT yet an architecture-seed (consuming session-bridge primitive unbuilt → vocabulary not stable enough to graduate).
- **Compacted from:** the design spec (KEPT, compacted) + `superpowers/plans/2026-05-28-b-manifest-app-manifest-staged-intents.md` (RETIRE — every load-bearing task landed; `fix(B-MANIFEST) Task N` commits `bbb707887`/`488e3f484`; unchecked boxes are stale).
- **Watch-outs:** ceremony IDs validated only as non-empty strings (typo passes manifest validation, fails at graduation-time registry lookup — silent-failure trap). **Residual open thread:** the four `SessionLifecycleState` values landing as protocol-core Standings via a §5.1 patch to the Capability-Profile spec — verify if landed (plant pointer THERE, not here). Paths-not-taken in §10 (coupling required-vs-optional; multi-pillar staged intents rejected; third-party-pillar authorization at composition-root). **HELD, not verified-stable** — zero pillar manifests declare a stagedIntent; session-bridge consumer crate does not exist.
- **Excluded (FALSE DUPE — leave in pile, own compaction):** `plans/2026-05-18-app-manifest-modularization.md` (lamad manifest `$ref`-split refactor, LANDED, belongs to the structural-refactor sprint cluster) + the session-bridge spec/plan (genuinely live). *Confidence: HIGH.*

### B.7 — Conductor Agent-Info Substrate Gossip *(rendered as curated-history A.2.8; memory pointer in §C)*
- **Home:** see **A.2.8** (curated record) + the `project_multi_doorway_human_registration` memory update in §C. The cluster's own verdict was "canonical architecture seed, NOT a consolidated active spec — work is done and merged." A curated record + memory pointer satisfies that; no separate seed file needed. *Confidence: HIGH (soak HELD → §Flags).*

---

## C. Inline pointers to plant

> Small, additive edits to the live doc a future planner will actually land on. No content removed.

| # | Live doc to edit | Pointer planted |
|---|---|---|
| C1 | `genesis/data/stories/james-son--as-stewardee--stewarded-device-sync.md` (append to existing `historian_precedents:` block) | Junk-`2026-05-14` precedent note: the 6 graduated/memorialized bodies are git-recoverable; **two enumerated rubrics did not migrate to prose** — (1) ungrudging-service design rules (design externalities as first-class; never instrument gratitude; never punish defection) → retrieve when REA/valueflows externality modeling begins; (2) the six stewardship principles → enshrine in a durable `genesis/docs/content/elohim-protocol/` doc when imagodei custodial work begins. Do NOT re-create the dated archive. *(Full text in the `2026-05-14` triage `drafted_pointer_text`.)* |
| C2 | `history/INDEX.md` | Add 12 rows (4 junk-`2026-05-15` + 8 dead-arch records). |
| C3 | `genesis/docs/superpowers/specs/2026-05-16-graph-native-projection-substrate-design.md` | `Depends on:` one-liner → A.2.1 (EPR foundation phases P1→P4 landed by waves) + secondary note → A.2.2 (became the substrate topology surfaces reprojected onto). |
| C4 | `genesis/docs/superpowers/specs/2026-05-29-durability-topology-felt-resilience.md` | A.2.2 paths-not-taken watch-out: bespoke view-federation codec superseded by GraphQL Viewer resolvers; run the pre-flight table+writer+proxy-identity grep before any "live-aggregate over existing tables" plan. |
| C5 | `genesis/docs/superpowers/specs/2026-06-01-coherence-substrate-design.md` (~86-88/203-206/231) + `genesis/docs/plans/2026-05-21-qahal-mvp-roadmap.md` (Sprint 0.5 rows) | A.2.3: a2o directory-restructure was tried & abandoned; archetype axis via `cites:`/coverage-spine; Tier-0 authoring gap (faith-community/life-group/wisdom-commons = zero primary scenarios, not value-scanner-fillable) is the live carry-forward. |
| C6 | `doorway/doorway-service/src/server/CLAUDE.md` (dispatch-contract section) | A.2.4 one-liner: registry-as-universal-fallback + quilt/pantry vocab compacted; do not reintroduce prefix guards here. |
| C7 | `genesis/docs/content/elohim-protocol/architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md` + `genesis/docs/superpowers/specs/2026-05-19-qahal-collective-membership-dht-design.md` (lines 9/29/206) | A.2.5 curated note replacing the dangling reference to the retired R&O guidance doc; **repoint the 3 dead qahal references** to the Wave 3 interop canon. |
| C8 | `genesis/docs/content/elohim-protocol/architecture/2026-04-18-experience-story-epr-design.md` (top) | A.2.6 status note: v1 mechanical discernment LANDED as elohim-agent Phase 3 in Rust (TS gate reverted); this spec remains authoritative data-model + seven-valence reference. |
| C9 | `genesis/docs/content/elohim-protocol/architecture/2026-05-11-attestation-consolidation-design.md` (7-stage section) | A.2.7 verify-before-remove watch-out (CustodianCommitment/ContentSuccession had live callers); LANDED at `34fcf1070`. |
| C10 | `.claude/memory/project_multi_doorway_human_registration.md` | A.2.8: conductor peer-cache layer now substrate-warmed (first of three federation-wiring-audit layers closed; doorway-federation reconciliation + session.doorway_url single-pin remain open). |
| C11 | `.claude/memory/project_peer_native_account_canonical_surface.md` | B.3 merge: tighten "What's still open" (trustMode-from-/auth/me answers the open items); add `supersedes`/`landed_as`; NO-THIRD-PORTAL + transport-agnostic watch-outs. |
| C12 | `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md` §5.1 | B.6 residual: confirm/land the four `SessionLifecycleState` values as protocol-core Standings. |

---

## D. Bodies to retire to git (gated removals)

> The ONLY removals. Each is git-recoverable at its live path (verified during triage). Approve in bulk per-cluster
> or veto specific files. **Do NOT start D until A–C have landed.**

### D.0 — Junk drawers (clear whole dirs after pointers land)
- **`.claude/archive/2026-05-14/` (8 files)** — all already-decomposed-to-living-surface; clear after C1 lands. Bodies in git.
- **`.claude/archive/2026-05-15/` (56 files)** — clear after the 4 A.1 records + their pointers land (51 plain + 1 already-distilled `p2p-dataplane-visibility-design` + 4 curated-source bodies). Bodies in git at original live paths.
- **`.claude/archive/2026-06-01/` (13 files)** — all `status: superseded` with verified-live successors; clear (no records needed). Bodies in git.

### D.1 — EPR Codec & Storage Foundation (dead-arch; after A.2.1)
`superpowers/plans/`: `2026-04-21-elohim-epr-codec-crate-plan.md`, `2026-04-22-elohim-epr-storage-foundation-plan.md`, `2026-04-22-elohim-epr-storage-foundation-plan-BATCH-C-PIVOT.md`, `2026-04-23-epr-phase-2c-libp2p-federation-plan.md`, `2026-04-24-epr-phase-2c-batch-d-completion-addendum.md`, `2026-04-30-epr-phase-3-5-trust-compute-gradient-plan.md`, `2026-05-11-epr-delivery-master.md`, `2026-05-11-epr-w2a-record-predecessor-plan.md`, `2026-05-11-epr-w2b-integrity-notify-keyrotation-plan.md`, `2026-05-16-epr-foundation-closure.md`. `plans/`: `2026-04-21-rno-lessons-cross-wave-guidance.md` *(also covered by A.2.5/D.6 — dedupe the list)*, `2026-04-26-epr-phase-3-manifest-resolver-kickoff-prompt.md`, `2026-05-15-epr-foundation-completion-post-attestation-kickoff-prompt.md`, `2026-05-15-epr-w2b-resumption-handoff.md`, `2026-05-16-epr-wip-disposition.md`. — *Justification:* landed-in-code (live `elohim/epr/` crate + storage modules); checkboxes are tracking-debt. **LEAVE the two graph-native successor docs.**

### D.2 — Light Up the Topology / Graph (dead-arch; after A.2.2)
`superpowers/specs/2026-05-01-light-up-the-graph-design.md`, `superpowers/specs/2026-05-01-light-up-the-topology-design.md`, `plans/2026-05-19-topology-resilience-qahal-synthesis.md` *(NOTE: cluster B.4 lists this as still-live-leave-in-pile; **CONFLICT → §Flags, default LEAVE**)*, `plans/2026-05-20-light-up-the-topology.md`, `superpowers/plans/2026-05-07-topology-substrate-completion-m1-plan.md`. — *Justification:* landed in evolved form (services + GraphQL + Angular surfaces in tree). **LEAVE the 2026-05-29 sprint kickoff + the graph-native substrate docs.**

### D.3 — Scenario Archaeology & Archetype Map (dead-arch; after A.2.3)
`plans/2026-05-22-scenario-archaeology-and-archetype-map.md`, `plans/2026-05-22-archaeology-decisions-digest.md`. — *Justification:* migration proposed, never executed (0/10 decisions landed); superseded by coherence-substrate-design. **LEAVE `value-scanner-content-audit` (still-live index).**

### D.4 — Doorway Blob Registry Routing & Vocabulary (dead-arch; after A.2.4)
`superpowers/plans/2026-04-28-doorway-blob-registry-routing.md`, `superpowers/plans/2026-04-30-vocabulary-cleanup-sprint-kickoff.md`. — *Justification:* landed (`8022ff361`/`51adf5118`/`4c1580133`); truth = live `classify_dispatch` + green tests + `vocabulary.md`.

### D.5 — *(reserved — folded into D.1 EPR cluster; the R&O guidance doc retires under D.6)*

### D.6 — R&O Lessons Cross-Wave Guidance (dead-arch; after A.2.5)
`plans/2026-04-21-rno-lessons-cross-wave-guidance.md`. — *Justification:* last orphan of the R&O-lessons set (3 siblings already retired in `dfa4c7121`); frame + 6 principles already canonized. **Requires C7 repoint of the 3 qahal references first.**

### D.7 — Experience-Story Discernment Gate stub (dead-arch; after A.2.6)
`superpowers/plans/2026-04-18-experience-story-discernment-gate.md` (18-line relocated stub). — *Justification:* work landed in Rust `elohim/elohim-agent/gate-client/`; the stub's target is the SUPERSEDED rakia plan. **LEAVE the live design spec; rakia submodule untouched.**

### D.8 — Attestation Consolidation impl plan (dead-arch; after A.2.7)
`superpowers/plans/2026-05-11-attestation-consolidation-implementation-plan.md`. — *Justification:* landed at `34fcf1070` (migrations + validator + manifest declarations). **LEAVE the live design spec + the separate live recovery-m4 plan.**

### D.9 — Sweettest (dupe; after B.1 seed)
`superpowers/specs/2026-04-22-sweettest-integration-layer-design.md`, `superpowers/plans/2026-04-22-sweettest-integration-layer-plan.md`, `superpowers/specs/2026-05-24-sweettest-stage-efficiency-design.md`. — *Justification:* landed (crate + sync rule + build-manifest + shards). **LEAVE the sprint landing-record for its own pass.**

### D.10 — Doorway SSR (dupe; after B.2 seed)
`superpowers/specs/2026-05-07-doorway-ssr-runtime-design.md`, `superpowers/plans/2026-05-07-doorway-ssr-runtime.md`, `superpowers/specs/2026-05-08-ssr-capability-design.md`. — *Justification:* landed (elohim-render crate, schemas, a2o features). **DEPLOY env-held — does not block body retirement (code + tests landed).**

### D.11 — Peer OAuth Portal (dupe; after B.3 merge)
`superpowers/specs/2026-05-25-peer-oauth-portal-design.md`, `superpowers/notes/2026-05-25-peer-oauth-portal-substrate-audit.md`, `superpowers/plans/2026-05-25-peer-oauth-portal-plan.md`. — *Justification:* landed & merged (`ee9658495`..`a224c3e79`; elements package + a2o scenarios). **LEAVE the session-bridge spec/plan (live).**

### D.12 — Doorway Hub Edge / Stewardship Chain (dupe; after B.4 amend) — **CONDITIONAL**
`superpowers/specs/2026-05-08-doorway-hub-edge-design.md` (retire after seed absorbs boundaries). `plans/2026-05-19-doorway-stewardship-chain-design.md` — **retire ONLY the LANDED-substrate portion's tracking; the §8 task-#16 WIRING is HELD-still-live.** Recommend: retire the hub-edge spec; **HOLD the stewardship-chain plan** until the wiring checklist is either landed or moved to a live successor (see §Flags).

### D.13 — Capability Profile + Element Contract (dupe; after B.5 amend)
`superpowers/specs/2026-05-20-capability-profile-element-contract-design.md`, `superpowers/plans/2026-05-20-capability-profile-element-contract-plan.md`, `superpowers/plans/2026-05-20-protocol-omni-component.md`. — *Justification:* M1-M3 landed (capability primitive in `elohim-core/src/capability/`, CEM plugin, omni component, EprNavContextView). Deferred #3-#7 preserved in seed watch-outs.

### D.14 — App-Manifest Staged-Intents (dupe; after B.6 in-place compaction)
`superpowers/plans/2026-05-28-b-manifest-app-manifest-staged-intents.md` (retire). The design spec is COMPACTED-IN-PLACE, not retired. — *Justification:* B-MANIFEST tasks landed (`bbb707887`/`488e3f484`). **LEAVE the false-dupe modularization plan + the live session-bridge docs.**

### D.15 — Conductor Agent-Info Gossip (after A.2.8)
`superpowers/specs/2026-05-28-conductor-agent-info-substrate-gossip-design.md`, `superpowers/plans/2026-05-28-conductor-agent-info-substrate-gossip.md`. — *Justification:* landed & merged (`c88febe93`; module + 14 tests + catalog row #13 + a2o feature). Soak HELD does not block body retirement (code merged).

---

## Flags

**Still-live — LEAVE IN PILE (do not touch):**
- `genesis/docs/superpowers/specs/2026-05-16-graph-native-projection-substrate-design.md` + `plans/2026-05-16-graph-native-projection-substrate.md` (live successor substrate).
- `genesis/docs/superpowers/plans/2026-05-29-light-up-the-topology-sprint-kickoff.md` (active in-flight sprint, status Draft, ~3 days old).
- `genesis/docs/plans/2026-05-22-value-scanner-content-audit.md` (active standing index; referenced by coherence-substrate-design + qahal-homepage-ux).
- `genesis/docs/superpowers/specs/2026-05-28-session-bridge-design.md` + `plans/2026-05-28-session-bridge-implementation.md` (net-new `crates/session-bridge/`, verified NOT landed).
- `genesis/docs/plans/2026-05-18-app-manifest-modularization.md` (FALSE DUPE — landed `$ref`-refactor, its own structural-refactor cluster).
- The live design/architecture specs being amended/pointed-into (experience-story EPR, attestation-consolidation, hub-boundaries, cradle-to-grave gradient, wave3 VF/hREA) — amendment targets, NOT bodies.
- `genesis/docs/superpowers/sprints/2026-05-24-sweettest-stage-efficiency-w1-w2-w3-w5.md` (landing-record; own pass).

**CONFLICT — operator judgment:** `plans/2026-05-19-topology-resilience-qahal-synthesis.md` appears as body-to-git in the topology dead-arch cluster (D.2) AND as still-live-leave-in-pile in the hub-edge dupe cluster (B.4). It was recently touched (mtime 2026-06-02). **Default: LEAVE in pile** (the more conservative read); operator may retire it under D.2 if confirmed fully superseded by the realized GraphQL Viewer resolvers.

**Held — verify before any seed/record claims "verified-stable" (recommend ci-investigator pass):**
- **Doorway SSR alpha pod** — handoff `cf53a76c2` notes deploy BLOCKED on Harbor registry storage EIO. Code+tests landed; do NOT assert in-cluster green. (B.2 / D.10.)
- **Conductor agent-info gossip** — `ENABLE_CONDUCTOR_AGENT_INFO_GOSSIP` default false; 24h matthew+adam soak outstanding. Code merged. (A.2.8 / D.15.)
- **Stewardship-chain wiring** — substrate landed; task-#16 (imagodei Attestation kinds, `verify_custody_chain`, `request_kind` for transitions) NOT in DNA. HOLD the plan body (D.12 conditional). (B.4.)
- **Experience-story gate** — structural-verified (files + tests exist); no CI evidence gathered. Treat "verified-stable" as HELD. (A.2.6.)
- **Staged-intents vocabulary** — on-disk + validates, but UNEXERCISED end-to-end (zero pillar manifests declare a stagedIntent; session-bridge consumer unbuilt). Assert only "substrate landed", not "feature works". (B.6.)
- **Topology a2o @wip** — env-blocked browser-tier scenarios should be flagged HELD, not asserted green. (A.2.2.)

**Landed-truth canonical-seed candidates to confirm-in-place (not new files):** REA blob-projection action conventions + standing-policy debitWeights + PlacementGapKind/HubSummary enums (topology) → confirm seeded in `elohim/sdk/schemas` / bootstrap manifests / view-schema CONVENTIONS, not buried in retiring bodies. The "hub is the abstraction" constraint → confirm recorded as a durable view-schema rule.

**Low-confidence / operator judgment:** none below HIGH. The single nuance worth a glance: the two enumerated rubrics in junk-`2026-05-14` (ungrudging-service design rules; six stewardship principles) are git-only after clearing — the C1 pointer is the only thing telling a future author they exist. If the operator wants belt-and-suspenders, promote those two rubrics into a durable `elohim-protocol/` ethics doc NOW rather than relying on git + pointer.

---

## Then: MemPalace

After A–D land, the live surface is clean: the pile is ~116 bodies lighter, the museum holds ~16 curated records, 7 canonical surfaces carry the dupe-cluster truth, and every retired body's lesson is reachable via a planted pointer. **Re-point MemPalace ingestion at this cleaned surface and run a fresh re-mine** — it will now index curated history + canonical seeds instead of decomposed junk-drawer bodies and duplicate plan threads, so the drawers fill with load-bearing wisdom rather than residue. The pile:canonical ratio dropping toward ~1.5–2× is exactly the signal that MemPalace is mining a curated corpus, not a junk pile.
