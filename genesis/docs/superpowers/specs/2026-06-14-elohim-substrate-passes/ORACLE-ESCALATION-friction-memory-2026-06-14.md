---
title: "ORACLE-ESCALATION — The Friction-Signal Memory (HOW the signals that trigger escalation are built)"
date: 2026-06-14
status: PROPOSAL FOR OPERATOR BLESSING — working draft, NOT cite-sealed, NOT a decision, NOT code
component_of: the friction-escalation organ (the algedonic System-4 trigger that REPLACES the per-sprint vision-hat ritual)
supersedes_in_oracle: ORACLE-injection-2026-06-14.md step-0.6 always-on ritual (the heavy per-sprint tax)
keeps_from_oracle:
  - ORACLE-stack-2026-06-14.md         # the cite-sealed ladder (enter at vision, descend per rung) — KEEP
  - ORACLE-feedback-loop-2026-06-14.md # vision-comparator.py + vision-gaps.jsonl — KEEP (the RUNTIME-observed sensor arm)
binds_organs:
  - .claude/scripts/_lib/runtime_harvest.py   # the PURE evaluate/reconcile/fingerprint core to mirror verbatim
  - .claude/scripts/runtime-harvest.py        # the I/O-shell + --hook headline shape to mirror
  - .claude/scripts/ci-harvest.py             # cursor/occurrence/degrade-quiet sibling harvester
  - .claude/data/{ci-findings,runtime-findings}.jsonl  # sibling ledgers; friction.jsonl joins them
  - .claude/data/dev-intent.jsonl             # the in-flow exploration-capture ledger (the cheapest existing emit)
  - .claude/skills/agentic-developer/SKILL.md # /shift: the in-flow emit-point (bail-grammar §39-55, wishlist/blocker §99-101/§703-716, leave-no-orphan §126-144)
  - genesis/docs/shifts/SPRINT-RESULT-TEMPLATE.md  # the close-of-shift roll-up the harvester reads
  - .claude/agents/cartographer.md            # the drain (vision hat), NOT a fixing agent
reuses_pattern: findings-sentinel-pattern-design (flag → agent → canon → stasis) — feedback_deterministic_flag_agent_canon_stasis_pattern
do_not_cite_seal: true
---

# The Friction-Signal Memory

> *"When to escalate, and how to build up the memory signals that trigger this meta-process, is the
> tricky part."* — the operator

This component owns the **HOW half** of that tricky part: the memory that *accumulates friction* so the
escalation organ knows, deterministically, when a wall has stopped being weeds-work and become a pattern
that needs the architectural rung redrawn. It is the **algedonic afferent nerve** — the pain channel that
runs quiet while System 1 (the implementer in the weeds) executes free, and fires upward to System 4 (the
meta-process) *only when something hurts the same way K times, or hurts all at once at one seam.*

---

## 0. Reconciliation with the completed ORACLE (what we keep, what we supersede)

The just-completed oracle built two things that survive this correction **unchanged**:

- **KEEP — the cite-sealed STACK/ladder** (`ORACLE-stack-2026-06-14.md`): enter at vision, descend per
  rung. The friction organ *escalates TO* this ladder — friction is keyed by the **rung it implicates**,
  so a friction cluster points the meta-process at exactly which rung to open. The ladder is the
  GROUND step's target; friction is the trigger that sends you there.
- **KEEP — the vision-comparator** (`vision-comparator.py` + `vision-gaps.jsonl`, the 4th
  findings-sentinel): it is the **RUNTIME-OBSERVED sensor arm** of the same organ. It watches the
  *running substrate* and asks "did the system break a vision promise?" This component is its
  **DEVELOPMENT-OBSERVED twin**: it watches the *act of construction* and asks "did the developer keep
  hitting the same wall?" Two afferent nerves, one ledger family, one drain (the cartographer).

We **supersede** exactly one thing: `ORACLE-injection-2026-06-14.md`'s **always-on step-0.6 vision-hat
ritual** — the heavy per-sprint tax the operator rejected. The ladder and the journal Vision-Hat block
do not vanish; they become **the artifact the meta-process produces when escalation fires**, not a gate
every sprint must pass. Most sprints never run 0.6. The friction memory is what *decides* when a sprint
should.

