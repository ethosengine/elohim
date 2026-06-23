# Handoff archive — DEPRECATED holding pen (pending decompose)

> **Discipline changed 2026-06-23.** Handoff docs are **not** a blessed artifact class.
> A concluded shift/sprint must **decompose** its handoff into a *living surface* — the
> No-Dumping-Grounds law (`agentic-developer` Close step 5): wisdom → memory / curated
> history, open work → `genesis/data/timeline/backlog/`, narration → git history, then the
> standalone file is **removed**, not archived.

The earlier rule here ("keep ≤4 active handoffs at the repo root; move the rest here rather
than deleting") was a **volume gate masquerading as discipline**. It never required
decomposition, so ~1,670 lines of handoff sprawl accumulated across 16 files (repo root +
`.claude/handoffs/`) with their signal un-harvested. Archiving-instead-of-decomposing is the
anti-pattern, not the cure.

## The rule now

- **Finishing a shift?** Decompose — don't write a handoff. Cross-session notes live in the
  sprint-result under `.claude/shifts/<id>.sprint-result.md` and are decomposed on close.
- **No free-floating `HANDOFF-*.md`** at the repo root or in `.claude/handoffs/`.
- This directory is a **temporary holding pen** for not-yet-decomposed docs, **operator-purgeable**
  after harvest — never a permanent store.

## Open follow-up

- Remaining un-decomposed docs + their harvest targets are inventoried in
  `genesis/data/timeline/backlog/handoff-sprawl-decompose-2026-06-23.md`.
- Recommended (operator-gated): extend the `agentic-developer` No-Dumping-Grounds law to name
  *handoff documents* explicitly in its decompose scope, so the discipline is enforced at the
  tool boundary rather than relying on this README.
