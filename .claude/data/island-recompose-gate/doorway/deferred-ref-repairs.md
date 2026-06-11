# Deferred Reference Repairs — doorway-service island retirement (2026-06-11)

Apply AFTER the operator gate approves `git rm` of the 7 docs in `doorway/doorway-service/`
(ARCHITECTURE.md, FEDERATION.md, SCALING.md, REACH.md, RECOVERY-PROTOCOL.md, RECOVERY-SPRINT-PLAN.md, EDGE-DESIGN.md).

Placeholders (now resolved — substitute at apply time):
- `<SCALING-SEED-PATH>` = `genesis/docs/content/elohim-protocol/architecture/2026-06-11-doorway-two-axis-scaling.md`
- `<HISTORY-CONSOLIDATION-FEDERATION-PATH>` = `genesis/docs/content/elohim-protocol/history/2026-06-11-doorway-consolidation-federation-arc.md`
- `<HISTORY-RECOVERY-ARC-PATH>` = `genesis/docs/content/elohim-protocol/history/2026-06-11-doorway-recovery-protocol-arc.md`

Every old-text block below was verified verbatim against the file on 2026-06-11. All quoted line
numbers are as of commit-time of this prep; match on text, not line number.

NOTE: `doorway/CLAUDE.md` and `doorway/doorway-service/CLAUDE.md` are managed surfaces — apply those
two edits through the cite tooling discipline (seal/describe/propagate), not raw Edit.

---

## 1. GOSPEL REWRITES

### 1.1 `doorway/CLAUDE.md` — "## Reference Documentation" (lines 150–159, full section rewrite)

**OLD (exact, end of file):**

```markdown
## Reference Documentation

Detailed design docs live in `doorway-service/`:
- `ARCHITECTURE.md` — Component-level details: bootstrap, signal, gateway, cache, resolver
- `FEDERATION.md` — Cross-doorway patterns, DID discovery, P2P bootstrap role
- `SCALING.md` — Two-axis scaling model, graduation flywheel, K8s modeling
- `REACH.md` — Reach enforcement rules, caching, DNA integration
- `RECOVERY-PROTOCOL.md` — Social recovery, shard distribution, agency restoration
- `RECOVERY-SPRINT-PLAN.md` — Recovery protocol implementation phases
- `genesis/graphos/vocabulary.md` — Storage and distribution vocabulary register
```

**NEW (reworked 2026-06-11 — hub-edge spec was deleted in 53190a234; replaced with existing targets):**

```markdown
## Reference Documentation

Component design lives in this file (architecture, trust model, routing, two scaling axes, federation,
reach enforcement) and in `doorway-service/CLAUDE.md` (Rust implementation orientation). The former
`doorway-service/` design-doc island (ARCHITECTURE, FEDERATION, SCALING, REACH, RECOVERY-PROTOCOL,
RECOVERY-SPRINT-PLAN, EDGE-DESIGN) was retired to git 2026-06-11. Deeper design:
- `genesis/docs/content/elohim-protocol/architecture/2026-06-11-doorway-two-axis-scaling.md` — scaling model seed (two-axis model, graduation flywheel, conductor pool, K8s modeling)
- `genesis/docs/content/elohim-protocol/history/2026-06-11-doorway-consolidation-federation-arc.md` — gateway consolidation + federation arc (history)
- `genesis/docs/content/elohim-protocol/history/2026-06-11-doorway-recovery-protocol-arc.md` — recovery protocol arc (history)
- `genesis/docs/content/elohim-protocol/resilience/` — resilience canon (doorway docs are derived from these epics)
- `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md` — recovery protocol, current design
- `genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md` — access tiers (anon / hosted / steward-via-web)
- `genesis/graphos/vocabulary.md` — Storage and distribution vocabulary register
```

### 1.2 `doorway/doorway-service/CLAUDE.md` — "## Design Documentation" sibling list (lines 73–81)

Keep the "Upward anchors" subsection that follows — it stays valid unchanged.

**OLD (exact):**

