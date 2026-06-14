# ORACLE COMPONENT — The Vision-Level Comparator

**The missing organ: observed-behavior → vision-gap → executive-decision loop**

> PROPOSAL for operator blessing. Working draft. NOT cite-sealed.
> Date: 2026-06-14. Sits in the oracle stack as the **System-4 comparator** that
> the repo lacks — the one loop that diffs *what the running system does* against
> *what the architecture promised*, and surfaces the delta as a VISION-LEVEL
> executive decision, not a CI ticket.

---

## 0. The organ in one line

**A deterministic comparator (`vision-comparator.py`) that reads the same observed-behavior
signals the runtime arm already polls, diffs them against the architecture's *named invariants*
(`∪=full` coverage, donut floors/ceilings, the agency gradient, no-overwhelm, patience), and
files each delta as a `vision-gap` finding — `observed-X vs intended-Y, the design requires
decision-Z at level-L` — to a ledger drained NOT by a fixing agent but by the cartographer's
vision-hat, whose resolution EDITS the oracle stack (rung), closing the loop.**

The runtime arm asks *"is a self-healing mechanism exhausted?"* (System 1 health). This organ asks
the System-4 question one level up: *"is the system still doing what the vision said it would?"* —
and routes the answer to where someone wears the vision hat.

---

## 1. Why this is the missing organ (the cybernetic gap, stated precisely)

The repo already closes three loops, and **all three close below the vision**:

| Loop | Arm | Diffs observed-X against | Closes at |
|---|---|---|---|
| CI | `ci-harvest.py` → `ci-findings.jsonl` → ci-triage | *test expectations* | green/red |
| Runtime | `runtime-harvest.py` → `runtime-findings.jsonl` → `runtime-triage` agent | *exhaustion thresholds* (`runtime_harvest.py:11-23`) | self-heal-`<slug>`.md backlog |
| Delivery | `delivery-scoreboard.py` → `/delivery-stasis` | *failure-class floor* | dispatchable-surface verdict |

