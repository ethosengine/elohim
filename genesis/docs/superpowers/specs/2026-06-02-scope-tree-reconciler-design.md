---
title: "Scope-Tree Reconciler — The Plate Narrows by Moving Files, Links Survive by Content-Addressing"
id: scope-tree-reconciler-design
status: Draft
created: 2026-06-02
tier: design-spec
topic: [scope, focus, cluster-state, requires-env, held-tree, compile-enforced, reconciliation, content-addressing, cites, dead-link, capability-not-box, substrate, planning, narrow-expand-plate]
class: process-meta
process_subdomain: doc-lifecycle
derived_from:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-10-memory-lifecycle-design.md  # comet shape (dogfood breadcrumb — lineage, NOT a domain claim; see history/2026-06-02-d4-name-collision)
cites:
  - genesis/manifests/cluster-state.yaml
  - placement | the contract this proposes held/ doctrine and requires_env capability vocabulary for | sha256:f84d7cb16bea9379
  - .claude/scripts/memory-kit/placement-audit.py
  - .claude/scripts/memory-kit/memory-coherence-audit.py
  - spec-plan-compaction-loop-design | the compaction loop whose path-based cites this upgrades to content-addressed so they survive held/ moves | sha256:958940bdf5a41b40
  - unified-memory-loop-design | the loop machinery this reconciler plugs the narrow-plate scope discipline into | sha256:99100efd20d10129
  - .claude/skills/epr-content-addressing/SKILL.md
  - genesis/Jenkinsfile
  - semantic-computable-links-design | the slug-resolving cite system this depends on so file moves stay HELD not DEAD | sha256:405c25775e06a985
depends_on:
  - genesis/docs/superpowers/specs/2026-06-02-semantic-computable-links-design.md  # moves are unsafe until cites resolve by slug (HELD-CITE, not DEAD-CITE)
refines:
  - genesis/docs/superpowers/specs/2026-06-02-spec-plan-compaction-loop-design.md  # upgrades its path-based cites: to content-addressed so born-linked survives held/ moves
proposed_amendments:
  - genesis/manifests/cluster-state.yaml             # PROPOSED §2.2 — capability-named resources (remote-compute), provided-by mapping
  - .claude/scripts/memory-kit/memory-coherence-audit.py  # PROPOSED §4.3 — cite_resolves() resolves by slug across live + held/
  - genesis/docs/PLACEMENT.md                        # PROPOSED §3 — held/ doctrine + requires_env capability vocabulary
requires_env: []   # design spec; landable on household-compute (no remote-compute needed to build the reconciler)
---

# Scope-Tree Reconciler

## 1. The phenomenon, at three levels

The hardware substrate flexes — a remote peer flaps on and off as a steady state, not a failure (`project_seed_whoever_is_ready`). When it does, the *work that is exercisable* changes: with remote-compute down, cross-node P2P stories cannot shake out. The runtime layer already reconciles to this deterministically: `genesis/Jenkinsfile`'s `Probe Substrate` stage sets `ELOHIM_REMOTE_COMPUTE_STATUS` and every stage seeds/verifies/tests *whoever is ready* (the 2026-06-02 substrate-probe landing). That is the **runtime** enforcement.

This spec is the same phenomenon at the other two levels:

| Level | Mechanism | Prevents |
|-------|-----------|----------|
| **Runtime** (Jenkins) | probe-and-skip — *built* | a live run cascade-failing on a dead peer |
| **Compile / structural** | artifact physically *outside* the runner's glob path | a remote-compute story being *runnable at all* when remote-compute is absent |
| **Planning / focus** | `held/` tree + STOP `CLAUDE.md`; planner scans live tree only | attention being *spent* on un-shakeout-able work |

All three reconcile to **one signal** (`cluster-state.yaml`) through **one vocabulary** (`requires_env:` capability names). The runtime probe and the planning state are the same truth two ways; v2 (§5) closes the gap between them.

## 2. The signal and the vocabulary

### 2.1 cluster-state.yaml is the planning-layer's substrate signal

