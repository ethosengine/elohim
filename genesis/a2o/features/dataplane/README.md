# Dataplane validation suite

This directory holds per-concern P2P dataplane scenarios — executable BDD specifications for
cross-node synchronization, blob custody, routing, and peer-mesh behavior.

## Concern taxonomy

Each feature file in this directory tags every scenario with exactly ONE `@concern:<name>` tag.
The `byConcern` aggregator in `scripts/lib/aggregate.ts` uses this tag to bucket pass/fail/pending
results, and `scripts/lib/render-markdown.ts` renders a "Dataplane validation by concern" section
in the sprint report.

| concern | what it covers |
|---|---|
| `content-sync` | CRDT convergence — two nodes exchange an Automerge document and reach identical state |
| `peer-mesh` | DHT-peer discovery, connection establishment, and gossip propagation across household nodes |
| `blob-replication` | EPR blobHash metadata propagation to federation peers; cross-peer EPR record consistency (RED-FIRST: gap on elohim.host as of 2026-06-29) |
| `epr-projection-fallback` | EprRouter fallback when blobHash is null on a federation peer; peer-proxy or syncing-status response rather than "App not found" (RED-FIRST: gap on elohim.host as of 2026-06-29) |
| `federation-deploy` | An EPR resolves on ALL federation doorways (alpha-A AND elohim.host) — kills the per-host stageSpaBlob crutch. Primary surfaces: per-doorway `GET /db/content/{id}.blobHash` non-null + `GET /` not App-not-found across all doorways. |
| `blob-durability` | Deterministic floor — blob heal-on-read (race-fetch), chaos/churn survival, grandma-vertical felt safety, household-diversity placement, salvage placement, governed distribution. Sourced from `features/resilience/` (not this directory). |
| `keyspace-coverage` | Cluster-wide RS coverage, placement-gap counts, and weave-lens capacity eyes. Sourced from `features/resilience/operational-weave.feature`. |
| `reconcile-inventory` | Commitment inventory reconciliation — commitment-backed card counting, custody-pair naming, peer-discovered commitment convergence, substrate delivery reconciliation. Sourced from `features/resilience/`. |

Add new concerns here as they are introduced. Concern names must be lowercase kebab-case (e.g.
`@concern:my-new-concern`), and must appear in the table above before the feature file is merged.

### Deterministic-floor concerns (sourced from `features/resilience/`)

The three concerns `blob-durability`, `keyspace-coverage`, and `reconcile-inventory` are the
**deterministic floor** of the dataplane validation suite. Their scenarios live in
`genesis/a2o/features/resilience/` (not this directory) but are tagged `@concern:<name> @dataplane`
so the `byConcern` aggregator surfaces them in the same matrix alongside the live concerns above.
This means the per-concern sprint report and the agentic-developer loop measure the resilience layer
**without re-authoring** any scenarios — tagging is the only bridge needed.

Files and their concern assignments:

| file | concern |
|---|---|
| `resilience/app-blob-heal-on-read.feature` | `blob-durability` |
| `resilience/chaos-peer-churn.feature` | `blob-durability` |
| `resilience/governed-distribution.feature` | `blob-durability` |
| `resilience/grandma-photos-survive-node-loss.feature` | `blob-durability` |
| `resilience/household-diversity-dataplane.feature` | `blob-durability` |
| `resilience/observable-distribution.feature` | `blob-durability` |
| `resilience/resilience-dimensions.feature` | `blob-durability` |
| `resilience/salvage-placement.feature` | `blob-durability` |
| `resilience/operational-weave.feature` | `keyspace-coverage` |
| `resilience/commitment-backed-card-lighting.feature` | `reconcile-inventory` |
| `resilience/household-reciprocity.feature` | `reconcile-inventory` |
| `resilience/substrate-reconciliation.feature` | `reconcile-inventory` |
| `resilience/conductor-memory-soak.feature` | (untagged — conductor OOM, not a dataplane concern) |

## Tagging conventions

```gherkin
@e2e @dataplane @concern:content-sync
Feature: Content sync convergence
  ...

  @requires:alpha-cluster-6peer
  Scenario: Two household nodes converge on an Automerge document within 5 s
    ...
```

Required tags per scenario:

| tag | required | notes |
|---|---|---|
| `@e2e` | yes | marks the scenario as executable by the a2o runner |
| `@dataplane` | yes | routes it into dataplane-specific CI gates |
| `@concern:<name>` | yes | exactly one; drives the `byConcern` rollup |
| `@requires:<cap>` | when needed | substrate scope gate (see `a2o/CLAUDE.md`) |

