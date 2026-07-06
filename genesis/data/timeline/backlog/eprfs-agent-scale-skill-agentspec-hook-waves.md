---
id: "backlog-eprfs-agent-scale-skill-agentspec-hook-waves"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Scale eprfs capability projection beyond `agent` — skill, agent-spec (CLAUDE.md), hook waves"
slug: "eprfs-agent-scale-skill-agentspec-hook-waves"
written: "2026-07-06"
author: "eprfs-agent capability-projection V2 plan (Task 8 backlog capture)"
status: "backlog"
priority: "medium"
jobs: [elohim]
tags: [eprfs, eprfs-agent, projection, capability-projection, skill, agent-spec, hook, frontmatter-dialect]
cites:
  - genesis/docs/superpowers/plans/2026-07-06-eprfs-agent-capability-projection-v2-plan.md
  - elohim/eprfs/eprfs-agent/src/canonical.rs
  - elohim/eprfs/eprfs-agent/src/binding.rs
---

## What

V2 proved the author-once/project-many pattern for exactly one capability class: `agent`
(`.claude/agents/*.md`). The same pattern should scale to the other elohim-agent capability
classes already declared in the V1 vocabulary, in dependency order:

1. **`skill`** (`SKILL.md`) — 106 skills + 38 plugin-namespaced skills today. Frontmatter carries
   the dispatch trigger (`description:` IS the routing signal skill-audit watches), so the
   round-trip must be byte-faithful there above all else.
2. **`agent-spec`** (`CLAUDE.md`) — 140 across the repo. Distinctive: its "metadata" isn't a YAML
   frontmatter block at all — the scope + `appliesTo` framing lives in the prose header and the
   managed-surface cite discipline (`.claude/scripts/_lib/managed_surfaces.py`). Projecting this
   class means deciding what "canonical form" even means when the dialect has no frontmatter.
3. **`hook`** (56 today) — BINDS phases (PreToolUse/PostToolUse/SessionStart/…) + matchers in
   `settings.json`, not in the hook file itself. The projection source of truth may need to be the
   settings.json binding entry, with the hook script as a referenced blob rather than the
   projected artifact.

Each wave gets the same acceptance gate V2 proved for `agent`: parse → project → materialize →
verify-drift, exercised as a round-trip test against a real fixture (lossless migration) plus a
drift-detection assertion (live projection, not a one-shot migration).

## Why sequence this way

Dependency order, not arbitrary: `skill` is closest in shape to `agent` (frontmatter + markdown
body, same YAML dialect risk class — see the sibling backlog item
`eprfs-agent-real-corpus-parse-hardening.md`, which gates ALL of these waves on real-corpus
parseability, not just `agent`). `agent-spec` is a genuinely different dialect (no frontmatter) and
should come after the tooling has one clean win to generalize from. `hook` is bound externally
(`settings.json`), which is a new *binding* topology, not just a new dialect — hardest, comes last.

## Design direction (not built here)

Reuse `eprfs-agent`'s `ProjectionBinding`/`normalize` machinery as-is (`elohim/eprfs/eprfs-agent/src/binding.rs`)
— the render/normalize contract is capability-class-agnostic. Add a `FrontmatterDialect` trait (or
enum) per class so `CanonicalAgent`-shaped parsing (`elohim/eprfs/eprfs-agent/src/canonical.rs`)
generalizes to `CanonicalSkill`, `CanonicalAgentSpec`, `CanonicalHook` without forking the
projection/materialize/verify pipeline — those stay in `eprfs-core`/`eprfs-local` and are already
domain-neutral (V2's whole point). Each new class is its own crate or module sibling to
`eprfs-agent`, never a fork of it.

## Blocked on

Not blocked structurally, but sequenced AFTER `eprfs-agent-real-corpus-parse-hardening.md` closes —
scaling to more classes while the flagship class parses 0% of its live corpus would just multiply
the same defect surface.

## Provenance

Surfaced by `genesis/docs/superpowers/plans/2026-07-06-eprfs-agent-capability-projection-v2-plan.md`
Task 8 (scope fence: "V2 does the `agent` class only... `skill` → `agent-spec` → `hook` projection
are LATER waves, captured as backlog in Task 8, not built").
