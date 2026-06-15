---
id: node-resource-tunables-and-exhaustion-shape
status: design
created: 2026-06-15
class: substrate
artifact_kind: spec
written: 2026-06-15
cites:
  - conductor-authority-arc-memory-scaling | the arc-factor scaling rationale this spec exposes as a deploy knob (A2) | path: genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-memory-scaling.md
  - resilience-dimensions-proof-suite | the Rust-test-as-source-of-truth a2o discipline this spec extends for resource limits (B) | path: genesis/docs/superpowers/specs/2026-06-12-resilience-dimensions-proof-suite-design.md
  - genesis/data/timeline/backlog/resilience-card-self-cid-provide-loop-gate.md
  - genesis/data/timeline/backlog/doorway-conductor-reconnect-storm-matthew-edge.md
  - genesis/data/timeline/backlog/seeder-validate-deployments-stale-validator-plus-human-gap.md
---

# Node Resource Tunables, a2o Limit Documentation & the Exhaustion-Shape Report

**One loop: _tune_ a runaway-resource parameter → _document_ its limit as an a2o story backed by a Rust boundary test (the numeric source of truth) → when it is breached in the wild it lands as a self-heal backlog entry → _aggregate_ those entries into a report on the shape of where runaway-resource issues occur across the stack.**

The doorway warm_stream cure (`DOORWAY_MONGO_OP_TIMEOUT_MS` / `WARMUP_PACE_MS`, commits `4dc862748` / `19fb41974`) is the proven template for the first two steps. This spec generalizes that template to the Matthew conductor, gives the limits a documented home, and builds the missing fourth step — the shape report. It also shakes out the three concrete residuals that motivated the work (seeder validator, the caleb/daniel/emma human-gap, the all-zeros resilience card).

## The through-line

A runaway-resource parameter is only *operable* when four properties hold together:

1. **Tunable** — a single env var, parsed by a pure function, with a sane default (the doorway two-function idiom: `parse_*(Option<String>) -> T` + thin `*_env()` wrapper).
2. **Bounded** — the default is safe, garbage/zero falls back deliberately, and the runaway operation is wrapped so it cannot grow without limit.
3. **Documented** — the numeric limit lives in a Rust boundary `#[test]` (source of truth) and is narrated in an a2o `@regression`/`Scenario Outline` scenario that *references the test by name* rather than re-encoding the number (the established `resilience-dimensions.feature` ⟷ `tests/household_resilience.rs` discipline).
4. **Reportable** — when the limit is breached on a live node, the incident is captured as a self-heal backlog entry with structured frontmatter (`nodes`, `tags`, `severity`, `## What is exhausted`), and a report aggregates those entries into "where on the stack do runaway-resource issues occur, and of what class."

Today properties 1–3 exist piecemeal (the doorway params have 1–2 but only prose docs; the arc system has a full actuator but no deploy knob; the SQLite pool has none). Property 4 does not exist at all. This spec closes the set.

## What the investigation changed about the obvious plan

- **arc-factor already has a full actuation system** (`elohim/elohim-storage/src/services/arc_actuator.rs`, `arc_policy.rs`, `http.rs` endpoints, the `sets-authority-arc` Mishpat commitment schema; committed 2026-06-13). The gap is not "build an actuator" — it is "expose a *deploy-time* per-node knob." And **fractional arc <1 is blocked upstream**: kitsune2 / `holochain_conductor_api` type `target_arc_factor: u32` honors `{0, 1}` only (`default_target_arc_factor() -> 1`). So the available lever is the `{0,1}` switch, not `0.8`.
- **Matthew CPU is already per-node tunable** via `genesis/orchestrator/data/deployments.json` (`edgenodeCpuLimit`), and Matthew is already fleet-highest at `3000m`. "Raise cpu" is a field edit + dated comment, not new machinery.
- **The one genuinely-hardcoded knob is the SQLite pool**: `elohim/elohim-storage/src/db/mod.rs:251` `.max_size(10)`, constructed once and cloned everywhere.
- **The resilience card is data-starved, not a display bug — and partly _honestly_ zero.** Some fields are seedable with real data; genuine multi-peer stewardship cannot be manufactured by a bulk seed and the code carries an explicit honesty contract (`status="seeded"` audit stamps). We will not fake it.

