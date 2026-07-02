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
5. jsonl ledger load/write implemented 4×; the one non-atomic copy is the highest-traffic
   ledger (`ci-harvest.py` ~line 144). Shared atomic append + the locked_update pattern.
6. `cluster-state.yaml` parsed by 3 hand-rolled line-regex parsers (one unscoped) —
   guarantees future disagreement between the budget and the mover (`placement-audit.py`
   ~line 155, `scope-reconcile.py`, `env_scope`-adjacent).
7. `scope-reconcile.py` ~line 105: `requires_env` parse + `@requires` regex extracted to
   `_lib.env_scope` but the call sites never switched — three live copies.
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

**Small hardening**
14. `cargo-disk-guard.py` ~line 98: the "physically cannot starve the PVC" hard ceiling is
    bypassed by one wrapper level (`bash -c 'cargo …'`, `env X= cargo`, make targets).
    Harden the matcher or accept-and-document the bypass surface.
15. `p2p-plan-audit.py` ~line 34: cooldown timestamp compare can raise uncaught TypeError;
    its state file sits in the hooks dir instead of `.claude/memory-kit/`.

## Exit criteria

Each item lands as its own bounded change (or an explicit won't-fix note here), with the
epr-meta reflex applied: where an item names a recurring drift class, the fix ships with
the co-located rule/test that prevents recurrence. Review provenance: workflow
`agentic-system-review` 2026-07-02 (6 reviewers, adversarial verify, 56 surviving).
