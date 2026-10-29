---
id: "backlog-governance-terminal-authority-frame-guard-missing"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "No governance guard catches a terminal-authority absolute (\"a collective must always keep at least one human steward\") — the human-loop principle lives only in memory, and the sovereignty guard keys on a substring this framing never uses"
slug: "governance-terminal-authority-frame-guard-missing"
written: "2026-10-29"
author: "claude (operator-observed miss during the Nostr Compass survey mint pass)"
status: "open"
priority: "medium"
area: ".claude/epr-meta"
domain: "governance"
jobs: [elohim]
relatedNodeIds: [feedback_human_loop_not_terminal_authority, feedback-identity-sovereignty-ontology-guard]
tags: [governance, epr-meta, ontology-guard, terminal-authority, compose-gate, frame]
---

# An absolutist authority invariant went through every gate

**What happened (2026-10-29).** Minting the [Nostr Compass survey](epr:nostr-compass-ecosystem-cross-pollination-2026-10-29),
the session proposed Marmot's admin rule as an integrity invariant: *"a collective must always keep at
least one steward"*. The operator rejected it twice:

- Commons pools are anticipated to run with no direct human stewards.
- Authority over a holon's acts also comes **from the holons above it**: *"if country A wants to commit an
  atrocity, there is an enforcement that can come from the constitutional framework above."*

The operator asked why no governance check fired. The corrected row is
[arch-authority-in-integrity-backlog](epr:arch-authority-in-integrity-backlog) row 19.

**Why nothing fired (checked, not assumed):**

1. **No rule covers this class.** `.claude/epr-meta/policies.yaml` registers frame-based ontology guards
   for sovereignty and ownership (`sovereignty-ontology-guard`, `ownership-ontology-guard`) and none for
   terminal authority. The principle that neither species is terminal, the method is, lives only in agent
   memory (`feedback_human_loop_not_terminal_authority`), which records three earlier recurrences in prose
   (2026-07-23, 2026-08-29 ×3). The sovereignty guard's scope is the substring `sovereign`
   (`policies.yaml:164-175`), and an absolute like "must always keep a human steward" never contains it.
2. **The edit-time gates match `Edit|Write` only** (`.claude/settings.json`: `epr-meta-resolver`,
   `sovereignty-guard-signal`, `blind-reader-review`). Most of the session's backlog rows were written
   through `python3` in Bash, which no edit-time gate sees. The pre-push compose-gate is the
   harness-agnostic backstop ([compose-gate-runs-after-fastpath-exits](epr:compose-gate-runs-after-fastpath-exits)),
   so shell writes are evaluated at reach. Here it would also have passed, because of (1).
3. **Replay.** Feeding the PreToolUse resolver a synthetic `Write` of each touched file's current content
   returned no verdict for any of the eleven files. Research rules are birth-only (`new: true`), and the
   backlog has no rule for the class.

## The guard to add (proposal; a policy needs ratification)

A **terminal-authority ontology guard**, the third sibling of the sovereignty and ownership guards. It
has the same shape: a content-addressed **frame** read over the folded net-new text by a background
reviewer. It is never a keyword deny, because the drift is a *framing*, not a word.

- **Frame `frame-terminal-authority@1`.** It flags a passage that makes one source of authority terminal
  or absolute. That covers:
  - a species: "routes to a human as the final floor", "a human must always decide", "the elohim decide
    alone";
  - a roster: "authority resolves only through the collective's own members", as though nothing above a
    holon can bind it;
  - a minimum headcount as a validity invariant: "≥1 human steward".

  It passes the canonical shapes: the method is sovereign; subsidiarity within constitutional bounds;
  constraints from enclosing holons enforced downward; councils drawn to the reach of the decision;
  humans as evidence, signal, consent and exit; never orphaned from a *declared* governing method.
  Canon: `genesis/docs/content/elohim-protocol/governance-layers-architecture.md:78-95,126` and
  `manifesto.md` (the elohim-consensus core).
- **Scope.** `*.md` writes under `genesis/docs/`, `genesis/data/timeline/backlog/`, `genesis/research/`
  and `docs/specs/`, gated on governance vocabulary (`steward`, `council`, `authority`, `collective`,
  `holon`, `govern`, `must always`, `final`), not on one word. Research keeps its membrane: a
  quotation of another project's terminal-authority rule is admitted when the survey says plainly how
  we differ, as `sovereignty-frame:` does today.
- **Class: `dispatch`** to a reviewer (storyteller, as for the sovereignty guard) that reports ALIGNED or
  quotes the smallest passage with a concise reframe, and never edits. A deny would tax the critique
  this repository has to write.
- **Pre-push parity.** The same frame evaluates at reach, so shell-written text is covered.

**Also check (◐, unmeasured):** whether pre-push replays *birth-time dispatch* rules (the research
`cross-pollination-dedupe` librarian dispatch, the backlog `cluster-first-at-birth` inject, the
blind-reader dispatches) for files born through a shell write. If it does not, a shell-born file silently
skips its birth review. That is a governance gap separate from the missing guard.

## Acceptance

1. The ratified policy row exists with `established_by` from a deliberation, not a solo stamp.
2. Replaying the three recorded recurrences and today's sentence through the frame flags each one. The
   corrected row 19 text, the governance-layers canon passage and a research quotation with a declared
   membrane all pass.
3. A shell-born test file carrying the sentence is flagged at pre-push.
