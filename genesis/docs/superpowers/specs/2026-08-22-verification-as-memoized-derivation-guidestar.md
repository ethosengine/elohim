---
title: "Verification as a memoized derivation — the generational guidestar for developer iterations and CI/CD runs"
id: verification-as-memoized-derivation-guidestar
tier: spec
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: >
  Slice 1's four edits land AND a local-to-fleet env-delta is rendered as a diff of two
  run-identified reports carrying comparable `env` blocks (including `sut`) — proving S5 needs
  nothing built — AND `habits-status.py` renders NOT MEASURED against a real degraded lane.
  Until all three hold this stays Draft, because a guidestar whose first slice has not been
  walked is an argument, not a habit. OR superseded by a fresh reader contesting the §2
  classification or the §3 laws on evidence rather than preference.
created: 2026-08-22
domain: D2
topic: [verification, measure, fold, memoization, observation, attestation, honest-absence, meadows, ashby, beer, rea, valueflow, habits, ci, localdev, graduation, commons-compute]
informed-by:
  - genesis/docs/superpowers/specs/2026-08-12-requisite-variety-guidestar-epr-family-composition.md
  - genesis/docs/superpowers/specs/2026-08-13-dev-system-equilibrium-stocks-design.md
cites:
  - "requisite-variety-guidestar-epr-family-composition | The composition law this guidestar is governed by — its §3a admission rule (a second independent asker, never the first) and §6 what-not-to-build are the authority every refusal in §5 cites | sha256:e1cf9e52fbe95c11 | path: genesis/docs/superpowers/specs/2026-08-12-requisite-variety-guidestar-epr-family-composition.md"
  - "dev-system-equilibrium-stocks | The Meadows equilibrium leg whose commitments stock this guidestar keeps but corrects — §S6 shows the DRAINING verdict is window-selected (7d drains, 90d fills 17:1) and moves the predicate to turnover under a declared horizon | sha256:5306c437d02200f2 | path: genesis/docs/superpowers/specs/2026-08-13-dev-system-equilibrium-stocks-design.md"
  - "latency-valueflow-chain | The second independent framework demanding one-measure-many-environments (its §11 two-tier validation) — which is what admits Law II under the composition law's §3a rather than as speculation | sha256:9b3106cb3707e838 | path: genesis/docs/superpowers/specs/2026-08-20-latency-valueflow-chain-design.md"
  - "middot-measure-primitive-design | The measure/fold primitive this guidestar extends to a second family — its derivation triple (subject × measure@version × env) is Law II's key, and its read-free/trust-choose/verify-recompute economics are Law III | sha256:336ab2b4619b9144 | path: genesis/docs/superpowers/specs/2026-08-04-middot-measure-primitive-design.md"
  - "measure-dynamics-confidence-ontology-design | The six enforced measurement laws any run-cost number must import rather than re-derive — why §S7 types compute as a Quantity and refuses a bare seconds scalar on the fold body | sha256:52d601baa6117450 | path: genesis/docs/superpowers/specs/2026-08-11-measure-dynamics-confidence-ontology-design.md"
  - "epr-rea-valueflow-fabric | The fabric whose fulfill path this guidestar corrects — §S3 supersedes fulfill.rs dropping every scenario name, status and observed value, and §S6 withdraws the new flow event an earlier draft proposed adding here | sha256:1cec32527dbff6d7 | path: genesis/docs/superpowers/specs/2026-07-18-epr-rea-valueflow-fabric-design.md"
  - "observation-event-layer-design | The Observation ≠ Event ≠ Attestation law this guidestar obeys — and whose producer §6 verifies is absent at six independent points, so any slice depending on observation rows is a build, not wiring | sha256:2b57787e60a0ddc6 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md"
  - "compute-commitment-substrate-floor-design | The floor/ceiling split Law III rests on — the deterministic floor completes alone with no AI and no network, while the elohim ceiling authors and retires the declarations it executes and never gates one | sha256:614e30134ee0d7ab | path: genesis/docs/content/elohim-protocol/architecture/2026-05-04-compute-commitment-substrate-floor-design.md"
  - "ci-orchestrator-recurring-anti-patterns-museum | The frequency-ranked evidence for Law I — NOT_BUILT/ABORTED reading as zero failures is the same honest-absence failure as 76-of-80 skips, and the museum ranks it first | sha256:0e325f2f174689ae | path: genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md"
  - "deploy-is-not-a-graph-node | The unfixed structural lesson §2 inherits — deploy is still not a manifest node, which is why this guidestar refuses a Jenkins baseline state machine rather than formalizing the stored level around it | sha256:f84c6484627baedf | path: genesis/docs/content/elohim-protocol/history/2026-06-02-deploy-is-not-a-graph-node.md"
  - genesis/manifests/habits.yaml
  - genesis/a2o/schemas/sprint-report.schema.json
  - genesis/a2o/scripts/build-sprint-report.ts
  - genesis/a2o/scripts/substrate-verify.ts
  - genesis/scripts/jenkins-sync.sh
  - genesis/orchestrator/gate-runner.mjs
  - elohim/epr/src/verdict.rs
  - elohim/epr/src/measure.rs
  - elohim/epr-rea/src/stock.rs
  - elohim/epr-rea/src/fold.rs
  - elohim/epr-rea/src/epistemic.rs
  - elohim/eprfs/epr-cli/src/flow/fulfill.rs
  - elohim/eprfs/epr-cli/src/flow/stocks.rs
  - elohim/elohim-storage/src/graduation/diversity.rs
  - elohim/brit/brit-epr/src/elohim/attestation/validation.rs
  - elohim/rakia/rakia-brit/src/baselines.rs
  - elohim/sdk/README.md
  - .claude/scripts/measure-fold.py
  - .claude/scripts/habits-status.py
  - .claude/epr-meta/measures.yaml
  - .claude/epr-meta/recipes.yaml
---

# The Generational Guidestar — Verification as a Memoized Derivation

*Governs how developer iterations and CI/CD runs are modelled, and how the Jenkins layered report stock and the epr-meta valueflow become one thing.*

*Status: guidestar, second draft — adversarially reviewed. It states what is true about the design and what follows. §6 sequences the work, §8 names what is open, §9 records what the review changed.*

---

## Do this first

**Make the local household lane leave a durable trace: `just test mesh` calls `build-sprint-report.ts` into a run-identified path, and the report carries three new fields — `declared`, `env`, `gitCommit`.**

That is the single highest-leverage change. Everything else in this document composes on top of it, and nothing else in this document is possible without it. It is roughly four edits over parts that already exist (`justfile:70-85`, the sprint-report schema, `habits-status.py`, `recipes.yaml`). Today 24 honest local runs a day land as raw cucumber JSON in `/tmp/elohim-local-mesh/reports/`, get wiped on container restart, and count for exactly nothing — while the lane that produces them is the *only* lane where destructive chapters can run at all (`processControl` is false on the fleet). The lane with the highest discovery value produces the least durable evidence. That inversion is the central defect of the current loop.

Second, in the same evening, for about an hour: **render NOT MEASURED in the habits headline.** `habits-status.py` already prints the session line; have it also print, per concern, `passed:0 failed:0 pending:N` from the newest report's `byConcern`. Tonight that fires on three habits — `dataplane-convergence`, `notary-authority`, `operator-runtime-surface` — which currently read green while their named evidence lane (edge #1376: 80 scenarios, 3 passed, 1 failed, **76 skipped**) measured nothing. It renders the disagreement; it does not edit the register.

Everything below is why those two changes are the right ones, and what must *not* be built alongside them.

---

## 1. The experience we are building toward

**§1a — what this design produces, all legs buildable from the repo as it stands today.**

It is 8:40pm. Matthew has a day job; this is the third hour of his evening he can spend here. His elohim partner opens the session and prints one block — no scrolling, no dashboard, no link to click.

```
HABITS  8 green · 4 red · 0 unwired · active: doorway-failover, operator-runtime-surface
  NOT MEASURED  3 concerns (edge #1376: 76 of 80 scenarios skipped)  ← read this first
  dataplane-convergence   status: green   observed_status: not-measured   last_measured: 6d ago
  notary-authority        status: green   observed_status: not-measured   last_measured: 6d ago
  operator-runtime-surface status: green  observed_status: not-measured   last_measured: 6d ago
  → the register and the evidence disagree. Rule 4 says a status flip is yours to make.
```

He touches `elohim/elohim-storage/src/p2p/behaviour.rs` and types `just test mesh`. Before a peer boots, the terminal prints the derivation — not a progress bar:

```
subject  a2o/dataplane/resiliency-saga @ tree:4f2a1c9   (11 chapters)
measure  a2o-dataplane@1  (record-only: folds are written, never read as a skip)
env      household · 3 peers · sut:b91f… (storage+doorway+dna+fixtures) · processControl=true
   would-be HIT   7 chapters   (recorded; accuracy of this line is itself being measured)
   would-be MISS  4 chapters
declared 11 · exercised 0 · permitted 0 · refused 0 · referred 0 · not measured 11
```

