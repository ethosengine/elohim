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

## Chapter / concern / proof table

| # | Chapter | Concern tag | Proof signal | Status today |
|---|---------|-------------|---------------|---------------|
| 1 | The device awakens | `@concern:saga-01-device-awakens` | `GET /health` `conductor.connected=true` + peer healthy | GREEN — stable baseline |
| 2 | The household forms | `@concern:saga-02-household-forms` | `elohim_identity_fill_discovered_cids >= 1` + labeled `elohim_identity_fill_total{action="created"} >= 1` | RED — born red until the identity-fill ceremony cure deploys |
| 3 | matthew uploads elohim-host-landing into his eprfs | `@concern:saga-03-eprfs-upload` | served head matches declared head + blobHash non-null on alpha-A | GREEN — matthew is the deploy-time author peer |
| 4 | The doorway serves | `@concern:saga-04-doorway-serves` | `GET /` → 200 with the rendered SPA shell | GREEN |
| 5 | adam co-stewards via a rea-agreement | `@concern:saga-05-co-steward-agreement` | `GET /api/v1/commitments?action=replicates-commons&state=active` reports ≥1 row within 60s | **RED — born red**, the loop's work queue |
| 6 | Blobs sync to one head | `@concern:saga-06-heads-converge` | `/health` `divergentAnchor<=0` + `caughtUp=true` locally; served head matches on both federation doorways | GREEN locally; cross-node scenario needs `@requires:alpha-cluster-6peer` |
| 7 | Custody is witnessed | `@concern:saga-07-custody-witnessed` | labeled `elohim_custody_class_count{class="stocked"} >= 1` | **RED/PENDING — born red**, gauge not yet deployed (Track 3 landing now) |
| 8 | Capacity is reported | `@concern:saga-08-capacity-reported` | `elohim_custodian_free_bytes > 0` (also `stewarded_bytes >= 0`) | Live infrastructure; free-bytes expected non-zero, stewarded-bytes gated on chapter 2 |
| 9 | Projector caches carry the head | `@concern:saga-09-projectors-carry` | `GET /api/v1/resilience/elohim-host-landing/household` `commitmentBackedReplication.totalPledgedBytes >= 1` | **RED — born red**, `household_resilience.rs:131` hard-codes the field to its zero default (`// T15: computed`) |
| 10 | The card tells the truth | `@concern:saga-10-card-tells-truth` | same non-zero `householdsStewarding` on both alpha-A and elohim.host; `@wip`/`@browser-only` rendered-card companion | **RED — born red** on the numeric compare; the rendered scenario stays `@wip` |

## RED-FIRST is correct

Chapters 5, 7, 9, and 10 are **born red** by design — they name the loop's actual work
queue for the durability arc, not a test-authoring gap. Do not weaken their assertions
to make them pass; the assertion IS the specification for the fix. A chapter flipping
red→green across two consecutive dataplane-validation builds is the measurable signal
that its cure landed (see `../README.md`'s "Agentic-developer loop consumption"
section for how the loop reads this).

## Reuse over reinvention

Every chapter reuses an existing step definition wherever one already fits:
`steps/dataplane.steps.ts` for `/health`, `/metrics`, `/db/content`, and served-head
comparisons; `steps/resilience.steps.ts` for the `/api/v1/resilience/*` household
snapshot (`When I read "<path>"` + `Then the response field "a.b" is at least <n>`
supports dotted paths). New glue lives only where no existing primitive fit: a
label-aware Prometheus step (chapters 2, 7), a commitment-count poll (chapter 5), a
raw-body HTML check (chapter 4), and a cross-doorway truth compare + `runLook` card
verdict (chapter 10) — all in `steps/dataplane/resiliency-saga.steps.ts`.
