---
title: k8s bridge — runtime envelope render — implementation plan
id: k8s-bridge-runtime-envelope-render-plan
status: Draft
class: process-meta
context-tier: disclosed
steward: agent:architect@gpt-6-astra
graduation-trigger: Four station commitments discharged with named tests and gate EXIT lines; deployments.json pinned and verified for every active alpha human; the aggregate-capacity ask has not fired on a stale envelope since the observe command landed.
date: 2026-09-05
serves: dev-system-equilibrium
cites:
  - "k8s-bridge-runtime-envelope-render | k8s bridge | sha256:63e7994caba5a53a | path: genesis/docs/superpowers/specs/2026-09-05-k8s-bridge-runtime-envelope-render-design.md"
  - "compute-envelope-tevah | Tevah | sha256:ac9364d4b024290f | path: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md"
---

# k8s bridge — runtime envelope render — plan

Use the valueflow-authoring, valueflow-implementer and valueflow-reviewer skills. One checkbox
per station is one mintable intent. Implementation legwork is delegated to Codex (GPT-6 astra)
in an isolated worktree; review is an independent Opus reader; the top model decides, judges
coherence, and integrates. Commits are path-limited; nothing is pushed by an implementer.
Every station keeps the S0 manifest CID `bafyreihagg75knog3e2fkiygpghcgqge35ovzka3vxken2zjleqnerdcaa`
(the three household-mesh berths) unmoved — a manifest without an envelope must serialize exactly
as before. This plan adds no active habit and does not claim equilibrium.

Operator steering (2026-09-05): "could some of this be a bit easier if we had a bit clearer way of
using ark manifests to set the limits of our compute?" — yes: one declaration (the manifest), one
render (k8s), one observation (the ledger command), one typed ratification. Reduce friction for
models in flight; no push-time flags; the epr-pvc guide-star stays unprecluded.

## Station 1 — The manifest declares the envelope (ark-core, declaration only)

Files: `elohim/ark/core/src/manifest.rs`, `elohim/ark/core/src/rea.rs`, new
`elohim/ark/seam-registry.yaml` and `elohim/ark/.epr-meta`; tests in the same modules.