## Adding a new suite

1. Pick or extend a concern name from the table above (or add a new row).
2. Create `features/dataplane/<concern-slug>.feature` (or add scenarios to an existing one).
3. Tag every `Scenario:` with `@e2e @dataplane @concern:<name>`.
4. Add `@requires:<cap>` on any scenario that needs a cluster capability (e.g. `@requires:alpha-cluster-6peer`).
5. Write step definitions under `steps/dataplane/<concern-slug>.steps.ts` (or extend existing).
6. Run `pnpm test:unit` to verify your scenario appears in the `byConcern` rollup of a synthetic report.
7. Run `pnpm scan:coverage` to confirm coverage mapping is sane.

## Sprint-report output

When any dataplane scenario runs, the sprint report includes:

```markdown
## Dataplane validation by concern

| concern       | status | passed | failed | pending |
|---|---|---|---|---|
| content-sync  | ❌     | 3      | 1      | 0       |
| peer-mesh     | ✅     | 2      | 0      | 0       |

### ❌ `content-sync`
- ✅ Content sync delivers to peer — `features/dataplane/content-sync.feature`
- ❌ Content sync fails under partition — `features/dataplane/content-sync.feature`
...
```

The JSON counterpart lives at `summary.byConcern` in `reports/sprint-report.json`.

## Status glyphs

| glyph | meaning |
|---|---|
| ✅ | all scenarios in this concern passed |
| ❌ | at least one scenario failed |
| ◌ | no failures, but at least one scenario is pending/not-yet-implemented |

## CI wiring

The `Dataplane Validation` stage in `elohim/holochain/Jenkinsfile` runs after `Deploy Edge Node - Alpha` on `dev` pushes (and on `feat-*`/`claude/*` branches that touch `doorway/**`, `elohim/elohim-storage/**`, or `elohim/holochain/edgenode/**`). The bash body lives in `scripts/ci/run-dataplane-validation.sh` (CPS size-limit discipline: no inline heredoc in the Jenkinsfile).

The stage is **advisory**: `catchError(buildResult:'SUCCESS', stageResult:'UNSTABLE')`. A red concern surfaces as UNSTABLE — visible in the Jenkins stage view — without blocking the orchestrator's downstream cascade (seeding, genesis). Once every concern passes on `dev`, flip the `catchError` to `buildResult:'FAILURE'` to harden it to a gate.

Artifacts archived per build:
- `genesis/a2o/reports/sprint-report-dataplane.json` — machine-readable; consumed by the agentic-developer loop
- `genesis/a2o/reports/sprint-report-dataplane.md` — human-readable stage-view summary
- `genesis/a2o/reports/cucumber-report-dataplane.json` — raw cucumber output (scenario-level detail)

The cucumber run uses `--format json:reports/cucumber-report-dataplane.json` (alongside config defaults), keeping this run's output distinct from the main `cucumber-report.json`.

## Agentic-developer loop consumption

The loop reads `sprint-report-dataplane.json` → `summary.byConcern{}` as its **per-concern measure surface**.

```json
{
  "summary": {
    "byConcern": {
      "blob-replication":       { "passed": 0, "failed": 2, "pending": 0, "scenarios": [...] },
      "epr-projection-fallback":{ "passed": 0, "failed": 1, "pending": 0, "scenarios": [...] },
      "content-sync":           { "passed": 3, "failed": 0, "pending": 0, "scenarios": [...] },
      "peer-mesh":              { "passed": 2, "failed": 0, "pending": 0, "scenarios": [...] }
    }
  }
}
```

Semantics for the loop:
- A concern with `failed > 0` (❌) is a **named candidate** for the next fix iteration.
- A concern flipping ❌ → ✅ (`failed` drops to 0, `passed > 0`) across two consecutive builds is **measurable forward progress** — equivalent to a ci-findings ledger entry resolving.
- A concern with `passed == 0` and `failed == 0` (◌) means no scenarios ran for it; the substrate scope gate (`@requires:<cap>`) may have skipped them when the capability is unavailable.

No new ledger is needed. The `byConcern` block IS the measure surface — it is the dataplane analog of the `ci-findings.jsonl` fingerprint table, but concern-scoped rather than error-fingerprint-scoped.
