# Deferred ref-repairs inventory — holochain docs/ island retirement (pass B, operator-gated)

Re-swept 2026-06-11 (this session): `grep -rn` repo-wide for `P2P-DATAPLANE`,
`SYNC-ENGINE`, `COMMUNITY-COMPUTE`, `ARCHITECTURE-GAP`, `DEPLOYMENT-RUNTIMES`,
`docs/REACH`, `docs/claude.md`, `docs/DEVELOPMENT`, `holochain/docs`, excluding
`.git`, `node_modules`, `target`, `dist`, `.angular`. Retiring set: 8 docs in
`elohim/holochain/docs/` (ARCHITECTURE, ARCHITECTURE-GAP, COMMUNITY-COMPUTE,
DEPLOYMENT-RUNTIMES, DEVELOPMENT, P2P-DATAPLANE, REACH, SYNC-ENGINE) + the
legacy lowercase `claude.md`; `docs/README.md` STAYS (rewritten — see
`draft-readme-reconciled.md` in this folder). All repairs below PRESUPPOSE the
retirement unless marked apply-now — apply at the operator gate, not before.

**Replacement targets legend:**
- `CC-ARC` = `genesis/docs/content/elohim-protocol/history/2026-06-11-community-compute-founding-vision-arc.md` — **PENDING placement** (exact final name may shift; re-verify with `ls` at apply time)
- `DS-ARC` = `genesis/docs/content/elohim-protocol/history/2026-06-11-p2p-dataplane-sync-engine-design-arc.md` — **PENDING placement** (same caveat)
- `DNA-GOSPEL` = `elohim/holochain/dna/CLAUDE.md` — **EXISTS** (verified; id holochain-integrity-layer-gospel)
- `README-R` = `elohim/holochain/docs/README.md` reconciled body — **EXISTS at gate** (rewritten in this recompose)
- `STRAND` = `genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md` — **EXISTS** (verified)
- `STORAGE-GOSPEL` = `elohim/elohim-storage/CLAUDE.md` — **EXISTS** (verified; carries §P2P Data Plane & Reach concern routing)
- `REACH-SCHEMA` = `elohim/sdk/schemas/v1/enums/reach.schema.json` + `elohim/epr/src/reach.rs` — **EXIST** (verified)

---

## A. Live skills (repair pairs)

### A1. .claude/skills/automerge-sync/SKILL.md

**L249** — class: table row (Key Files)
- Current: `| \`elohim/holochain/docs/SYNC-ENGINE.md\` | Primary design document |`
- Proposed: `| \`genesis/docs/content/elohim-protocol/history/2026-06-11-p2p-dataplane-sync-engine-design-arc.md\` | Design history (the retired SYNC-ENGINE.md; live sync behavior is THIS skill) |`
- Target: DS-ARC — PENDING placement.

**L255** — class: table row (Key Files)
- Current: `| \`elohim/holochain/docs/P2P-DATAPLANE.md\` | Overall P2P architecture |`
- Proposed: `| \`elohim/elohim-storage/CLAUDE.md\` | Live P2P data-plane truth (§P2P Data Plane & Reach; design history: the dataplane+sync arc row above) |`
- Target: STORAGE-GOSPEL — EXISTS. (Avoids two rows pointing at the same arc.)

### A2. .claude/skills/libp2p-discovery/SKILL.md

**L307** — class: table row
- Current: `| \`elohim/holochain/docs/P2P-DATAPLANE.md\` | Bootstrap flow, architecture |`
- Proposed: `| \`genesis/docs/content/elohim-protocol/history/2026-06-11-p2p-dataplane-sync-engine-design-arc.md\` | Bootstrap-flow design history (retired P2P-DATAPLANE.md); live data-plane truth: \`elohim/elohim-storage/CLAUDE.md\` |`
- Targets: DS-ARC — PENDING placement; STORAGE-GOSPEL — EXISTS.

### A3. .claude/skills/libp2p-transport/SKILL.md

