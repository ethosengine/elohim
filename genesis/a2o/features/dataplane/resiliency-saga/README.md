# The resiliency-saga

Ten ordered `a2o` chapters narrating one continuous story of the Elohim Protocol's
durability arc — from a single device waking up to a resilience card that tells the
truth on every doorway that serves it. Each chapter is an executable BDD feature file
that runs in the edge pipeline's advisory **Dataplane Validation** stage
(`elohim/holochain/Jenkinsfile`, after **Deploy Edge Node - Alpha**) and rolls up
per-concern in `genesis/a2o/reports/sprint-report-dataplane.json`.

## The narrative

matthew boots a device. He uploads `elohim-host-landing` into his eprfs, and his
device hosts a doorway so a browser can reach it. That's not durable on its own — one
device is one point of failure — so adam co-stewards the content through a
Mishpat-notarized rea-agreement: a `Commitment` naming him a replicating steward,
projected into elohim-storage's `rea_commitments` table. Once that agreement is
notarized, the blobs sync until every peer serving the content converges on ONE head,
not merely "a" head each. Custody of a shard is not real until it is *witnessed* — a
peer must observe and classify who currently holds it, not merely intend to. Each
custodian reports its free and stewarded capacity so the mesh's aggregate posture is
visible, not assumed. Doorway-operator agreements are struck in kind — the
in-scope-of grant that lets a doorway project an EPR at all. Projector caches (the
things reading from a materialized view rather than the live DHT) must carry the same
head the mesh already converged on, or a "cached" resilience card is lying by
omission. And finally: the resilience card rendered on `elohim.host` and on alpha
must tell the SAME truth — two doorways, one truth, or the card is worthless as a
felt-safety signal.

A chapter is not always one node. When a sprint discovers that a chapter is really a
pipeline of nodes — or that an unnamed node sits between two chapters — those nodes
are **minted into the story as stations**, so the next sprint measures them directly
instead of re-deriving them by log archaeology, and so earned progress is visible
rather than hidden behind a single flat red (story-refinement discipline, born
2026-07-26). A station is a capability proof inside a chapter; a green station is
never a green chapter — the chapter's finish line stays exactly where it was.

## Chapter / concern / proof table

Statuses below reflect the board recorded at edge #1264, corroborated where noted by
live probes on 2026-07-30; a status can run ahead of the last recorded build when a
cure has landed and verified but not yet completed its own recorded dataplane-validation
pass — that lag is called out per-row rather than assumed away.