The dividing line, stated once: **the ladder + comparator are the standing organs; the friction memory
is the algedonic trigger that decides when to consult them.** Beer's System 1 runs free; System 4 is
triggered, never scheduled.

---

## 1. What a FRICTION SIGNAL is (the definition the whole organ keys on)

A friction signal is a single in-flow observation that **the substrate resisted the work** — the work got
done (or got blocked) but only by paying a tax the design should not have charged. Six concrete shapes,
all already emitted in prose somewhere in the `/shift` loop today:

| friction kind | what it looks like in the weeds | where it already surfaces in `/shift` today |
|---|---|---|
| `bottleneck` | a performance wall — the operation is correct but too slow to use | journaled measurement line |
| `blocker` | the work cannot proceed without a decision/capability above the rung | bail-grammar (`SKILL.md` §39–55): a blocker bail must "show the palette-conformant workarounds attempted" |
| `workaround` | it got done, but by fighting the substrate — a hack that the design should have made unnecessary | journaled "this is not a workaround… " notes; wishlist redirects |
| `substrate-fight` | the natural way is forbidden/missing; the code shape is contorted to satisfy a seam | the "Observed anti-patterns" the journal accumulates (`SKILL.md` close) |
| `test-vs-design` | a test cannot pass without violating the design (the judge and the vision disagree) | bail-with-proposal (judge is off-limits, `SKILL.md` §102–110) |
| `rung-wrong` | the rung is wrong or missing — the spec/path/policy the implementer reads does not match reality | the leave-no-orphan escalation surface (`SKILL.md` §126–144) |

The key recognition: **every one of these is ALREADY being written down in prose** — in the journal, in
the bail, in the wishlist, in the sprint-result's "Observed anti-patterns." The implementer already pays
the cost of *noticing* the friction; today that notice dies in a gitignored journal. This component does
exactly one new thing: **it captures the same notice as one cheap structured line, keyed so a pattern can
be recognized across sprints.** It adds zero new noticing burden. It harvests a notice already made.

---

## 2. The friction ledger — `.claude/data/friction.jsonl`

Sibling to `ci-findings.jsonl` / `runtime-findings.jsonl` / `vision-gaps.jsonl`. One line per LIVE
friction pattern (not per instance — the fingerprint folds instances together). Schema mirrors
`runtime_harvest.py:166-170` so the cartographer reads a familiar shape, plus the three fields that make
it a *friction* signal (the PATTERN key, the RUNG key, the attempt-count):

```jsonc
{
  "ts": "2026-06-14T...Z",
  "fp": "a17c4e9b22d0",          // fingerprint(rung + kind + normalized(pattern)) — the PATTERN, not the instance
  "class": "friction",           // sibling to "ci-failure" / "self-heal-exhaustion" / "vision-gap"
  "kind": "substrate-fight",     // one of the six §1 kinds
  "rung": "ARCHITECTURE",        // the RUNG it implicates — the seam-clustering key (vision|architecture|composition|primitives|runtime|delivery)
  "rung_cite": "graph_engine.rs ContentGraphResolver seam",  // best-effort pointer to the implicated rung doc/seam (cite-gen later)
  "pattern": "content-graph walk needs depth>1 but resolver trait has no batched-descent method",  // the normalized wall, not the instance
  "shift": "frontend-eyes-sprint",   // provenance: which shift hit it (NOT scored — see §6 love test)
  "attempts": 1,                 // how many DISTINCT shifts re-hit this fingerprint (recurrence = K)
  "status": "open",              // open → escalated → (closed by disappearance OR by rung-edit)
  "first_seen": "2026-06-14",
  "last_seen": "2026-06-14",
  "clean_streak": 0,             // sweeps since last re-hit; at CLOSE_STREAK the line is DELETED (decomposed)
  "decision_ref": null           // set on drain: the rung commit / backlog entry that resolved the pattern
}
```

### 2.1 The two keys that make it a pattern-memory, not an instance-log

- **`fp = fingerprint(rung + kind + normalize(pattern))`** — the **PATTERN fingerprint**. `normalize()` is
  copied verbatim from `runtime_harvest.py:25-34` (strip ANSI, collapse whitespace, mask counts/durations/
  timestamps/IPs) so "the 3rd time" and "the 9th time" hit the *same* fp. **Recurrence = the same fp
  re-fired across distinct shifts** → `attempts` increments. This is the self-healing loop's
  *bounded-recovery EXHAUSTION* shape, one rung up: the normal fix was tried, the friction keeps
  returning, the recovery budget is spent → elevate.