```markdown
## Design Documentation

Sibling design docs:
- `ARCHITECTURE.md` — Bootstrap, signal, gateway, cache, resolver components
- `FEDERATION.md` — Cross-doorway patterns, DID discovery, P2P bootstrap
- `SCALING.md` — Two-axis scaling, graduation flywheel, K8s modeling
- `REACH.md` — Reach enforcement rules, caching, DNA integration
- `RECOVERY-PROTOCOL.md` — Social recovery and shard distribution
- `RECOVERY-SPRINT-PLAN.md` — Recovery protocol implementation phases
```

**NEW (repo-root paths used deliberately; the surviving "Upward anchors" list keeps its `../../` style) (reworked 2026-06-11 — hub-edge spec deleted):**

```markdown
## Design Documentation

The sibling design-doc island was retired to git 2026-06-11. Component design lives in `../CLAUDE.md`
and this file; deeper design:
- `genesis/docs/content/elohim-protocol/architecture/2026-06-11-doorway-two-axis-scaling.md` — scaling model (two-axis, graduation flywheel, K8s modeling)
- `genesis/docs/content/elohim-protocol/history/2026-06-11-doorway-consolidation-federation-arc.md` — gateway consolidation + federation arc (history)
- `genesis/docs/content/elohim-protocol/history/2026-06-11-doorway-recovery-protocol-arc.md` — recovery protocol arc (history)
- `genesis/docs/content/elohim-protocol/resilience/` — resilience canon (doorway service scenarios are derived from these epics)
- `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md` — recovery protocol, current design
- `genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md` — access tiers
```

---

## 2. GENESIS CANON

### 2.1 `genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md`

Four spots. All four are live-canon pointers in an actively load-bearing architecture doc —
**none qualifies as lineage-prose; all four get repairs.**

**Spot A (line 69) — JWT contract pointer.**

OLD (exact):

```markdown
**Where the gate lives:** Same `app_auth.rs` but with `authenticated=true` and `agent_pub_key` populated. The doorway's JWT contract is in `doorway/doorway-service/src/routes/auth_routes.rs` and `doorway/doorway-service/RECOVERY-PROTOCOL.md`.
```

NEW:

```markdown
**Where the gate lives:** Same `app_auth.rs` but with `authenticated=true` and `agent_pub_key` populated. The doorway's JWT contract is in `doorway/doorway-service/src/routes/auth_routes.rs`; the recovery-side design is `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md` (`RECOVERY-PROTOCOL.md` retired to git 2026-06-11 — arc preserved in `<HISTORY-RECOVERY-ARC-PATH>`).
```

**Spot B (line 91) — "Where this lives today" present-tense reference.**

OLD (exact):

```markdown
**Where this lives today:** Mostly vision. `RECOVERY-PROTOCOL.md` describes the social-recovery flows. The conductor-proxy code path is **not implemented**. This is what Pattern Recovery (below) ships.
```

NEW:

```markdown
**Where this lives today:** Mostly vision. The social-recovery flows are designed in `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md` (the original `RECOVERY-PROTOCOL.md` was retired to git 2026-06-11 — arc preserved in `<HISTORY-RECOVERY-ARC-PATH>`). The conductor-proxy code path is **not implemented**. This is what Pattern Recovery (below) ships.
```

**Spot C (line 284) — Related-artifacts REACH bullet.** (Note: the current text says `doorway/REACH.md`
but the file actually lives at `doorway/doorway-service/REACH.md` — the path was already loose.)

OLD (exact):

```markdown
- `doorway/REACH.md` — the reach-gate primitive
```

NEW:

```markdown
- `doorway/CLAUDE.md` §Reach Enforcement — the reach-gate primitive (`REACH.md` retired to git 2026-06-11)
```

**Spot D (line 285) — Related-artifacts RECOVERY-PROTOCOL bullet.**

OLD (exact):

```markdown
- `doorway/doorway-service/RECOVERY-PROTOCOL.md` — recovery vision
```

NEW:

```markdown
- `<HISTORY-RECOVERY-ARC-PATH>` — recovery vision arc (`RECOVERY-PROTOCOL.md` retired to git 2026-06-11; current design: `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md`)
```

### 2.2 `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md` (line 13)

Append-only repair — the "Builds on" lineage stays, gains a retirement note.

OLD (exact):

```markdown
**Builds on:** `doorway/doorway-service/RECOVERY-PROTOCOL.md` (Jan 2026) phase boundaries
```

