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
| `blob-custody` | libp2p/iroh blob placement; the salvage-diversity strategy re-places under household awareness |
| `peer-mesh` | DHT-peer discovery, connection establishment, and gossip propagation across household nodes |
| `routing` | EprRouter correctness — concern-scoped content reaches the right renderer, no empty-router poison |
| `projection` | DHT-to-storage projector lag; coordinator hot-swap; signal delivery to the Angular subscriber |
| `blob-replication` | EPR blobHash metadata propagation to federation peers; cross-peer EPR record consistency (RED-FIRST: gap on elohim.host as of 2026-06-29) |
| `epr-projection-fallback` | EprRouter fallback when blobHash is null on a federation peer; peer-proxy or syncing-status response rather than "App not found" (RED-FIRST: gap on elohim.host as of 2026-06-29) |

Add new concerns here as they are introduced. Concern names must be lowercase kebab-case (e.g.
`@concern:my-new-concern`), and must appear in the table above before the feature file is merged.

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
