---
id: "backlog-eprfs-agent-real-corpus-parse-hardening"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "eprfs-agent parses 0 of 23 live .claude/agents files — harden CanonicalAgent::parse for real frontmatter (gates real-corpus onboarding)"
slug: "eprfs-agent-real-corpus-parse-hardening"
written: "2026-07-06"
author: "eprfs-agent capability-projection V2 plan (Task 7 dry-run discovery, Task 8 backlog capture)"
status: "backlog"
priority: "high"
ci_status: blocked
jobs: [elohim]
tags: [eprfs, eprfs-agent, yaml, frontmatter, parse, round-trip, real-corpus, style-preserving]
cites:
  - genesis/docs/superpowers/plans/2026-07-06-eprfs-agent-capability-projection-v2-plan.md
  - elohim/eprfs/eprfs-agent/src/canonical.rs
  - elohim/eprfs/eprfs-agent/examples/project_agents.rs
  - elohim/eprfs/eprfs-agent/tests/fixtures/code-reviewer.md
  - .claude/agents/
---

## What

The Task 7 dry-run (`cargo run -p eprfs-agent --example project_agents -- <repo-root>`) run over
the live `.claude/agents/*.md` tree — 23 files — parsed **0 of 23**. Every single file fails
`CanonicalAgent::parse` with `frontmatter is not valid YAML: mapping values are not allowed in this
context at line 2 column <N>` (N varies 263–937 per file — it fails partway through the
`description:` value, not at a fixed offset).

## Root cause (confirmed, not a parser bug)

Every current `.claude/agents/*.md` file's `description:` frontmatter value is authored as one long
**unquoted plain YAML scalar** that embeds an `Examples: <example>Context: … user: '…'
assistant: '…' <commentary>…</commentary></example>` prose convention — i.e. the value itself
contains multiple `word: ` (colon-space) sequences (`Examples:`, `Context:`, `user:`, `assistant:`).
In YAML block context, an unquoted plain scalar containing a bare `: ` is read by any strict parser
(`serde_yaml` included) as the start of an illegal same-line mapping key — hence "mapping values are
not allowed in this context." This is a real property of the live corpus, not a defect in the
example or in `verify_projection`/`has_drift` (neither panicked; every failure was caught and
reported via the `skip` path exactly as `project_agents.rs` was designed to do).

The round-trip acceptance fixture (`eprfs-agent/tests/fixtures/code-reviewer.md`) deliberately uses
a short, colon-free `description:`, so Tasks 1–6's acceptance gate never exercised this input shape.

## Impact

The V2 substrate + round-trip acceptance are proven correct on a hermetic FIXTURE, but the REAL
`.claude/agents` corpus is **0% onboardable today**. "Author-once, project-many over the live tree"
— the whole point of the projection compiler — is blocked until `CanonicalAgent::parse` tolerates
the real frontmatter authoring convention. This item GATES the follow-up the plan's T6 rationale
explicitly deferred ("a follow-up wave (Task 8 backlog) runs it over the live tree") — that
follow-up cannot proceed until this closes. It also gates all downstream scale waves in
`eprfs-agent-scale-skill-agentspec-hook-waves.md` (skill/agent-spec/hook frontmatter is authored by
the same humans/conventions and likely shares this risk class).

## Design options

- **(a) RECOMMENDED** — parse reserved free-text scalar fields (`description`, and any other field
  known to carry unconstrained prose) line-wise: take the raw value verbatim to end-of-line (or to
  the next top-level key at column 0) instead of handing that span to a YAML value parser, then run
  `serde_yaml` only over the remaining structured/`extra` frontmatter. Render already emits
  `description: {value}` verbatim, so the round-trip contract still holds — this is purely a parse
  side change.
- **(b)** Pre-quote/escape scalar values (wrap in a block scalar or quoted string) before handing
  the frontmatter block to `serde_yaml`. Simpler but riskier: requires correctly detecting which
  raw spans need quoting without a real YAML parse pass first (chicken-and-egg).
  - **(c)** Adopt a more lenient/forgiving YAML parser library. Broadest fix but adds a new
  dependency and changes error-reporting characteristics; likely defers correctly to whichever class
  the scale waves land in.

## Fold-in: style-preserving round-trip for non-scalar `extra` frontmatter

The T3 code review separately noted: render preserves frontmatter **data** best-effort for
non-scalar `extra` fields today (fix landed at `831bdb06b`), but authored **style** (block vs flow
YAML, key ordering nuances beyond what's locked) may still drift byte-for-byte from the original
author's formatting. This is a smaller, related "real-corpus fidelity" gap — fold it into the same
hardening wave rather than opening a separate item, since both are instances of "the parser/renderer
pair needs to get closer to byte-perfect against real, human-authored frontmatter," not just against
the hermetic fixture.

## Blocked on

Nothing structurally — this is a parser change inside `eprfs-agent::canonical`, no new crate or
cross-crate dependency required for option (a). Recommend picking this up before or alongside the
first scale wave, since it changes the acceptance bar every subsequent capability class will be held
to.

## Provenance

Surfaced by Task 7's dry-run of
`genesis/docs/superpowers/plans/2026-07-06-eprfs-agent-capability-projection-v2-plan.md` over the
live `.claude/agents/` tree (23 files, 0 parsed); captured as backlog per Task 8's mandate to keep
V2's scope genuine while not losing the discovery.
