---
id: agentic-context-tooling-consolidation-queue
kind: backlog
status: open
title: Agentic context-tooling consolidation queue — structural findings deferred from the 2026-07-02 system review
tags: [decision-record, agentic-tooling, hooks, memory-kit, consolidation, context-budget, epr-meta]
occurred_at: 2026-07-02
---

# Agentic context-tooling consolidation queue (2026-07-02 review residue)

A 6-dimension, adversarially-verified review of the `.claude/` context-management system
(hooks · `_lib` · memory-kit · skills · agents) produced 56 surviving findings. The bounded
correctness fixes landed same-day (dead lint/CI hooks, PreToolUse JSON emission, Edit-blind
gates, timeout-unit normalization, stale roots, budget alignment to the real ~24.4KB harness
cap, sentinel echo-guards, MEMORY.md became a generated projection). This entry canonicalizes
the **structural** residue so it drains deliberately instead of by-the-way.

**Weigh every engine-internal item against the brit/eprfs supersession** (see memory
`project_brit_next_gen_epr_meta_foundation` — canonical rule engine is migrating to brit;
an eprfs FUSE projection would replace the hook transport entirely). Transport-shape items
(1, 5, 10) survive that migration; engine-internal items (2, 3, 11–13) may be moot — check
before investing.

## The queue (severity-ordered within class)

**Hook-transport shape**
1. ~14 separate Python interpreter spawns per Edit/Write (PreToolUse + PostToolUse lists in
   settings.json) → one table-dispatcher process reading a hook registry. Biggest per-edit
   latency lever; also the natural seam for the eprfs port.
2. SessionStart headline is a 3-deep python-spawning-python chain (~7 interpreters);
   `load-project-context.py` shells `placement-audit.py --headline` which shells gate
   subprocesses (`cleanup-pressure.py --status`, …). Flatten to library calls.

**Shared-logic duplication (`_lib` exists; call sites never switched)**
3. Frontmatter parsed ~6 divergent ways; `placement-audit.py` alone reimplements it 5×
   despite `_lib.frontmatter`.
4. Repo-root resolved by 4 competing conventions (`parents[N]`, env, walk-up, hardcoded)
   — `cleanup-apply.py`'s `parents[2]` bug (fixed same-day) was this class biting.
   **PARTIAL 2026-08-12**: the three call sites that already imported `_lib` at module
   level (`decompose.py`, `placement-audit.py`, `scope-reconcile.py` — the last one ran
   its own fifth `next(p for p in … if (p / ".git").exists())` walk-up convention) now
   call `_lib.paths.repo_root_from_file`; ROOT-value parity verified before/after. Left
   unclaimed: `habits-status.py` and `saga-status.py` are deliberately import-light
   SessionStart surfaces (a `_lib` import there was previously ruled out on purpose —
   see `habits-status.py`'s own comment — and `saga-status.py` already has a safe
   `.git`+`genesis` walk-up, not the naked-`parents[N]` bug shape); ~9 more files under
   `memory-kit/`, `dev-dashboard/`, `converge/` (`context-ratchet.py`, `memory-review.py`,
   `spec-coherence-index.py`, `path-update-apply.py`, `skill-audit.py`,
   `state-machine-gen.py`, `sprint-distill.py`, `plan-status.py`, `converge-apply.py`,
   `converge-scan.py`) have no `_lib` bootstrap at all today — switching them means adding
   new sys.path machinery to each, a materially bigger and riskier change than a call-site
   swap; left as a bounded follow-up rather than done blind across 9 files in one pass.
5. jsonl ledger load/write implemented 4×; the one non-atomic copy is the highest-traffic
   ledger (`ci-harvest.py` ~line 144). Shared atomic append + the locked_update pattern.
