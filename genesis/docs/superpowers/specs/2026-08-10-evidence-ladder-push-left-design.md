---
title: Evidence Ladder + Push-Left Pressure — closing the prose-to-delivered chain
id: evidence-ladder-push-left
status: Draft
class: process-meta
context-tier: disclosed
steward: cartographer
graduation-trigger: increments 1-3 landed and walking OR superseded
created: 2026-08-10
topic: [evidence-ladder, push-left, rea-cost, cite-admissibility, epr-flow, quiesce-gate, validate-only, simulacra, habits]
cites:
  - genesis/manifests/habits.yaml
  - scripts/ci/fleet-quiesce-gate.sh
  - genesis/a2o/features/dataplane/resiliency-saga/README.md
  - scope-tree-reconciler-design | Supplies the tier-position vocabulary: @requires: tags + cluster-state.yaml already encode where a concern sits on the evidence ladder — the ladder reinterprets the scope setpoint as a cost tier, adding no new metadata | sha256:5332b1422eb86eb2 | path: genesis/docs/superpowers/specs/2026-06-02-scope-tree-reconciler-design.md
---

# Evidence Ladder + Push-Left Pressure

**Operator directive (2026-08-10, mid-session, verbatim intent):** every intention
must connect to a promise in code — flowing over habits, valueflows, cites,
schemas, interfaces, DI, code-generators, tests, and e2e validations — end-to-end,
with compiler-time confidence that nothing dangles. Any agent must know **at a
command**: what's not compiling yet, where we left off, what's left, what's
orphaned — so any sprint can take accountability. The authoritative validation
that closes a loop should live as close to machine code as the property allows,
proven in local micro before the deployed environment confirms it. And the
2.5-hour CI cycle **is the REA cost of the pipeline** — accumulated cost must
itself generate redesign pressure to push validation left.

## 1. The incident that priced the problem

The saga register sat at 8/11 for weeks while the fleet itself improved, because
the recording instrument was structurally unable to record — and the facts that
would have unblocked it existed only in prose:

- `[edge:validate-only]` (measure without deploying) shipped 2026-07-30
  (b2dfd0de2) and was re-requested as a missing wishlist item on 2026-08-09.
- The fleet-quiesce gate's predicate reads **storage-A (matthew) only**; the
  closing shift diagnosed it as blocked on the shem trio — a claim about code
  with no fingerprint binding it to the code.
- Six deploy-coupled banking runs (~8 pipeline-hours) produced **2** recorded
  measures — ≈4h per unit of evidence. The same scenarios measured locally cost
  13.4s, then 0.3s: a ~1000× cost-per-evidence ratio, invisible because nothing
  priced it.

## 2. The rule (admissibility)

Nothing load-bearing may exist as prose alone.

1. **An intention** (habit, backlog item, wishlist line, flip condition) is
   admissible only if it names its runnable proof edge (`@concern` tag, gate
   clause, test id).
2. **A claim about code** is admissible only if it carries a cite whose
   fingerprint covers the code region that makes it true. When the code changes,
   the claim visibly expires (DEAD-CITE) — drift becomes a build error, not a
   3-hour CI discovery. (Same trick ts-rs plays at the Rust→TS boundary, lifted
   to the intention layer.)

Enforced at the three moments that already exist: write-time (`.epr-meta`
compose gates), push-time (husky gate), walk-time (`epr flow` / renderers).

## 3. The evidence ladder (test → evidence is TWO rows)

Development evidence and CI evidence are distinct rungs of one ladder; the same
`@concern` scenarios are the artifact at every rung — never a parallel suite.

| Tier | What runs | Cost | State 2026-08-10 |
|---|---|---|---|
| T0 compiler | cargo/tsc/clippy, schema-contract, codegen freshness | seconds | closed |
| T1 in-process simulacra | sweettest (multi-conductor DHT); elohim-storage integration tests (`sync_libp2p_convergence.rs`, `household_resilience.rs`, `epr_atom_federation_integration.rs`) | seconds–minutes | exists, coverage thin |
| T2 local-stack simulacra | hc-start stack + the same cucumber suite via `E2E_*` pointers | minutes | single-peer only |
| T3 live-fleet local measure | local cucumber against deployed doorways | seconds (not hermetic) | proven (notary 3/3 ×2) |
| T4 banked CI evidence | Dataplane Validation + quiesce gate → sprint-report → register | hours | closed |

**Ascend-only rule:** a concern is eligible for a T4 banking run only when green
at its highest locally-runnable tier; a T4 red must be reproduced at the lowest
tier that can express it before a fix is attempted. (The heal-leg `break` fix
shipped with a T1 regression test proven red-on-old-code — that was correct
practice as heroics; this rule makes it admission criteria.)

**Tier position needs no new metadata:** `@requires:` tags + `cluster-state.yaml`
(the scope-reconciler vocabulary) already encode it. No tags = T2-runnable;
household-testable = T3; `@requires:alpha-cluster-6peer` = T4-pinned.

## 4. REA cost accounting → redesign pressure (the self-optimizing part)

A T4 run is an **economic event**: wall-clock + compute consumed, evidence
produced (a gate-skip no-measure is cost with zero output). jenkins-sync already
ingests durations. Per concern, the walker accumulates: cumulative T4 spend,
cost-per-green-evidence, and the ratio vs its cheapest green tier. Past a
threshold, mint a `redesign-pressure` finding through the existing
flag→agent→canon dispatch pattern. The push-left queue is then **read off the
ledger**, not judged: the concern burning the most pipeline-hours per unit of
evidence is the next down-ladder migration target. Waste prices itself: three
gate-skip no-measures in one night flags the instrument as the defect.

## 5. Increments (each on an existing rail — no new instruments)

1. **Admission cites** — `.epr-meta` rules on `genesis/manifests/habits.yaml` +
   the backlog dirs: checks/flip-conditions/claims must carry cites to the
   enforcing surface (same deny-at-write that governs memory frontmatter).
   Kills the misdiagnosis + capability-amnesia classes.
2. **Push-time tag coherence** — pre-push clause: `[build:edge]` with a
   changeset touching no edge-watched paths (read from the build manifest, not
   hardcoded) is refused with a pointer to `[edge:validate-only]`. Kills
   measurement-by-deploy.
3. **The one command** — complete `epr flow status/walk` to traverse
   intention → proof → evidence with compiler-format output
   (`ERROR dangling-proof …`, `WARN orphan …`, `ERROR stale-claim …`), a tier
   column (highest tier green + when), and the cost column from §4. Answers:
   not-compiling / left-off / what's-left / orphaned. SessionStart points here.
4. **T2 multi-peer simulacra** — grow hc-start from one trio to 2 storage peers
   + 2 doorways (minimal topology expressing head-election, ghost anchors,
   authority refusal, and the quiesce predicate), reusing deployments.json
   personas + the same seeder + the same feature files. Grow only when a named
   concern moves down-ladder (never speculatively). Kill/restart a local peer to
   reproduce churn-window dynamics in minutes.
5. **Cost accounting + redesign-pressure sentinel** — §4 wiring.

Habit binding: increments serve `notary-authority` (active) and the saga
register directly — the first down-ladder migrations are exactly the concerns
that held the register at 8/11.

## 6. Non-goals

- No new registers, ledgers, or ranking scripts beyond completing the existing
  walker (2026-08-06 nomenclature-review lesson stands).
- No parallel test suite: one set of `@concern` scenarios, many rungs.
- No semantic truth claims from fingerprints — a cite binds a sentence to exact
  bytes and expires it on change; that expiry, enforced at write/push/walk, is
  the whole (and sufficient) guarantee.
