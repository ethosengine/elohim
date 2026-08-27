---
version: 1
updated: 2026-08-27
vision: >
  A p2p backplane for a socially resilient, reach-gated substrate — a human-scale,
  capture-resistant foundation for communication that can scale to global
  coordination. Every guarantee constitutive, none opt-in.
order:
  # The register's PRIORITY sequence — the one habit property that genuinely cannot
  # decentralize, because "the top red" is an operator judgment across domains, not a
  # property of any single lane. Declaration is local (each habit's own .epr-meta atom);
  # ranking is here. A declared habit missing from this list is not an error — it sorts
  # last until the operator ranks it, which is the admission decision made visible.
  - dataplane-convergence
  - blob-durability
  - notary-authority
  - doorway-failover
  - reach-enforced-everywhere
  - identity-cross-signed
  - operator-runtime-surface
  - dev-system-equilibrium
  - sync-scale-honesty
  - measure-honesty-local
  - conductor-capacity-represented
  - governance-plane-single-evaluator
---
The delivery habits: what this system RELIABLY DOES, each bound to a runnable check
that proves it. This file is the register's CROSS-CUTTING leg — the vision, the priority
`order:`, and the covenant below. Each habit itself is DECLARED where its concern lives:
`<dir>/.epr-meta/<id>.habit.md`, in the governance package of the directory whose
behaviour it describes. `genesis/manifests/habits.yaml` is the GENERATED projection of
that walk (`.claude/scripts/habits-project.py`), kept so every reader that knows it by
path still works; it is nobody's home.

THE NAME IS THE MANIFESTO'S OWN. genesis/docs/content/elohim-protocol/manifesto.md
quotes James Clear: "You don't rise to the level of your goals, you fall to the level
of your systems." This file is that level. It does not hold what we INTEND — intentions
live in specs and plans. It holds what the system is OBSERVED to do, and what it has
stopped doing. A habit is proven by repetition and evidence, never by declaration,
which is exactly the discipline this register enforces.

WHY THIS EXISTS (2026-07-02, operator directive): cohesion was stored in prose
(240+ specs/plans, memory files, backlogs) which requires a strong reader every
session — the selection problem, not the work, is what sessions fail at. This file
inverts control: these habits are the INTERFACES; specs/plans/memories are commentary
hanging off them; sessions are interchangeable implementations of "make the top red
green"; CI + the convergence loop is the composition root. Sibling of cluster-state.yaml
(declared substrate capability) — this file is declared delivery truth. Read by
.claude/scripts/habits-status.py, which emits the session-start headline.

NAMING HISTORY (2026-08-06): was `spine.yaml`/`nodes:`, briefly `charter.yaml`/`articles:`.
"Spine" implied vertebrae in SEQUENCE — the one property this register does not have
(the ordered structure is the resiliency-SAGA, chapters 01..11 with a frontier), and an
agent reading it conflated the two registers on exactly that axis. "Charter" collided
with a shipped wire field (`pub charter: String`, qahal collective charters, 28 Rust +
26 generated TS files) and imported a granted-from-above frame this project rejects.
"Habit" collides with nothing, needs no metaphor essay, and is already our own word.

A HABIT IS NOT A TEST SUITE. A suite is evidence; a habit is the practice the evidence
is FOR. Three things follow, and each is why this file exists:
  - checks: are heterogeneous — an a2o scenario AND a cargo test AND a live observation
    can jointly prove one habit. No suite spans those.
  - green suites systematically over-claim (#[ignore] is a CI no-op, host-green !=
    CI-green, `cargo check` compiles tests without running them). evidence: is where you
    ARGUE the habit holds; guard: is what would silently break it.
  - unwired has no analog in testing. A missing test is mere absence; an unwired habit is
    one we have committed to with NO way to observe whether we keep it — declared and
    counted. That is the most valuable state in this file.

THE COVENANT (anti-pile admission control):
  1. A habit is declared in the `.epr-meta` of the directory whose BEHAVIOUR it
     describes — one authority (`.epr-meta`), scope derived from placement, everything
     else a projection of it. There is NO headcount cap: it bounded declaration, and
     12-in-one-directory is a smell where 60-across-8-lanes is not. Every habit is born
     with `retire-when:` instead — an exit CONDITION, never a date. A headcount only ever
     refuses the next habit; an exit condition retires the ones that are done. (Amended
     2026-08-27. The cap's real cost was rule 2's valve: at 12/12 a slot was never spent
     on a commitment we could not yet observe, so `unwired` sat at zero for the life of
     the flat file while several lanes had exactly that to declare.)
  2. status: green | red | unwired.
     - red     = a runnable check EXISTS and fails -> this is schedulable work.
     - unwired = no runnable check yet -> NOT schedulable; its ONLY legal first move is
       writing the red (see first_move). Prose specs do not advance it.
  3. Max 2 habits flagged active: true (the WIP fence). Finishing beats starting. This
     bound stays GLOBAL and is the one thing composition does not distribute — it bounds
     ATTENTION (one operator, one day job), and attention does not compose. Enforced as a
     roll-up over the resolved tree since 2026-08-27; it was prose no script read before.
  4. Status flips require evidence (build #, live probe, test run) — never edit status
     from memory or intention.
  5. New specs/plans should cite the habit they serve; a plan that serves no habit is a
     candidate for held/, not for writing.
  6a. A habit's `checks:` string carries the `@concern:` tag that is the `check_id` in a
     sprint report — the single join across register, CI and Gherkin. Declaration is
     local; that namespace stays GLOBALLY unique, enforced by the census.
  6. Session contract: (a) move reds toward green with proof, (b) file new reds as
     runnable checks, (c) one-line delta here. Nothing else is a deliverable.

BEST-OBSERVED RATCHET (2026-08-11, convention — not yet a numbered covenant rule; promoting
it is the operator's call). An optional `best_observed:` field beside `evidence:` records the
STRONGEST run a habit has ever had, alongside the LAST one. Meadows' way out of the *Drift to
Low Performance* trap: "keep performance standards absolute — even better, let standards be
enhanced by the BEST actual performances instead of being discouraged by the worst." We have a
documented eroding-goal channel (PVC-deferral makes "green" mean "deferred, not passed"), and
last-observed evidence cannot tell the two apart. A high-water mark makes a later weaker green
read as visibly weaker rather than equivalent. First subject: measure-honesty-local.