NEW:

```markdown
**Builds on:** `doorway/doorway-service/RECOVERY-PROTOCOL.md` (Jan 2026) phase boundaries (retired to git 2026-06-11 — arc preserved in `<HISTORY-RECOVERY-ARC-PATH>`)
```

### 2.3 `genesis/docs/superpowers/specs/2026-06-10-deterministic-reach-archetype-floor-design.md` (line 47, table row)

The row cites `REACH.md` as a live encoding site of the geographic reach vocabulary. The vocabulary
analysis survives in `doorway/CLAUDE.md` §Reach Enforcement and the reconciliation strand.

OLD (exact, one table row):

```markdown
| `doorway/.../cache/access_control.rs` (+ `doorway/CLAUDE.md`, `REACH.md`) | a **geographic** vocabulary (invited/local/neighborhood/municipal/bioregional/regional) | deny-by-default for six of eight core levels; only `commons` is anon-servable today |
```

NEW:

```markdown
| `doorway/.../cache/access_control.rs` (+ `doorway/CLAUDE.md` §Reach Enforcement; `REACH.md` retired to git 2026-06-11 → `genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md`) | a **geographic** vocabulary (invited/local/neighborhood/municipal/bioregional/regional) | deny-by-default for six of eight core levels; only `commons` is anon-servable today |
```

### 2.4 `genesis/data/timeline/backlog/subject-routing-locus-census.md` — three edits

**Edit A — row 12 (line 67): mark DONE, mirroring the lamad ✅ row format.** (The gospel is already
id'd `doorway-service-gospel` and cites `resilience-protocol-spec` — see the census's own table at
line 127 — so `plain` is also stale.)

OLD (exact):

```markdown
| 12 | `doorway/doorway-service` | true-locus | protocol-canonical | **refinements OF** genesis resilience spec (derived_from) | plain | id; route FEDERATION/RECOVERY-PROTOCOL as derived, not canonical | med |
```

NEW:

```markdown
| ✅ | `doorway/doorway-service` | true-locus | protocol-canonical | **refinements OF** genesis resilience spec (derived_from) | **id'd `doorway-service-gospel`** | DONE — island RECOMPOSED 2026-06-11 | — |
```

**Edit B — still-to-route list (lines 136–138): drop doorway-service.**

OLD (exact):

```markdown
(`avodah`/`elohim`/`infrastructure`/`mishpat`/`steward-node`). Island docs still to route+retire:
elohim-storage {EDGE/P2P-ARCHITECTURE, REACH}, doorway-service {ARCHITECTURE, FEDERATION, SCALING, RECOVERY-*,
EDGE-DESIGN}, holochain {LINK_ARCHITECTURE}.
```

NEW:

```markdown
(`avodah`/`elohim`/`infrastructure`/`mishpat`/`steward-node`). Island docs still to route+retire:
elohim-storage {EDGE/P2P-ARCHITECTURE, REACH}, holochain {LINK_ARCHITECTURE}.
```

**Edit C — insert a RECOMPOSED section after the Lamad proving-rep section (after line 145),
mirroring its format.**

OLD (exact, the insertion anchor):

```markdown
Recipe + firing order for the remaining pillar islands: `pillar-island-recompose-recipe.md` (this dir).

### No-gospel homes resolved — 2026-06-11 (layered-drift disposition)
```

NEW:

```markdown
Recipe + firing order for the remaining pillar islands: `pillar-island-recompose-recipe.md` (this dir).

### Doorway-service island RECOMPOSED — 2026-06-11

`doorway/doorway-service/` design docs (7 files: ARCHITECTURE, FEDERATION, SCALING, REACH,
RECOVERY-PROTOCOL, RECOVERY-SPRINT-PLAN, EDGE-DESIGN) retired to git after full recomposition:
1 architecture seed (`<SCALING-SEED-PATH>`), 2 history records (`<HISTORY-CONSOLIDATION-FEDERATION-PATH>`,
`<HISTORY-RECOVERY-ARC-PATH>`); component design absorbed into `doorway/CLAUDE.md` +
`doorway/doorway-service/CLAUDE.md`; canon anchors: hub-edge spec (2026-05-08), recovery
phase-2-revised spec (2026-04-22), access-tier patterns (2026-05-23).

### No-gospel homes resolved — 2026-06-11 (layered-drift disposition)
```