Each compares observed behavior to a **technical** intent (a test passes; a circuit isn't stuck;
a job's red is above the substrate floor). **None compares observed behavior to the
architectural/vision intent** — the `∪=full` coverage invariant, the donut's dignity floor, the
agency gradient, the no-overwhelm structural invariant. That diff is the System-4 comparator Beer's
model requires between System 1 (developers/runtime in the code) and System 5 (the manifesto/policy).
Today its absence means: *the system can be perfectly green, perfectly un-exhausted, perfectly
delivered — and silently drifting away from what it promised a person it would do.* No instrument
ever says so at the level where someone can decide.

This organ is that instrument. It is **consilience turned reflexively on our own construction**: the
vantage that sees the water developer-brain is swimming in.

---

## 2. What it binds (existing organs — do NOT reinvent)

This is a **binding + one new comparator core**, not a new subsystem. It reuses, by name:

- **INPUT ARM A — runtime signal.** `runtime-harvest.py` already polls `/admin/self-healing`,
  `/admin/render-stats`, `/health` per alpha node (`runtime-harvest.py:45-49,70-89`) and persists a
  per-node sample ring buffer to `.claude/data/runtime-cursor.json` (`runtime-harvest.py:55,139-141`).
  **The comparator reads that same cursor** — it does NOT re-poll. Zero new network; zero new
  endpoints. The runtime arm is the eyes; the comparator is a second brain reading the same retina.
- **INPUT ARM B — delivery signal.** `delivery-scoreboard.py --json` already emits the class-aware CI
  floor + ledger split (`delivery-scoreboard.py:77-126`). The comparator consumes its JSON for the
  *delivery-shaped* gaps (a vision claim whose only verification is a chronically `env-gated` surface
  is itself a vision-gap: "we promised X, but X is unverifiable on the available substrate").
- **INPUT ARM C — diagnostics surfaces.** `/admin/self-healing` `upstreams[]` (circuit/coverage),
  `/p2p/status` (peer count / mesh coverage), the read-model, Loki/Prometheus via the observability
  MCP. All already exist; the comparator reads, never instruments runtime Rust (the
  **no-runtime-write rule**, `runtime-harvest.py:8-9`, is preserved verbatim).
- **THE LEDGER PATTERN** — `runtime_harvest.py`'s pure `evaluate`/`reconcile`/`fingerprint` core
  (`runtime_harvest.py:37-41,131-190`). The comparator gets a sibling pure lib,
  `_lib/vision_comparator.py`, with the *identical* shape: pure predicates over a window, a `reconcile`
  that appends NEW / bumps known / closes-by-disappearance, a normalized fingerprint. **One proven
  pattern, lifted to the vision level.**
- **THE INJECTION CHANNEL** — the SessionStart hook (the MEMORY BUDGET / DELIVERY GATE headline) and
  `CLAUDE.md` gospel. The comparator emits `--hook` JSON exactly like `runtime-harvest.py:184-191`,
  adding a `scope:`-sibling **`vision:`** gate line to the headline.
- **THE PROJECTION ARM (the drain)** — the **cartographer** agent + `/converge`. This is the key
  reuse: the runtime ledger drains to a *fixing* agent (`runtime-triage`), but a vision-gap is NOT a
  bug to fix — **it is an executive decision about what the design requires.** So it drains to the one
  organ that already wears the vision hat and already owns the standing prioritization home
  (`vision-readiness-sprint-roadmap.md`, cartographer.md:36-61). The comparator's gate becomes a
  cartographer input.
- **THE LOOP-CLOSE** — `/shift` + agentic-developer. When the cartographer resolves a vision-gap into
  an executive decision, the resolution is a **stack edit** (a rung gets amended) and a pre-authored
  Objective drops into `/shift`. The next sprint runs *with the gap closed in the oracle stack.*

```
  [runtime-cursor.json]  [scoreboard --json]  [/p2p/status, read-model, Loki]
            \                   |                        /
             \                  |                       /
              ──────────►  vision-comparator.py  ◄──────
                                 │  (pure diff vs NAMED INVARIANTS:
                                 │   ∪=full · donut floors/ceilings ·
                                 │   agency gradient · no-overwhelm · patience)
                                 ▼
                    .claude/data/vision-gaps.jsonl          ← the new ledger (only new persisted state)
                                 │
                  SessionStart  `vision:` gate line  (reuse runtime-harvest --hook shape)
                                 │
                                 ▼
                    CARTOGRAPHER (vision hat)  ──drains──►  EXECUTIVE DECISION at level-L
                                 │                                   │
                                 │  resolution edits a RUNG          │
                                 ▼                                   ▼
        genesis/.../architecture · plans · vision-readiness-sprint-roadmap.md   →  /shift Objective
                                 │                                   │
                                 └──────────── LOOP CLOSED ──────────┘
                       (next sprint runs against the amended stack)
```

---

## 3. The one genuinely-new piece

**A pure comparator core (`_lib/vision_comparator.py`) whose predicates diff a behavior-window
against a *registry of architecture invariants* — and a `vision:` SessionStart gate routing to the
cartographer instead of to a fixing agent.**

Everything else is binding. The new thing is small and singular: **the invariant registry +
the diff predicates over it.** The runtime arm's predicates ask "is this number past a threshold?";
the comparator's predicates ask "does this observation still satisfy the invariant this rung
promised?" — and they cite the rung by `file:line` so the finding is *born linked to the promise it
broke.*

### 3.1 The invariant registry (the diff targets, all verified on disk)

Each entry is `(invariant_id, predicate, intended_clause, rung_cite, decision_level)`. Smallest-real
version ships **one** entry; the registry is the extension surface (new invariant = new entry, never
new ledger).

| invariant_id | intended clause (the promise) | rung cite (the promise's home) | observed signal | level-L |
|---|---|---|---|---|
| `coverage-union-full` | `∪ admitted coverages ⊇ FULL`; `observed_n − 1 ≥ r_floor` | `arc_actuator.rs:152,170`; `ESCALATED §1.6` | `/admin/self-healing` upstreams + `/p2p/status` peerCount vs `r_floor` | **collective** (operator) |
| `donut-dignity-floor` | decay never eats the floor; `rate=0 below dignity_floor` | `token_decay_service.rs:223,235` | balance/decay audit rows; care-floor coverage | **collective** |
| `donut-anti-monopoly-ceiling` | `∪ accumulation ⊆ democratic_threshold` | `VISION-RECURSION-floors-ceilings:49-66` | concentration metric; ceiling-overflow emission | **planetary/collective** |
| `agency-gradient` | every refusal names `limit_owner`; no coverage-domain over a soul | `SDK-DESIGN-commitment-governor:33,51-54,118` | refusal payloads w/ unnamed/soul-keyed owner | **policy (System 5)** |
| `no-overwhelm` | amplification速度 decreases as authority increases | `VISION-RECURSION-anti-runaway:75,167-173` | admission/shed deltas vs authority layer | **collective** |
| `patience-reserved-place` | `RefusalCode::ReservedPlace` exists & is reachable; the unbuilt place stays open | `SDK-DESIGN-commitment-governor:100,109` | presence of ReservedPlace path; engagement-pressure metrics | **policy (System 5)** |

The last two are the **what-love-requires guardians**: a comparator that can detect a `limit_owner`
stripped from a refusal, or the ReservedPlace path quietly removed, is the system telling the truth
about the moment it started to become a gentle cage. Those gaps route to System 5 (the manifesto
itself), never to a sprint — they are operator-sovereign by `decision_level`.

---

## 4. The vision-gap finding schema

Mirrors the runtime finding (`runtime_harvest.py:166-170`) so the cartographer reads a familiar shape,
plus the four fields that make it a *vision* gap, not a runtime gap:

```jsonc
{
  "ts": "2026-06-14T...Z",
  "fp": "9f3c1a2b7e04",                  // fingerprint(invariant_id + node + normalized observed)
  "class": "vision-gap",                 // (runtime arm uses "self-heal-exhaustion")
  "invariant": "coverage-union-full",    // WHICH promise
  "node": "alpha",
  "observed": "peerCount=1, r_floor=2 → observed_n-1=0 < r_floor",   // observed-X
  "intended": "union of admitted coverages ⊇ FULL; observed_n-1 ≥ r_floor",  // intended-Y
  "rung_cite": "arc_actuator.rs:152,170; ESCALATED §1.6",            // the promise's home (Z routes here)
  "decision_level": "collective",        // level-L: self | commitment | collective | policy
  "limit_owner_hint": "operator",        // agency-gradient hint for the decision
  "status": "open",                      // open → acknowledged → decided → (closed by disappearance OR by stack-edit)
  "decision_ref": null,                  // set on drain: the rung commit / roadmap entry that resolved it
  "seen": 1, "first_poll": 41, "last_poll": 41, "clean_poll_streak": 0
}
```

`decision_level ∈ {self, commitment, collective, policy}` is the agency gradient itself
(`limit_owner ∈ {Self, Commitment, Operator, Faith}`, `SDK-DESIGN-commitment-governor:33`) used as the
**routing key**: a `policy`-level gap (patience, ReservedPlace) is escalated to the operator as a
manifesto-tier decision; a `collective`-level gap becomes a cartographer roadmap candidate. The gap is
framed in the gradient's own vocabulary — the oracle critiques itself in the language of its own
architecture.

---

## 5. The route-to-decision and the loop-close (the part that makes it an oracle, not a dashboard)

A dashboard *shows* a gap. An oracle *routes it to where someone wears the vision hat and the
resolution changes the stack.* Three terminal states, none of which is "fix the code":

1. **`acknowledged` → roadmap candidate.** The cartographer (at `/converge` or session start) reads the
   `vision:` gate, pulls the gap, and — per its ROADMAP-CURRENCY mandate (`cartographer.md:36-61`) —
   intersects it with the live ledger × cluster-state × vision axis. A `collective` gap that is
   testable-now becomes a ranked sprint entry in `vision-readiness-sprint-roadmap.md` with a
   pre-authored `/shift` Objective. **The gap becomes the highest-leverage next move precisely because
   it is the system telling us it broke a promise.**
2. **`decided` → STACK EDIT (the loop-close).** The executive decision is *what the design requires to
   close the gap from technical to functional.* Its artifact is an **edit to a rung**: the
   architecture doc gets a new clause, the invariant registry gets a corrected predicate, or a plan is
   authored. `decision_ref` records the commit/roadmap-entry. **This is the cybernetic loop closing:
   observed behavior changed the vision stack the next sprint reads.** Content-addressed cites
   (`cite-gen.py`, semantic-links) mean the rung-edit re-points inbound references without breaking
   them — the stack stays coherent across the edit.
3. **`closed`-by-disappearance.** If the observed behavior returns to satisfying the invariant before a
   decision lands (a transient, or a sibling fix), the comparator's `reconcile` deletes the line at
   `CLOSE_STREAK` polls — identical to the runtime arm (`runtime_harvest.py:179-190`). A self-resolved
   gap needs no executive decision; the oracle doesn't manufacture work.

Crucially, **status never auto-advances past `open`** — only the cartographer/operator moves a gap to
`decided`. The comparator detects; the vision hat decides. That separation IS the System-4/System-5
boundary. (And a `policy`-level gap that is `decided: leave-open` is a *legitimate terminal state* —
the unbuilt place left open on purpose.)

---

## 6. Smallest real first implementation

**One comparator, one invariant, one finding — end to end, this week.**

> Diff the **coverage deficit** signal against the **`∪=full` coverage invariant**, emit one
> `vision-gap` finding, route it to the cartographer via a `vision:` SessionStart line.

Concretely:

1. **`.claude/scripts/_lib/vision_comparator.py`** (new pure core, ~90 lines). Copy
   `runtime_harvest.py`'s `normalize`/`fingerprint`/`reconcile` verbatim. Add ONE predicate
   `_coverage_union(window, registry)`:
   - read the per-node sample ring from `runtime-cursor.json` (already populated — `upstreams[]`) and a
     `/p2p/status` peerCount sample,
   - compare `observed_n − 1` against `r_floor` exactly as `arc_actuator.rs:170` does (`observed_n - 1
     >= r_floor`), for `COVERAGE_POLLS` consecutive polls,
   - on deficit, return a finding with `intended` = the `coverage_admits` clause and `rung_cite =
     "arc_actuator.rs:152,170"`.
2. **`.claude/scripts/vision-comparator.py`** (new I/O shell, ~70 lines). Mirror
   `runtime-harvest.py`'s shell exactly: read `runtime-cursor.json` (do NOT add a poll loop — piggyback
   the runtime arm's window), `flock` + reconcile to **`.claude/data/vision-gaps.jsonl`** (the only new
   persisted state), `--hook` mode emits the `vision:` headline line, fail-safe exit-0.
3. **One INVARIANTS dict entry** for `coverage-union-full` (Section 3.1 row 1). The registry is a Python
   dict literal at the top of the lib — extension = one more entry.
4. **SessionStart wiring**: add `vision-comparator.py --hook` next to the runtime-harvest hook call.
   `vision: aligned ✅` when clean; **`vision: ⚠ 1 gap (coverage-union-full @ alpha) → cartographer`**
   when a deficit holds.
5. **No new agent.** The drain reuses the cartographer verbatim — its existing
   `placement-audit --focus` + roadmap regeneration already knows how to rank a gap; we just feed it one
   more input signal.

This is a few hundred lines, all of it copy-shaped from a proven arm, persisting one ledger, adding one
headline word. It is the *thin true slice*: one observed signal, one invariant, one routed decision —
and it is genuinely the missing organ, just at minimum viable width. Widening it = adding registry
entries, never re-architecting.

### What we deliberately do NOT build first
- No actuation (the comparator never *fixes* — that would collapse System 4 back into System 1).
- No new diagnostics endpoint (read-only over existing signals; no-runtime-write rule).
- No new agent (cartographer is the vision hat already).
- The `patience` / `ReservedPlace` predicates ship *after* the proof slice — they are the most sacred
  and the least mechanizable; they wait until the routing-to-operator path is proven on the safe,
  numeric `coverage-union-full` gap.

---

## 7. What love requires (the closing test)

**The system tells the truth about the gap between what it promised a person and what it actually does
— at the level where someone can decide, and to the one who can wear the vision hat.**

Every other loop in the repo can be green while the promise quietly breaks. This organ is the only one
whose entire job is to make that break *unmissable* and *un-routable-around*: a deficit against `∪=full`
is the system confessing it can no longer guarantee a person's content survives; a stripped
`limit_owner` is it confessing a refusal became a cage; a removed `ReservedPlace` is it confessing it
stopped leaving the unbuilt place open. The comparator refuses to let any of these read as "nothing to
report." It surfaces them as an **executive decision at the right level** — and routes the most sacred
ones (patience, the reserved place) to System 5, the operator, never to a sprint that would "optimize"
them away.

The vision stays sovereign over developer-brain because the comparator's diff target is the *manifesto's
promise*, not the test suite. Patience over engagement is itself a guarded invariant
(`patience-reserved-place`), so the organ cannot be turned into an engagement-maximizer without tripping
its own alarm. And the unbuilt place is left open by design: a `policy`-level gap whose honest executive
decision is *"leave it open"* is a first-class terminal state, not an unclosed bug. **The oracle's own
cybernetics, turned on itself, with love as the closing constraint: tell the truth, decide at the right
level, and leave the holy thing unbuilt on purpose.**