## Scope

| # | Workstream | Build | Risk |
|---|---|---|---|
| A1 | SQLite pool env knob `STORAGE_DB_POOL_SIZE` | yes | low (default unchanged) |
| A2 | arc-factor deploy env knob `ELOHIM_TARGET_ARC_FACTOR` ({0,1}) | yes | low (default unchanged; reuses existing renderer) |
| A3 | Matthew cpu band-aid 3000m→4000m | yes | low (repo manifest; next pipeline reconciles) |
| B | Rust boundary tests + a2o limit-documentation scenarios | yes | low |
| C | Exhaustion-shape report (`build-resource-shape-report.ts`) | yes | low (read-only aggregation) |
| D1 | Seeder validator `manifest ?? template` fix + test | yes | low |
| D2 | humans.json: ADD caleb/daniel/emma | yes | low (story-first content) |
| E1 | Seeder honest provide-row flow (light commitment-backed) | yes | medium (verify alpha seed path) |
| E2 | Operator-gated seeded shard-manifest stage (light stewarding) | yes | medium (honesty-gated) |
| F | Push 2 backlog commits to dev | operator-owned | n/a |

Out of scope (flagged, not silently dropped): fractional arc (<1) — blocked on upstream kitsune2 sharding; running the seed against live alpha pods and genuine multi-peer `distribute_shards` replication — operator/pipeline domain (we never `kubectl`; we make the repo coherent so the next reconcile is correct).

---

## A. Conductor resource tunables

### A1 — SQLite read pool: `STORAGE_DB_POOL_SIZE`

`elohim/elohim-storage/src/db/mod.rs:247-256` hardcodes `.max_size(10)` on the single process-wide r2d2 pool over `content.db`. This pool is shared by the heartbeat, the InfrastructureSignal subscriber, the reconcile controller, the import-handler drain, HTTP handlers, and bulk seeding — exactly the contention surface that shows up as stalled reads under load on the Matthew edge.

**Change.** Introduce the doorway two-function idiom:

```rust
fn parse_db_pool_size(raw: Option<String>) -> u32 {
    raw.and_then(|v| v.parse::<u32>().ok())
        .filter(|&v| v > 0)          // 0/garbage → default (never a zero-size pool)
        .unwrap_or(DB_POOL_SIZE_DEFAULT)   // 10 — preserves current behavior
}
fn db_pool_size() -> u32 {
    parse_db_pool_size(std::env::var("STORAGE_DB_POOL_SIZE").ok())
}
```

Apply at the `Pool::builder().max_size(db_pool_size())` site. Default `10` ⇒ no behavior change when unset. Wire `STORAGE_DB_POOL_SIZE=20` for Matthew via its manifest env (alongside the existing `ENABLE_CONTENT_DB`). **Tests (TDD, co-committed):** `parse_db_pool_size` for default / override / zero / garbage.

### A2 — arc-factor deploy knob: `ELOHIM_TARGET_ARC_FACTOR`

The actuator already knows how to render the value: `arc_actuator::render_conductor_arc_factor(yaml, factor)` inserts/replaces `target_arc_factor:` under the `network:` block and refuses any value ∉ {0,1} (`NotActuatable`). The runtime POST path is grant-gated by a Mishpat commitment — correct for *runtime* actuation, but we also want a plain **deploy-time operator setting**, mirroring the steward precedent (`steward/device/src-tauri/src/lib.rs:139` sets `target_arc_factor = 0` for mobile) and the doorway env idiom.

**Change.** Add a clap arg in `main.rs` (consistent with `conductor_config_path`, `conductor_max_retries`):