- **`rung`** — the **RUNG key**. Every friction line is filed under the abstraction rung it implicates
  (the same seven-rung vocabulary the ladder uses). **Seam-clustering = N DISTINCT fingerprints all keyed
  to the same rung** → the friction is not one bad wall, it's a rung that is structurally wrong. This is
  the operator's *"elevate the PATTERN of the problem, not the instance, to the architectural/
  compositional design layer"* made mechanical: the rung-key IS the design layer the pattern belongs to.

These two keys give the organ its **two doors of automatic escalation** (the AUTOMATIC half of the
operator's two-door design):

1. **Recurrence threshold** — `attempts ≥ K_RECUR` on one fingerprint (the same wall, K shifts). Default
   `K_RECUR = 3`, mirroring `runtime_harvest.py`'s `OPEN_POLLS = 3` (three consecutive = a pattern, not a
   blip). *Same wall hit 3 times = bounded-recovery exhausted.*
2. **Seam-clustering threshold** — `N_distinct_fp(rung) ≥ K_CLUSTER` open lines on one rung. Default
   `K_CLUSTER = 3`. *Friction clustering at one seam = that rung needs redrawing, even if no single wall
   recurred.*

When either threshold crosses, the harvester flips the implicated lines to `status: escalated` and emits
the algedonic line (§5) that calls the meta-process. **One blocker is weeds-work and stays `open`,
silent. The pattern is the signal.**

---

## 3. In-flow capture — how `/shift` emits a friction line at near-zero cost

The cardinal constraint (§6): capturing friction must be **near-zero-cost** so it never interrupts the
weeds-work it records. We achieve that by **piggybacking the three prose moments the implementer already
makes** — never adding a new "stop and classify your pain" step.

### 3.1 Three emit-points, all already firing prose

| emit-point in `/shift` (today) | what the implementer already writes | the one structured line we harvest |
|---|---|---|
| **bail / blocker** (`SKILL.md` §39–55) — a blocker bail must already enumerate the workarounds attempted | the bail prose + workarounds-attempted list | `{kind:"blocker", pattern:<the wall>, rung:<implicated>}` |
| **wishlist / workaround** (`SKILL.md` §99–101, §703–716) — redirect/wishlist/blocker curation | the wishlist entry | `{kind:"workaround"|"substrate-fight", pattern, rung}` |
| **sprint-result close** (`SPRINT-RESULT-TEMPLATE.md` "Observed anti-patterns") | the distilled anti-patterns the journal accumulated | one friction line per distinct anti-pattern |

The implementer does NOT hand-write JSON. The emit is one of two cheap mechanisms, in order of preference:

- **Preferred — harvest at close (deterministic, zero in-flow cost).** `friction-harvest.py` reads the
  **sprint-result** (already written at close, `SKILL.md` step 728-730) + the journal's "Observed
  anti-patterns" / "Permission wishlist" / bail blocks, and the **`dev-intent.jsonl`** exploration
  captures, and emits friction lines by parsing those existing structured-prose sections. The implementer
  writes the sprint-result they already write; the harvester does the JSON. **This is the true smallest
  version (§4): no change to in-flow behavior at all — one new harvester reading artifacts that already
  exist.**
- **Optional in-flow — a one-liner append.** For a friction worth flagging *mid-shift* (a recurring wall
  the implementer wants the organ to start counting now, before close), a single command:
  `friction.py emit --kind substrate-fight --rung architecture "<pattern>"` — one line, no interview,
  returns instantly, append-only. Modeled on `dev-intent.jsonl`'s append-a-line ergonomics. This is the
  in-flow door for when the implementer's judgment says "this one matters" before the close-harvest would
  catch it.

### 3.2 Runtime/CI friction keys into the SAME rung-indexed ledger

The existing harvesters already detect substrate-level friction; they just file it at CI/runtime altitude.
A thin **rung-classifier** lets them also contribute to `friction.jsonl`:

- `ci-harvest.py` — a CI failure that is `blocked` with the same fingerprint across `seen ≥ K` builds
  (e.g. the `ci-alpha-cluster-degraded-substrate` cluster already in `ci-findings.jsonl`) is a friction
  pattern: emit `{kind:"blocker", rung:<classified>, pattern:<normalized line>}`.
- `runtime-harvest.py` — a self-heal-exhaustion that recurs (`clean_poll_streak` never reaching
  `CLOSE_STREAK`, re-firing across cycles) is runtime bounded-recovery exhaustion → `{kind:"bottleneck"|
  "substrate-fight", rung:"runtime"}`.

The rung-classifier is a small deterministic map (path/job/endpoint → rung), the friction-side sibling of
the injection component's `rung-map.yaml`. **One rung vocabulary, three feeders (shift / ci / runtime),
one ledger.** Seam-clustering then naturally fuses dev-observed and machine-observed friction on the same
rung — the strongest possible escalation signal is "the developer fights this seam AND CI keeps failing on
it AND runtime keeps exhausting on it," and that fuses automatically because all three key to one rung.

---

## 4. Smallest real first implementation

**The friction ledger + the close-harvest emit + the rung-key — proving recurrence and seam-clustering on
ONE feeder, this week.**

1. **`.claude/data/friction.jsonl`** — the new ledger (the only new persisted state). Empty to start.
2. **`.claude/scripts/_lib/friction.py`** (new pure core, ~80 lines). Copy `runtime_harvest.py`'s
   `normalize` / `fingerprint` / `reconcile` **verbatim**; change `fingerprint` to key on
   `(rung, kind, normalize(pattern))`; add the two threshold predicates `_recurrence(entries)` (any
   `attempts ≥ K_RECUR`) and `_seam_cluster(entries)` (any rung with `≥ K_CLUSTER` distinct open fps).
   `reconcile` gains one rule beyond the runtime core: a re-hit fingerprint from a **distinct shift**
   increments `attempts` (recurrence), not just `seen`.
3. **`.claude/scripts/friction-harvest.py`** (new I/O shell, ~70 lines). Mirror `runtime-harvest.py`'s
   shell: at SessionStart/close, read the latest sprint-result + journal "Observed anti-patterns"/wishlist
   sections (one feeder — the `/shift` close), parse to friction candidates, `flock`+reconcile to
   `friction.jsonl`, `--hook` mode emits the `friction:` headline line (§5), fail-safe exit-0.
4. **The `--hook` line** — `friction: quiet ✅` when no threshold crossed; the algedonic line (§5) when
   one is. Wired next to the runtime/vision hook calls in `load-project-context.py`.
5. **No new agent, no actuation.** Escalated lines drain to the **cartographer** (the same vision hat the
   vision-comparator drains to) — the GROUND→DECIDE→UPDATE→HAND-BACK meta-process is owned by the
   *sibling* escalation-organ components, not this one. This component's whole job is to make the
   *trigger* fire correctly and cheaply.

### What we deliberately do NOT build first
- **No in-flow `friction.py emit` yet** — the close-harvest proves the loop with zero in-flow cost; the
  mid-shift one-liner is a later sharpening (§3.1 optional door).
- **No ci/runtime feeders yet** — prove recurrence + seam-clustering on the `/shift` feeder first, then
  the ci/runtime rung-classifier *composes as a new feeder, never a new ledger* (exactly as the
  comparator widens by adding registry entries, never machines).
- **No auto-advance past `escalated`** — the harvester detects the pattern and flips `open → escalated`;
  only the meta-process moves a line to `decided`. Detection is deterministic; the design decision is the
  vision hat's. That separation IS the System-1/System-4 boundary, the same one the comparator holds.

This is a few hundred lines, all copy-shaped from the proven runtime arm, one new ledger, one headline
word, one harvester reading artifacts that already exist. It proves the entire **HOW** half — friction
noticed in the weeds → folded by pattern → counted by rung → threshold crossed → algedonic line fires —
on the one feeder that already writes the prose.

---

## 5. The algedonic line (the trigger, and its stasis)

The headline line is the afferent nerve reaching System 4. Three states:

```
friction: quiet ✅                                              # no threshold crossed — say nothing, stay in the weeds
friction: ⚠ recurrence (substrate-fight @ ARCHITECTURE ×3) → escalate   # same wall K times
friction: ⚠ seam-cluster (ARCHITECTURE: 4 distinct) → escalate          # friction clustering at one rung
```