| # | Chapter | Concern tag | Proof signal | Status today |
|---|---------|-------------|---------------|---------------|
| 1 | The device awakens | `@concern:saga-01-device-awakens` | `GET /health` `conductor.connected=true` + peer healthy | GREEN — stable baseline |
| 2 | The household forms | `@concern:saga-02-household-forms` | `/db/humans` rows for the household members carry non-null `agentPubKey` + `householdId=household-dowell` (finish line re-aimed 2026-07-29 off the activity-shaped fill gauges — see Measurement-timing cause 4); sweep-liveness station: `elohim_identity_fill_total{action="skipped_present"} >= 1` | GREEN — recorded 2026-07-29 on the re-aimed durable-truth finish line (all three household members carry non-null `agentPubKey` + `householdId`, `stewardingCollectives=1`); per-member counts remain a separate, still-gated concern behind the operator's captured-UUID chain migration |
| 3 | matthew uploads elohim-host-landing into his eprfs | `@concern:saga-03-eprfs-upload` | served head matches declared head + blobHash non-null on alpha-A | GREEN — matthew is the deploy-time author peer |
| 4 | The doorway serves | `@concern:saga-04-doorway-serves` | `GET /` → 200 with the rendered SPA shell | GREEN in steady-state serving — the flap decomposes into two distinct causes (2026-07-30): (1) the raw-GET racing the doorway pod restart, cured 2026-07-29 by the bounded `/health` wait in the step path; (2) a deeper structural cause — every edge deploy restarts the alpha conductors, reopening adam's extended projection catch-up window (hours; a per-space provisioning ceiling, operator-decision documented in `genesis/data/timeline/backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md`) — during which `elohim.host` correctly serves the SPECIFIED 503 shed contract on `/` (doorway-catching-up-page design). Steady-state serving vs. deploy-window shed: the chapter records green only when validation runs outside that window. Remedy direction: a validate-only edge pipeline mode (decouples measurement from deploy restarts) plus the operator ceiling items in the backlog entry |
| 5 | adam co-stewards via a rea-agreement | `@concern:saga-05-co-steward-agreement` | **Decomposed into stations (2026-07-26).** Station A: an active item pin on `/api/v1/pins` references the content **and** `/api/v1/pins/{id}/pull` reports `caughtUp`. Station B: `/api/v1/commitments/facing/rea` carries ≥1 `replicates-commons` row in state `active`. Finish line (unchanged): `GET /api/v1/commitments?action=replicates-commons&state=active` reports ≥1 row within 60s | GREEN — stable across 3+ consecutive builds since 2026-07-29. The cure sprint proved this chapter is a *pipeline* (pin → provide tick → mishpat notarization → bounds-validated announce → graduation → mishpat→rea mirror → rea projection) with eight stacked defects along it; the closing defect — the mishpat→rea mirror never minting a live `active` commitment into `rea_commitments` — is now cured, and stations A and B land green together with the finish line |
| 6 | Blobs sync to one head | `@concern:saga-06-heads-converge` | `caughtUp=true` + `elohim_projection_heal_outcomes_total{outcome="healed"}>=1` + `elohim_projection_reconcile_converged>=1` locally (reframed 2026-07-26 off the per-request-windowed `/health divergentAnchor`, which flapped); served head matches on both federation doorways | Local-converged leg GREEN in steady state — races the same deploy-window restart as chapter 4 (`elohim_projection_reconcile_converged` reads 0 within minutes of a restart, then recovers). Cross-node one-head leg (needs `@requires:alpha-cluster-6peer`) still RED, pending B reaching `caughtUp=true` plus one `authorHeadOnce` declare cycle (a `[build:app]` push) — mechanism proven, documented in the same `self-heal-adam-projection-catchup-exhaustion-full-arc.md` backlog entry. Three `@wip` stations still mint the ghost-witness sweep's uninstrumented failure classes (chain-contention livelock, stale-anchor create collision, wall-clock-budget abandonment) |
| 7 | Custody is witnessed | `@concern:saga-07-custody-witnessed` | labeled `elohim_custody_class_count{class="stocked"} >= 1` | Data path CURED (2026-07-30): the custody manifest + self-held evidence backfill in elohim-storage's reconcile path (`manifest_backfill_pass`) projects manifests + self-held `shard_locations` for every blob the node verifiably holds, and the genesis seeder (`seed-commitments.ts`) now resolves content's `serverBlobHash` at seed time with a matthew self-custody pair for `elohim-host-landing` in the defaults — live-verified `stocked=1` on alpha-A. Honest classes (`none`/`unknown`/`stocked`) now flow correctly; recorded-green pending the next dataplane-validation run (see Documented residue below for the blobHash/serverBlobHash join-key gotcha this cure depends on) |
| 8 | Capacity is reported | `@concern:saga-08-capacity-reported` | `elohim_custodian_free_bytes > 0` (also `stewarded_bytes >= 0`) | GREEN — live infrastructure; free-bytes non-zero, stewarded-bytes now unblocked by chapter 2's recorded-green identity fill |
| 9 | Projector caches carry the head | `@concern:saga-09-projectors-carry` | `GET /api/v1/resilience/elohim-host-landing/household` `commitmentBackedReplication.commonsCommitments >= 1` | GREEN — stable across 3+ consecutive builds since 2026-07-29. The field rides the served `ResilienceSnapshotView` (the prior wiring gap is closed); `commonsCommitments` flips 0→1 the same moment chapter 5's mirror cure lands, which it now has. Re-aimed off `totalPledgedBytes`, which stays unreachable by design from a content-tier commitment (`replication_commitment.rs:182-194` pledges 0 bytes for `replicates-content`/`replicates-commons` content commitments; they're counted, not summed) and has no capacity-tier pledge producer in the codebase at all (backlog residue below) |
| 10 | The card tells the truth | `@concern:saga-10-card-tells-truth` | same non-zero `stewardingCollectives` on both alpha-A and elohim.host; `@wip`/`@browser-only` rendered-card companion | GREEN — both `elohim.host` and alpha-A serve identical non-zero `stewardingCollectives`, unblocked now that chapters 5 and 9 have landed; the rendered `@wip`/`@browser-only` card-verdict scenario remains a separate, not-yet-executed companion |