Eleven declared, and the denominator did not come from the runner. It came from `habits.yaml` `checks:` and the `@concern:` registry. If he had scoped the run by tag, the unexercised chapters would still print as NOT MEASURED rather than vanishing — which is the exact failure mode that let 76/80 hide for a wave.

At 11:04pm a chapter reds. Not a wall of a hundred console artifacts — one line:

```
fp 7bbfcf89 · seen 59× · first 1500 · last 1647 · class: substrate · status blocked
→ this is the substrate, not your diff.
```

That distinction — *your red* versus *the network's red* — is the most dignity-preserving thing this loop can offer one person at 11pm, and it is computed from a **local** findings ledger built from his own folds, so it fires offline, on a plane, with Jenkins unreachable. He goes to bed. His elohim asks one question before closing: *"you abandoned the Automerge-first ordering around 10:20 — record it as a failed-approach note?"* One keystroke. In three weeks a fresh session inherits the consequence without re-walking the dead end.

The next morning the fleet lane runs, and the two lanes are comparable for the first time because both reports now carry an `env` block:

```
DIVERGENCE   a2o-dataplane@1 · ch04 blob-durability
  household/3-peer   permitted 8 · refused 0 · referred 0 · not measured 0
  fleet/alpha-7      permitted 0 · refused 0 · referred 0 · not measured 8
  env delta          fixtures: household-act1  ⟂  (absent)
  → refer(community, ContestedEvidence, note: "env-divergence: fixtures")
     receiver: the operator's referred queue · drain: becomes a red or a new declared env axis
```

That is a `diff` of two files. It requires no comparison entity, no reconcile stage, no new enum. And it is the discovery that took five stacked invisible defects and a human reading two logs the last time it happened.

**Everything above is buildable from what exists.** There is exactly one participant in it. That is honest, and it is worth building on its own terms: durable self-memory, an externally-anchored denominator, an offline substrate-vs-your-diff line, and a lane comparison between two environments one operator already owns.

**§1b — the horizon, with its unbuilt legs marked.**

The scene the first draft opened with — *"6 of your folds were recomputed by 3 households overnight; 1 DIVERGED"* — is the generational target and it is **not reachable by anything in §6.** Naming what it needs, precisely:

- **A fold exchange leg (UNBUILT, not in any list).** `measure-fold.py` has no fetch, import, or verify-foreign-fold path; it reads and writes one local directory. Git-tracked files are distributed by clone/push of this monorepo, so a household that is not the operator has no write surface and no read surface that does not require repo access. Nothing in the repo carries a fold between peers.
- **A verification rule for a stranger's fold (UNBUILT).** A foreign fold is an imported claim; it becomes evidence only by **recompute-and-match**, never by being read. An unsigned foreign fold counts as zero until `Observation::canonical_signing_bytes` is wired and signed — otherwise the diversity number is trivially self-farmable and strictly worse than none.
- **A motive for witnessing (MISNAMED in the first draft).** "It costs that household nothing — they were running their gate anyway" is false as written. A matching fold needs the *identical* derivation triple; two households working on their own edits produce folds at different subject CIDs and never collide. **The one surface where triples collide for free is a shared commit** — many participants running the gate on `dev` HEAD. That is the witness surface, and it is the only form of witnessing that is genuinely a byproduct rather than donated recompute. Say it that way or the commons has no mechanism.
- **A second household (ABSENT).** With one operator, three co-owned mesh peers and seven co-owned fleet peers, nothing can honestly reach `witnessed`. The terminal must print `claimed · 0 independent witnesses`, never a green tick. That line is the design's honesty applied to its own instruments, and it ships in §1a.

The horizon is not decoration and it is not dropped. It is the thing the floor is shaped to receive. But a reader must be able to tell which sentences are built, which are buildable, and which need a person who has not arrived yet — which is the same discipline this document demands of every metric it touches.

---

## 2. The classification — S1..S7 settled

### S1 — A localdev household-mesh run
**Vocabulary: a *household witness*. A Process at env `household`, whose durable residue is a run-identified report and (later) a fold. EPR Category C.**

It is not "a test" and not a rehearsal. It is the application of a measure to a subject under an environment — the derivation triple `(subject-CID × measure@version × env-CID)`. Its diversity coordinates are honestly *1 household · 1 host · 3 agents*; the three mesh peers are one family seeded into one collective, and 1 is the correct number.

It is also the **only** lane where destructive chapters are constructible. For kill-a-peer concerns the household mesh holds the only evidence that exists, and the fleet should be reading it.

> **Superseded:** the shipped treatment of the local lane as non-evidence. `just test mesh` sets `CUCUMBER_JSON_REPORT` and execs cucumber (`justfile:70-85`); `build-sprint-report.ts` is invoked only from CI scripts. That is the defect named at the top of this document.

### S2 — A Jenkins alpha-fleet run
**Vocabulary: a *fleet witness*. Same class as S1, differing in exactly one component of the key: env.**

Jenkins' authority is architectural inheritance, not a protocol fact. Its real contribution is **environment diversity** — 7 peers, WAN NAT, real DNS, real PVC pressure, real quiesce. It contributes no *observer* diversity: one operator owns and funds all seven peers. Retire its **authority** (baseline, archive, verdict); keep its **witness**. A datacenter-class observer is a healthy permanent end-state.

> **Superseded:** the per-pipeline Jenkins baseline. It is a stored level, which `epr-rea`'s founding law forbids ("a stored level is a second home for a number that already has one"). Its two documented pathologies — baseline-rollback over-build, and `NOT_BUILT`/`ABORTED` reading as zero failures — are the two highest-frequency traps in the museum. **Do not build the explicit baseline state machine the orchestrator README proposes.** Delete the concept; derive the level from the log; carry promotion in a git ref.

### S3 — A single assertion's outcome inside a run
**Vocabulary: a *check-witness*. `CheckWitness{check_id, outcome, summary, observed}`, EPR Category C — never persisted, reconstructed by re-evaluation.**

`CheckOutcome` stays at **three** values: `Passed | Failed | Skipped` (verified, `elohim/epr/src/verdict.rs`). It projects to the verdict spine:

| outcome | renders as |
|---|---|
| `Passed` | `Permit` |
| `Failed` | `Refuse` |
| `Skipped` | **NOT MEASURED**, carrying its skip reason |
| *(declared subject with no witness at all)* | **NOT MEASURED** |

**No fourth enum value, and — reversing the first draft — no `Skipped → Refer` mapping either.** `verdict.rs:60-70` states the ceiling law plainly: *"`Refer` is FIRST-CLASS… It is never a fallthrough, never a timeout, never an error."* A cucumber scenario skipped because a fixture manifest went missing is a broken apparatus, not a governance question, and it has no competent layer to route to. Under the 76/80 example, mapping skips to `Refer` would manufacture ~76 governance escalations from one degraded run and make "referred" the noisiest number on the headline. The distinction the design actually wants is *declared-but-unwitnessed*, and NOT MEASURED already carries it.

NOT MEASURED remains **derived, never emitted** — because the entity that would write it is the entity that failed. An apparatus that crashed emits nothing; a PVC-deferred gate emits nothing; a harness that never started emits nothing. Absence stays absence, and it is countable because the denominator was declared *outside the run* (Law I).

A run's headline is five numbers forever: **declared · permitted · refused · referred · not measured.**

> **Superseded:** `genesis/a2o/scripts/lib/aggregate.ts:271-277` collapsing `skipped`/`undefined` → `pending`; `fulfill.rs:395` incrementing `skipped_pending` and no-opping; `fulfill.rs:100-102` deserializing `ConcernScenario{surface}` — one field — dropping every scenario's name, status and `observed` value; and `publish-results.ts:101-109`, a fourth incompatible mapping in which an all-skipped scenario reads as `fail`.

### S4 — The run's report bytes
**Vocabulary: the *evidence body* of a fold. Run-identified, content-hashed, EPR Category C.**

Not an economic resource; not a stock. A single mutable path has a level of exactly 1 and an outflow identically equal to its inflow, because writing it destroys its predecessor.

> **Superseded:** `sprint-report-dataplane.json` as a single mutable slot that `recipes.yaml:87-89` names as the `a2o:verdict` stage source — a recipe terminating in a slot with no history. Observed in practice: the `.json` and `.md` halves of that slot carrying two different runs from two different lanes. Also superseded: `fingerprint: true` on `substrate-verify-*.json` (inert — the reports embed `generatedAt`, so no two builds collide and the fingerprint DB can never answer a cross-build question), and archive-in-a-post-block as durability.
>
> **Correction to the first draft:** the cure is **run-identified filenames under a stable glob**, not content-addressed filenames. `recipes.yaml` stages declare `paths:` as globs; a content-addressed store has derivation-key filenames that no glob can name, which would break the stage contract while trying to fix it. `reports/sprint-report-dataplane-<runId>.json` under `reports/sprint-report-dataplane-*.json` restores history and leaves the recipe intact.