**L256** — class: table row
- Current: `| \`elohim/holochain/docs/P2P-DATAPLANE.md\` | Architecture design document |`
- Proposed: `| \`genesis/docs/content/elohim-protocol/history/2026-06-11-p2p-dataplane-sync-engine-design-arc.md\` | Architecture design history (the retired P2P-DATAPLANE.md) |`
- Target: DS-ARC — PENDING placement.

### A4. .claude/skills/holochain-storage-api/SKILL.md

**L330** — class: table row
- Current: `| \`elohim/holochain/docs/ARCHITECTURE.md\` | Overall architecture |`
- Proposed: `| \`elohim/holochain/docs/README.md\` | Holochain-layer pointer index (ARCHITECTURE.md retired 2026-06-11) |`
- Target: README-R — EXISTS at gate.

**L331** — class: table row
- Current: `| \`elohim/holochain/docs/P2P-DATAPLANE.md\` | P2P data plane design |`
- Proposed: `| \`elohim/elohim-storage/CLAUDE.md\` | Live P2P data-plane truth (design history: \`genesis/docs/content/elohim-protocol/history/2026-06-11-p2p-dataplane-sync-engine-design-arc.md\`) |`
- Targets: STORAGE-GOSPEL — EXISTS; DS-ARC — PENDING placement.
- ADJACENT (true-regardless, optional): **L328** `| \`holochain/sdk/storage-client-ts/src/generated/\` | ...` is a stale pre-`elohim/` spelling → `elohim/sdk/storage-client-ts/src/generated/` (live dir, verified). Not island-inbound; fix while in the file.

---

## B. Agent definition

### B1. .claude/agents/rust-architect.md

**L562** — class: prose (reference list; CHECK the managed-surface registry — `.claude/scripts/_lib/managed_surfaces.py` — before editing; route through cite tooling if registered)
- Current: `- \`elohim/holochain/docs/claude.md\` (infrastructure guide)`
- Proposed: `- \`elohim/holochain/dna/CLAUDE.md\` (integrity-layer gospel — link budget, upgrade rails) and \`elohim/holochain/docs/README.md\` (holochain-layer pointer index; docs/claude.md retired 2026-06-11)`
- Targets: DNA-GOSPEL — EXISTS; README-R — EXISTS at gate.
- COORDINATE: the dna-island pairs file (`/tmp/island-recompose-holochain-dna/deferred-ref-repairs.md` §A7) repairs the ADJACENT line L563 (LINK_ARCHITECTURE) in the same list — apply both in one pass to avoid line-number skew between the two inventories.

---

## C. Backlog entry (mark-retired phrasing)

### C1. genesis/data/timeline/backlog/deprecation-devfile-start-doorway-dead-command-retire.md

**L76** (fragment spans ~L74-78) — class: prose (sibling-concern parenthetical)
- Current (fragment): `\`doorway:start\` package-script snippet embedded in\n\`elohim/holochain/docs/claude.md:837\` points at the same dead\n\`../holochain/doorway/target/release/doorway\` path — a docs-snippet concern, not\nthis command, left out of this fingerprint's scope.)`
- Proposed: `\`doorway:start\` package-script snippet embedded in\nthe now-retired \`elohim/holochain/docs/claude.md:837\` (retired 2026-06-11,\nholochain docs island recompose) pointed at the same dead\n\`../holochain/doorway/target/release/doorway\` path — that sibling docs-snippet\nconcern is closed by the retirement.)`
- Target: none needed (statement of fact once retirement lands).

---

## D. ALREADY-BROKEN class — can apply NOW (true regardless of retirement)

### D1. elohim/holochain/dna/imagodei/STEWARDSHIP_PHILOSOPHY.md

