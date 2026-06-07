# HANDOFF — Brainstorm: EPR acquisition affordances + async pull queue + multipeer striping

_Last updated: 2026-06-07 · Author: Claude Opus · Branch: `dev` (3 local commits ride the next dispatch) · Session mode: **handing off into a `/brainstorm` session** (p2p-design-gate mandatory, first structural move)._

_The previous handoff (Slice-3 push + alpha watch) is RESOLVED in substance: the Slice-3 routing substrate is **alpha-verified live** (grants upserted · alias 302 · claimed 302-to-mount · sitemap). Still-pending tails are at the bottom._

---

## Goal

Run a **brainstorming session** that elevates the **acquisition family** — captured 2026-06-07 as one seed (spec **Appendix E** of `epr-route-claims-link-conformance-design` + the backlog's BRAINSTORM SEED entry) — into a canonical spec. The family is one coherent feature: *what a link lets you DO beyond browsing, the queue that does it, and the bandwidth underneath*. Three entangled questions:

1. **The async PULL queue as substrate primitive.** The write half exists: the publish **drain** queue (`status.drain {total, published, pending}`, watched by `wait-for-drain.ts`) reconciles local writes outward. Design its mirror: a declared **desired-content set** (a pin, a cluster closure, an offline subscription) reconciling INWARD — `{total, fetched, pending}`, resumable, prioritized, hash-verified per item, observable with the same wait-for semantics. This is the P1 reconciliation-controller pattern (`project_principle_p1_reconciliation_controller`: DHT=manifest, controller reconciles) pointed at the local node. Open: what IS the desired-set entity (gate: agent-scoped B? its provide-effect B2/A?), where does it live per-device vs per-agent, and what are its lifecycle states?

2. **Acquisition affordances on the link surface.** The ladder (Appendix E): browse → **Open in {pillar}** (parent §7.5 context menu over the claims table — designed, unbuilt) → **download / offline** (Tauri-direct `:8090` exists; apps-sw cache + automerge offline-first; NO UI affordance) → **pin as peer** ("save and replicate" — downloading BECOMES provisioning: a REA *provide* commitment + quilt custody; note the Epic-B gap: `content:<reach>` provide rows exist only in `test_util`) → **sync a cluster** (parent EPR: an album / course / module / lesson = an EPR-head graph walk; `GateHintRelation::ContentToSync` is the existing vocabulary hook). Open: what bounds a cluster closure (relationship types? path steps? depth? size budget?); reach × pinning (may I pin gated content I can read? does my replica serve it? — collides with the **capability-by-hash open decision** below); affordance placement (epr-link popover? context menu? viewer chrome?).

3. **Multipeer striping (the torrent question).** Evidence-based today: apps-sw does peer-**scored sequential failover** (`/api/v1/peers/delivery` → walk `ScoredPeer[]` one at a time, whole-zip). Every striping primitive EXISTS uncomposed: `sharding.rs` + `p2p/blob_protocol.rs` (storage), reassembly-verified `blob_store` (hash check at `blob_store.rs:496`), **`elohim-bitswap`** (libp2p-0.54 port — wired into **steward/node only**), RS(N,K) erasure quilt (tiered-quilt, D5). Compose: scored peers × shard-ranged requests × hash-verified assembly = the bandwidth story for ladder rungs 3–5. **Doorway stays single-target by gospel** (`doorway/CLAUDE.md` No Blob Fan-Out) — swarm belongs to substrate P2P + client delivery, never the doorway.

**Economics thread through all three (shefa):** completing a pull at commons reach naturally flips the node to *providing* — the read→host loop the trust-compute gradient expects ("distribution cost scales with trust"). Provide commitments are the standing; reciprocity is observable.

## The p2p-design-gate load-bearing questions (run the gate FIRST)

- Is a **pin** a `Commitment` (action `provide`/`custody-blob` kin) from the start, or operational-then-committed (pin locally = C/B; *announce as provider* = the notarized step)? The stagespablob/rea-compute-commitment-primitive 5-step recipe applies; Mishpat headroom 11/100.
- The **desired-set**: agent-scoped private (B, device-roaming via source chain?) vs household-shared? Its identity (content-derived over the set? composite?).
- A **cluster** is a relationship-closure — derived (A2) over existing EPR-head links, never a new entry type. The closure RULE (which relations traverse) is the design surface.
- Pull-queue state itself = Operational (C, reconstructable from desired-set + local inventory) — mirror the drain queue's classification.

## Reading list (by role)

**The seed + contract (read first):**
- `genesis/docs/superpowers/specs/2026-06-06-epr-route-claims-link-conformance-design.md` — **Appendix E** (the family, verbatim evidence), Appendix D (flow traceability), §5.3 epr-summary-hint (the pre-fetch decision envelope), §2 R1–R3 (gradient invariants any pull design must honor — audit compute OFF the request path).
- `genesis/data/timeline/backlog/epr-routing-complementary-captures.md` — the BRAINSTORM SEED entry + the **blob capability-by-hash open decision** (interacts hard with pin-gated-content) + load-time bundle integrity capture.

**Substrate canon:**
- `genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md` (D5) — quilt/RS(N,K) custody; the cluster-pin's custody home.
- `genesis/docs/superpowers/specs/2026-04-19-self-healing-p2p-dataplane-design.md` — Plans 1–3 (distribute-at-ingest, verifier, reconstruction); the pull queue is their demand-side sibling.
- `genesis/docs/architecture/rea-compute-commitment-primitive.md` + `2026-05-25-stagespablob-substrate-correct-deploy.md` §1 — the provide-commitment shape + 5-step new-action recipe.
- `genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md` — gradient economics + AttentionTending (the pull queue's prefetch decisions are tending-adjacent).
- `genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md` — links/relationships (the closure walks these) + ContentToSync semantics.
- Memories: `project_principle_p1_reconciliation_controller` · `project_inventory_exchange_not_byte_replication` (gossip = metadata-only; bytes need the dataplane) · `project_resilience_snapshot_humans_junction` (Epic-B provide-rows gap).

**Gospel rails:** `doorway/CLAUDE.md` (No Blob Fan-Out — single-target dispatch) · `.claude/skills/automerge-sync/SKILL.md` (offline-first content) · `.claude/skills/p2p-design-gate/SKILL.md`.

## Code anchors (ground truth, verified 2026-06-07)

- `app/elohim-app/src/apps-sw.ts` — ScoredPeer + `/api/v1/peers/delivery` + `_capability` HEAD probe + sequential peer walk (~l.214) + JSZip extract; cache `apps-v2` (deterministic-zip + auto-invalidation = Sprint-2 debt noted in header).
- `genesis/seeder/src/wait-for-drain.ts` + `status.drain` — the write-queue mirror to copy shapes from.
- `elohim/elohim-storage/src/sharding.rs` · `src/p2p/blob_protocol.rs` · `src/blob_store.rs:496` (reassembly hash verify).
- `elohim/elohim-bitswap/` — forked libp2p-bitswap (0.54); consumed by `steward/node/Cargo.toml` ONLY.
- `app/elohim-elements/elohim-core/src/loader/loader.ts` — the CID-verify-on-load pattern (default-on, hard-fail) any pull fetch should reuse.
- `elohim/elohim-views/src/projection.rs` `GateHintRelation::ContentToSync` — existing vocab hook for cluster sync.
- `app/elohim-elements/elohim-core/src/elohim-epr-popover.ts` + `elohim-context-menu.ts` — the affordance surfaces (§7.5 menu rides claims).

## What worked / constraints discovered (carry forward)

- **Declare+grant claims pattern** (Slice 3) — if pins announce as providers, the same write-time-governance/read-time-table shape applies.
- **Conductor-path upsert semantics**: re-POST same commitment id = idempotent row replace (each POST notarizes anew); 409 paths use explicit supersession (`9d069d6d3`). Pin lifecycle updates can lean on either.
- **anon-reach set is {commons, public}** (`anon_reach_readable`, one definition per side) — pull/provide eligibility must use the same single authority.
- **One-dispatcher rule** (`feedback_concurrent_push_mutual_abort`): coordinate pushes; verify runs SPAWN.

## Still-pending tails from the Slice-3 arc (not brainstorm-blocking)

1. **Shell DI fix verification**: `04fb91d9c` + mint-fix `2dafbde72` are on origin; once the operator's rebuild-all deploys a shell newer than `main-2OW3WZQR.js`, run `E2E_DEVICE_MODE=playwright E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host pnpm exec cucumber-js --tags '@deep-link and not @wip'` (genesis/a2o) — expect 9/9 (was 7/9; the 2 reds are the un-deployed DI fix; plus the new View-as-Content scenario).
2. **Nexus/Harbor incident closure**: clears on a ≥3-green rebuild streak (`ci-nexus-harbor-pvc-jam-incident.md`); recurrence post-rebuild re-escalates (registry-substrate SPOF on 3rd recurrence: `harbor-registry-spof.md`).
3. 3 local commits ride the next dispatch (`b15a16ee3` appendices, `6d1b6024d` CI triage, `dbf15fc91` shift ledger).
4. Deferred-by-design gaps unchanged: `#6-2` gate face (next plan candidate), `#7-5` crawler+sweep, `#5-3` hint consumption; scenario gaps listed in Appendix D (fragment-survival, nav-stack handoff, canonical-correctness, Loader-verify pin).

---

_Open this file in a fresh conversation with `/brainstorm` to begin: scope = "EPR acquisition affordances — async pull queue + cluster pinning + multipeer striping." The reading list above is self-contained._