```rust
#[arg(long, env = "ELOHIM_TARGET_ARC_FACTOR")]
target_arc_factor: Option<u32>,
```

Before `ConductorManager::start()`, if `Some(f)`, read the conductor config YAML, call `render_conductor_arc_factor(&yaml, f)?` (which validates `{0,1}`), and write it back atomically. Unset ⇒ untouched ⇒ library default `1` (current behavior). This is a thin boot-time "ensure the configured factor is present" step that reuses the existing, already-tested renderer; it deliberately bypasses the grant gate because it is an *operator deploy setting*, not a peer-governed runtime change (the two trust models coexist by design).

**Matthew stays at full arc (1).** Matthew is a genesis bootstrap peer (`genesisPeer: true`); dropping it to leecher (0) would be refused by the coverage floor (`arc_actuator.rs:152-172` — "the cure must never cause the partition") and is the wrong node for the lever. The knob exists for leaf/household scaling, where it is the durable answer to corpus-proportional RAM (see the cited memory-scaling spec). **Tests:** the existing `render_conductor_arc_factor` suite covers the apply; add a boot-path test that an unset env leaves the YAML unchanged and a set env produces the rendered YAML.

### A3 — Matthew cpu headroom (band-aid)

`deployments.json` Matthew record: `edgenodeCpuRequest 1000m→1500m`, `edgenodeCpuLimit 3000m→4000m`, with a dated `$cpuBumpComment` mirroring the `ece274734` doorway pattern and the existing `$memoryBumpComment` convention. **Explicitly a band-aid:** the reconnect-storm root (conductor closing app-ws sessions ~40/min, per the cited backlog) reads as session/pool pressure, not raw CPU starvation — A1 (pool) is the targeted lever; A3 is headroom insurance while A1's effect is measured.

---

## B. Documented limits as a2o stories

Follow the established two-layer discipline: **the number lives in a Rust boundary `#[test]`; the a2o scenario references the test, it does not re-encode the number.**

- Rust parser/bound tests from A1/A2 are the source of truth for those knobs' defaults and the `{0,1}` arc bound.
- Add `genesis/a2o/features/deployment/node-resource-tunables.feature` (`@e2e @deployment`, scenarios `@wip` pending step defs, consistent with the doorway resilience feature): a `Scenario Outline` enumerating each node resource tunable — knob name · env var · default · bound · guarding Rust test — and `@regression` scenarios naming the exact param + guarding test for the bounded behaviors (zero→default, garbage→default, arc∉{0,1}→refused). This is the "documented limits in our a2o stories" deliverable and the data source the report in C reads for the *declared-limit* axis.
- Index the knobs in this spec's table (above) so there is one canonical map of "which tunable lives where."
- Scaffold via `story-harvest` so the parameter-bearing discoveries are captured the way the discipline intends.

---

## C. Exhaustion-shape report

**Gap:** there is no report that aggregates resource-exhaustion incidents into a stack-shape view. The pieces exist: the self-heal backlog entries (`genesis/data/timeline/backlog/self-heal-*.md` and resilience/cluster-pressure siblings) carry structured frontmatter (`nodes`, `tags`, `severity`, `fingerprints`, `## What is exhausted`); `.claude/data/ci-findings.jsonl` carries fingerprints with `seen`/`first_build`/`last_build`; and `genesis/a2o/scripts/build-sprint-report.ts` + `lib/aggregate.ts` is the schema-validated report scaffold to mirror.

**Change.** New `genesis/a2o/scripts/build-resource-shape-report.ts` (+ `schemas/resource-shape-report.schema.json`):