### S5 — The localdev↔fleet comparison
**Vocabulary: an *env-delta*. Not an entity — a free query over two reports sharing `(subject × measure)` and differing in `env`.**

Once both reports carry an `env` block, this is a `diff`. Nothing is built. A match is corroboration; a **mismatch is the more valuable outcome** — the discovery that the measure is env-sensitive on an axis nobody declared.

It is represented as `Decision::Refer{layer, reason: ContestedEvidence, note: "env-divergence: <axes>"}`. **Not a new `ReferReason` variant.** `ReferReason` is closed at three, ts-rs exported, and pinned by `schema_contract.rs` — minting a fourth there has a larger blast radius than the fourth `CheckOutcome` value this document spends a paragraph refusing, and it has exactly one asker. `ContestedEvidence` means "the evidence itself is disputed — an active contest, not a settled fact," which is precisely two observers disagreeing, and `note: Option<String>` exists for exactly this.

**And a `Refer` must name a receiver and a drain.** An escalation channel with no endpoint is the instrument-with-no-reader shape in its most consequential form. The receiver is the operator's referred queue; the drain is that a referred item must resolve into **either** a red on a named habit **or** a new declared env axis (which changes the key and re-opens the question honestly). The referred count is in the five-number headline so a growing queue is visible, not silent.

> **Superseded before it is built:** the orchestrator README's "Predictive Build-Graph Vision" step 2 (predicted-build-graph.json + actual-build-graph.json + a Reconcile stage). Widen the env declaration instead.

### S6 — What accumulates
**Vocabulary: the *fold* is a third thing — a memoized application, neither event nor stock. Its instrument is a ratio of rates (hit/miss), never a level.**

**No new Meadows stock is admitted, and — correcting the first draft — no new flow event either.**

A Meadows stock is **conserved**: what flows out of one flows into another, and something drains it. **A fold is non-rival.** Reading it depletes nothing. *"Reading a fold is free"* read as physics rather than slogan **is** the statement that a fold cannot be a stock. Env-invalidation is not an outflow: a fold keyed on env E remains permanently true about env E; a toolchain bump does not destroy it, it changes the question. A thing with an inflow and no possible outflow is a monotone accumulator, which is a log.

The first draft then proposed admitting "the skip as a positive flow event" so a level would "fold out of the existing `stock_over_window` with zero vocabulary change." **That mechanism does not exist and the claim is withdrawn.** Verified: `StockName` is closed to a single variant `Commitments` (`stocks.rs:683-685`), fed by `classify_record`, `derive_commitment_view` and `commitment_stock_resource()` — all commitment-specific. A skipped-run level needs a new `StockName` variant, which is exactly what `stocks.rs:1144` asserts is a spec change. Refusing four stocks and then smuggling a fifth in under a different noun is the failure this document exists to prevent.

**NOT MEASURED needs no new event at all.** It is a derived count: declared-subject-set minus witnessed-subject-set. Zero new verbs, zero new stocks, zero new units.

The `commitments` stock is kept, with two corrections. Its DRAINING headline is **window-selected**: 7d reads DRAINING and "equilibrium: yes", 14d reads FILLING, 90d reads FILLING at 17:1, and the only automated consumer hardcodes `WINDOW_DAYS = 7`. Lifetime: 583 minted against 22 discharged. So **(i)** the window is declared in `habits.yaml` beside the check, where changing it is a visible covenant edit rather than a CLI argument; **(ii)** the predicate moves from outflow≥inflow to **turnover under a declared horizon** — the measure `stocks.rs` itself names as "the measure that separates dynamic equilibrium from silting" and then excludes on a typing technicality (see Q5).

### S7 — The compute and attention a run spends
**Vocabulary: compute is a *typed cost* recorded on the fold. Attention is a *bound* on the pair's own commitment. Neither is a stock; neither settles.**

Compute is rival, fungible and denominable, so recording it is legitimate — but **not as a bare scalar.** A raw seconds number on the fold body is a Level with no declared window, no `MeasureKind` and no `Confidence`: the precise shape law 27 calls a three-times-diagnosed failure mode, and it would set the precedent as the first number entering that schema (the two folds on disk carry `computedAt`/`computedBy`/`evidence`/`env`/`findings` and no measure-typed field at all). If cost goes on the fold it goes as a `Quantity{value, kind, confidence}` imported from `elohim/epr::measure`, or it does not go.

It is also **not in slice 1.** Its reader — cost-per-unit-of-evidence — matters, but nothing consumes it until folds are being read, and Q4 has not settled what it denominates.

Attention is **advisory forever**: pricing focus-hours against compute-seconds mints the exchange rate that converts a commons into a labour market. It gets `Bound{limit, unit: "focus-hour", sense: Ceiling}` and the algedonic signal `fold::bound_evidence` already produces.

> **Superseded:** `bound: None` on every commitment minted on this path. The pair's own exhaustion is currently structurally unable to hurt.

### Ruling on the five unreconciled lifecycle vocabularies

One deletion; no additions.

- **`ReachLevel{Unknown, Built, Deployed, Verified}` — DELETED.** `compute_reach` derives from mere non-emptiness of three vectors: a step with a FAILED build attestation and an UNREACHABLE deploy attestation computes `Verified`. It is a fourth evidence vocabulary colliding by name with the 8-level protocol reach enum, which it is not.
- `plan verdict` and `affected` — **kept**, orthogonal: one is intent, one is a reason for inclusion.
- `cite verdict` — **kept**, a different domain (link liveness).
- `saga chapter` and `habit status` — **kept**. `unwired` is load-bearing and has no analog in testing.

**Three axes replace it, and they compose but never merge:**

| axis | question | carrier |
|---|---|---|
| **audience** | who may see this | `reach` — 8-level, inside the CID, untouched |
| **evidence strength** | how independently confirmed | the middot ladder: `claimed → witnessed → confirmed` |
| **outcome** | how did the check come out | `Decision`: permit / refuse / refer, plus derived not-measured |

---

## 3. The three laws

### Law I — The denominator is declared outside the run; absence is derived, has a clock, and never mutates the covenant

*A run's denominator is the concern set declared in `habits.yaml` `checks:` and the `@concern:` registry — never a set the runner computes for itself. A run reports which declared subjects it exercised; every unexercised declared subject renders NOT MEASURED regardless of the runner's tag scope or path filter. Every habit declares a `max_evidence_age`; a green whose newest witness is older than it renders NOT MEASURED too. No conversion may collapse `Refer` into `Refuse` or `Permit`; nothing may collapse NOT MEASURED into anything; no ratio may be computed over a denominator that includes it. And no machine writes `status:` — a measured absence writes `observed_status:` and `last_measured:` beside the human-owned field, and the tool renders the disagreement.*

**Why the denominator moved out of the run (this is a correction, not a flourish).** The first draft said "a run declares its subject set before it starts," which places the denominator inside the entity that failed. `just test mesh <scope>` writes a generated cucumber config that strips `paths` so a tag expression can scope the run; a scoped run's self-declared set is exactly what it intended to run, so it reports 100% permitted and 0 NOT MEASURED while every other concern is silently unexercised. The 76/80 case only surfaced because those scenarios ran-and-skipped and appeared as `pending` — a re-scope by tag would have made them vanish from the report entirely and the honest headline would read clean. An externally-anchored denominator is the whole point.

**Why time is a term.** The dominant solo-operator failure mode is not a bad run; it is two weeks of not touching it. Demotion on *measured* absence fires only when a report exists. A run that never happens produces no report, so nothing demotes, and the register is most confident exactly when it is least informed. `habits.yaml` carries no per-habit observed-at and `habits-status.py` computes no age. The shipped prior art already knows this: `DeployAttestationContentNode` carries `liveness_ttl_sec` ("self-invalidating") and `ValidationAttestationContentNode` carries `ttl_sec`. Inherit that discipline.

**Why nothing mutates `status:`.** Covenant rule 4: *"Status flips require evidence… never edit status from memory or intention."* A script writing status is a machine editing a covenant. Worse, `unwired` is explicitly *"no runnable check yet → NOT schedulable; its ONLY legal first move is writing the red"* — so auto-demoting a degraded-lane habit to `unwired` would assert something false about the check's existence, make three habits structurally unschedulable at the moment they most need scheduling, and prescribe writing a red that already exists. Auto-demoting to `red` is no better: `red` means schedulable work, and a machine asserting schedulable work on a covenant is the same overreach with a different label. Rendering the disagreement gives the operator everything and takes nothing, and it preserves the best-observed ratchet that a status mutation would flatten.

