# Deferred ref-repairs inventory — steward/node island retirement (operator-gated)

Swept 2026-06-11 (this session):
`grep -rn 'steward/node/ARCHITECTURE\|node/ARCHITECTURE\.md' --exclude-dir={.git,node_modules,target,dist,.angular} .`
plus a relative-spelling sweep `grep -rn 'ARCHITECTURE\.md' steward/` and a
`grep -rn 'P2P-COMPUTE-FOOTPRINT'` sweep (same exclusions). Retiring set:
**steward/node/ARCHITECTURE.md only** (524 lines, design-era).
simulation/P2P-COMPUTE-FOOTPRINT.md verdict belongs to the simulation fan-out
agent (likely STAY) — placeholder rows below. All repairs PRESUPPOSE the
retirement unless marked otherwise — apply at the operator gate, not before.
House pattern follows /tmp/island-recompose-holochain-docs/deferred-ref-repairs.md.

**Replacement targets legend (existence-checked this session with `ls`):**
- `NODE-GOSPEL` = `steward/node/CLAUDE.md` — **EXISTS** (PLACED + sealed this
  session; id `steward-node-gospel`; cite-gen --seal gate green).
- `NODE-ARC` = `genesis/docs/content/elohim-protocol/history/2026-06-11-elohim-node-architecture-founding-arc.md`
  — **EXISTS** (PLACED + sealed this session; id
  `elohim-node-architecture-founding-arc`; INDEX.md row 13 appended).
- `DS-ARC` = `genesis/docs/content/elohim-protocol/history/2026-06-11-p2p-dataplane-sync-engine-design-arc.md`
  — **EXISTS** (verified; 14857 bytes; id `p2p-dataplane-sync-engine-design-arc`).
- `EPIC` = `genesis/docs/content/elohim-protocol/architecture/2026-05-02-elohim-hub-boundaries-design.md`
  — **EXISTS** (verified; 28360 bytes; owns hub composition / crate-map).
- `STRAND` = `genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md`
  — **EXISTS** (verified; records steward reach.rs at L27 and dormancy at L81).

---

## A. Crate-source doc comment (repair pair)

### A1. steward/node/src/main.rs

**L9** — class: code doc-comment (`//!` crate docs; safe text edit, no behavior).
- Current: `//! See README.md and ARCHITECTURE.md for details.`
- Proposed: `//! See README.md and CLAUDE.md for details.`
- Target: NODE-GOSPEL — EXISTS (placed + sealed this session).
- Class: **retirement-presupposing** — ARCHITECTURE.md is a valid pointer until
  the gate retires it.

---

## B. Cross-island citers retiring in sibling sessions — NO ACTION IF SIBLING RETIRES

### B1. elohim/holochain/docs/ARCHITECTURE.md (holochain docs island — RETIRING session 5)

- **L115**: `See [elohim-node/ARCHITECTURE.md](../elohim-node/ARCHITECTURE.md) for implementation details.`
  (already-broken pre-reorg spelling — component lives at `steward/node/` now)
- **L262**: `- [elohim-node/ARCHITECTURE.md](../elohim-node/ARCHITECTURE.md) - Infrastructure runtime details`
- Disposition: the CITER retires in session 5 → both pairs **MOOT at their
  gate**. If it somehow survives, repoint L115 → NODE-GOSPEL (PENDING) and
  L262 → NODE-GOSPEL + NODE-ARC (PENDING). Existence-check the citer first.

---

## C. No-action class

1. **`.claude/memory-kit/2026-06-03/drain-groups.json:1`** — dated memory-kit
   artifact carrying `"steward/node/ARCHITECTURE.md": {"FOLD": ["feedback_config_yaml_over_cli.md"], …}`.
   Historical drain record; no-action (house precedent: 5B §G dated-artifact class).
2. **`genesis/data/timeline/backlog/pillar-island-recompose-recipe.md:225`** —
   the recompose's own recipe/process doc (`ISLAND: steward/node/ARCHITECTURE.md (+ check simulation/ …)`);
   the parent session updates it as part of the gate ceremony. No independent repair.
3. **`genesis/data/timeline/backlog/subject-routing-locus-census.md:70`** (row 15)
   — registry flip handled as a DEFERRED note in `disposition-map.md` (this
   folder), not a pairwise repair here.
