---
title: Genesis Docs Placement Contract — "this goes here"
id: placement
status: Accepted
created: 2026-06-01
---

# Placement contract — we own where things live

The genesis docs are the protocol's interpretability layer. Every doc has ONE home, chosen by its KIND and
STATUS. Superpowers (brainstorming / writing-plans / executing-plans) is the *generator*; this contract plus
the memory-ceremony agents are the *librarian* that files and enforces. Three homes, mirroring the memory comet:

| Home | Path | Holds | Marker |
|---|---|---|---|
| **CANONICAL** | `content/elohim-protocol/architecture/` | living cross-cutting truth (gospel-tier) | `tier: architecture` + INDEX row |
| **HISTORY** | `content/elohim-protocol/history/` | distilled paths-not-taken / superseded / settled decisions; each bidirectionally linked to a canonical | `tier: history` + INDEX row |
| **ACTIVE** | `superpowers/specs/` + `superpowers/plans/` | in-flight specs + plans only | `status: Draft\|Design\|In-flight` |

**The subject-class axis (the fourth "home" the §"Dev-doc home undefined" issue asked for).** A doc's
`class` decides *which* set of homes its residue decomposes into. **`protocol-canonical`** → the three
homes above (the rich flow). **`process-meta`** → a CLAUDE.md **gospel-diff** + a `.claude/` **tool**
(`history/` stays LIVE for a tried-and-failed lesson; `architecture/` + `a2o/<pillar>` are NULL). The
class registry + per-class flow is **`.claude/subject-routing.yaml`** (read by the brainstorm/decompose
gates) — this is the class axis the same way `genesis/manifests/cluster-state.yaml` is the env axis; the
"fourth home" is not a new directory but the `.claude/`+CLAUDE.md gospel target process work already lands in.

## Lifecycle — every artifact has a next state (this is the anti-debt rule)

```
ACTIVE  (captured → in-flight)
  └─ on landed | superseded | abandoned →
       distilled to a HISTORY record  (one-sentence lesson + pointer + bidirectional canonical link)
         └─ raw body retires to git history
```

Nothing stays in ACTIVE after it has landed or died. **A doc with no next state is the debt.**

## The authority rule cartographer proved (2026-06-01)

Checkbox state and in-doc status tables are **NOT** authoritative — they go stale. (The iroh delivery-master
showed every gate ⏳ / `[x]=0` while git held landing commits for all of them.) Authority, in order:

1. a maintained `status:` frontmatter, 2. the `landed_commit:` SHA, 3. code-existence on disk.

Therefore, on landing, a plan/spec MUST gain:

```yaml
status: landed          # or: superseded | abandoned
landed_commit: <sha>    # or, when superseded:  superseded_by: <history-record-id>
```

## Retirement requires VERIFICATION, not a "landed" claim (the hard gate)

A "landed" claim is as untrustworthy as a checkbox. Agent developers — and humans — report "delivered" when
code merely **exists**, while the soak never ran, a regression slipped in, or the substrate is half-built.
**Code-existence + a landing commit is NECESSARY but NOT SUFFICIENT to retire.**

A plan/task retires to `history/_retired/` ONLY when it is **verified-stable** — graded against ACTUAL evidence
of working, not self-asserted:

- the area's tests pass and the relevant CI pipeline is green (and *not* cascade-masked — see `feedback_cascade_halt_masks_failures`);
- any required soak / parity / stress window actually **ran clean** (the evidence exists, not just the harness);
- no open regression it introduced.

Verification is **graded by an external check** (ci-investigator / CI / soak-tracker), the same way *reach is
earned, not self-asserted* (`project_reach_earned_genesis_seeder_grades_homework`). **A plan cannot grade its
own homework.** The verifying evidence (passing CI ref, soak window, test IDs) is recorded on the retired record.

## Plans are not atomic — DECOMPOSE on retire

A "landed" plan usually mixes verified-stable tasks with unfinished / broken / regressed ones. On retire, SPLIT it:

- verified-stable tasks → `history/_retired/` (with the verification evidence attached);
- not-done / unverified / regressed tasks → **STAY in ACTIVE** as the next implementation sprint's input.

`history/_retired/` holds **only** verified-stable work. If it would hold anything unverified, it is a dumping
ground and this rule has failed. (`_retired/` is therefore not created until the first cluster passes the gate.)

## Scope is RECONCILED, not assigned — the coherence graph IS an EPR feedback graph