---

## 3. MANIFESTS

### 3.1 `genesis/orchestrator/manifests/doorway/README.md` (line 14)

OLD (exact):

```markdown
See `doorway/SCALING.md` for the full scaling model (graduation flywheel, conductor pool, human topology).
```

NEW:

```markdown
See `<SCALING-SEED-PATH>` for the full scaling model (graduation flywheel, conductor pool, human topology).
```

### 3.2 `genesis/orchestrator/manifests/doorway/{staging.yaml:12, alpha.yaml:12, prod.yaml:31}` — identical comment in 3 files

Checked `staging-read.yaml` and `alpha-b.yaml` as instructed: **neither contains a SCALING.md
reference** — no edit needed there. Apply the same one-line edit in each of the three files
(staging.yaml line 12, alpha.yaml line 12, prod.yaml line 31):

OLD (exact, per file):

```yaml
# See doorway/SCALING.md for the full model.
```

NEW:

```yaml
# See <SCALING-SEED-PATH> for the full model.
```

### 3.3 `elohim/elohim-hub/README.md` (line 54)

EDGE-DESIGN.md was itself a pointer-doc to the hub-edge spec — but that spec was DELETED in
`53190a234` (same rot class as the original §1.1/§1.2 blocks; reworked 2026-06-11). Route to the
parent doorway gospel, which owns the doorway-side framing (gateway role, no-per-domain-proxy rails).

OLD (exact):

```markdown
The doorway is one role a hub can take, not a mandatory layer. See `doorway/doorway-service/EDGE-DESIGN.md` for the doorway-side framing.
```

NEW:

```markdown
The doorway is one role a hub can take, not a mandatory layer. See `doorway/CLAUDE.md` for the doorway-side framing (the `EDGE-DESIGN.md` pointer-doc was retired to git 2026-06-11; design lineage in `genesis/docs/content/elohim-protocol/history/2026-06-11-doorway-consolidation-federation-arc.md`).
```

---

## 4. CROSS-ISLAND NOTES (holochain + elohim-storage doc islands)

**Shared observation:** every `./doorway/*.md` link in `elohim/holochain/docs/` is ALREADY a dead
relative link — `elohim/holochain/docs/doorway/` does not exist, so these resolve nowhere today.
The retirement only removes the conceptual target. All of these files belong to the HOLOCHAIN island
(README/ARCHITECTURE/P2P-DATAPLANE/REACH are on or adjacent to the still-to-route list); a sibling
holochain-island session may retire them wholesale. Minimal repairs below in case those files survive.
**Each edit: cross-island — may be mooted by the holochain-island session.**

### 4.1 `elohim/holochain/docs/README.md` (lines 108–109, table rows)

OLD (exact, two rows — line 109 is an extra found beyond the task list, same treatment):

```markdown
| [doorway/FEDERATION.md](./doorway/FEDERATION.md) | Doorway federation, DIDs, P2P bootstrap role |
| [doorway/ARCHITECTURE.md](./doorway/ARCHITECTURE.md) | Doorway internals, routes, caching |
```

NEW:

```markdown
| `<HISTORY-CONSOLIDATION-FEDERATION-PATH>` | Doorway federation, DIDs, P2P bootstrap role (FEDERATION.md retired to git 2026-06-11) |
| `doorway/CLAUDE.md` | Doorway internals, routes, caching (ARCHITECTURE.md retired to git 2026-06-11) |
```

### 4.2 `elohim/holochain/docs/README.md` (line 126, reading order)

OLD (exact):

```markdown
5. **doorway/FEDERATION.md** - Understand doorway's role
```

NEW:

```markdown
5. **`doorway/CLAUDE.md`** (+ `<HISTORY-CONSOLIDATION-FEDERATION-PATH>`) - Understand doorway's role
```

### 4.3 `elohim/holochain/docs/ARCHITECTURE.md` (lines 263–265)

OLD (exact, three bullets — 263 and 265 are extras found beyond the task list, same treatment):