**Evidence.** The apparatus fails the same way at every altitude. `NOT_BUILT`/`ABORTED` counted as 0 failures. A support-load crash shipping SUCCESS with zero scenarios. PVC-deferred heavy gates reading as passed ("dev green = deferred, not passed"). A 2-byte `mesh.json` indistinguishable from a run that never happened. And the live one: `habits-status.py` reads 8 green · 4 red · 0 unwired, while edge #1376 reads 80 scenarios / 3 passed / 1 failed / 76 skipped, with `notary-authority {passed:0, pending:4}`, `operator-runtime-surface {passed:0, pending:3}`, `content-sync {pending:4}`, `blob-replication {pending:2}`, `peer-mesh {pending:3}`, `epr-projection-fallback {pending:2}`. **Three habits read green while their named evidence lane measured nothing.** The flow ledger is honest about this (`fulfill.rs:250` requires `pending == 0`, so nothing discharged). The register is not — because "flips require evidence" has a promotion path and no rendering of demotion.

**A register that can only ratchet upward is a register nobody can trust.**

### Law II — One measure, many environments; the environment is in the key, and the key includes what is under test

*Localdev and CI are not two kinds of thing, two tiers, or two authorities. They are one Process applying one measure to one subject, differing in exactly one component of the derivation triple. The environment is declared and hashed — and for any measure whose subject is not itself the code, the env MUST carry a content hash of the system under test. A divergence at a declared env is a discovery, never a failure.*

**Why the SUT clause is mandatory, and why it is the correction that most changes the plan.** Clippy works as the fold MVP because *its subject IS the code*: `measures.yaml` keys it on the crate tree hash with env-sensitivity `[rustc-version, clippy-version, cargo-lock, clippy-toml, rustflags]`, so an edit always moves the key. Lifting that to a2o inverts the relationship: the subject becomes a scenario (a feature file) and the code under test lives somewhere else entirely. Every env candidate previously offered — fixture manifest, DNA hash, conductor image tag, peer count, per-peer archetype, doorway build — is a *deployed-artifact identity*. None moves when the operator edits an uncommitted source file, and CLAUDE.md's own rule says coordinator-only changes never move the DNA hash at all. Under that key, an edit to `elohim/elohim-storage/src/p2p/behaviour.rs` moves neither subject nor env for any chapter: the terminal would print eleven hits, spend nothing, and report permitted — **for a binary nobody ran.** That is the C4 laundering failure this document's first law exists to abolish, reproduced by the document's own flagship demo. `measure-fold.py`'s docstring already confesses the class at the crate tier ("a path-dep edit that does not move the lock will not move the key… stale cache hit"); the a2o lift makes the corner case the common case.