On `⚠`, the implementer (or operator) knows the pattern has earned the meta-process: **GROUND** (open the
implicated rung on the ladder — the §0-KEEP stack — surface trajectory + precedent for THIS pattern, not
read everything) → **DECIDE** at the right level → **UPDATE** the rung-below specs/paths/policy (cite-
sealed) → **HAND BACK** (implementer resumes in the weeds). That meta-process is the sibling components'
scope; this component guarantees it fires *only when earned*.

**The MANUAL door supplies the same signal by judgment.** When the operator says "go read the docs, get
the vision and trajectory," that IS the algedonic line, hand-asserted, skipping the threshold. Mechanism:
`friction.py escalate --rung <r> --reason "<operator phrase>"` writes one `status: escalated` line with
`kind: "manual"` and `attempts: K_RECUR` (pre-satisfied). Same ledger, same drain, same meta-process — the
threshold is just supplied by judgment instead of accumulation. **Both doors fire the one organ.**

**Stasis — no re-fire (the sentinel pattern's defining property).** Inherited verbatim from
`feedback_deterministic_flag_agent_canon_stasis_pattern`:
- An `escalated` line never re-fires the algedonic warning — it surfaces its current `status`/`decision_ref`
  once per session, then stays quiet (presence-suppresses-dispatch).
- When the meta-process **UPDATES a rung**, the friction is `decided`; `decision_ref` records the rung edit.
  The next sweep sees the pattern is gone (the substrate stopped resisting) and **DELETES the line at
  `CLOSE_STREAK` clean sweeps** (close-by-decomposition — the rung commit is the record). Reintroduction
  reads as a fresh fingerprint → re-fires = regression handling for free.
- A pattern **blocked on an operator value-call** is `escalated` + `decision_ref: blocked-operator-call`:
  surfaced once, parked, never nagged. Patience over engagement, in the friction memory itself.

This is the operator's *"friction entry goes to stasis; no re-fire"* requirement, satisfied by reusing the
exact mechanism the deprecation/ci/runtime sentinels already proved.

---

## 6. What love requires (the closing test)

**Capturing friction must be near-zero-cost, so it never interrupts the weeds-work it records — and it must
measure the SUBSTRATE's resistance, never the developer's velocity.**

Two refusals, both load-bearing:

- **The cost refusal.** If logging friction took a stop-and-classify step, it would become the per-sprint
  tax the operator just rejected, wearing a new hat. So the smallest version adds **zero** in-flow steps:
  the implementer writes the sprint-result they already write, and the harvester does the rest. The
  implementer stays free in the weeds; the memory is built *from the prose they already leave behind.* The
  organ serves the work; it never nags it. A friction line that interrupted the work it records would be
  self-defeating — the substrate fighting the developer about logging the substrate fighting the developer.
- **The no-`govern(person)` refusal.** The `shift` field is *provenance, never a score.* The fingerprint
  keys on `(rung, kind, pattern)` — **never on who hit the wall.** There is no `friction-per-developer`
  metric, no velocity quota, no leaderboard. The memory flags *"the ARCHITECTURE rung resisted four times,"*
  never *"this developer struggles." Developer-brain is what we take OFF, not what we score* — the same
  refusal the comparator holds for the running system, held here for the act of construction.

The vision stays sovereign because the friction memory's whole purpose is to **know when the weeds have
become a wall the vision must redraw** — and to stay silent every other time. On most sprints it prints
`friction: quiet ✅` and the implementer never thinks about it. On the sprint where the same seam has been
fought three times, it puts the vision hat in reach *because the pattern earned it* — not because a ritual
scheduled it. The meta-process is escalated TO by the pain it accumulates, exactly as Beer's algedonic
signal bypasses the hierarchy and fires only when something hurts. **The implementer executes free; the
memory feels the friction; and when the friction becomes a pattern, the vision gets the call — once, at
the right rung, and then quiet again.**

---

*All emit-points, ledger writes, and headline wiring named here are operator-GATED. This is a proposal
for the friction-signal-memory component of the escalation organ — for operator blessing, NOT yet
cite-sealed, NOT a decision, NOT code. It KEEPS the oracle's ladder + comparator and SUPERSEDES the
always-on step-0.6 ritual.*
