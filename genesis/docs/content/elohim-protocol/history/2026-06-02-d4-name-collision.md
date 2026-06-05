---
title: "The D4 name-collision — why four process specs mis-filed as substrate"
id: d4-name-collision
tier: history
type: history-gotcha
created: 2026-06-02
class: process-meta
process_subdomain: doc-lifecycle
cites:
  - .claude/subject-routing.yaml
  - subject-routed-decomposition-design | the class gate this gotcha motivates — the fix for process specs mis-filing as D4 substrate | sha256:0d910143a8498b64 | path: genesis/docs/superpowers/specs/2026-06-02-subject-routed-decomposition-design.md
  - map | the product-only D1–D10 lattice with no meta axis that made D4 magnetic for process specs | sha256:de878342b28843e8 | status: stale — target content moved on; re-verify | path: genesis/docs/content/elohim-protocol/architecture/MAP.md
qualifies:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md    # the magnetic D4 seed (innocent — the dogfood vocabulary it shares is the trap)
---

# The D4 name-collision

**The gotcha.** Four dev-process specs — `spec-plan-compaction-loop`, `unified-memory-loop`,
`semantic-computable-links`, `scope-tree-reconciler` — all filed themselves as `domain: D4` (Memory
Lifecycle & Consolidation) and cited the product seed `2026-05-10-memory-lifecycle-design.md` as their
canonical parent. **None of them are product work.** Their entire residue lands in `.claude/` +
`PLACEMENT.md` (memory-kit scripts, the decompose/stasis tooling, agent definitions) — verified **0
product-code refs** each. They are about the *machinery of building*, not the running protocol.

**Why D4 was magnetic (the trap).** `MAP.md`'s subject-domain lattice (D1–D10) is **entirely product
domains** — there is no meta/process axis. A process spec has no honest D#, so its author reaches for
the nearest-sounding one. And it is *always* D4, because the dev tooling **dogfoods the protocol's own
vocabulary**: a doc-citation audit borrows "the comet — links survive dissolution," a doc-corpus
compaction loop borrows "compact / merge / memorialize," and the shared words make `memory-lifecycle-design.md`
feel like the parent. **The vocabulary collision is irresistible by construction** — the more the dev
process eats its own protocol's concepts, the harder it pulls process specs into product domains.

**The fix (the rule).** Classify by **deliverable-TARGET, never by vocabulary.** *Where does the landed
change physically live?* — `.claude/`+a CLAUDE.md (process) or `app/`+`architecture/` (product). A spec
that says "comet / EPR / cites" but amends only `.claude/` is **process-meta, full stop**; the shared
vocabulary is a `derived_from:` lineage breadcrumb, never a routing key. This is enforced by the
`vocab-vs-target-mismatch` gate signal (`.claude/subject-routing.yaml`; `decompose.py` fails loud on
`domain: D#` + all-`.claude/` targets) and the new subject-class axis (`MAP.md` Axis 0, pointing at the
manifest).

**Path not taken.** The first instinct was a *fourth class* — "method-bridge" — for process work that
dogfoods product primitives. It was collapsed: a class with no unique home is nomenclature debt. The
phenomenon is just process-meta carrying a `derived_from:` breadcrumb; the gate signal makes it legible
without a class. (Even the defining spec, hand-labeled, is plain process-meta.)

**Lesson for the next router.** A meta/process layer that *reuses its own product's vocabulary* will
mis-file itself into product domains unless classification keys on the deliverable target, not the
words. The dogfood is good (it proves the primitives); the routing must look past it.