Add `RuntimeManifest.envelope: Option<RuntimeEnvelope>` and `ChildSpec.quota: Option<ProcessQuota>`
exactly as spec §2.1 (all `#[serde(default, skip_serializing_if = "Option::is_none")]`; integers,
never f64). Extend `validate()` with the Σ refusal (`ManifestError::Invalid` naming both totals;
`bound.memory_bytes: None` never refuses; `headroom_bytes` is declared, never inferred). Add
`RuntimeEnvelope::memory_bound(threshold_pct) -> elohim_epr_rea::model::Bound` with
`source: LimitSource::Folded{rule: Composition::Sum}` — inherit the substrate ontology, mint
nothing. Passport `effective_tier` stays `None` (declared ≠ enforced). Register the two decision
points (the Σ predicate; the envelope→Bound projection) in the new seam registry with concern-canon
answers; route nothing new in the catalog (ark-core is already S3.3). Leave `hc-mesh.sh`'s `jq -n`
manifest untouched (absent envelope = today's bytes).

Proof: (1) a manifest with no envelope keeps the S0 CID; (2) Σ children + headroom > bound →
`Err(Invalid)` naming both totals; (3) Σ = bound − headroom passes; (4) `memory_bytes: None` never
refuses; (5) the projected `Bound` has `unit: "bytes"`, `source: Folded{Sum}`, and its
`breached_by`/`approached_by` agree with the declared bands; (6) `boundary::no_runtime_or_io_deps`
still green.

Gate: `just gate elohim-ark` — retain the actual EXIT line in the report. Claim the cargo berth
around cargo work.

- [ ] Station 1: `RuntimeManifest` carries a declared envelope with the Σ refusal and the epr-rea Bound projection; S0 CID unmoved; seam registry born.

## Station 2 — The bridge renders and verifies (bridges/k8s)

Files: new `bridges/k8s/` (Cargo workspace, crate `k8s-bridge` lib + `k8s-bridge` bin with
`render`, `verify`), `bridges/k8s/seam-registry.yaml`, `bridges/k8s/.epr-meta`,
`bridges/k8s/build-manifest.json` (gate project `k8s-bridge`), `.claude/epr-meta/seam-catalog.yaml`
(S3.6 `registry_crates: [k8s-bridge]`), `genesis/orchestrator/manifests/runtime/*.manifest.json`
(one per archetype in `archetype-resource-budgets.json`, envelope = that archetype's LIMIT budget
with headroom, plus `adam.manifest.json` superseding `family-node-base` for his justified
override), `genesis/orchestrator/data/deployments.json` (`runtimeManifest: {cid, path}` per active
human — edit VALUES only; keep the multiline `"name": "...",` layout `scope-reconcile.py` parses),
`genesis/seeder/src/validate-deployments.ts` (call-through or sibling check), `.husky/pre-push.bash`
(the deployments leg also runs `k8s-bridge verify`).

`render`: manifest → the eight k8s quantities, reproducing `scripts/ci/conductor-split-budget.sh`
exactly (5/8:3/8 memory, 1/2:1/2 cpu, floor, remainder to storage, `Mi`/`m` units); a manifest
without envelope renders nothing (pass-through). `verify`: for every active human, the pin's CID
equals `RuntimeManifest::cid()` of the file bytes, and the four `edgenode*` fields equal the
render; verdict type `DriftVerdict {Fresh | PinMismatch | RenderDrift | Unpinned}`; `Unpinned` is
`refer` until every active human is pinned, then a dated flip to `refuse`. The rendered
Deployment template gains the annotation `elohim.protocol/runtime-manifest: <cid>` via a new sed
placeholder (absent → line deleted, byte-identical otherwise — the `runtimeConfig` precedent).

Proof: golden test — for every archetype budget, `render` equals the bash script's JSON output
(spawn the script in the test; it is bash + awk); `verify` is `Fresh` on the pinned
deployments.json and `RenderDrift` when one field is perturbed; `PinMismatch` when a manifest byte
changes; `seam_matrix_test.py` counts S3.6 cells; `runtime-config-render.test.mjs` still green;
`validate-deployments.ts` tests green.

Gate: `just gate k8s-bridge`, `just gate genesis` (seeder tests), and the pre-push deployments leg.

- [ ] Station 2: every active alpha human's k8s envelope is a verified render of a pinned manifest CID, and the bridge is registered on S3.6.

## Station 3 — Capacity is observed, ratification is typed (the ledger stops asking stale questions)

Files: `bridges/k8s` (`observe` subcommand — Prometheus HTTP API via `PROMETHEUS_URL`, default
the observability datasource; refuses to write if any node's allocatable is missing),
`genesis/data/rakia/compute-capacity.json` + `.schema.json` (`cluster.ratifications[]` — source of
truth: repo governance record, Ephemeral (C) per spec §4, operator-signed via git, reconstructable
from the retired prose and the commit history; graduates to Linked (A2) on the `delegates-compute`
commitment's `bounds` — no SQLite table, no DHT entry in this slice),
`genesis/orchestrator/data/deployments.json` (retire `$computeEnvelopeRatification` prose into the
record), `.claude/scripts/_lib/epr_meta.py` and `elohim/eprfs/epr-cli/src/repository_validators.rs`
(read `ratifications` for the named dimension; freshness leg: `snapshotTimestamp` older than 30 days
→ the aggregate verdict is `refer` naming `k8s-bridge observe`, not `ask`), `genesis/orchestrator/
scripts/snapshot-capacity.sh` (documents that `observe` supersedes its cluster half),
`genesis/data/rakia/.epr-meta` and `genesis/orchestrator/data/.epr-meta` (prose updated).

Proof: `observe` against the live Prometheus reproduces today's promoted values (70000m /
246980 Mi / 7 Ready) and writes provenance; both validators pass the same fixture pair
(ratified limits overcommit passes; unratified fails; requests never ratify; memory never
ratifies; a 31-day-old ledger yields `refer`); the epr-meta git gate over a range that touches
deployments.json emits no `[ask]` when the ledger is fresh.

Gate: `python3 .claude/scripts/_lib/__tests__/*` (all), `just gate eprfs`, `just gate k8s-bridge`.

- [ ] Station 3: the cluster envelope is produced by `k8s-bridge observe`, ratification is a typed record both validators read, and a stale ledger refers instead of asking.

## Station 4 — Tevah register and habit delta (coherence)

Files: `genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md` (§12 register
entry 30 — already appended by the integrator on 2026-09-05: deployments.json is a lockfile-render
of the manifest CID; the guide-star check for register 21 answered — resources only, `data_root`
unminted; this station only confirms it still reads true after Stations 1–3 land), `.epr-meta/dev-system-equilibrium.
habit.md` (one evidence DELTA line: pushes that touch deployments.json no longer need a push-time
flag), `genesis/manifests/habits.yaml` (re-projected), memory note for the three-homes trap.

- [ ] Station 4: the tevah register carries the decision, the guide-star check is recorded, and the habit atom carries the delta with no status flip.
