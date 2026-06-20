---
title: "Scope-prose coherence switch — reconcile PROSE scope-claims against the live `available` flag (via cites)"
status: OPEN
class: process-meta
process_subdomain: doc-lifecycle
domain: D-process
surfaced: 2026-06-20
cites:
  - genesis/manifests/cluster-state.yaml
  - genesis/docs/superpowers/specs/2026-06-02-scope-tree-reconciler-design.md
---

# Scope-prose coherence switch

**The gap (cost: real).** `scope-reconcile.py` reconciles **structured** scope — the `held/` tree,
`requires_env:` frontmatter, and inbound `cites:` flip HELD-CITE↔healthy when a capability flips. It does
**not** touch **prose**, and two prose drifts went uncaught for ~3 weeks and mis-led an entire design arc:

1. **cluster-state `note` ⊥ `available` flag.** `shem` carried `available: true` with a prose
   `note: "offline / inaccessible … OUT OF SCOPE until it returns — held"` (stale since 2026-06-01,
   never cleared when shem returned). `scope-reconcile` read the flag (correct: shem available, `held: 0`),
   but every human/agent who read the *note* believed shem was down.
2. **Spec/plan prose scope-claims ⊥ live availability.** The Doorway Membrane arc (4 waves) baked
   "held until shem" / "design-only until shem" into the membrane spec + serve-routing plan — while shem
   was `available: true` the whole time. `@requires:shem` is a correct *requirement* tag, but the prose
   read it as *blocked* when the cap was live. Nothing flagged the contradiction.

**The switch (what we should have had).** A coherence lint — sibling to `placement-audit` / `scope-reconcile`,
ideally a PostToolUse or pre-push check — that:
- **(a) note↔flag:** flags any `cluster-state.yaml` resource whose `note` contains held/offline/OOS language
  while `available: true` (or available-language while `available: false|degraded`). The structured flag is
  authoritative; the note must not contradict it.
- **(b) prose↔live, via cites:** scans `specs/`+`plans/` for prose scope-claims that name a capability
  ("held until `<cap>`", "`<cap>` offline", "design-only until `<cap>`", "`@requires:<cap>` … held") and
  flags any whose named `<cap>` is currently `available: true` — a **false-held**. Walk the cite graph so a
  single capability flip surfaces every downstream doc that prose-asserts it as down (the cites already
  exist; this consumes them for scope, as the operator envisioned).

**Why it matters.** The structured machinery was correct (`held: 0`); prose is *unreconciled scope*, and a
single stale note propagated a false "held" through a whole arc. The fix turns "trust the prose" into a
checked invariant. Until it exists: **trust the `available` flag + the focus baseline over any prose note
or stale memory** (see memory `scope-flag-beats-prose-note`).

**Done when:** the lint exists, runs in the scope/placement cadence, and would have caught both (1) and (2)
on the day shem returned.