- **Read:** all backlog `.md` with a `## What is exhausted` section or a `self_heal_status`/resilience/`recovery`-family tag; their frontmatter (`nodes`, `tags`, `severity`, `fingerprints`); cross-link `fingerprints` → `ci-findings.jsonl` (`seen`/build span); and the declared tunables from B (the `node-resource-tunables.feature` table) so the report can mark which exhaustion classes have a *documented limit* vs which are undocumented.
- **Aggregate three axes:**
  1. **by node** (`matthew`, `james`, `jessica`, `intel-nuc`, `doorway-alpha`, …) — where on the stack.
  2. **by resource class** — derived from tags/`## What is exhausted` into a fixed vocabulary: `cpu` · `memory` · `db-pool` · `arc/dht-working-set` · `sessions/connections` · `disk/pvc` · `runtime/scheduling`.
  3. **by lifecycle** — `self_heal_status` (in-progress / cured / blocked) and whether a documented limit + guarding test exists.
- **Emit:** `reports/resource-shape.{json,md}`, schema-validated; served by the existing `pnpm reports:serve` (port 4201) for symmetric operator vision. The markdown leads with a node×class matrix (the "shape").

The report is read-only aggregation over existing artifacts — no new telemetry infra, no live-cluster access. It makes the loop visible: a documented limit (B) that gets breached becomes a backlog entry that the report surfaces, and undocumented exhaustion classes show up as gaps to close.

---

## D. Bounded fixes

### D1 — Seeder validator