**L1023** — class: prose. Target `../../REACH.md` resolves to
`elohim/holochain/REACH.md`, which NEVER existed at any commit (verified:
eb5b53133 moved `holochain/REACH.md` directly to `elohim/holochain/docs/REACH.md`;
the ref was VALID when authored 2026-01-13 at `holochain/dna/imagodei/` and broke
at the 2026-03 reorg).
- Current: `The protocol's reach system (see [REACH.md](../../REACH.md)) provides another structural barrier:`
- Proposed: `The protocol's reach system (vocabulary lineage + drift record: genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md; the DNA-notarized enum is elohim/sdk/schemas/v1/enums/reach.schema.json, matched by elohim/epr/src/reach.rs) provides another structural barrier:`
- Targets: STRAND — EXISTS; REACH-SCHEMA — EXISTS. Most-truthful-target rationale:
  the ladder this doc quotes immediately below (geographic 8, ordinals 0-7) is
  NOT the DNA-notarized enum — the strand entry is the only target that records
  that fact without canonizing either vocabulary. Do NOT "fix" the quoted ladder
  itself (it is the doc's own historical content).
- Class: **apply-now, true-regardless** — the link is broken whether or not the
  island retires.

---

## E. steward/node/README.md (PENDING + existence-check)

**L53** — class: prose. ALREADY-BROKEN (`../P2P-DATAPLANE.md` resolves to
`steward/P2P-DATAPLANE.md`, never existed there).
- Current: `See [P2P-DATAPLANE.md](../P2P-DATAPLANE.md) for the full architectural separation.`
- Proposed: `See \`genesis/docs/content/elohim-protocol/history/2026-06-11-p2p-dataplane-sync-engine-design-arc.md\` (design history) for the full architectural separation.`
- Target: DS-ARC — PENDING placement.
- **EXISTENCE-CHECK at apply time**: steward session 6 may rewrite this README
  wholesale. If `steward/node/README.md:53` no longer carries the line, the pair
  is moot — do not reintroduce it.

---

## F. Cross-island citers retiring in sibling sessions — NO ACTION IF SIBLING RETIRES

Each file below is slated in a SIBLING island session. At apply time,
existence-check the citer file first; if it was retired, its pairs are moot.
If it SURVIVES, apply the listed repointing. All listed relative links are
ALREADY-BROKEN at their current locations (pre-reorg `holochain/`-rooted
spellings).

### F1. doorway/doorway-service/FEDERATION.md (doorway island)
- **L3**: `> **See also**: [P2P-DATAPLANE.md](../P2P-DATAPLANE.md) for the overall P2P architecture` → DS-ARC (PENDING).

### F2. doorway/doorway-service/SCALING.md (doorway island)
- **L424**: `- **[../holochain/DEPLOYMENT-RUNTIMES.md](../holochain/DEPLOYMENT-RUNTIMES.md)** — The 4-stage agency journey` → CC-ARC (PENDING).
- OPEN QUESTION: which arc carries the 4-stage agency journey (Visitor → Hosted
  → App Steward → Node Steward)? It appears in both DEPLOYMENT-RUNTIMES.md and
  the old docs/README.md. Flag for the CC-ARC author; if CC-ARC omits it, the
  reconciled README's canon pointer is the fallback target.

### F3. steward/node/ARCHITECTURE.md (steward island; session 6 may rewrite)
- **L521**: `- [P2P-DATAPLANE.md](../P2P-DATAPLANE.md) - Overall P2P architecture` → DS-ARC (PENDING).
- **L522**: `- [SYNC-ENGINE.md](../SYNC-ENGINE.md) - Automerge sync design details` → DS-ARC (PENDING) + live behavior `.claude/skills/automerge-sync/SKILL.md`.
- **L523**: `- [elohim-storage/P2P-ARCHITECTURE.md](../elohim-storage/P2P-ARCHITECTURE.md) - Blob storage P2P` — citer-of-a-sibling-retiree (see F4); if P2P-ARCHITECTURE.md retires → STORAGE-GOSPEL.
- **L524**: `- [COMMUNITY-COMPUTE.md](../COMMUNITY-COMPUTE.md) - Family node vision` → CC-ARC (PENDING).

### F4. elohim/elohim-storage/P2P-ARCHITECTURE.md (storage island)
- **L4**: `> - [P2P-DATAPLANE.md](../P2P-DATAPLANE.md) - Overall architecture vision` → DS-ARC (PENDING).
- **L5**: `> - [SYNC-ENGINE.md](../SYNC-ENGINE.md) - Automerge sync design` → DS-ARC (PENDING).
- **L415**: `Content metadata syncs via Automerge CRDT (see [SYNC-ENGINE.md](../SYNC-ENGINE.md) for details):` → DS-ARC (PENDING) + `.claude/skills/automerge-sync/SKILL.md` for live behavior.
- **L474**: `- [P2P-DATAPLANE.md](../P2P-DATAPLANE.md) - Overall P2P architecture` → DS-ARC (PENDING).
- **L475**: `- [SYNC-ENGINE.md](../SYNC-ENGINE.md) - Automerge sync design` → DS-ARC (PENDING).

---

## G. No-action class

1. **`.claude/prompts/p2p-dataplane-sprint.md`** (L19, L45-47, L177-178) — dated
   prompt artifact using pre-reorg `holochain/...` spellings; historical record
   of a sprint brief. No-action.
2. **`genesis/data/timeline/backlog/pillar-island-recompose-recipe.md`**
   (L199-200 retiring-set listing, L205 lowercase-claude.md note) — the
   recompose's own recipe/process doc; the parent session updates it as part of
   the gate ceremony. No independent repair.
3. **`genesis/docs/claude.md` mentions** (`.claude/subject-routing.yaml:60,89,99`;
   `.claude/scripts/_lib/subject_routing.py:40`;
   `genesis/docs/superpowers/specs/2026-06-02-subject-routed-decomposition-design.md:12`)
   — a DIFFERENT, live file (`genesis/docs/claude.md` EXISTS — verified), not
   the retiring `elohim/holochain/docs/claude.md`. No-action.
4. **memory-kit dated artifacts + .claude/data ledgers** — the re-sweep found
   ZERO hits for any docs-island doc name in `.claude/memory-kit/` or
   `.claude/data/` (unlike the dna island). Nothing to mark.
5. **Internal cross-refs among the retiring docs themselves**
   (docs/ARCHITECTURE.md:3,67,254-261; docs/P2P-DATAPLANE.md:139,431-433;
   docs/SYNC-ENGINE.md:478-480; docs/COMMUNITY-COMPUTE.md:636,698;
   docs/ARCHITECTURE-GAP.md:242; docs/REACH.md:120,131,210-213) — die together
   at retirement. No repairs.
6. **docs/README.md internal pointers** (L80-81, L89, L97-100, L107, L117-118,
   L122-126) — superseded WHOLESALE by the reconciled README body
   (`draft-readme-reconciled.md`), not pairwise repairs. Evidence the rewrite is
   overdue: L107 `../elohim-node/ARCHITECTURE.md` was already broken (component
   lives at `steward/node/` now).
7. **dna-island §B3 coverage — do NOT duplicate.** The dna pairs file
   (`/tmp/island-recompose-holochain-dna/deferred-ref-repairs.md` §B3) already
   carries exact pairs for `docs/README.md:118` and `docs/ARCHITECTURE.md:268`
   (the `./dna/NETWORK_UPGRADES.md` already-broken refs). Disposition under THIS
   island: the ARCHITECTURE.md:268 pair becomes **MOOT** (the file retires
   whole); the README.md:118 pair is **superseded** by the wholesale README
   rewrite — the reconciled body carries the dna-upgrade-governance row,
   satisfying that pair's intent. Note this when applying the dna inventory.

---

## Stats

- **Repair pairs (text edits): 10** across **7 live files** (A1: 2 / A2: 1 /
  A3: 1 / A4: 2 (+1 adjacent optional) / B1: 1 / C1: 1 / D1: 1 / E1: 1).
- **Apply-now (true-regardless): 2** (D1; A4's adjacent L328 stale-path).
- **Conditional pairs (sibling-retire-dependent): 10 fragments across 4 files**
  (F1: 1 / F2: 1 / F3: 4 / F4: 5 — F3's L523 conditions on F4, not on this island).
- **No-action: 7 classes** (§G).
- **Targets:** DNA-GOSPEL, STORAGE-GOSPEL, STRAND, REACH-SCHEMA all EXIST
  (verified 2026-06-11); README-R exists at gate (this recompose); CC-ARC and
  DS-ARC are **PENDING placement** — exact final filenames may be adjusted at
  placement; re-verify with `ls` before applying any pair that names them.
