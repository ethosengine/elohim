---
title: "Semantic-Computable Links — Content-Addressed Citations the Tools Generate, Resolve, and Verify"
id: semantic-computable-links-design
status: Draft
created: 2026-06-02
tier: design-spec
topic: [cites, citation, content-addressing, slug, fingerprint, dead-link, semantic-computable, links, memory-coherence, cite-gen, migration, enforcement, born-linked, dissolution-gate, agent-awareness, memory-kit, stasis-loop]
class: process-meta
process_subdomain: memory
derived_from:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md  # the comet (dogfood breadcrumb — lineage, NOT a domain claim; see history/2026-06-02-d4-name-collision)
cites:
  - scope-tree-reconciler-design | the file-moving reconciler that depends on these content-addressed cites to move safely | sha256:1f7847ac624b0df7 | status: stale — target content moved on; re-verify | path: genesis/docs/superpowers/specs/2026-06-02-scope-tree-reconciler-design.md
  - spec-plan-compaction-loop-design | the compaction loop whose path cites this upgrades to slug+fingerprint with a dissolution gate | sha256:958940bdf5a41b40 | status: stale — target content moved on; re-verify | path: genesis/docs/superpowers/specs/2026-06-02-spec-plan-compaction-loop-design.md
  - unified-memory-loop-design | the loop this adds a cites_legacy scoreboard dimension and cite discipline to | sha256:99100efd20d10129 | path: genesis/docs/superpowers/specs/2026-06-01-unified-memory-loop-design.md
  - .claude/scripts/memory-kit/memory-coherence-audit.py
  - .claude/skills/epr-content-addressing/SKILL.md
  - placement | the contract whose three doc homes these content-addressed links survive moves between | sha256:f84d7cb16bea9379 | status: stale — target content moved on; re-verify | path: genesis/docs/PLACEMENT.md
  - managed-surface-edit-discipline-design | the 2026-06-05 sibling: edit-time registry + PRE/POST hooks that enforce this discipline at the surface (see §9.1) | sha256:e5afb16c974b109b | path: genesis/docs/superpowers/specs/2026-06-05-managed-surface-edit-discipline-design.md
refines:
  - genesis/docs/superpowers/specs/2026-06-02-spec-plan-compaction-loop-design.md  # upgrades its path-based cites: to content-addressed; adds the dissolution-time hard gate
proposed_amendments:
  - .claude/scripts/memory-kit/memory-coherence-audit.py    # PROPOSED §3 — resolve-by-slug; HELD-CITE / STALE-CANDIDATE / CITE-FORMAT-CANDIDATE verdicts
  - .claude/scripts/memory-kit/decompose.py                 # PROPOSED §5 — dissolution-time cite-correctness gate (BACK-fire)
  - .claude/workflows/memory-stasis-loop.js                 # PROPOSED §7 — cites_legacy scoreboard dimension + cites discipline
  - .claude/agents/librarian.md                             # PROPOSED §8 — convention + verdicts + cite-gen awareness
  - .claude/agents/historian.md                             # PROPOSED §8
  - .claude/agents/cartographer.md                          # PROPOSED §8
  - .claude/agents/storyteller.md                           # PROPOSED §8
requires_env: []
---

# Semantic-Computable Links

## 1. Why

Today a `cites:` entry is a repo-relative **path**, and `memory-coherence-audit.py`'s `cite_resolves()` is `p.exists()`. Path is *location*, not *identity* — so any tool that **moves a file** (the scope-tree-reconciler's `held/` moves; routine refactors; the compaction-loop's relocations) manufactures a `DEAD-CITE` storm. The deeper opportunity: once a link carries *identity + fingerprint + a human label*, the whole doc/spec/memory/story graph becomes **semantic-computable** — tools can **resolve** (which doc?), **verify** (did it change?), **surface** (what is it, in one sentence?), and **traverse** (the citation DAG) without a human in the loop. This is the substrate the reconciler, the dead-link audit, and the memory team all stand on.

This spec is the citation primitive the [scope-tree-reconciler](2026-06-02-scope-tree-reconciler-design.md) named as its dependency, grown to its full size: format, generation tool, enforcement, migration, and the agent/tooling awareness it forces.

## 2. The link primitive (the dogfood)

Mirror the protocol's own EPR content-addressing (`epr-content-addressing` skill), which separates two things a raw CID fuses:

- **Identity = a slug** — the doc's *permanent address*. Survives the `held/` move **and** ordinary edits. Each doc declares `id: <slug>` in frontmatter ("give content a readable name; that name is its permanent address").
- **Fingerprint = the CID** — *changes* when content changes; a verification signal, not an address.

A citation is a tiny EPR-style envelope — three authored fields, plus one optional **tool-managed** field:

```yaml
cites:
  - ref: iroh-recovery-e2e            # slug = permanent identity (survives move + edit)
    desc: the recovery e2e plan this milestone unblocks
    fingerprint: bafyrei…             # CID at cite-time; mismatch ⇒ re-verify, not dead
    status: held — needs remote-compute   # ← OPTIONAL, tool-stamped/removed (§3.1); absent when healthy
```

The first three are authored (via `cite-gen`, §4); **`status:` is never hand-written** — a propagation pass adds it when the target goes out of scope/stale/dead and removes it when the target returns (§3.1). The `desc` is the envelope's human-readable label; `status:` is the edge's health hint. So even when the target is sequestered in `held/`, the citing doc reads coherently, the human sees *what's wrong with the link inline*, and the auditor knows *what* it references. The graph becomes a content-addressed DAG with labeled, self-describing edges: the memory layer eating the protocol's own content-addressing.

## 3. The audit upgrade (memory-kit tooling)

`cite_resolves(ref)` changes from `p.exists(path)` to **resolve-by-slug across live AND `held/`**:

| Outcome | Verdict | Meaning |
|---------|---------|---------|
| slug resolves in live tree | OK | — |
| slug resolves in `held/` | `HELD-CITE` (informational) | cited doc is sequestered; not dead |
| slug resolves nowhere | `DEAD-CITE` | genuinely dangling |
| target CID ≠ cited `fingerprint` | `STALE-CANDIDATE` | content moved on; re-verify the lesson |
| `cites:` entry is a legacy path-string | `CITE-FORMAT-CANDIDATE` | needs migration to envelope form |

A slug→location index (built once per audit, live + held) replaces the per-path `exists()`. Legacy path-string cites keep resolving via `p.exists()` during migration (back-compat).

### 3.1 The propagation surface — self-describing edges (the epr-head analogy)

The audit *detects* the verdicts above; this is what makes them **visible on the edge itself**. Because every cite carries a `ref` (identity) and a `fingerprint`, the corpus is a **compute surface**: a reverse index (target → every cite that references it) plus fingerprint comparison lets a tool **propagate a node-state change to all citing edges**. When the scope-tree-reconciler moves a doc to `held/`, when a target's fingerprint drifts, or when a target dissolves, a **propagation pass materializes the verdict as the optional inline `status:` field** on each affected `cites:` block:

```
target iroh-recovery-e2e moves to held/  →  propagation stamps every live citer:
    status: held — needs remote-compute        (machine: HELD-CITE; human: reads in plain text)
target returns to live                    →  propagation removes the status: line (healthy = no field)
```

The field is **optional, tool-managed, programmatically added and removed** — never hand-maintained, absent on healthy links. This is the difference between an audit *report* (a separate file you must consult) and a self-describing *edge* (the health travels **with** the doc, in git, readable in the raw file). A doc that **stays on the plate** can cite something that is **out of scope**, and *what is wrong with that link is legible inline to human and machine alike* — no broken link, no silent staleness.

This is the **epr-head edge**: exactly as the protocol's ~500-byte metadata envelope lets a peer decide *follow / skip / re-verify* from the hint on the reference **without fetching the full content**, the `status:` field lets a reader or a tool decide what to do with a citation **without resolving the target**. The fingerprint is the compute primitive that makes the hint trustworthy (it *knows* when content moved on); the propagation pass is what keeps every edge's hint current. Materialization is a reviewable commit — consistent with the reconciler's own "the tree is the ledger": link-health history lives in git, same as scope-history.

## 4. cite-gen — the tool that makes it easy

`cite-gen` is the friction-killer that makes *both* soft authoring and the hard dissolution gate painless. Given a target (slug or path), it emits the ready envelope:

```
$ python3 .claude/scripts/memory-kit/cite-gen.py scope-tree-reconciler
- ref: scope-tree-reconciler
  desc: the reconciler whose held/ moves require links that don't dangle   # auto from target title; editable
  fingerprint: bafyrei…                                                     # computed from target's current content
```