4. **ARCHITECTURE.md's OWN outbound refs, L521-524** (`Related Documentation`:
   P2P-DATAPLANE, SYNC-ENGINE, elohim-storage/P2P-ARCHITECTURE,
   COMMUNITY-COMPUTE) — die with the file at retirement. No repairs. See
   COORD-2 below for the session-5B interaction.

---

## D. P2P-COMPUTE-FOOTPRINT inbound sweep (verdict pending — placeholder)

Repo-wide sweep found exactly TWO inbound refs:
1. `genesis/data/timeline/backlog/pillar-island-recompose-recipe.md:225` — recipe class (§C2).
2. `genesis/data/timeline/backlog/subject-routing-locus-census.md:70` — census row 15 (§C3 / disposition-map).

Both are gate-ceremony surfaces; NO live-doc repointing needed regardless of
the simulation agent's verdict. If the verdict is STAY (expected per
SESSION-STATE.md L12, L57), zero pairs materialize; if RETIRE/MOVE, the only
follow-up is the census Action-column text (disposition-map §3).

---

## COORDINATION NOTES (read at gate before applying ANY inventory)

### COORD-1 — session 5B pair E1 is SATISFIED by this session

5B's `/tmp/island-recompose-holochain-docs/deferred-ref-repairs.md` §E (E1)
targets `steward/node/README.md:53` (broken `../P2P-DATAPLANE.md`). THIS
session's README reconcile applies that exact repair as edit **R1**
(`draft-readme-edits.md` in this folder), pointing at DS-ARC (now verified
EXISTS — 5B recorded it as PENDING). At session 5's gate: existence-check
README L53; it will already cite the arc — **skip E1, do not double-apply**.
(5B's own E1 text anticipated this: "steward session 6 may rewrite this
README … the pair is moot — do not reintroduce it.")

### COORD-2 — session 5B §F3 goes MOOT if ARCHITECTURE.md retires

5B §F3 inventories four repair pairs INSIDE `steward/node/ARCHITECTURE.md`
(L521-524 → DS-ARC / CC-ARC / storage-gospel). This session's disposition is
RETIRE-at-gate, which makes **all four F3 pairs MOOT** (the citer dies).
Apply order at the consolidated gate: settle this island's retirement FIRST,
then strike F3 from 5B's apply list. If the operator declines the retirement,
F3 applies as written instead.

---

## Out-of-scope observations (report-only; NOT this island's gate)

- **Root `CLAUDE.md:209`** gotcha headline `### libp2p 0.53 API (steward/node)`
  — version label stale (crate is 0.54/0.54.1 per Cargo.toml:16 + Cargo.lock).
  The API tips beneath it (macros+ed25519, `with_codec()`, `StreamExt::next()`)
  were not re-verified against 0.54 here. Managed surface — route through cite
  tooling / claude-md-audit lane, not this gate.
- **`.claude/skills/libp2p-transport/SKILL.md:3,19,24`** — frames a
  0.53(elohim-node)-vs-0.54(elohim-storage) split that is NOT live truth (both
  crates 0.54; SESSION-STATE.md L34, corrected cross-session fact d).
  Skill-audit lane.
- **`genesis/docs/content/elohim-protocol/resilience/README.md:624`** claims
  steward reach.rs is "LIVE" — overclaim (zero consumers outside reach.rs;
  STRAND L81 records dormancy; SESSION-STATE.md L40). Report-only.

---

## Stats

- **Repair pairs (text edits): 1 firm** (A1 — main.rs:9, retirement-presupposing)
  **+ 2 conditional** (B1 — moot if the holochain docs island retires its
  ARCHITECTURE.md as planned).
- **No-action: 4 classes** (§C).
- **P2P-COMPUTE-FOOTPRINT: 0 live-doc pairs** (both inbound refs are
  gate-ceremony surfaces; §D).
- **Coordination notes: 2** (COORD-1 strikes 5B-E1; COORD-2 strikes 5B-F3 on
  retirement).
- **Targets:** DS-ARC, EPIC, STRAND, NODE-GOSPEL, NODE-ARC — **ALL EXIST**
  (ls-verified 2026-06-11; gospel + arc placed and sealed this session).