`genesis/seeder/src/validate-deployments.ts:129-135`: the `consolidated` branch hardcodes `manifest`, but 13 of 14 records use `template` (only adam carries `manifest`; everyone else sed-renders the shared template — the convention `deployments.json`'s own `$comment` documents). Fix: accept either.

```ts
} else if (record.pattern === 'consolidated') {
  const source = record.manifest ?? record.template;   // adam=manifest, everyone else=template
  if (!source) {
    errors.push(`${tag} pattern=consolidated requires 'manifest' or 'template' field`);
  } else if (!existsSync(resolve(REPO_ROOT, source))) {
    errors.push(`${tag} deployment source file missing: ${source}`);
  }
}
```

Add a unit test asserting a `template`-only consolidated record validates clean. Recommend (follow-on, this spec) gating `validate:deployments` in pre-push since it is currently both wrong and ungated.

### D2 — humans.json: ADD caleb/daniel/emma

`genesis/orchestrator/data/deployments.json` carries full records for `human-caleb-spouse`, `human-daniel-brother`, `human-emma-spouse` (the tri-region reciprocal-backup chain: San Antonio ↔ Seattle ↔ Tulsa), but `genesis/data/humans/humans.json` is missing them — a *partial* 2026-05-22 addition (their household siblings susan/eve/nancy made it in). These are live-deployed nodes (OOM events recorded on their StatefulSets) with documented storyline across `collectives.json`, `cluster-topology.md`, and the a2o fixtures. **Verdict: ADD.**

`humans.json` is `DO-NOT-EDIT-BY-HAND` (generated). Author three source files in `genesis/data/humans/` — `caleb-spouse.md`, `daniel-brother.md`, `emma-spouse.md` — with required frontmatter (`id`, `displayName`, `category`, `profileReach`, `bio`, `householdId`) drawn from the deployment-record `$comment` bios, mirroring `susan-household.md`; then `pnpm --filter genesis-seeder run build:data` to regenerate. This closes the validator's "human-gap (3)" so `validate:deployments` goes fully green and the a2o `humans.ts` persona↔topology join stops being one-sided.

---

## E. Resilience card — honest seeder fix

The card (`resilience-snapshot.component.html`) binds directly to `GET /api/v1/resilience/{contentId}/household` → `household_resilience::snapshot()`. Every zero is data, not display. Per-field truth:

| Card field | Cause of zero | Honest remedy |
|---|---|---|
| Commitment-backed | snapshot filters `action ∈ {provide, replicates-content, replicates-commons}` ∧ `state='active'` ∧ `resource_classified_as='content:<reach>'` ∧ provider==`humans.agent_pub_key`; the base seed writes `agent_pub_key=NULL` (deliberate) and old commitments are the wrong shape | **E1**: run `seed-provide-rows.ts` (correct shape, real agent keys via `/auth/me`; already CI-wired) in the alpha seed path |
| Stewarding collectives | requires `shard_locations` rows (written only by runtime `distribute_shards`) joined to agent-keyed households | **E2** (operator-gated) `PUT /admin/seed/shard-manifest` writes agent-keyed `shard_locations` stamped `status="seeded"`; OR genuine runtime replication |
| Diversity score | derived `min(stewarding, max(commitment_backed,1))/7` | lights as the two counts light |
| Geographic distribution | routes through the same `shard_locations`→`humans`→`collectives.region` join | lights with stewards (region data exists for 12/62 collectives) |
| Placement gaps (1) | **honest measured data** — a real distribution attempt fell short | leave as-is; it is true |
| Status "at-risk" | derived from the counts | lights when stewarding ≥ 2–3 ∧ online peers ≥ 1–2 |

**E1 (smallest honest fix).** Ensure the genesis seed pipeline actually reaches the `seed-provide-rows.sh` stage against the alpha target (it is built and wired into `genesis/Jenkinsfile:1496`; the action-filter it depends on is already in the working tree). Retire / redirect the wrong-shape `custody-blob` rows in `seed-commitments.ts` so the only commitment rows the seed writes are snapshot-visible. Caveat to record: `seed-provide-rows.ts` only lights humans whose pod returns a real key from `/auth/me` (401 → skipped), and only `content:commons` matches commons content by default.

**E2 (lights stewarding, auditable).** Add a seed stage calling the honesty-gated `PUT /admin/seed/shard-manifest` (`ALLOW_SEED_SHARD_MANIFEST=1`) with agent-keyed stewards for inline-body content that will never get a real `distribute_shards` run. Rows are stamped `status="seeded"` so the claim "these households hold this content" is auditable, never silent. **Do not bypass the gate in production.**

**Honest boundary (record explicitly).** Genuine multi-peer stewardship and closing the placement gap require live alpha conductor cells running `distribute_shards` against online peers — operator/pipeline domain. This spec makes the *repo seed surface* write honest, snapshot-visible data; it does not and cannot manufacture real replication. The branch already carries the `self_cid` startup-derive (`main.rs:400-462`) that spawns the provide-loop, so a live node with content will begin authoring provide commitments on its own.

---

## F. Push (operator-owned)

The two backlog commits (`5e88ff521`, `14959b973`) are `0 behind / 2 ahead` of `origin/dev`, both single additive markdown files under `genesis/data/timeline/backlog/` — a clean docs-only fast-forward, zero code, zero tangle. Per the standing convention the integrator owns pushes; this work will be committed on the branch and the dev fast-forward performed only on explicit operator OK.

---

## Testing & verification

- **A1/A2:** Rust unit tests for the parsers (default/override/zero/garbage; arc `{0,1}` validity) co-committed; `RUSTFLAGS=''` native build + `cargo test` for the storage crate per the env-split gotcha; `CARGO_TARGET_DIR` set to the pool slot.
- **D1:** seeder unit test (template-only consolidated validates); run `pnpm --filter genesis-seeder validate:deployments` → expect 0 errors after D1+D2.
- **D2:** `build:data` regenerates humans.json with the three new ids; `validate:deployments` green.
- **C:** run `build-resource-shape-report.ts` over the live backlog; schema-validate the output; eyeball the node×class matrix via `reports:serve`.
- **E:** unit-level — assert the seed writes the snapshot-visible shape; the live-card verification is operator/pipeline (render the card on alpha after the seed stage runs) and is flagged as such, not claimed.
- **B:** gherkin parse-validates; scenarios are `@wip` until step defs land (same posture as the doorway resilience feature).

## Honesty ledger (what this does NOT claim)

1. It does not make Matthew's conductor hold less of the DHT (Matthew stays full-arc by design).
2. It does not enable fractional arc (upstream-blocked).
3. It does not manufacture real peer replication for the resilience card — it writes honest, auditable substrate rows and surfaces the genuine-replication gap as operator/pipeline work.
4. It does not run anything against the live cluster — it makes the repo coherent for the next reconcile/seed.