So: the a2o/dataplane env carries `sut` — a content hash over the built storage/doorway/conductor binaries, the DNA hash, and the fixture manifest. And because the *other* half of that question (what moves a chapter's subject) has no answer in the repo (`graph-walker.mjs` resolves changed files to gate *projects* via manifest globs, not scenarios; `scan-coverage.ts` does no impact analysis), the fold ships **record-only** until both questions are measured (§6, Q1a/Q1/Q2).

**Evidence.** The 76/80 regression is the cost, paid in full: a wave re-grounded scenarios to the household mesh (locally 21/22 green — exactly right), the fleet lane silently lost its fixture context, and four habits kept reading green while their named evidence measured nothing. **Two programs drifted. One program with two envs cannot.** The museum says the same from the other side — *host-green ≠ CI-green; the gap is the environment* — and the latency spec independently reached the identical rule (§11), which under §3a is a second independent framework demanding the same distinction. Nothing new is admitted; the distinction is already in the key.

Two consequences a naive reading gets backwards. **The trust direction reverses for destructive chapters** — the household mesh holds the only evidence that exists for kill-a-peer. And **a fleet run's env cannot be fully hashed**: cluster state, PVC pressure and live DNS are operator-owned and not derivable from the repo. Its unhashable components are declared `unknown` in the key rather than omitted — the same discipline as `Interval::unknown()` over a nullable, applied to a derivation key.

### Law III — Reading is free, trusting is a choice, verifying costs a recompute; the floor completes alone, and dependence lives only at covenant altitude

*The network's contribution arrives as a free read and is never a precondition for the operator's loop. The deterministic floor — run the checks, hash the env, write the fold, count the observers, render what is missing — completes with no AI, no peer online, and no network, degraded to `claimed` and honest about it. The elohim ceiling authors and retires the standing agreements the floor executes; it never gates one. A `Refer` routes; it never blocks. But non-blocking is not the same as non-load-bearing: a **reliability claim at covenant altitude** — a habit's flip to green — MAY require independent witness where independent witness is possible, authored as a covenant edit in `habits.yaml` and read from the parent EPR. That requirement never gates a push, a merge, a deploy, or the evening.*

**Why the second half was added.** The first draft's "evidence tier gates nothing, ever" made `claimed → witnessed → confirmed` change no outcome anywhere in the system — which is exactly *a number with no reader*, the shape this corpus has refused at least six times, minted in the act of refusing it for everyone else. And it went further: by removing every path through which another participant could ever come to matter, it engineered interdependence out of a design whose telos is a commons. The correction is a distinction, not a retreat: **refusing to gate the operator's loop is correct and non-negotiable; refusing to gate a covenant promotion is a different refusal, and it is wrong.** `habits.yaml`'s own doctrine is that "green suites systematically over-claim"; the cure for that at register altitude *is* diversity-gated promotion, which is a governance act read from the parent EPR (decision 1), not a block on shipping. Where independent witness is impossible today — which is everywhere today — the rule evaluates to "not possible," the habit renders `claimed · 0 independent witnesses`, and the operator flips it under rule 4 as they always have. The ladder now has exactly one reader, at exactly one altitude, and it costs the evening nothing.

**Evidence.** `measure-fold.py`'s own docstring already states the free-read law. The cost asymmetry that makes it matter is quantified twice independently: **~100× iteration cost** (local plan ~5s/$0 vs CI 5–75min) and **~1000× cost-per-evidence** (six deploy-coupled banking runs ≈ 8 pipeline-hours produced 2 recorded measures — ~4h per unit of evidence, against 13.4s then 0.3s for the same scenarios locally). And `verdict.rs:60-70` already defines `Refer` as ceiling-protected and structurally uncollapsible.

The operator constraint makes the first half non-negotiable rather than aesthetic: **one developer, a full-time day job, funding infrastructure out of pocket.** A design in which an unavailable participant stalls the floor means he cannot ship at 11pm on a Tuesday.

### Law IV (addendum) — a catastrophic signal never rides the rollup

*A measure may mark a chapter **algedonic**. An algedonic red prints on its own line above the headline and is never inside a denominator.*

Beer's requirement is a channel a catastrophic signal takes *without* passing through the variety-attenuating aggregation. A five-number headline over a declared set is an aggregation, and NOT MEASURED is itself a count. A class that must never be averaged — a fresh agent key minted by an unintended reinstall, a DHT partition, blob-durability loss — would otherwise render as one `refused` among N and inherit the rollup's latency. §7 diagnoses the analogous defect one layer down (`should_emit` with no `should_clear`); the run plane must not repeat it.

---

## 4. What this unifies

**In one sentence: the Jenkins "layered stock" is neither layered nor a stock. It is one measure family's evidence body plus a pile of environment facts that were thrown away. Give the report a run identity and an `env` block, and every layer lands in exactly one of three places — an env-key component, the evidence body, or a non-identifying observer annotation.**

| Jenkins layer today | lands as |
|---|---|
| build result `SUCCESS`/`UNSTABLE`/`FAILURE` | **deleted.** A 3-valued lossy scalar with no denominator. Replaced by the five-number headline over the externally-declared subject set. |
| `seed-results-*.json` (fingerprinted) | **env-key component** — the seed/fixture manifest CID. This is precisely the axis that silently diverged in the 76/80 regression. |
| `substrate-verify-<cmd>.json` ×7 (`runner:"facade"`) | each assertion → a **CheckWitness**; the frozen `A = {meshAdjacency, uploadPreflight, …}` table → the **check_id vocabulary** (already deliberately stable, diffed for one-character drift); `runner` → an **observer annotation**, never identity; the file → **evidence body**. |
| per-peer / per-doorway deploy **junit XML** | **deleted as a transport.** Deploys are not test results. The peer roster and per-peer `deviceArchetype` — computed today and buried inside a failure *message string* — become **env-key components**. This is where the peer-diversity dimension is currently annihilated between two ends that both already model it. |
| `sprint-report-dataplane.json` (single mutable slot) | **run-identified evidence body** under a stable glob, so `recipes.yaml`'s `a2o:verdict` stage keeps its path contract and gains history. |
| ~100 per-scenario console/pageError artifacts | **findings inside the report**, deduped by the sha256/12-hex fingerprint that is *already* a content address — and classified `substrate` vs `code` at write time, so the 11pm situating line is computable with no network. |
| `ci-findings.jsonl` (`seen`/`first_build`/`last_build`/`status`/`backlog`) | **kept, and mirrored locally.** Today it is written only by `.claude/scripts/ci-harvest.py` polling Jenkins, so the most dignity-preserving line in the loop does not exist offline. A local ledger of the same shape, fed from local runs, fixes that without touching a single enum. |
| the Jenkins fingerprint database | **deleted** — inert by construction (reports embed `generatedAt`; no two builds collide). |
| per-pipeline baseline + `build-state.json` | **deleted.** Staleness keys on attestations: `stale = (latest_build is None) OR (latest_build.inputsHash != current_inputs_hash)`. Leapfrog becomes structurally impossible — the ref cannot advance past an unbuilt commit because no attestation exists for it. |
| `publish:results` npm script | **the dead script line is struck now** (zero callers outside `package.json:43` and one `console.log`). `publish-results.ts` itself is **kept in tree as the shape to revive**: it is the only written path that puts run results on the content graph, and that leg returns with the exchange leg (§6). Do not delete the file. |
| `gate-runner.mjs` stdout JSON line | **writes a run-identified record** — the local gate's first durable trace ever. |
| `/tmp/elohim-local-mesh/reports/` | **calls the aggregator that already exists** and that this lane simply never calls. |

**On the valueflow side**, the join is the same object. `recipes.yaml`'s `elohim-dev-pipeline@1` runs manifesto → epic → architecture-seed → spec → plan → intent → scenario → validation, and **the build/CI lane is entirely absent from it** — the doc lane and the build lane meet at exactly one point today. `a2o:verdict` sources the run-identified path; `gate:verdict` joins as a sibling stage. Same producer, same shape, same key; they differ only in measure family — the shipped corollary of decision 14, *a build step and an a2o scenario share the same plan/dispatch/attest machinery and differ only in `executor.kind`.*

`epr flow project` mints commitments **carrying env**, so a 3-peer household green can no longer discharge the identical promise only a 7-peer fleet run could keep. `epr flow fulfill` **gains a `Process`** (it writes `process: None` at all four construction sites today, which is why a local run and a fleet run would be indistinguishable in the ledger even if local ever called it), and **keeps its regression state machine verbatim** — the time-order-vs-append-order handling, the dedupe by atom CID, the stale-recovery gating, and the refusal to let a red drain the stock are correct and hard-won.

`genesis/scripts/jenkins-sync.sh` is already the bridge and its header is already the design decision: *the only step that knows Jenkins exists is the artifact fetch; a fetched report is an imported claim and never becomes stronger by being folded.* Extend it past the one dataplane concern it covers. It deletes itself at the rakia graduation.

---

## 5. What is refused, and why

- **No new Meadows stock, and no new flow event either.** Non-rivalry, plus `stocks.rs:1144`, plus the verified fact that `StockName` is closed to one variant and the "folds out of the existing stock" mechanism does not exist. NOT MEASURED is a derived count over an externally-declared denominator: zero new verbs, zero new units.
- **No new diversity counters on `EpistemicStanding`.** `elohim/elohim-storage/src/graduation/diversity.rs` already ships `DiversityThreshold{distinct_households, distinct_collectives, distinct_regions, distinct_archetypes, min_count}` and `threshold_met(conn, subject_cid, observation_kind, &threshold)`, consumed by `graduation/attestation.rs`, with a `threshold_met_with_three_households` test. Copying three counters plus `min_distinct_households` into `epr-rea` is minting a second diversity ontology beside a shipped one — the exact §6 violation. When the ladder is lit, the epr-rea fold **reads** that summary or accepts a pre-computed diversity tuple. (It is also not "~40 lines, pure, no network": `ReviewEvent{event_cid, verb, magnitude}` carries no observer identity at all, so any in-fold counter is wire work.)
- **No fourth `CheckOutcome` value and no fourth `ReferReason` variant.** The entity that would write `NotMeasured` is the entity that failed; `EnvironmentDivergence` has one asker and a ts-exported, contract-tested enum to move. `ContestedEvidence` + the existing `Option<String>` note carries it.
- **No reason field added to `Refuse`.** env-red vs code-red is real and load-bearing, but it is a *finding classification*, not a verdict discriminator — it belongs in the findings and the local ledger, which already have the shape. Adding it to the spine would be the same blast radius this section just refused twice.
- **No per-run DHT writes.** ~29 runs/day ≈ 10,600 items/year against an anchor of ~3,469 content heads already costing ~2.5h quiesce after a deploy; each becomes a per-item sweep row, Kad record, election candidate and conductor round-trip on **every peer, forever.** Entry-type capacity is not the question and never was. Only a diversity-graduated attestation touches the head plane, at habit granularity — roughly 12/year, not 10,600.
- **No second measure ontology, no fifth register, no CI health score, no build-confidence index, no variety metric, no dashboard.** *The cure is reaching the existing one, or widening the seam so it can be reached, never minting another.*
- **No comparison entity, drift dashboard, or predicted-vs-actual reconcile stage.** The key already expresses it; the comparison is a `diff`.
- **No Jenkins baseline state machine**, despite the orchestrator README's own backlog item proposing it. It formalizes an illegal stored level.
- **No skip mapped to `Refer`, and no `Refer` without a named receiver and drain.**
- **No reach value that encodes an evidence claim. `commons-attested` is refused outright.** Reach is audience; standing is evidence strength. This is a constraint on **new** values, so it does not canonize a vocabulary and does not violate `elohim-storage/CLAUDE.md:195`; it makes the reconciliation converge instead of accrete.
- **No credit, standing, leaderboard, scheduling priority, or minted value for witnessing.** *Witness, don't pay.* A one-person operation with idle hardware is the exact adversary that Goodharts a witness-reward inside a week. **No settlement for attention, ever.**
- **No per-developer trust score.** Standing is per-*subject*.
- **No distributed build farm** (rakia stage 2). The valuable thing peers give is **env diversity** and **shared-commit collision**, not spare cycles. *Take the fold, defer the farm.*
- **No gate on the elohim ceiling, and no gate on the operator's loop, ever.** (Covenant-altitude promotion is the one exception, and it is a governance act, not a block — Law III.)
- **No unsigned self-declaration raising a diversity count.** An unsigned diversity number is strictly worse than none. Do **not** flip `ELOHIM_ATTRIBUTION_CROSS_SIGNED=enforce` to "make it safe" — no peer can mint a proof yet.
- **No `verification:*` observation kind whose subject duplicates the `@concern:` tag.** The observation's subject must resolve *through* that tag.
- **No auto-graduation to Canon.** `classify` already refuses it: the maximum mechanical status is `Reviewed`.
- **No archive-as-durability.** A Jenkins `fingerprint: true` is enclosure with an expiry date. Same verdict on `/tmp` and on any single mutable slot.
- **No 13th habit.** `habits.yaml` is at 12/12 on disk and covenant rule 1 says a candidate must displace one or wait. See the deliverable at the end of this document.

---

## 6. The graduation path

**The honest sequence is: run → durable report → fold (record-only) → fold (read) → independent recompute → attestation.** Not local → CI → production.

### Slice 1 — four edits, buildable tonight

This is deliberately small. The first draft proposed roughly thirty new or changed surfaces; four of them deliver most of §1a, and every one is over a part that already exists.

1. **`just test mesh` calls `build-sprint-report.ts` into a run-identified path.** `justfile:70-85` currently execs cucumber straight to a fixed `$MESH_DIR/reports/mesh.json`. Write `reports/sprint-report-<lane>-<runId>.json` under a stable glob.
2. **The sprint-report schema gains `declared`, `env` and `gitCommit`.** `declared` is read from `habits.yaml` `checks:` / the `@concern:` registry — not computed by the runner. `env` is the declared environment block including `sut`. `GIT_COMMIT` is read exactly once today (to salt a probe blob) and never written, which is why no artifact in the stock can answer *"what did commit X measure?"* — the sibling `CoverageGapReport` already carries `gitCommit`; reach it, do not mint a schema v2.
3. **`habits-status.py` prints NOT MEASURED per concern**, plus `observed_status:` / `last_measured:` beside the human-owned `status:`, plus a `max_evidence_age` render. It never writes `status:`.
4. **`recipes.yaml` `a2o:verdict` points at the run-identified path** under the stable glob.

**S5 then needs nothing built:** local↔fleet divergence is a `diff` of two reports with comparable `env` blocks. Edit 2 is what makes that true, which is why it is not optional.

Riding along, cheap and independent: **classify each finding `substrate` vs `code` at write time and mirror `ci-findings.jsonl`'s shape locally**, so the 11pm line fires offline. And **strike the `publish:results` script line** (keep the file).

### Slice 2 — the fold, record-only

5. **Generalize `measure-fold.py`'s executor to a second measure family** (a2o/dataplane). *Correction to the first draft:* the hit/miss-before-compute line already ships — `measure-fold.py:342` prints it unconditionally before any cargo is spawned. The work is a second family, not a print statement.
6. **Ship the a2o fold as RECORD-ONLY:** always write it, always print the *would-be* hit/miss line, **never skip compute.** This preserves the whole durable-evidence win with none of the correctness risk, and it makes the hit/miss line's own accuracy measurable before anyone depends on it. The fold becomes read-as-a-hit only when Q1a (subject granularity), Q1 (env recipe, judged on measured **stale**-hit rate) and Q2 (determinism) have answers.
7. **`gate-runner.mjs` writes a run-identified record.**
8. **Env on the a2o commitment; a `Process` on fulfill events; stop dropping `name`/`status`/`observed`** at `fulfill.rs:100-102`.
9. **`gate:verdict` joins `recipes.yaml` as a sibling stage.**
10. **Declare the equilibrium window in `habits.yaml`** beside the check, so changing it is a covenant edit rather than a CLI argument.

Every declaration this adds — env recipe, denominator set, algedonic marks, max evidence age — **must have a default the run emits and a human can later correct.** Otherwise the honesty discipline becomes the evening, which is the one budget this design exists to protect.

### Held until participant #2 exists

Explicitly held under §3a, because the author's own evidence disqualifies them today: with one operator and ten co-owned peers, nothing can honestly reach `witnessed`.

- `distinct_observers` / `distinct_hosts` on `EpistemicStanding` — and when unheld, built as a **seam** onto `graduation::diversity`, not as new counters.
- Tri-state `threshold_met` (SQL `COUNT(DISTINCT x)` ignores NULLs, so "nobody looked" and "one household looked, below threshold" render identically — the honest-absence defect inside the graduation gate itself). Real, and still held.
- **The `Affirm` producer.** `ReaVerb::Affirm` has zero production emitters, so `fold_standing` can currently only ever return `Emergent` or `Contested` — the top three rungs of the shipped ladder are unreachable dead code. `note.rs` deliberately chose the inert `Cite` for observations precisely because Cite never touches affirm/dismiss weight; the vocabulary has been kept honest, and `Affirm` is the review-weight verb waiting for its producer. Using `Produce` here would be the invention. Build it when there is something to affirm.
- The co-ownership unit test (Q6).

What ships instead, and costs nothing: **`claimed · 0 independent witnesses`** rendered wherever a tier would appear. It delivers the entire honesty payload with none of the machinery.

### The exchange leg — named, unbuilt, separately costed

This is the leg the first draft sold §1 on and never scheduled. Three parts, in order:

- **A read surface for folds that does not require repo write access.** Publish the `dev` HEAD fold set somewhere fetchable. This is the smallest thing that makes household #2's first hour worth anything.
- **A verification rule.** A foreign fold is an imported claim. It becomes evidence by **recompute-and-match** at the identical triple, never by being read. Unsigned foreign folds count zero until `Observation::canonical_signing_bytes` is wired.
- **The collision surface.** Participants running the gate on a **shared commit** produce identical triples at no marginal cost. That, and not donated recompute, is what makes witnessing a byproduct. Decision 15's "same-work threshold, free-in-kind reciprocity" is the shipped design art for the coordination half.

### Needs new wire — sequenced honestly

- A typed `evidence: Vec<Cid>` on `FlowEvent`. Moves the golden CID; a deliberate version bump. (The two-slot tag convention already carries `steward:<git-author-email>`, so observer identity is threadable today — `fulfill.rs`'s hardcoded `ci:dataplane` is a regression against a path that already works.)
- Real iroh-blob backing for the observation stream log, plus the `OBSERVATION_LOG_ALPN` fetch handler, plus the `/api/v1/observations` route-collision fix.
- **A producer that writes observation rows for verification subjects — a six-module build, not wiring.** Verified at six independent points: zero production writers (the only construction site outside `db/models.rs` is inside `#[cfg(test)]`), zero non-test callers on the manager and the graduation evaluator, gossip present only as `pub mod`, an `ObservationLog` that is a `Vec` in RAM whose "CID" is a rolling BLAKE3 naming no fetchable object (so every `iroh://<observer>@<log_cid>#<offset>` an attestation would carry is a dead link), and an unreachable read API. **Any slice depending on observation rows must be planned as a build.** Slice 1 and slice 2 depend on none of it.
- Cross-signed `AgentPeerBinding` — blocked, and correctly so.
- `rakia-cli` / `rakia-executor` / `rakia-peer` / `rakia-attest` actually built (all commented out in `Cargo.toml:5-10`; no `fn main` exists in rakia).
- The graduated attestation crossing to the DHT, on the existing `Attestation` carrier.

### Needs never

Per-run DHT writes. A build farm. Attention settlement. A fourth attestation primitive (rakia adds none, ever — "verified by N peers" *composes* brit's per-peer attestation).

### Where the fold store lives

**Git-tracked working-tree files now.** `.claude/data/measure-folds/clippy-pedantic/` holds two tracked files (verified: both `"evidence": "claimed"`, both `computedBy: Matthew Dowell`). The repo has run this experiment and it works — durable, diffable, offline-readable, surviving a container wipe, travelling with a clone. **And it is single-operator by construction**, which is exactly why the exchange leg is named as unbuilt rather than assumed.

**Notes-refs later.** rakia principle 3 is right in argument and currently fiction: `git for-each-ref 'refs/notes/*'` returns nothing, no rakia/brit binaries exist, `brit-helper.sh` always takes its `WARN: brit not installed … exit 0` path, and `.husky/pre-push.bash:530` calls a `--target` flag the CLI does not have, swallowed by `|| true`. **Migrate the day `cargo build -p rakia-cli` produces a binary and `brit-build-ref put` writes a ref `git for-each-ref` can see.** Not before.

**The per-observer iroh log loses for runs** on two grounds: a run has a natural artifact boundary, so §3a refuses the primitive; and the "authoritative log" the observations migration header names does not exist, which means those SQL rows are declared Category C while being the sole copy — accidentally Category B, and nothing knows it.

### What the elohim half of the pair actually does

Law III promises the ceiling "authors and retires the standing agreements the floor executes," and the first draft then gave it nothing to do — the ENRICH-never-GATE constraint honored by absence, which fails the telos from the other side. Named work, none of it gating:

- **Author and retire the declarations.** The env recipe, the denominator set, `max_evidence_age`, the algedonic marks, the supersession lifecycle. The floor executes them deterministically forever; the ceiling is what changes them, with an argument.
- **Narrate a divergence into a named env axis.** A `Refer{ContestedEvidence}` is a question; turning "these two runs disagree" into "fixtures is an undeclared env dimension" is discernment, and it is the drain that keeps the referred queue from silting.
- **The Discerner posture on an `observed_status` / `status` disagreement.** The tool renders the disagreement; the elohim argues which way it resolves and what evidence would settle it; the operator flips it under rule 4.
- **The failed-approach note.** One keystroke at 11pm, authored by the partner who was watching.

---

## 7. Leverage

Ranked by Meadows' ordering.

**12–11. Parameters and buffers (lowest).** Cache keys, the 45s timeout, the 64KB tail read, the sccache heal sentinel. Correct engineering, near-zero leverage. Note the tell: the `dev-system-equilibrium` habit's own evidence log records a command-count drop 362→322 as a delta — a parameter win logged against a paradigm-level habit.

**10–9. Stock-and-flow structure, and delays.** Four measured delays drive real overshoot: the mint→discharge lag (hours to days, derivable today only via a git hop because `validFrom` is null on every commitment); the ~20min deploy churn plus ~2.5h quiesce, which produces the documented measurement-by-deploy anti-pattern (an operator who cannot see convergence inside the delay redeploys and adds churn — a textbook delayed balancing loop); the PVC relief valve, which converts a disk stock into an unmeasured verification-debt stock with an unbounded perception delay; and `should_emit` having one threshold and no `should_clear`, so a stock at the band edge re-fires every cycle — the missing System 2 damper. **The `max_evidence_age` term in Law I is the perception-delay bound this list has been missing.**

**8–7. Balancing and reinforcing loops.** The PVC watermark loop is the one working balancing loop, and its relief valve creates the debt above. The reinforcing loop we want to light is durable-report → comparable-lanes → earlier divergence → fewer five-defect incidents.

**6. Information flows.** The NOT MEASURED headline. The env-delta diff. The substrate-vs-your-diff line reading a *local* ledger. The algedonic line above the headline. All four are pure experience changes over data that already exists or is one field away.

**5. Rules.** Anchor the denominator outside the run. Declare the window in `habits.yaml`. Give each habit a max evidence age. Never let a machine write `status:`. Delete the `|| echo` that makes the aggregator non-blocking — the single most load-bearing step in the developer valueflow cannot currently fail the build that produces it.

**4. Self-organization — the biggest structural move.** Let the local lane produce durable evidence. Today the outflow channel is fleet-only, and that is the Ashby failure stated exactly: the inflow arm is git-driven and unstoppable, the outflow arm is reachable only from a lane running ~5×/day behind a 45-minute quiesce gate. **The regulator has strictly less variety than the disturbance, by construction.** Count it: the disturbance space is at least 25 distinguishable failure classes across the museum, the substrate, the frontend and the toolchain; the regulator's vocabulary is **four units** — `green-run`, `red-run`, `artifact`, `doc` — with no compute unit, no time unit, no env label and no peer-class label anywhere in the flow log. The excess is absorbed by the operator's own memory: the ~40 memory files **are** the variety absorber the instrument cannot be, which is precisely why they need a weekly stasis loop. *The memory drift gate is the algedonic signal of an under-varied instrument.*

**3. Goals.** `outflow ≥ inflow per stock per window` is **choosable** and is being chosen: 7d DRAINING, 14d FILLING, 90d FILLING at 17:1, with the only automated consumer hardcoding 7. **A gate whose green is selected by an argument is not a gate.** Restate the goal as turnover under a declared horizon.

**2. Paradigm (highest).** *A verification result is a private artifact of the CI system that produced it.* Replace with: *a verification is a memoized derivation, owned by no one, reusable by anyone, verifiable by recompute.* Evidence the shift has not happened: two folds on disk, both `claimed`, both computed by the same person, never witnessed; `publish-results.ts` with zero callers; `refs/notes/*` empty. **The shape is built and unlit.**

### The single change that buys the most

**Make the local household lane leave a durable, run-identified, env-declaring trace.**

It is a self-organization intervention (Meadows #4), and it is the only item that *compounds*. It gives the regulator variety; it makes S5 expressible at all, so local↔fleet divergence stops living in a human's head; it turns 24 invisible runs a day into the system's primary evidence stream; it is the precondition for every fold, every graduation, every witness, and every free read.

*Honest caveat, which the first draft got wrong:* it is not "mostly wiring past the aggregator." Edits 1, 3 and 4 are wiring. Edit 2 — the `env` block with a `sut` hash and an externally-anchored `declared` set — is genuine design work, and it is the half that makes the other three worth anything.

---

## 8. Open questions, and how each gets settled

Every one carries a runnable check, a measurement, or a named operator decision. None may be settled by argument.

**Q1a (new, ahead of Q1) — what moves an a2o chapter's subject CID when a Rust file changes?** *Measurement.* Verified absent from the repo: `graph-walker.mjs` resolves changed files to gate *projects* via manifest globs; `scan-coverage.ts` does no impact analysis. There are two reachable answers and both are bad in a different direction: subject = feature-file tree hash means editing `behaviour.rs` moves no chapter subject (every chapter reads HIT and the developer skips testing his own change); subject = a closure including the SUT means any storage edit moves every chapter (0 hits exactly when he is iterating, and the free read is available only to people not changing anything). Implement two or three closure recipes, replay them over the forward-collected corpus, and report hit rate **and false-hit rate against known-broken commits**. Until this has an answer, the a2o fold stays record-only.

**Q1 — what composes the env-key for an a2o/dataplane run?** *Measurement, forward-collected.* The first draft proposed replaying "the last 60 days of archived sprint-reports"; that corpus does not exist (`genesis/a2o/reports/` is gitignored, the report is a single mutable slot, `/tmp` is wiped on restart). **Start recording env candidates now and decide in 60 days.** Candidates: narrow (fixture manifest + DNA hash + SUT hash), medium (+ conductor image tag + peer count), wide (+ per-peer archetype + doorway build). **The adoption criterion is MINIMIZE MEASURED STALE-HIT RATE subject to a usable hit rate** — not maximize hit rate, which actively rewards the narrowest key and therefore maximizes the laundering class. The fixture manifest and the SUT hash are in the key non-negotiably.

**Q2 — is a chapter's outcome deterministic at a frozen env-key?** *Measurement, and it gates Q1.* Run one kill-a-peer chapter 20× at a byte-identical env and count outcome variance. **Carry both branches, because the likely answer is "no":** a 3-peer Holochain mesh with real gossip, real timing and ~20min restart churn is close to the worst determinism candidate in the repo. If variance is zero, the fold caches the verdict and the free read is a saving. **If variance is non-zero, the fold caches a distribution with `Confidence{interval, basis}`, the run is not skippable, and the hit/miss line degrades from a saving to a prior.** That is a different product with a different §1a, and the guidestar carries it rather than depicting only the lucky branch. Either way `measures.yaml` gains a `deterministic:` field (it has an `env-sensitivity` list and no determinism field today). **Caching a flaky green launders one lucky run into a standing claim.**

**Q3 — does the observation layer get fixed, or routed around?** *Named operator decision, evidence in hand.* **Recommendation: route around it for the first two slices.** Neither needs it.

**Q4 — what denominates `costSeconds`?** *Measurement, deferred out of slice 1.* Record **both** wall-clock and metered CPU-seconds on the first 20 folds — additive, no key change — then compute cross-observer variance across the three mesh peers and the fleet at equal work. Lower variance becomes the ratio's denominator; the other stays a displayed field. Wall-clock on a shared devspace under documented PVC pressure and CFS throttling is exactly what needs measuring. Whatever wins is recorded as a `Quantity{value, kind, confidence}` from `elohim/epr::measure` — never a bare scalar.

**Q5 — mint `Duration{per}`?** *Named operator decision.* The spec author deferred it by name as "not an implementer's call" when there was one consumer. Restating the equilibrium goal as turnover-under-horizon is the second consumer, which is §3a's own admission trigger. Evidence in hand: turnover reads 280.50 / 224.40 / 327.86 across three windows while the rate-pair verdict flips DRAINING → FILLING → FILLING.

**Q6 — do peers the operator owns count as distinct observers?** *Named operator decision, declared in `measures.yaml`.* **Recommendation: no.** Co-owned peers contribute env diversity only. Settle the mechanism with a unit test when the ladder is unheld: folds computed across all 7 alpha peers plus all 3 mesh peers must **not** advance past `claimed`. **And name the unlock**, so `witnessed` is not permanently unreachable with no trigger: it lights when a second household holds a signing key the operator does not, and recompute-matches a fold at a shared commit. That condition is Q14's whole job.

**Q7 — is `a2o:scenario-green` rebased from Commitment to Intent?** *Measure, then operator decision.* Run `epr flow status --json` and count commitments whose provider is a `tool:` string and whose `satisfies` is empty. If that is essentially all 561, the structural claim is confirmed by measurement rather than by reading. Transplant safety is a runnable check: port the state machine onto a `Process`-carrying record shape and confirm the golden-CID test at `fulfill.rs:562-577` still pins.

**Q8 — when does the store migrate to notes-refs?** *Runnable gate, no discussion.* When `cargo build -p rakia-cli` produces a binary **and** `brit-build-ref put` writes a ref `git for-each-ref` can see.

**Q9 — is a second stock ever admitted?** *Gated on a written, runnable invalidation rule.* Author `invalidates:` for the clippy family in `measures.yaml`, implement it, run it over 30 days. If it drains folds without a human deleting a directory, the outflow is real. If the rule cannot be written, **the refusal is permanent.**

**Q10 — what happens to a fold whose measure is later found wrong?** *Design decision needing an author.* `ConfidenceError::NarrowingRefused` says narrowing requires a new observation, not a mutation — so a fold needs a **supersession** path, not an invalidation path. Author a `supersedes` field and have the operator ratify the lifecycle.

**Q11 — where is a concern's diversity threshold declared?** *Named operator decision.* **Recommendation: `measures.yaml`, beside the measure.** `habits.yaml` must stay a covenant capped at 12 rows, not become config.

**Q12 — what reach does a fold carry?** *Deferred, with the constraint adopted now.* Reach is inside the CID, so a mint-time choice is irreversible and five vocabularies are mid-reconciliation. The adoptable constraint — **no new reach value may encode an evidence claim** — is what makes that reconciliation converge.

**Q13 — does the fold ever gate anything?** *Settled:* **no.** A measure carries no teeth, ever. The one place evidence strength is read is a habit's promotion to green (Law III), which is a covenant edit by a human, not a fold gating a run.

**Q14 (new) — how does household #2 arrive, and what makes their first hour worth it?** *Settle by measurement, like the others.* Publish the `dev` HEAD fold set somewhere fetchable without repo write access, and count reads. Then: what does a second household need to *write* a fold, and what verification does theirs undergo before it is read as anything? This is the question a commons design must carry, and its absence from the first draft is why every peer-facing sentence in that §1 had no producer.

**Q15 (new, open, no recommendation) — is there a seat for a participant who does not run a gate?** Every form of participation in this design is *own hardware and execute a measure*, which is narrower than the ontology it claims. The one hook that could carry non-executing participation is the governance act Canon requires — correctly identified as un-auto-graduatable and then left unstaffed. Filed as open rather than solved: under §3a it has one asker, and building a surface for it now would be exactly the speculation this document refuses everywhere else. It should not be forgotten.

---

**The deliverable, in this layer's own terms, is one line — and it is not a new habit.** `habits.yaml` is at 12/12 and covenant rule 1 says a candidate must displace one or wait.

The delta lands on **`measure-honesty-local`**, whose invariant already governs exactly this ("every quantity this repo tracks declares its kind… the governance gate refuses a measure declaration with no kind — proven on this repo's own corpus, not merely asserted in prose"). Extend the invariant with its run-plane clause — *a run's report declares its denominator from outside the run, declares its environment including the system under test, and renders declared-but-unwitnessed as NOT MEASURED* — add the check, and **flip the habit green → red**, with edge #1376 as the evidence (76/80 skipped while three habits read green). That is an evidence-backed flip authored by a human, which is what rule 4 asks for, and rule 6(b) explicitly wants new reds filed as runnable checks.

It flips back to green when a household report and a fleet report over the same subject, each carrying `declared`, `env` and `gitCommit`, can be compared by the tool rather than by a person reading two logs.

If the operator would rather keep `measure-honesty-local` green and give this its own row as `run-evidence-addressed` / `@concern:run-evidence`, then it **waits** until a red retires — `dev-system-equilibrium` and `identity-cross-signed` are the nearest candidates. Either path is legitimate; adding a 13th row is not.

---

## 9. What the panel changed under challenge

Three independent critics read the draft against the code. Twenty-nine objections; twenty-four landed, five were rejected with reasons. The design that survived is smaller, later, and more honest about who is in the room.

**Blocking objections that landed and changed the design:**

| objection | what moved |
|---|---|
| **The a2o fold key omits the system under test.** Clippy works because its subject *is* the code; lifting that to a2o makes every env candidate a deployed-artifact identity that does not move on a local source edit. The draft's own flagship demo would print ten hits for a binary nobody ran. | Law II gained a mandatory `sut` clause. Q1's adoption criterion inverted from *maximize hit rate* to *minimize measured stale-hit rate*. The fold now ships **record-only** — written always, read never — until Q1a, Q1 and Q2 have answers. §1 was rewritten so the vignette no longer depicts the failure it forbids. |
| **Law I let the runner compute its own denominator** — a scoped `just test mesh` would report 100% permitted while every other concern went silently unexercised, and the 76/80 case would have vanished entirely rather than surfacing as `pending`. | The denominator moved out of the run: `habits.yaml` `checks:` + the `@concern:` registry. Slice 1 edit 2 changed accordingly. |
| **"The skip folds out of the existing `stock_over_window` with zero vocabulary change" is false on disk**, and it smuggled back a stock the section had just spent a page refusing. Verified: `StockName` is closed to one variant, fed by three commitment-specific functions. | The skip flow event is **withdrawn entirely**. NOT MEASURED is a derived count over an external denominator: zero new events, zero new stocks, zero new units. |
| **The one primitive admitted under §3a minted a second diversity ontology.** `graduation/diversity.rs` already ships `DiversityThreshold` + `threshold_met()` with household/collective/region/archetype/min_count. And `ReviewEvent` carries no observer identity, so it was never "~40 lines, pure." | Replaced with a seam: the fold reads the shipped summary or takes a pre-computed tuple. Also moved into the held-until-participant-#2 section. |
| **The deliverable breached the covenant it invoked** — `habits.yaml` is at 12/12 and the draft added a 13th while naming nothing to displace. | The deliverable is now a delta on `measure-honesty-local`, with the displacement path named if the operator prefers a row. |
| **Auto-demoting green → `unwired` destroys the state the register calls its most valuable.** `unwired` means *no check exists* and is explicitly not schedulable — the opposite of what three degraded-lane habits need. | No machine writes `status:` at all. `observed_status:` + `last_measured:` are written beside it and the tool renders the disagreement. |
| **There is no transport for a foreign fold, and none was sequenced** — so every commons, witness and compounding claim had no producer anywhere in the repo. | §1 split into §1a (built/buildable, one participant) and §1b (horizon, legs marked unbuilt). The exchange leg is named and separately costed. Q14 added. |
| **Witnessing had no motive force** and "it costs that household nothing" was false — a matching fold requires an identical triple, so it is donated recompute, not a byproduct. | The **shared commit** is named as the one surface where triples collide for free. That is the witness mechanism; donated recompute is not. |
| **The hit/miss line depends on a subject-granularity function that does not exist** — `graph-walker.mjs` resolves to gate projects, not scenarios. | Q1a added, ahead of Q1. Record-only mode makes the interim safe. |

**Major objections that landed:**

- **`Skipped → Refer` makes `Refer` a fallthrough**, which its own doc-comment forbids, and would have manufactured ~76 governance escalations from one degraded run. Withdrawn — skips render as NOT MEASURED.
- **`ReferReason::EnvironmentDivergence` mints a fourth variant in a closed, ts-exported, contract-tested enum** with one asker. Withdrawn — `ContestedEvidence` + the existing `note`.
- **`Refer` routed to a community with no members, no owner and no drain** — an unbounded queue, the very instrument-with-no-reader the draft refuses six times. Receiver and drain are now named, and the referred count is in the headline.
- **No time dimension**, so a habit nobody runs for two weeks still reads green. `max_evidence_age` added to Law I.
- **env-red vs code-red had one producer, and it was Jenkins-bound** — the most dignity-preserving line in the loop did not exist offline. A local findings ledger of the same shape now feeds it.
- **No algedonic bypass** — every signal reached the top through the same rollup. Law IV added.
- **`costSeconds` as a bare scalar violates law 27**, which the draft cited two sections earlier. Typed as `Quantity` and moved out of slice 1.
- **The evidence ladder was decision-inert by construction**, which is the instrument-with-no-reader shape. It now has exactly one reader at covenant altitude: a habit's promotion to green may require independent witness where witness is possible.
- **Non-blocking was conflated with non-dependence**, engineering interdependence out of a design whose telos is a commons. Law III now distinguishes the loop (never gated) from the covenant (may depend).
- **The elohim half had no authored role** — ENRICH-never-GATE honored by giving the ceiling nothing to do. Four named responsibilities added, none of them gating.
- **The plan was ~30 surfaces where ~4 edits deliver most of the felt experience.** Cut to a four-edit slice 1 and a held section.
- **Q2's likely answer is "no" and only the lucky branch was depicted.** Both branches now carried.
- **The declaration load all landed on the one person the design protects.** Every declaration must ship a default the run emits.

**Minor corrections:** the hit/miss line already ships (`measure-fold.py:342`) — item 4 is "generalize to a second family," not "print the line"; `recipes.yaml` stages are globs, so the cure is run-identified filenames under a stable glob, not content-addressed names; Q1's 60-day replay corpus does not exist, so it is forward-collection; `publish-results.ts` is retired as a script line but kept as a file, since deleting it removes the only content-graph leg before a replacement exists.

**Objections rejected, with reasons:**

1. **"Demote green → `red` instead of `unwired`."** Rejected. A machine writing `red` is still a machine editing a covenant, and `red` asserts schedulable work. The objection behind it (don't write `unwired`) landed in full; the proposed alternative did not. `observed_status:` beside `status:` gives the operator everything and asserts nothing.
2. **"Put a reason on `Refuse`."** Rejected. env-red vs code-red is real and now shipped — but as a *finding classification*, not a verdict discriminator. Adding a field to `Refuse` has the same blast radius (ts-rs export, `schema_contract.rs` pin) this document refuses twice for `CheckOutcome` and `ReferReason`; a design cannot refuse an enum change on principle and then make one for its own convenience.
3. **"Hold the graduation ladder entirely until participant #2."** Rejected in part. The *machinery* is held (counters, tri-state, `Affirm` producer, co-ownership test). The *reader* is not: a ladder with no consequence anywhere is the instrument-with-no-reader failure, and habits.yaml's documented over-claim problem is the reader it was missing. The rule is authored now, evaluates to "not possible" today, and renders `claimed · 0 independent witnesses`. It costs nothing and it stops the ladder from being inert by construction.
4. **"Re-title the document; the commons is decorative."** Rejected as a conclusion, accepted as a diagnosis. Every §1 moment did survive with zero peers, and that was worth naming — but a guidestar's job is to state the generational target, not to shrink to what one person can build this month. The fix is marking which legs are unbuilt (§1b), naming the shared-commit collision surface, and adding the arrival path as Q14 — not renaming the telos to match the current population.
5. **"Build a seat for the non-executing participant."** Rejected for now, filed as Q15. Under §3a it has one asker; building a surface would be the speculation this document refuses everywhere else. Recorded so it is not lost rather than solved so it can be shipped.

**What survived untouched, because all three critics tested it and it held:** the refusal of a new stock; the refusal of a fourth `CheckOutcome` value with its *the entity that would write it is the entity that failed* argument; the deletion of `ReachLevel`; the three-axis split of audience / evidence strength / outcome; the refusal of per-run DHT writes; the refusal of witness credit and attention settlement; the deletion of the Jenkins baseline; and the ruling that a fold gates nothing.