It already exists (`genesis/manifests/cluster-state.yaml`): an operator-declared (NOT auto-derived — `feedback_no_kubectl_from_dev_env`) availability map. `placement-audit.py --focus` already reconciles plans to it: an artifact whose `requires_env:` ⊄ available resources is **BLOCKED-BY-ENV → held, not regressed**. This spec adds the *physical* move that `--focus` only computes.

### 2.2 Capability, not box (PROPOSED resource rename)

`requires_env:` must name the **capability**, not the machine — because this design exists *because the machine changes*. A requirement hard-coding a hostname breaks the moment the canvas moves hardware, the exact event it guards. Resources become capabilities with a `provided-by` hint:

```yaml
resources:
  household-compute: { capability: local household P2P, provided-by: household-nodes, available: true }
  remote-compute:    { capability: remote multi-tenant canvas (cross-node proving ground), provided-by: shem, available: false }
  soak-scale:        { capability: stable multi-peer soak cluster, provided-by: alpha-cluster-6peer, available: degraded }
  image-registry:    { capability: CI image pulls, provided-by: harbor.ethosengine.com, available: false }
```

This aligns all three layers: runtime `nodeTypes: ['remote']` (pool), the env signal `ELOHIM_REMOTE_COMPUTE_STATUS` (already renamed capability-not-box, 2026-06-02), and planning `requires_env: [remote-compute]`. Migration touches `cluster-state.yaml` + the two artifacts currently saying `requires_env:[shem]` (`iroh-recovery-e2e.md`, `PLACEMENT.md`).

## 3. Part A — The reconciler

### 3.1 The tree is the plate (file-granularity)

In a shared monorepo working tree, the filesystem *is* the shared registry: a `git mv` broadcasts the scope change atomically to every consumer at once — the test runner's glob, the planner's scan, and the next agent that `ls`-es the directory all see the same tree state with no side-table to sync. The crudeness is the coordination guarantee.

```
genesis/a2o/features/**            ← live: Jenkins globs this, planner sees it
genesis/a2o/held/features/**       ← held: OUTSIDE the glob → structurally unrunnable
                    held/CLAUDE.md ← "STOP: needs remote-compute. See genesis/manifests/cluster-state.yaml."
```

`held/` lives **outside** the runner's scan path (`features/**`), not as a subdir of it — a tag-skip still *parses* the file, and one gherkin typo in a parsed-but-skipped feature aborts the whole E2E run with a blank report (a2o's own gotcha). Moving the file out makes it **structurally unrepresentable in the build** — the runner can no more import it than a compiler can call a function not in scope. That is the difference between a runtime guard ("don't run this") and compile enforcement ("this can't be run"). Granularity is **per-file**: `requires_env:` is one-per-file frontmatter; a file that straddles a remote-compute scenario and household-only ones must be **split** — an honest constraint, since a story either is in the executable set or isn't.

### 3.2 scope-reconcile.py (the move)

Reads `cluster-state.yaml` → available capability set. For each artifact carrying `requires_env:`:
- `requires_env ⊄ available` AND in live tree → `git mv` to `held/<same-relative-path>`.
- `requires_env ⊆ available` AND in `held/` → `git mv` back to live.
- Idempotent; round-trip is lossless (relative path preserved under `held/`).
- Emits a summary; the operator runs it after confirming a `cluster-state.yaml` edit. **Passive** — no `/converge`, no ceremony re-run.

### 3.3 Git is the scope ledger; nothing is forgotten