```markdown
- [doorway/ARCHITECTURE.md](./doorway/ARCHITECTURE.md) - Gateway details
- [doorway/FEDERATION.md](./doorway/FEDERATION.md) - Multi-doorway + P2P bootstrap
- [doorway/REACH.md](./doorway/REACH.md) - Doorway reach enforcement
```

NEW:

```markdown
- `doorway/CLAUDE.md` - Gateway details (ARCHITECTURE.md retired to git 2026-06-11)
- `<HISTORY-CONSOLIDATION-FEDERATION-PATH>` - Multi-doorway + P2P bootstrap (FEDERATION.md retired to git 2026-06-11)
- `doorway/CLAUDE.md` §Reach Enforcement - Doorway reach enforcement (REACH.md retired to git 2026-06-11)
```

### 4.4 `elohim/holochain/docs/P2P-DATAPLANE.md` (line 434, table row)

OLD (exact):

```markdown
| [doorway/FEDERATION.md](./doorway/FEDERATION.md) | Cross-doorway communication |
```

NEW:

```markdown
| `<HISTORY-CONSOLIDATION-FEDERATION-PATH>` | Cross-doorway communication (FEDERATION.md retired to git 2026-06-11) |
```

### 4.5 `elohim/holochain/docs/REACH.md` (lines 120 and 210) — extras found beyond the task list

OLD (exact, line 120):

```markdown
See [doorway/REACH.md](./doorway/REACH.md) for implementation details.
```

NEW:

```markdown
See `doorway/CLAUDE.md` §Reach Enforcement for implementation details (doorway REACH.md retired to git 2026-06-11).
```

OLD (exact, line 210):

```markdown
- [doorway/REACH.md](./doorway/REACH.md) - Doorway's role in reach enforcement
```

NEW:

```markdown
- `doorway/CLAUDE.md` §Reach Enforcement - Doorway's role in reach enforcement (REACH.md retired to git 2026-06-11)
```

### 4.6 `elohim/elohim-storage/REACH.md` (line 371) — extra found beyond the task list

Cross-island: `elohim/elohim-storage/REACH.md` is itself on the census still-to-route list — may be
mooted by an elohim-storage island session. The relative link is also already broken
(`../doorway/REACH.md` resolves to nonexistent `elohim/doorway/REACH.md`).

OLD (exact):

```markdown
- [../doorway/REACH.md](../doorway/REACH.md) - Doorway reach enforcement
```

NEW:

```markdown
- `doorway/CLAUDE.md` §Reach Enforcement - Doorway reach enforcement (doorway REACH.md retired to git 2026-06-11)
```

---

## 5. LINEAGE VERDICTS (no repair — prose stays valid)

- **`.claude/sprints/scaling-identity.md:4`** — `> **Context**: [doorway/SCALING.md](../../doorway/SCALING.md) — the dual-axis scaling model`.
  **Verdict: LINEAGE, stays.** Archived sprint-plan header recording what informed the sprint at
  authoring time. The dual-axis model survives in `doorway/CLAUDE.md` §Two Scaling Axes and
  `<SCALING-SEED-PATH>`; the dangling link in a sprint archive is acceptable historical record.
- **`.claude/sprints/decoupling-p2p-cicd-complete.md:221`** — `doorway/SCALING.md` appears in a
  completed sprint's touched-files inventory. **Verdict: LINEAGE, stays.** Historical record of what
  that sprint modified.

## 6. OBSERVATIONS (pre-existing, NOT caused by this retirement)

- **`steward/node/README.md:202`** — `See [REACH.md](../REACH.md) for enforcement details.` resolves
  to `steward/REACH.md`, which does not exist. Pre-existing dead link, unrelated to the doorway
  retirement (it references a never-written steward-side reach doc, not the doorway one). No repair
  here; flag for a future steward-island session.
- `genesis/data/timeline/backlog/pillar-island-recompose-recipe.md` lines ~164–174 enumerate the
  7-doc island as the session's work plan — the recompose session itself updates the recipe; not a
  dead-ref repair.
- No references to the 7 docs exist in `doorway/README.md`, `doorway/doorway-app/`, or
  `doorway/doorway-service/src/` (verified by grep) — the code tree is clean of doc citations.