6. `cluster-state.yaml` parsed by 3 hand-rolled line-regex parsers (one unscoped) —
   guarantees future disagreement between the budget and the mover (`placement-audit.py`
   ~line 155, `scope-reconcile.py`, `env_scope`-adjacent).
   **LANDED 2026-08-12**: `_lib/cluster_state.py` is now the one parser. Verified sites:
   `placement-audit.py load_cluster_state()` (~line 157 — this was the unscoped copy, no
   `resources:` block-boundary check at all), `scope-reconcile.py _parse_cluster()` /
   `_parse_provides()`, and `focus-baseline.py _cluster_detail()` (the `env_scope`-adjacent
   third copy this entry hedged on naming) all delegate to it now. No live disagreement was
   found against the current file (all three parsers agreed on the current 5-resource
   set); the unscoped-parser risk is pinned as a regression test
   (`_lib/__tests__/cluster_state_test.py`, asserting through the CALL SITES so re-inlining a
   private parser turns the suite red) rather than left latent. `_flip_resource`
   (the WRITER) stays its own line-based edit — mutation must preserve file formatting/
   comments and is a different concern from the three read-side parsers this item named.
   **Merging four regexes forces three semantic calls** — all three documented in
   `_lib/cluster_state.py`'s module docstring and pinned by test; the first pass got #1 and
   #3 wrong by adopting the narrowest merged behaviour, caught in adversarial review:
   (1) a column-0 `#` comment is NOT a block terminator (only `^[A-Za-z]` is). The naive merge
   took `_parse_provides`'s `^[A-Za-z#]`, so one ordinary section comment between two resources
   would drop every resource below it — and `--apply` then `git mv`s their specs/plans/features
   into `held/` and flips `deployments.json` `suspended`. cluster-state.yaml already carries five
   comment blocks.
   (2) a resource must DECLARE `available:` to be an availability CLAIM — `available_map()` omits
   a `role:`-only block, so a merely-planned capability never silently benches work. The format
   cannot distinguish "forgot to state availability" from "planned, never a runtime claim", so
   the conservative option wins. RESIDUAL, deliberate: `all_names()` (the mover's `known`, also
   `--set` validation + unknown-cap drift) still contains it, so a doc requiring a role-only
   resource is held by the mover while the budget reader counts it OPEN. Both HEAD readers behaved
   exactly this way; closing it means changing the MOVER's policy — a scope decision, not a parser
   one. **Open follow-up**: decide whether an undeclared resource should gate at all.
   (3) a duplicate resource key MERGES first-explicit-wins instead of RESETting (a copy-paste
   re-declaration used to wipe the first block's `available: true`).
7. `scope-reconcile.py` ~line 105: `requires_env` parse + `@requires` regex extracted to
   `_lib.env_scope` but the call sites never switched — three live copies.
   **LANDED 2026-08-12**: `scope-reconcile.py requires_env()` / `_feature_requires()` now
   call `_lib.env_scope.parse_requires_env` / `.requires_tags`; the third live copy
   (`focus-baseline.py`'s own `_REQUIRES_TAG` in `_scenario_caps()`) switched too. All 30
   assertions in `memory-kit/__tests__/scope_reconcile_test.py` (which exercises
   `requires_env`, `available_caps`, `_requires_split`, `_scope_verdict` directly) still
   pass unchanged.
8. Status vocabulary + ACTIVE homes duplicated hook↔audit with "must mirror" comments as
   the only sync mechanism (`placement-drift-signal.py` ~line 58 ↔ placement-audit).

**Engine internals (brit-supersession check first)**
9. `_lib/epr_meta.py`: each cascade manifest stat+read+YAML-parsed 4× per Write/Edit (add
   an mtime-keyed parse cache); coverage nudge repeats ~480B static instruction per
   unclaimed gap-root (cap/dedupe per session) and may fire inside skip_dirs subtrees the
   census never scans (verify).
10. `_lib/cite_graph.py`: `envelope_verdict` re-reads + re-hashes the target once per
    inbound cite — O(edges) I/O (memoize per-path fingerprints); `build_slug_index`
    silently last-wins on duplicate `id:` (warn — brit's SlugIndex carries the same
    first-vs-last ambiguity in its parity-hardening backlog; fix congruently).
11. `_lib/subject_routing.py` ~line 222: `load_routing` cascade machinery (~70 lines) has
    zero runtime consumers — documented-but-unwired; wire it or delete it.

**Unwired / dormant instruments**
12. `context-ratchet.py` is wired into nothing and its baseline froze 2026-06-02 — either
    a gate consumes it or it goes.
13. No aggregate byte budget across the SessionStart injectors (~87KB / ~22k tokens fixed
    session-open cost, no meter). Design item: a context-budget meter fed by each
    injector, surfaced as a headline token — the projector's budget discipline generalized
    to every always-on surface. `genesis/agentic/bin/pool-preflight.sh` (~2.9KB static
    reference text) is the first trim candidate once metered.

**Dynamics of the agentic layer** (from [Meadows](epr:meadows-systems-dynamics-cross-pollination-2026-08-11), mint pass 2026-08-11)

16. **`retire-when:` on compose-gate rules, hooks, and sentinels — the intervenor's removal
    condition.** Meadows' *shifting the burden to the intervenor* trap: *"If the intervention
    designed to correct the problem causes the self-maintaining capacity of the original system
    to atrophy or erode, then a destructive reinforcing feedback loop is set in motion… If you
    are the intervenor, work in such a way as to restore or enhance the system's own ability to
    solve its problems, **then remove yourself**"* (Thinking in Systems, ch.5 + appendix). Every
    hook, sentinel, auto-triage agent, and ledger in `.claude/` is an intervenor added because
    the system did not reliably do something itself, and the census this item minted below
    confirmed the exposure on disk: 101 of 101 carried no removal condition (verified
    2026-08-11, before this item's own delivery).

    **DELIVERED 2026-08-11** (commit `63e81325c`, `feat(epr-meta): retire-when — give every
    intervenor an exit`). The mechanism, not the proposal, landed: an optional `retire-when:`
    clause on both governance surfaces — `_lib/epr_meta.py` (`retire-when` joined
    `_KNOWN_RULE_KEYS`; `_validate_retire_when` + `_RETIRE_NEVER_RE` refuse the two contentless
    shapes, an empty condition and a bare `never` with no reason — `never` is admitted only as a
    *reasoned* constitutional floor, same discipline as `status: unwired` in habits.yaml) for
    `.epr-meta` manifest rules and registry policies, and a module-level `RETIRE_WHEN = "..."`
    convention on 8 hooks (`cargo-disk-guard.py`, `claude-md-drift-signal.py`,
    `deprecation-sentinel.py`, `epr-meta-resolver.py`, `jenkinsfile-method-size.py`,
    `map-drift-signal.py`, `memory-coherence-signal.py`, `placement-drift-signal.py`).
    `retire-when` is deliberately EXCLUDED from the policy content hash — `_lib/epr_meta.py`'s
    `_HASH_EXCLUDE_KEYS` and `eprfs-meta/src/canonical.rs`'s `HASH_EXCLUDE_KEYS` both read
    `{"contentHash", "status", "superseded_by", "retire-when"}`, a deliberate **two-implementation
    invariant** pinned by `intervenor_retire_when_test.py`: adding the key to the Python gate
    alone first silently un-enforced every backfilled policy on the Rust side (caught by a
    live-root test, not by inspection — the worst possible failure mode for a governance
    registry, since it routes live deny/ask rules to judgment). Backfilled onto all 12 active
    registry policies and three `.epr-meta` manifests (`genesis/docs/superpowers/.epr-meta`,
    `genesis/docs/superpowers/specs/.epr-meta`, `genesis/research/.epr-meta`). A new meter,
    `_lib/intervenor_census.py`, counts three declared populations (`.epr-meta` rules, registry
    policies, hooks) — INCLUDING ITSELF — and is surfaced by `placement-audit.py --epr-meta`.

    **Remainder (live, `placement-audit.py --epr-meta`, 2026-08-12): 58 of 99 intervenors still
    lack a removal condition** — rules 36/56, policies **0/12** (every registry policy now
    carries one), hooks 22/31 (8 of 31 do). 41 are declared, 9 of those reasoned `never`. The
    population still owed an exit is almost entirely manifest rules and hooks outside the eight
    backfilled — this stays a queue item, not closed, until `lacking` drains toward the census's
    own stated exit (0 for 8 consecutive weeks with `never_declared` not outgrowing `total` —
    `RETIRE_WHEN` in `intervenor_census.py`).
17. **Model carrying capacity is a vector, not a context window — delegation as limit-matching.**
    Meadows/Liebig: *"At any given time, the input that is most important to a system is the one
    that is most limiting"*; *"any physical entity with multiple inputs and outputs is surrounded
    by layers of limits."* Capacity dimensions (generalization, tool use, coding, cybersec,
    finance/accounting, long-form writing, instruction literalness, hallucination propensity)
    move independently, including *within* a family across versions. `feedback_delegate_narrow_tasks_to_cheaper_tiers`
    is a correct standing directive currently executed on **cost intuition** — a one-dimensional
    proxy for a multi-dimensional fit. The right question per delegation is *which dimension
    binds for this task, and which model has headroom there*: a long mechanical sweep binds on
    context and patience, an architecture judgment on generalization, a security review on
    neither. The agent roster encodes some of this implicitly in its Haiku/Sonnet/Opus
    assignments; none of it is written as a falsifiable limiting-factor claim. ⚠ The
    per-version variation is operator-observed, not measured — **measure the axes before
    building policy on them.**
18. **Cost per *verified* result, not cost per token — hallucination as a sink-side limit.**
    Every other capacity dimension is source-side (how much can this model draw on);
    hallucination is what a model *emits into shared context* that verification must then
    absorb. In Meadows' terms it is an emission with an absorption cost, and the absorption
    process is `verification-before-completion`. A tier that is cheap per token and expensive
    per verification can have a worse emission/absorption ratio than a costlier tier that emits
    less to check — the arithmetic behind `feedback_verify_the_measure_before_the_ranking`.
    Pairs with item 13's context-budget meter: same shape, different denominator.

**Small hardening**
14. `cargo-disk-guard.py` ~line 98: the "physically cannot starve the PVC" hard ceiling is
    bypassed by one wrapper level (`bash -c 'cargo …'`, `env X= cargo`, make targets).
    Harden the matcher or accept-and-document the bypass surface.
15. `p2p-plan-audit.py` ~line 34: cooldown timestamp compare can raise uncaught TypeError;
    its state file sits in the hooks dir instead of `.claude/memory-kit/`.
19. **`cite-gen --seal` silently no-ops on `.epr-meta` — and reports a green gate over it.**
    `.epr-meta` manifests accept a `cites:` field and several use it
    (`genesis/docs/superpowers/.epr-meta`; `elohim/epr/.epr-meta` and `elohim/epr-rea/.epr-meta`
    as of 2026-08-12). `cite-gen.py --seal` accepts the path, does **not** parse its cites, and
    prints `cites: 0 sealed … ✅ gate: all cites content-addressed + resolvable`. Observed
    directly: a hand-written `sha256:PENDING` placeholder survived `--seal` untouched **and the
    gate reported green.** The false green is the expensive half — silence invites a second look;
    reported success ends the inquiry.
    *Why it is more than tidiness:* the `.epr-meta` `cites:` field is the **only structured
    surface where code declares which documents it answers to.** Everything else runs one way —
    doc→doc and doc→code are sealed envelopes, while code→spec is a raw path string in a doc
    comment (**163 files**) and code→research is **absent entirely** (zero `.rs` files reference
    `genesis/research/`). The named thinkers appear inside `epr`/`epr-rea` only as prose — 14
    Meadows, 3 Liebig, 1 Beer — never as addressable references. So the crossing from the research
    corpus into the crate family has no verified reverse channel
    ([requisite-variety guidestar](epr:requisite-variety-guidestar-epr-family-composition) §1c/§1d).
    *The symptom already happened:* on 2026-08-12 a grep-driven audit concluded the protocol's
    limitarian position was "docs-only, zero code" and was **wrong** —
    `elohim-storage/src/services/measure.rs` implements it (`ge_alpha`, `gini`,
    `top_quantile_share`, `composite_concentration`) and contains no occurrence of the word
    "limitarian." A working reverse channel would have caught it.
    *Fix shape:* (a) parse `.epr-meta` frontmatter cites, reusing `_lib.epr_meta`'s existing
    reader rather than adding a second parser — the interface-first rule this tree enforces;
    (b) **fail loud on an unparseable/placeholder fingerprint instead of reporting green**;
    (c) decide and record the envelope form for a code-side cite (manifests currently use the
    bare-slug form; that may be correct for a manifest, but it should be a decision, not an
    oversight). Red-first: assert `--seal` on a fixture carrying a bad fingerprint FAILS.
    *Out of scope, noted:* the 163 raw `genesis/docs/...` path strings in `.rs` doc comments are
    the same failure one layer down — a path-cite that dies on a move. Whether Rust doc comments
    should carry sealed cites at all is a larger question; do not fold it in.

20. **Developer CLI surface: consolidation foundation landed; 362→322 scripts, eight public verbs.**
    Census + consolidation direction in
    [dev-cli-hygiene-script-census](../backlog/2026-08-16-dev-cli-hygiene-script-census.md)
    (2026-08-16). Four surfaces carry 63% (seeder 69 · elohim-app 62 · root 54 · a2o 42);
    `build`×19 / `test`×16 / `lint`×10 name fragmentation; the existing root justfile
    (last touched 2026-06-23) has zero cargo-pool awareness so its native-cargo recipes are
    denied by the disk-guard or mint rogue legacy `target/`s. Direction: revive the root
    Landed 2026-08-16: exactly eight public root verbs; 32 typed gate projects across 13
    manifests; one shared detector/executor for humans + pre-push; explicit cargo-pool
    workspace mapping; environment-suffixed seeder aliases collapsed into
    `seed action [profile] [scope] [limit]`; 40 scripts and two obsolete lifecycle scripts retired;
    fake content dry-run/validate modes removed; gospel/skills/READMEs synchronized and
    governed. The census doc carries the retained-script decisions and green evidence.

21. **The habits register is the last global singleton — composition, and the welded-shut pressure valve.**
    Surfaced 2026-08-27 while opening `genesis/a2o/features/stewardship/`: the lane had a real
    habit to declare and no slot to declare it in. **Three facts, all verified against the tree,
    not inferred:** (a) the caps are **prose-only** — `habits.yaml` says *"Max 12 habits"* and
    *"Max 2 active"*, and **no script enforces either**; (b) `habits-status.py:57` is
    `HABITS = _ROOT / "genesis" / "manifests" / "habits.yaml"` — one hardcoded path, and its only
    two `glob` calls are for *report/evidence* discovery, not manifest discovery, so there is no
    cascade to cap; (c) **zero habits are `unwired`** (8 green, 4 red, 2 active, 12 total) even
    though gospel calls that state *"declared and counted on purpose"*.
    (c) is the load-bearing one and it is independent of today's lane: with no free slots you
    never spend one on a commitment you cannot yet observe, so **the register's own valve for
    honest uncertainty is welded shut** — it has stopped being able to represent "we committed
    to this and cannot yet watch it."
    **The design conflates two different bounds.** `max 2 active` bounds ATTENTION (one operator,
    one day job) and should stay global — it survives composition as a roll-up assertion over a
    tree. `max 12` bounds DECLARATION — what exists — and 12-in-one-directory is a smell where
    60-across-8-lanes is not.
    **Why flat-12 was right when written** (stated in the register's own header): cohesion used to
    live in prose across 240+ specs/plans, and *"the selection problem, not the work, is what
    sessions fail at."* A flat 12 is maximally legible — you can hold it in your head.
    **What dissolves the objection to composing:** `.epr-meta` never asks anyone to read every
    manifest — the resolver walks only the path of the file being touched, so scope is DERIVED
    FROM WHERE YOU ARE. "The top red" becomes "the top red in scope", which avoids the
    meaningless cross-domain comparison flat-12 forces (*is `dataplane-convergence` more
    important than the stewardship appeal path?*). The precedent is already shipped here:
    `seam-registry.yaml` is per-crate with a documented birth rule plus census/cascade/matrix
    roll-up, and `.epr-meta` already has `covers: subtree` to terminate a walk.
    **Direction — OPERATOR-DECIDED 2026-08-27, still unimplemented (gospel-tier change):**
    *"everything should be coupled to the `.epr-meta` because it carries that context/scope
    that's authoritative."* So this is explicitly **NOT** "give habits their own cascade mirroring
    seam-registry" — that would mint a SECOND scope authority alongside `.epr-meta`, and two
    hand-written homes for one truth is a failure mode this repo has already paid for twice
    (`cluster-state.yaml` vs `ELOHIM_REMOTE_COMPUTE_STATUS`, and the `deployments.json`
    `suspended` flags that drifted until they were made **derived, never hand-written**). One
    scope authority; everything else is a projection of it.
    Concretely: a habit is declared in a **`habits:` block inside the `.epr-meta`** of the
    directory whose behaviour it describes, inheriting the resolver, the cascade, `covers:
    subtree` termination, `cites:`, and `retire-when:` for free — no new resolver, no new
    manifest kind, no second birth rule. `habits-status.py` stops reading a hardcoded path and
    resolves through the SAME `.epr-meta` walk the PreToolUse gate uses. `genesis/manifests/
    habits.yaml` becomes either the root `.epr-meta`'s own block (cross-cutting habits, which is
    where most of today's 12 legitimately belong) or a **generated roll-up projection** — the move
    `MEMORY.md` already made in this same review (see the intro), and for the same reason.
    Keep `max 2 active` global as a roll-up assertion over the resolved tree. Drop the headcount
    cap; the discipline that actually prevents accumulation is the one `.epr-meta` already
    enforces — every habit bound to a runnable check (already required) **and born with a
    `retire-when:`**, which is item 16's mechanism applied to habits. Headcount is the wrong
    instrument; **exit conditions are the right one.**
    Two things to get right at implementation: a habit's `checks:` string IS its `@concern:` tag
    IS the `check_id` in a sprint report (the single join across register/CI/Gherkin), so
    composition must not fragment that namespace — concern ids stay globally unique even when
    declaration is local. And the a2o `.epr-meta` `new-feature-subdir-needs-meta` rule already
    guarantees new subtrees are born with a manifest, so habit declaration would land in a place
    that provably exists rather than needing its own birth rule.
    **The honest caveat, and the cheap test:** one data point is not a trend. Leave the
    stewardship habit undeclared and see whether the next two or three lanes also hit the wall.
    If they do, the cap is binding and composition earns itself; if not, flat-12 really is more
    legible and this is over-engineering. What tips toward "binding" is the zero-`unwired` count,
    which is the register failing to represent honest uncertainty regardless of today's lane.
    **LANDED 2026-08-27** — the operator directed implementation rather than waiting on the cheap
    test, and the wall the test was measuring is gone, so the test is moot. What shipped:
    `<dir>/.epr-meta/<id>.habit.md` is the declaration (frontmatter) plus its evidence ledger
    (body), discovered by `.claude/scripts/_lib/epr_habits.py` — `in_scope()` walks UP for "the
    top red in scope", `census()` walks DOWN for the register. `habits-status.py` no longer reads
    a hardcoded path; `genesis/manifests/habits.yaml` is now GENERATED by
    `.claude/scripts/habits-project.py` (pre-push runs `--check`), so all twelve existing readers
    keep their path and shape. All 12 habits migrated to the crate/lane whose behaviour they
    describe — six to `elohim/elohim-storage`, which is a measurement rather than a placement
    accident. The round-trip was proven exact against HEAD (the sole diff being the uncommitted
    `DELTA 2026-08-27b` block already in the working tree), because the split is a text operation
    over source lines, never a YAML re-emit. Headcount cap dropped; `retire-when:` required and
    authored for all 12; `max 2 active`, globally-unique `@concern:` ownership, one-habit-per-id,
    and a stale covenant rank are now ENFORCED roll-ups (all four were prose no script read).
    Priority moved to `.epr-meta/habits-covenant.md` `order:` — declaration is local, but "the
    top red" is a cross-domain operator judgment, the same reason `max 2 active` stays global.
    Birth is gated by the `habit-declaration-at-birth@1` registry policy bound at the repo root
    (`deny`, ratified as operator-2026-08-27), proven firing and clearing against a synthesized
    write. 11 new assertions in `_lib/__tests__/epr_habits_test.py`, all green, plus the 1155
    package-projection checks after folding the gospel and skill edits back to their packages.
    **First fruit:** `custodial-authority-answerable` declared `unwired` in
    `elohim/holochain/dna/imagodei/.epr-meta` — the stewardship lane's habit, and the first
    `unwired` the register has ever carried. That count going 0 → 1 is the valve reopening.


## Exit criteria

Each item lands as its own bounded change (or an explicit won't-fix note here), with the
epr-meta reflex applied: where an item names a recurring drift class, the fix ships with
the co-located rule/test that prevents recurrence. Review provenance: workflow
`agentic-system-review` 2026-07-02 (6 reviewers, adversarial verify, 56 surviving).