Each reconcile is a reviewable commit; `git log --follow held/` is the version-controlled history of *what was in scope when*. `placement-audit --held` lists the sequestered set so held work is **focused-out, never forgotten** (the comet's submerge/surface applied to scope — `memory-lifecycle-design`, D4). Expanding = moving back = also a commit.

## 4. Part B — Content-addressed citations (what makes the move safe)

### 4.1 The collision

`memory-coherence-audit.py`'s `cite_resolves()` is `p.exists()` (line 106) — pure path existence. A tool whose job is *moving files* would manufacture a `DEAD-CITE` storm on every scope change, fighting the auditor. Path is *location*; a citation wants *identity*. Part A cannot ship safely without Part B.

### 4.2 Owned by the companion spec

The link primitive (slug-identity + CID-fingerprint + envelope `desc`), the `cite_resolves()` audit upgrade (`HELD-CITE` ≠ `DEAD-CITE`), the `cite-gen` tool, soft/hard enforcement, and the corpus migration are specified in **[Semantic-Computable Links](2026-06-02-semantic-computable-links-design.md)** — this reconciler's enabling dependency. In one line: a cite carries a stable slug (survives the move) + a CID fingerprint (change-detect) + a one-sentence label, and the audit resolves by slug across live **and** `held/`, so a held target reads as `HELD-CITE`, never `DEAD-CITE`. That spec's gap-items must be built before (or alongside) this reconciler's move-mechanics — moving files is only safe once links survive moves.

## 5. Part C — The propose gesture (v2; deferred)

The runtime probe already *observed* reality: `genesis/Jenkinsfile`'s `Probe Substrate` writes `substrate-status.json`. v2 surfaces that artifact to the planning layer as a **proposed** `cluster-state.yaml` diff (fetched via Jenkins artifact, never by kubectl-from-dev — `feedback_no_kubectl_from_dev_env`): drift shown in the SessionStart headline, operator confirms (probe proposes, operator pins), then runs `scope-reconcile.py`. This closes the full reconciliation loop (substrate → observe → manifest → reconcile → glob+planner). **Out of v1** — the core (cluster-state + reconcile + held tree) delivers narrow/expand the plate with zero Jenkins dependency; v2 only removes the manual "check kubectl, hand-edit" step.

## 6. Scope & non-goals

- **v1 = artifacts only** (`.feature` stories + design specs/plans). Code-layer compile-enforcement (Rust/TS `cfg`/feature gates so capability-dependent code won't build for a household-only target) is a **non-goal** here — separate, heavier, bigger blast radius.
- **v1 = passive + operator-confirmed.** No event-driven push; no cartographer re-rank (the plate refocuses by what's *visible*, not by re-scoring).
- The propose gesture (§5) is **v2**.

## 7. Decomposition (gap-items)

- [ ] `cluster-state.yaml` capability-rename + `provided-by` (§2.2), and migrate the two artifacts saying `requires_env:[shem]` → `[remote-compute]` (`iroh-recovery-e2e.md`, `PLACEMENT.md`).
- [ ] `scope-reconcile.py` — read cluster-state, `git mv` live↔held by `requires_env` subset check, idempotent, lossless round-trip, move summary (§3.2).
- [ ] `held/` tree convention OUTSIDE the runner glob + STOP `CLAUDE.md` template + glob-exclusion verification (§3.1).
- [ ] `placement-audit --held` listing so held work is focused-out, never forgotten (§3.3).
- [ ] Per-file `requires_env` frontmatter contract + straddle-split lint — the file is the granularity unit (§3.1).
- [ ] **Dependency:** build [Semantic-Computable Links](2026-06-02-semantic-computable-links-design.md) (content-addressed cites) first — moves are unsafe until links survive moves; its gap-items own the doc-id / envelope / `cite_resolves` work (§4.2).
- [ ] `PLACEMENT.md` amendment — held/ doctrine (move vs compute label) + capability `requires_env` vocabulary (§2.2/§3).

## 8. Open questions

- **Slug allocation**: human-chosen slug vs derived-from-title? (EPR uses readable slugs; collisions need a guard.)
- **Fingerprint scope**: hash the whole file, or a canonical content region (so a frontmatter edit doesn't trip `STALE-CANDIDATE`)?
- **held/ root granularity**: one `held/` per artifact tree (`a2o/held/`, `docs/superpowers/held/`) vs a single repo-root `held/` mirror?
- **Specs that cite a held spec**: does the citing spec itself get held, or just carry a `HELD-CITE`? (Lean: carry the note; holding is driven only by `requires_env`, not by transitive citation.)
