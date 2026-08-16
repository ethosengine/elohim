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

20. **Developer CLI surface: 362 npm scripts across 25 packages + a drifted root justfile.**
    Census + consolidation direction in
    [dev-cli-hygiene-script-census](../backlog/2026-08-16-dev-cli-hygiene-script-census.md)
    (2026-08-16). Four surfaces carry 63% (seeder 69 · elohim-app 62 · root 54 · a2o 42);
    `build`×19 / `test`×16 / `lint`×10 name fragmentation; the existing root justfile
    (last touched 2026-06-23) has zero cargo-pool awareness so its native-cargo recipes are
    denied by the disk-guard or mint rogue legacy `target/`s. Direction: revive the root
    justfile with ~8 manifest-driven verbs (`gate` resolves `build-manifest.json
    gate.projects` — kills the pre-push two-detection-path drift class), bury RUSTFLAGS/pool
    gotchas in recipes, collapse the seeder's verb×corpus×env matrix to parameterized verbs,
    prune by census with a deprecation-echo cycle. First surgical pass ran 2026-08-16
    (broken `build:sophia` path fixed; obsolete `sonar:preview` removed; census doc carries
    the verified-alive list so the next pass doesn't re-derive it). Gospel drift found, not
    yet fixed: root CLAUDE.md documents `pnpm run cypress:run` in elohim-app — no such
    script exists (E2E moved to `genesis/a2o`; the live aliases are `e2e`/`e2e:browser`).

## Exit criteria

Each item lands as its own bounded change (or an explicit won't-fix note here), with the
epr-meta reflex applied: where an item names a recurring drift class, the fix ships with
the co-located rule/test that prevents recurrence. Review provenance: workflow
`agentic-system-review` 2026-07-02 (6 reviewers, adversarial verify, 56 surviving).
