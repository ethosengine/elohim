---
title: "Managed-Surface Edit Discipline — One Registry, Pre-Edit Injection, No Per-Hook Scope Drift"
id: managed-surface-edit-discipline-design
status: Draft
created: 2026-06-05
tier: design-spec
topic: [managed-memory, hooks, registry, cite-graph, gospel, CLAUDE.md, edit-discipline, scope-drift, path-locator, refresh]
class: process-meta
process_subdomain: memory
cites:
  - semantic-computable-links-design | the envelope/graph discipline this registry gives an edit-time enforcement surface (its §9.1 records the path:/refresh/gospel deltas) | sha256:91622260b60e4f33 | path: genesis/docs/superpowers/specs/2026-06-02-semantic-computable-links-design.md
  - subject-routed-decomposition-design | the CLASS axis this SURFACE axis composes with — same homes, orthogonal question, cross-checked by test | sha256:0d910143a8498b64 | path: genesis/docs/superpowers/specs/2026-06-02-subject-routed-decomposition-design.md
  - .claude/scripts/_lib/managed_surfaces.py
  - .claude/hooks/managed-surface-context.py
  - .claude/hooks/cite-seal-signal.py
requires_env: []
---

# Managed-Surface Edit Discipline

## 1. Forensic — why this exists

2026-06-05: an agent asked to add concern-routing "cites-style pointers" to four pillar CLAUDE.mds
hand-wrote slug-only cites, needed two operator corrections before reaching the cite tooling, and then
had to extend the slug index live (gospel CLAUDE.mds had never been graph members). The operator's
expectation — "the tooling was already built and should have been used" — was right at the strategy
layer (the envelope discipline was complete since 06-03) and unsatisfiable at the implementation layer:

- **Per-hook scope hardcoding.** `cite-seal-signal` knew `(genesis/docs, .claude/memory)`; so did
  `cites-migrate`, `cite-propagate`, and the audit — each privately. When the graph grew, each scope
  copy silently stayed behind. Scope drift recurs per-hook unless ONE registry answers membership.
- **No pre-edit surface.** Every signal hook fired AFTER the mistake. Nothing told the agent, at
  Edit-time, "this file is cite-graph-managed; here is the exact tooling."

## 2. The two axes

- `.claude/subject-routing.yaml` — the CLASS axis: what kind of WORK routes to which home
  (FRONT-fire at brainstorm/plan, BACK-fire at decompose). Unchanged.
- `_lib/managed_surfaces.py` — the SURFACE axis (this spec): given a FILE, which managed-memory
  surface class is it, what discipline + tooling apply at edit time, and is it a cite-graph member.
  Patterns anchor to the same homes the class axis names; the test suite cross-checks the two axes
  so they cannot drift apart (`__tests__/managed_surfaces_test.py`).

## 3. The registry

Ordered, first-match-wins: routing-config · process-gospel · gospel (CLAUDE.md, graph member) ·
memory · spec · plan · architecture-seed · history · doc · story · timeline-entity · a2o-feature ·
skill · agent. Each entry: `match` rules, `in_cite_graph`, a 1-2 sentence `discipline`, exact
`tools`. `classify(path, repo_root)` and `in_cite_graph(path, repo_root)` are the only API.

## 4. The hooks (PRE + POST halves of "you don't have to remember")

- **PRE** — `managed-surface-context.py` (PreToolUse Edit|Write): classify the target; if managed,
  inject label + discipline + tooling as additionalContext BEFORE the edit. Once per (file, process
  tree); ~45ms cold; fail-open. This is the hook that prevents the §1 episode shape.
- **POST** — `cite-seal-signal.py`: unchanged debt logic, scope now `managed_surfaces.in_cite_graph`
  (doc-roots + gospels) instead of a private DOC_ROOTS copy.

## 5. Envelope deltas (recorded in semantic-links spec §9.1)

`path:` materialized-locator field (tool-managed cache; stamped at mint, refreshed every propagate
pass; agents follow cites with a plain Read) and `cite-gen --refresh` (the deliberate stale-dequeue
after re-verification — `--into` never auto-blesses fingerprint drift).

## 6. Non-goals + recorded decisions

- `.claude/` dot-dir CLAUDE.mds stay OUT of the cite graph (process-gospel class, no envelopes) —
  the gospel walk prunes dot-dirs; revisit only if concern-routing demand appears there.
- The other ten PostToolUse signal hooks keep their own (orthogonal, non-cite) scopes for now;
  migrate them onto the registry opportunistically when one of their scopes next drifts.
- No DENY behavior: discipline injection and debt nudges only — mutations stay operator-gated.

## 7. Decomposition (gap-items)

- [x] `_lib/managed_surfaces.py` registry + classify/in_cite_graph (52-assertion suite incl. axis cross-check).
- [x] PreToolUse `managed-surface-context.py` + settings.json wiring (live-fired in-session on spec + skill edits).
- [x] `cite-seal-signal.py` scope → registry.
- [x] `path:` segment + `materialize_paths` in cite_graph (38-assertion suite) wired through emit/--into/--seal,
      cite-propagate (corpus pass applied: 67 locators stamped), cites-migrate.
- [x] `--seal-all` + cites-migrate + cite-propagate corpora include cites-bearing gospel CLAUDE.mds.
- [x] `cite-gen --refresh` (deliberate stale-dequeue) + gospel cites re-blessed post-verification.
- [x] Doc currency: semantic-links SKILL.md (membership + path + refresh + preHook), spec §9.1 amendment.
- [ ] Migrate the remaining signal hooks' scopes onto the registry as they next change (opportunistic).
- [ ] skill-audit/agent-audit awareness: surface registry drift (a new managed home with no registry entry).

## 8. Verification

`python3 .claude/scripts/_lib/__tests__/managed_surfaces_test.py` (52 ✅) ·
`python3 .claude/scripts/_lib/__tests__/cite_graph_test.py` (38 ✅) · both hooks exercised via synthetic
stdin (gospel/spec/unmanaged/vendored, flag suppression, debt nudge) AND live-fired by the harness
in-session · `cite-gen --verify` green on all four pillar gospels · propagate dry-run at 0 after apply.
