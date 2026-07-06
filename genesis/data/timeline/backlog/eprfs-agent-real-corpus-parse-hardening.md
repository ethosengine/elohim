---
id: "backlog-eprfs-agent-real-corpus-parse-hardening"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "eprfs-agent real-corpus parse-hardening — LANDED (23/23 parse); residual: style-preserving byte-perfect round-trip for nested mcpServers frontmatter"
slug: "eprfs-agent-real-corpus-parse-hardening"
written: "2026-07-06"
author: "eprfs-agent capability-projection V2 plan (Task 7 dry-run discovery, Task 8 backlog capture); parse-hardening landed 2026-07-06 on integ/eprfs-agent"
status: "partially-resolved"
priority: "medium"
ci_status: in-progress
jobs: [elohim]
tags: [eprfs, eprfs-agent, yaml, frontmatter, parse, round-trip, real-corpus, style-preserving]
cites:
  - genesis/docs/superpowers/plans/2026-07-06-eprfs-agent-capability-projection-v2-plan.md
  - elohim/eprfs/eprfs-agent/src/canonical.rs
  - elohim/eprfs/eprfs-agent/examples/project_agents.rs
  - elohim/eprfs/eprfs-agent/tests/fixtures/code-reviewer.md
  - .claude/agents/
---

## Resolution — parse-hardening LANDED (2026-07-06, `integ/eprfs-agent`)

Design option **(a)** was implemented in `elohim/eprfs/eprfs-agent/src/canonical.rs`: a
`parse_frontmatter` helper tries strict `serde_yaml` first (so well-formed frontmatter keeps its
exact prior semantics), and only on a YAML error lifts the reserved free-text `description` field
**line-wise** (`lift_top_level_scalar` — verbatim to end-of-line), then re-parses the structured
remainder (which still carries nested blocks like `mcpServers:`). `render_claude` already emits
`description: {value}` verbatim, so the round-trip contract holds with no render change.

**Evidence (locally verified, native env `RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/eprfs-gate-target`):**
- Dry-run over the live tree (`cargo run -p eprfs-agent --example project_agents -- ../..`):
  **`parsed 23 agent capabilities`** — was 0/23, now **23/23**.
- Full eprfs workspace gate GREEN: `fmt` clean · `clippy --workspace --all-targets -D warnings`
  clean · `cargo test --workspace` = **45 passed / 0 failed** (was 38; +6 unit +1 integration,
  incl. the adversarial-review hardening regressions below).
- New regressions: `parses_real_corpus_colon_laden_description`,
  `lifts_description_without_disturbing_nested_extra`,
  `strict_parse_error_survives_when_description_is_not_the_cause` (guards the fallback never masks a
  genuinely-structural YAML error), `claude_render_round_trips_colon_laden_description_byte_perfect`,
  and integration `live_shape_colon_laden_description_round_trips_and_is_drift_clean`
  (new fixture `tests/fixtures/code-reviewer-live-shape.md`). The T6 acceptance stays green.

**Adversarial-review hardening (folded into the same change).** A 3-lens review surfaced a latent
silent-corruption defect in the fallback: `lift_top_level_scalar` deleted only the single physical
`description:` line, so a future description hand-wrapped across two physical lines (or a `description: |`
block scalar reached under an unrelated strict error) would orphan its indented continuation onto the
preceding `name:` key — YAML-folding it into a corrupt `slug` (e.g. `x three`) and returning `Ok`
(worse than the pre-fix clean error). Fixed: the lift now REFUSES (surfaces the strict error) when the
value is a block-scalar header (`is_block_scalar_header`) or an indented continuation line follows.
Regressions: `refuses_to_lift_a_wrapped_multiline_description`,
`valid_block_scalar_description_still_parses_via_strict_path`. Not corpus-triggered today (all 23
descriptions are single physical lines) but a real defect in the new code path.

## Remaining (the folded-in style-preservation residual — why this is `partially-resolved`)

The dry-run reports **`23 entries; 8 drifted`**. The 8 are exactly the agents that carry a nested
`mcpServers:` block. Their **data** round-trips (mcpServers survives into `extra`), but authored
block **style** does not: the author indents sequence items 2 spaces (`  - jenkins:` … 6-space
nested mapping), while `serde_yaml`'s emitter writes `- ` at column 0 with 4-space nesting — a pure
indentation delta, not data loss. The 15 pure-scalar agents round-trip **byte-perfect**
(drift-clean). Closing this residual to 23/23 byte-perfect requires preserving and re-emitting the
raw authored block for non-scalar `extra` fields (a style-preserving YAML pass), which changes the
`extra` representation and is a larger, separate change than the parse fix — kept here per the
original "fold into the same wave, don't open a separate item" intent. **Priority lowered high→medium:
the blocker (0% onboardable) is cleared; this is a fidelity nit, not a gate.**

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

- **(a) RECOMMENDED — ✅ IMPLEMENTED 2026-07-06** (see Resolution above) — parse reserved free-text scalar fields (`description`, and any other field
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