## RED-FIRST is correct

A chapter born red names the loop's actual work queue for the durability arc, not a
test-authoring gap. Do not weaken an assertion to make it pass; the assertion IS the
specification for the fix. Flipping red→green across two consecutive
dataplane-validation builds is the measurable signal that a cure landed — chapters 5,
9, and 10 in this saga flipped exactly that way once their respective cures shipped.
The discipline stays live wherever work remains open: chapter 6's cross-node leg and
chapter 7's finish line are this saga's current born-red work queue (see `../README.md`'s
"Agentic-developer loop consumption" section for how the loop reads this).

## Measurement-timing fix for the gauge-backed chapters (2026-07-26)

Chapters 2, 7, and 8 read `elohim_`-prefixed gauges that elohim-storage populates
from a periodic background sweep (~5-minute cadence for identity-fill and
custodian-capacity; custody-class is request-triggered — see cause 4 and the
chapter-7 feature header), not on every scrape. Their chronic "pending-env /
metric not reachable" status had FOUR compounding causes,
the third being the sweep-cadence race originally suspected:

1. **Env-wiring gap**: `resolveStorageUrl('alpha-A')` (`src/framework/dataplane/surfaces.ts`)
   read only `E2E_STORAGE_ALPHA`, which the Dataplane Validation stage never sets
   (it sets `E2E_STORAGE_URL`, the generic name several other step files already
   read for the same alpha-A/matthew target) — every alpha-A gauge assertion
   returned "storage metrics URL not set" before ever reaching a probe. Fixed by a
   fallback: `E2E_STORAGE_ALPHA ?? E2E_STORAGE_URL`.
2. **Routing bug**: the bare `Then metric "..." on peer "..."` step
   (`steps/dataplane.steps.ts`) routed only `p2p_`/`reconcile_`/`dedup_`-prefixed
   names to the direct storage URL — `elohim_`-prefixed names (chapter 2's bare
   scenario, both of chapter 8's) fell through to the DOORWAY's port-8080
   `/metrics`, which never carries them. Fixed by adding `elohim_` to that prefix
   check (the labeled-metric step in `steps/dataplane/resiliency-saga.steps.ts`
   already had it right).
3. **Sweep-cadence race**: once reachable, a single probe ~2 minutes post-restart
   can still race the sweep's own cadence. Fixed with `pollForGauge()`
   (`src/framework/dataplane/surfaces.ts`): 30s intervals, up to 6 minutes, shared
   by both the bare and labeled metric steps. A populated gauge resolves on the
   FIRST attempt (zero added cost); a genuinely-still-unswept gauge polls the full
   budget before declaring pending — never a hard failure.

4. **Activity-vs-truth trap** (found 2026-07-29, chapter 2): the original chapter-2
   proof signals were *activity-shaped* and structurally unable to stay green after
   the cure succeeded — `elohim_identity_fill_discovered_cids` is a per-sweep
   overwrite `IntGauge` (zeroed by every restart until a tick lands) and
   `identity_fill_total{action="created"}` reads 0 forever once every member row
   exists (the already-present short-circuit). A fully cured pod was
   indistinguishable from a never-run one, so the cure's success *regressed* the
   gauge. Fixed by re-aiming the finish line at the durable state the sweep
   produces (`/db/humans` row fields) and keeping a steady-state-nonzero liveness
   station (`action="skipped_present"`). The general rule: a chapter's finish line
   asserts the OUTCOME state; activity metrics are stations that prove the
   machinery ran, never the pass/fail proof. (Chapter 7's request-triggered
   custody-class gauge, fixed the same day, is the sibling instance: an
   emit-on-request gauge asserted as though a background sweep populated it.)

Chose a bounded in-step poll over a harness-level second pass because both step
definitions already had the `resolveStorageUrl`/`retry()` machinery to extend, the
fix stays local to the two files that needed it (no new failure feature files),
these two steps' only current callers are chapters 2/7/8 (verified — no other
`@dataplane` feature uses either step), and it composes with cucumber's per-step
`{ timeout }` override the same way the chapter-5 stations already do for their
60-75s polls.