Placement is **not** one-directional (hot → retired). A doc's scope — its *temperature* — is a continuous
function of the live verification state of the work it describes. This is the protocol's own nervous system
turned inward; every part already exists:

- **Each link is a deterministic verification edge.** A retired doc declares `verified_by:` edges (the
  existing `cites:` pattern — the cites re-open hook already re-opens a memory when a cited source changes,
  `project_memory_cites_edge`) to the tests / CI job / soak window / code that PROVE it. The edge carries a
  *check*, not a claim.
- **Regression is a FeedbackSignal that cascades back.** When a `verified_by` source regresses (test red,
  soak diverges, code deleted), the signal back-propagates along `covers` / `derives` edges — exactly the
  reach/feedback back-prop of the EPR nervous system (`project_social_reach_nervous_system`,
  `project_feedback_governance_are_reach_earning_machinery`).
- **The cascade WARMS the doc** — pulls it from `_retired` (cold/shelved) up the quilt temperature tiers
  (`project_quilt_pantry_vocabulary`, applied to docs): cold → warm → hot/ACTIVE, re-opened as the next
  sprint's input. A doc whose verification goes red **cannot stay retired**.
- **The controller reconciles eagerly** (`project_principle_p1_reconciliation_controller`): PLACEMENT is the
  desired state (manifest); the verification signals are reality; the hook reconciles the diff by adjusting
  temperature. Retire is the cold steady-state of a *passing* loop, not a one-way trapdoor.

**Two implementations, extend-first:**
- **Lightweight (now):** `verified_by:` frontmatter (the `cites:` edge) + the verification-leg check
  (ci-investigator / CI) + a `temperature:` field; the PostToolUse hook warms on regression. No EPR
  ingestion — reuses primitives that exist.
- **Native (graduation):** docs become real EPR atoms; verification *is* a FeedbackSignal on the EPR; scope
  *is* the EPR's earned, revocable reach. The avodah / spec-as-EPR path — net-new ingestion, a later phase.

## Availability ≠ regression — env-state narrows SCOPE without a cascade

"Can't verify it right now" splits into two opposite meanings. Conflating them floods the graph with false
regression signals — and a partial cluster is the **steady state, not a failure** (`project_seed_whoever_is_ready`),
so offline resources must NOT read as broken work. The four verification states:

| State | Meaning | Cascade | Scope |
|---|---|---|---|
| **VERIFIED-STABLE** | graded green on an available env | — | retire-eligible (cold) |
| **CLAIMED-ONLY** | code exists; verification never ran though it *could* | none | HOT — needs verifying |
| **REGRESSED** | was green, now red on an **available** env (the work broke) | **warm-cascade** along `covers`/`derives` | HOT — re-work |
| **BLOCKED-BY-ENV** | can't grade — a required env resource is **unavailable** | **NONE** (not the doc's fault) | HELD / out-of-current-scope |

The env layer (`genesis/manifests/cluster-state.yaml`) declares what is available (nodes, clusters, registry).
Each test / story / gate declares `requires_env:` (caps must match cluster-state names exactly — e.g. `shem`,
`alpha-cluster-6peer`, `harbor-registry`; an unknown cap is surfaced as vocab drift, never silently dropped). The scope
resolver: **runnable = requirements ⊆ available.** Anything requiring an unavailable resource is **BLOCKED-BY-ENV**
— held, not warmed, **no regression signal.** Updating cluster-state (e.g. `shem: offline`) cascades the *scope*
— it narrows what planning and verification consider in-scope right now — and re-widens it when the resource
returns. This lets planning focus on what's actually testable without lying about what regressed
(`project_placement_signals_are_shefa_inputs`: a gap is a signal, not an alarm).

## Enforcement (deterministic — to build, extends memory-kit)

A PostToolUse hook (extending the existing memory-kit drift accumulators) flags, on any genesis-doc write:
an ACTIVE-home doc whose `status` says landed/superseded (= **placement drift**), and surfaces the count at
SessionStart. Superpowers writes; the hook + ceremony agents file. This is the deterministic fire that keeps
the memory surface in balance — owned by us, not superpowers' defaults.

## Re-equip cartographer (decision pending)

Cartographer reconstructs landed/superseded state by git archaeology because no machine-readable index exists.
Two cheap fixes: (a) the `status`/`landed_commit` frontmatter above; (b) **read-only mempalace for cartographer**
— it is the only ceremony agent without it. See the tooling-context research.
