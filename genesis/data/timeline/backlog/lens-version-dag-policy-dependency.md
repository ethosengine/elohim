---
title: Lens version DAG + EPR→policy dependency-declaration (which HEAD applies is declared, not inferred)
created: 2026-06-27
status: OPEN
domain: D7-collective-governance
source: code-review of the plural-Mishpat lens-market slice (PR4 disposition) + architect design direction
severity: medium
needs: p2p-design-gate
---

**Design seed — NOT yet specced. The proper next step is a `p2p-design-gate`'d brainstorm → spec.**
Captured 2026-06-27 from the lens-market code review (finding #3: `version_parent` stored but never
consumed → v1 and v2 of one school both surface, splitting its standing) and the architect's reframing of
how supersession *should* work.

## The reframing

The reviewer called "v1 and v2 both surface" a recency bug and the obvious fix is "newest-in-chain hides the
older." **That bakes in the wrong model.** Two corrections from the architect:

1. **Lens versions are a DAG, not a linear chain.** A school can **fork** (branch a reading), **revert**
   (move its head back to an earlier `cid`), or **merge** (reconcile branches). `version_parent` is a
   git-shaped back-pointer; `cid == entry_hash` (immutable, content-addressed) is exactly what makes the DAG
   work — every version is a content-addressed node, edges are back-pointers, nothing is ever mutated.

2. **Which HEAD of a policy applies to an EPR is a DECLARED dependency, not an inference.** The model is
   `package.json`: the EPR (or whoever binds policy to it) declares *"I depend on policy P"* with a
   **constraint** — a range ("latest in this branch", `^2`) or a **pinned exact `cid`** (which is precisely a
   **lockfile** — content-addressed, reproducible). A resolver then picks the effective head per the
   declaration. **The binding decides the head; the infrastructure does not.**

## Why this is the load-bearing inversion

The "duplicate" the review flagged is not a recency bug — it is that **the binding-declaration layer does not
exist yet**. Surfacing all heads is the *honest default* until an EPR can declare which head it depends on.
Any "newest auto-supersedes" filter (or even linear chain-aggregation of standing) in `find_lenses_governing_epr`
would cement a head-selection policy the protocol should leave to the declaration — and we'd rip it out when
the binding lands. **So PR4 was DEFERRED, not implemented** (`find_lenses` surfaces all heads;
`elohim/elohim-storage/src/db/lenses.rs` carries a `// DEFERRED` note pointing here).

## What a spec must answer (p2p-design-gate first)

- **Entity (P2P-native, not a relational FK):** is the EPR→policy dependency a new notarized DHT entry
  (A-class), a derived link (A2), an agent-scoped declaration (B), or an operational/projected record (C)?
  Does a DHT entry type already exist (Mishpat headroom ~11/~100)? Is the constraint itself content-addressed?
- **Constraint grammar:** range vs branch-pointer vs exact-`cid` pin. The exact-`cid` pin is the lockfile —
  reproducible, audit-able, the natural fit for `cid`-canonical addressing.
- **Resolver:** given a declaration + the version DAG, compute the effective head(s). Fork/revert/merge
  semantics (a merge head has ≥2 `version_parent`s — the schema's single optional `version_parent` may need
  to become a set).
- **Standing across a chain:** each version earns its own affinity under its own `cid`; the declaration picks
  which applies. Does standing aggregate up a chain, or stay per-version? (This is why Option B's
  linear-chain affinity aggregation was ALSO deferred — premature under the declared-head model.)
- **Plurality is orthogonal:** distinct *schools* (georgist + beerian) always co-surface — no collapse. This
  design only governs a *single school's own* version DAG + which head an EPR binds.

## Relation to shipped work

- The lens-market read+write slice (S1–S9) shipped with `version_parent` stored but unconsumed — correct,
  pending this design.
- **Revocation (PR3) shipped** as the substrate primitive a "revert"/"yank" builds on (`lenses.revoked_at` +
  `db::lenses::set_revoked_at` wired into the `revokes-commitment` projection).
- Sibling deferred legs: `lens-selection-write-path-slice` (affinity producer), ballots/elections/bounty
  (Wave-1 remainder).