Three uses, one primitive:
- **Author one cite** — paste the block (or `--into <doc>` rewrites that doc's `cites:` in place).
- **Migrate the corpus** — the migration workflow (§6) calls `cite-gen` per doc.
- **Gate the dissolution** — `decompose.py` (§5) runs `cite-gen --verify` to confirm every cite is envelope-form and resolves before a spec graduates.

Nobody hand-writes a slug or a fingerprint. That is what "semantic-computable" buys: links are *generated and verified by tooling*, not typed.

## 5. Enforcement — soft at the FRONT, hard at the BACK

Placed on the compaction-loop's two fire points:

- **FRONT (authoring / born-linked) → SOFT.** New links are born content-addressed because the `semantic-links` template (a `.claude/skills/` reference) + `cite-gen` make the right form the path of least resistance, and the `/brainstorm` + compaction POST steps (which already write `cites:`) emit the envelope form. Drafts iterate freely; the audit only *flags* (`CITE-FORMAT-CANDIDATE`), never blocks. Matches the project's signal-driven, accumulator-not-cadence philosophy.
- **BACK (decompose / dissolution) → HARD.** When a spec self-dissolves at sprint end (compaction-loop BACK-fire), its links are precisely what *survive* it into the canonical graph — so `decompose.py` **blocks dissolution** until every `cites:` is envelope-form and resolves (`cite-gen --verify`). Strict exactly where correctness is load-bearing, and trivially satisfiable (`cite-gen --into <spec>`). Lenient while it's a draft; rigorous at graduation.

The legacy corpus converges via the stasis loop (§7), not a big-bang.

## 6. The migration workflow (batch, one-time)

`.claude/workflows/cites-migration.js` (Workflow), two passes:
1. **Allocate** `id: <slug>` to every doc/memory lacking one (deterministic from title; collision guard).
2. **Rewrite** — fan out agents over the corpus; each calls `cite-gen --into <doc>` to convert path-cites → envelopes. Idempotent (envelope cites skipped).

Run once to migrate the existing corpus; the stasis loop maintains it thereafter.

## 7. Stasis-loop wiring (maintenance)

`memory-stasis-loop.js` gains one scoreboard dimension and one discipline, exactly like `decompose_due` / `dumps`:
- **`cites_legacy`** (number) — count of `CITE-FORMAT-CANDIDATE` from the audit MEASURE step.
- **`cites` discipline** → `librarian`, goal: *"Legacy path-string cites remain (headline `cites:`). Run `cite-gen --into` on a batch to convert them to content-addressed envelopes. Lower `cites_legacy` toward 0 (fully semantic-computable)."*

Stasis for this dimension = `cites_legacy == 0`. The loop drains it as one more discipline; the headline surfaces it.

## 8. Agent awareness (the memory team must know)

The historian, librarian, cartographer, and storyteller author and curate the citation graph — they are the primary producers/consumers. Each agent definition gains awareness of:
- **The envelope convention** + `cite-gen` (never hand-write slugs/fingerprints).
- **The new verdicts** — `HELD-CITE` ≠ `DEAD-CITE` (a held cite is not a problem); `STALE-CANDIDATE` (re-verify), `CITE-FORMAT-CANDIDATE` (migrate).
- **`held/` moves are safe** — a cite to a sequestered doc resolves; do not "fix" a `HELD-CITE` by deleting the link.
- **The dissolution gate** — when decomposing a spec (BACK-fire), cites must be envelope-form and resolve first (`cite-gen --verify`).

Per-agent emphasis: **librarian** owns the audit + stasis `cites` discipline + the migration; **historian** resolves-by-slug when surfacing precedent (a held precedent still surfaces); **cartographer** traverses the citation DAG for ranking (semantic-computable links make this real); **storyteller** preserves `ref`+`desc` when graduating a lesson so the canonical story keeps its provenance edges.

## 9. Scope & non-goals

- **In:** the link envelope, doc `id:` slugs, the audit upgrade, `cite-gen`, soft/hard enforcement, the migration workflow, stasis-loop + agent + memory-kit wiring.
- **Out:** the scope-tree-reconciler's move-mechanics (its own spec); generalizing slugs into full DHT-notarized EPRs (these are internal planning artifacts — we adopt the EPR *reference model*, not its reach/REA/governance legs); a hard pre-push hook (rejected — friction; the BACK-fire gate + stasis drain cover it).

### 9.1 Scope amendment (2026-06-05) — gospel surfaces, the materialized locator, the deliberate refresh

Three deltas landed after the original scope (forensic: the gospel-cites episode — an agent hand-wrote
slug-only cites into CLAUDE.mds because every enforcement surface hardcoded the doc-root scope, and the
operator's correct "the tooling exists" expectation had no edit-time surface to land on):

- **Gospel CLAUDE.mds join the graph.** Id-declaring `CLAUDE.md` files repo-wide (vendored/dot dirs
  pruned) are slug-index members (`cite_graph.extend_index_with_gospels`); membership is OPT-IN by
  declaring `id:` — the wide walk admits nothing accidental. The gospel walk-scope lives in ONE
  authority function — `cite_graph.is_gospel_claude_md` (+ `GOSPEL_EXCLUDE_DIRS`) — called directly by
  the batch tools (`cite-gen --seal/--seal-all`, `cite-propagate`, `cites-migrate`,
  `memory-coherence-audit`) and fronted by the `_lib.managed_surfaces` registry for the hook layer
  (`cite-seal-signal`, `managed-surface-context`). Scope is defined once, never re-hardcoded per tool;
  genesis/data entity docs deliberately stay plain-path.
- **`path:` — the materialized locator.** The second tool-managed envelope field (after `status:`):
  every envelope carries its slug→path resolution as an inline CACHE so an agent follows a cite with a
  plain read, no resolver run. Stamped at mint (`emit`/`--into`/`--seal`), refreshed by every
  `cite-propagate` pass — a move self-heals on the next pass; a dead slug keeps its last path as a
  forensic breadcrumb. The slug stays identity; the fingerprint stays drift-truth; the path is only cache.
- **`--refresh` — the deliberate stale-dequeue.** `status: stale` is a re-verify QUEUE, and `--into`
  deliberately never auto-blesses drift. After re-verifying the citing doc's claims against the moved-on
  target, `cite-gen --refresh <doc>` re-blesses fingerprints (+ status + path) in one deliberate act.

The edit-time enforcement (PreToolUse discipline injection) and the full surface registry live in the
sibling spec: `managed-surface-edit-discipline-design` (cited above).

## 10. Decomposition (gap-items)

- [ ] Doc `id: <slug>` frontmatter convention + deterministic slug allocation + collision guard (§2).
- [ ] `cites:` envelope schema (`ref`/`desc`/`fingerprint`) + back-compat parser for legacy path-strings (§2).
- [ ] Fingerprint definition — canonical content region to hash so a frontmatter edit doesn't trip `STALE-CANDIDATE` (§2, open Q).
- [ ] `cite-gen.py` — resolve target → emit envelope; `--into <doc>` (rewrite in place); `--verify` (gate mode) (§4).
- [ ] `memory-coherence-audit.py` `cite_resolves()` → slug-index resolve across live+held; emit `HELD-CITE` / `STALE-CANDIDATE` / `CITE-FORMAT-CANDIDATE` / `DEAD-CITE` (§3).
- [ ] Optional tool-managed `status:` field on the cite envelope (the epr-head edge hint) + the reverse index (target → every citer) as the compute surface (§2, §3.1).
- [ ] Propagation pass — materialize the audit verdict as inline `status:` on each citer when a target goes held/stale/dead; remove it on recovery; triggered by a reconciler move, audit `--apply`, or the stasis loop; emitted as a reviewable commit (§3.1).
- [ ] `decompose.py` dissolution gate — block graduation until `cite-gen --verify` passes (§5).
- [ ] `.claude/skills/semantic-links/` template + wire it into the `/brainstorm` + compaction POST steps so new cites are born-correct (§5).
- [ ] `.claude/workflows/cites-migration.js` — two-pass corpus migration (allocate slugs, rewrite cites) (§6).
- [ ] `memory-stasis-loop.js` — add `cites_legacy` scoreboard dimension + `cites` discipline; add `cites:` headline line in `placement-audit.py` (§7).
- [ ] Memory-team agent-def updates (librarian / historian / cartographer / storyteller) — convention, verdicts, held-safe, dissolution gate (§8).

## 11. Open questions

- **Slug source**: human-readable from title (EPR style, collision-guarded) vs an opaque stable id? Lean readable.
- **Fingerprint scope**: whole-file vs content-body-only (exclude frontmatter so metadata churn doesn't re-verify). Lean content-body.
- **Cross-repo / external cites** (code paths, URLs): keep as path/URL strings (no slug) — the envelope is for *doc graph* nodes; code citations stay `cite_resolves`-by-path + the existing git-change `STALE` signal.
- **Memory entries vs specs**: same envelope for `.claude/memory/*.md` `cites:` as for specs? (Lean yes — one graph.)