## Reuse over reinvention

Every chapter reuses an existing step definition wherever one already fits:
`steps/dataplane.steps.ts` for `/health`, `/metrics`, `/db/content`, and served-head
comparisons; `steps/resilience.steps.ts` for the `/api/v1/resilience/*` household
snapshot (`When I read "<path>"` + `Then the response field "a.b" is at least <n>`
supports dotted paths). New glue lives only where no existing primitive fit: a
label-aware Prometheus step (chapters 2, 7), a commitment-count poll (chapter 5), a
raw-body HTML check (chapter 4), and a cross-doorway truth compare + `runLook` card
verdict (chapter 10) — all in `steps/dataplane/resiliency-saga.steps.ts`.

## Documented residue

Chapter 9's proof signal is deliberately scoped to `commonsCommitments`, not
`totalPledgedBytes`. Both fields ride the same served `commitmentBackedReplication`
object, but only a **capacity-tier** pledge (a `replicates-dwelling` commitment, or
the `capacity` variant of a `replicates-commons` commitment) contributes bytes
(`elohim-facings/src/folds/replication_commitment.rs:36-42, 182-194`); a
**content-tier** commitment — the kind chapter 5 proves — names an EPR and pledges 0
bytes by design. No capacity-tier pledge producer (a coordinator function or HTTP
lever that authors a `replicates-dwelling` / commons-`capacity` commitment) exists
anywhere in the codebase today. Building one is a backlog item this saga has
surfaced but does not itself resolve; when it lands, chapter 9's assertion can be
widened back to (or paired with) `totalPledgedBytes >= 1`.

Content rows carry TWO hashes — a client `blobHash` and a `serverBlobHash` — and the
blob store plus the custody fold (chapter 7's data path) both key by the SERVER hash.
A pledge or seed record classified by the client hash never joins the custody fold; it
silently reads as `none`/`unknown` forever, which cost hours to diagnose during the
chapter-7 cure. The canonical join key is `serverBlobHash` until the CID-first
migration lands — new code touching custody, seeding, or blob-hash comparisons should
resolve `serverBlobHash` explicitly rather than assume the client hash is fold-joinable.

Demand-autopins have no retirement path (surfaced by the chapter-5 cure work): a
pin whose acquisition exhausted its retry budget stays `active` forever — it
occupies one of `MAX_ACTIVE_PIN_ROWS`, is re-scanned by both 60s reconcile loops
every tick, and holds `pull.caughtUp` false in perpetuity (alpha's `e2e-*`
phantom pins show exactly this). Retirement semantics (TTL, an auditable
`abandoned` state, or source-side exclusion of ephemeral ids) is an operator
policy decision about what "I wanted this" means once provably unfetchable —
deliberately not decided by the cure sprint.
