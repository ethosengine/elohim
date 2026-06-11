# Disposition map — steward/node island recompose (session 6, fan-out C)

Drafted 2026-06-11. File → recomposed-into map + retirement list for the
operator gate. Liveness verdicts per SESSION-STATE.md Phase-0 table (verified
against the BUILD GRAPH — Cargo.toml/lock, src/ tree, wc -l — not prose).

## 1. File dispositions

| File | Disposition | Recomposed into | Gate action |
|---|---|---|---|
| `steward/node/ARCHITECTURE.md` (524 lines, design-era) | **RETIRE (gate)** | (a) crate-mechanics truth → `steward/node/CLAUDE.md` (NODE-GOSPEL, id `steward-node-gospel` — PENDING, placed this session); (b) designed-vs-shipped story (protocol rename, stub museum, SQL divergence, philosophy-inversion vs hub-optional floor) → `genesis/docs/content/elohim-protocol/history/2026-06-11-elohim-node-architecture-founding-arc.md` (NODE-ARC — PENDING, fan-out B) + INDEX.md row; (c) hub composition / two-swarms / crate-map → ALREADY owned by `genesis/docs/content/elohim-protocol/architecture/2026-05-02-elohim-hub-boundaries-design.md` (EPIC — EXISTS, verified 28360 bytes; no new absorption needed) | `git rm` after (a)+(b) verified placed; then strike 5B-§F3 (COORD-2) |
| `steward/node/README.md` | **STAYS — reconciled in place** | n/a (14 true-regardless edits in `draft-readme-edits.md`: 2 broken-link repairs, 2× 0.53→0.54, 5 tree corrections, 1 config-sample fix, 1 status bullet, R14 gospel pointer) | apply edits in Phase 2 (gospel placed first — R14 ordering) |
| `steward/node/simulation/P2P-COMPUTE-FOOTPRINT.md` | **STAY** (verdict: dated-but-honest 2026-04 analysis colocated with the live harness it operates; relocation would orphan Phase-1 testnet instructions and break census row 15 routing — `simulation-disposition.md` §1-3) | n/a — gospel concern-routing line routes to it (PLACED, with dated-2026-04 qualifier) | none |
| `steward/node/simulation/README.md` | **STAY** (structurally current; one aspirational edge: gRPC port maps point at a TODO stub — `simulation-disposition.md` §4, operator's call) | n/a | none (optional one-line gRPC-port note) |
| `steward/node/CLAUDE.md` | **CREATED (this session, additive) — PLACED + sealed + described** | n/a — new gospel, id `steward-node-gospel`; cite-gen gate green | done |
| `steward/node/src/main.rs` (L9 doc-comment) | STAYS (code) | n/a | deferred repair A1 (`deferred-ref-repairs.md`): `ARCHITECTURE.md` → `CLAUDE.md` |

## 2. Retirement list (operator gate)

1. `steward/node/ARCHITECTURE.md` — sole retiree this island. Blast radius
   TINY (sweep-verified this session): recipe:225 (ceremony), census:70
   (registry flip below), drain-groups.json:1 (dated, no-action),
   elohim/holochain/docs/ARCHITECTURE.md:115+262 (itself retiring, session 5),
   steward/node/README.md:391 (repaired by R14), steward/node/src/main.rs:9
   (deferred A1).

## 3. DEFERRED registry flips (describe-exactly notes for the gate)

### 3a. Census row 15 — `genesis/data/timeline/backlog/subject-routing-locus-census.md:70`

Current row (verbatim):
`| 15 | \`steward/node\` | true-locus | protocol-canonical | self | plain | id; route \`ARCHITECTURE\`, \`P2P-COMPUTE-FOOTPRINT\` | low |`

Gate should change (matching the row-✅ house style at census L54):
- **Gospel column**: `plain` → `**id'd \`steward-node-gospel\`**`
- **Action column**: `id; route \`ARCHITECTURE\`, \`P2P-COMPUTE-FOOTPRINT\``
  → `DONE — gospel placed; \`ARCHITECTURE\` retired (history arc
  2026-06-11-elohim-node-architecture-founding-arc); \`P2P-COMPUTE-FOOTPRINT\`
  routed via gospel` — the P2P-COMPUTE-FOOTPRINT clause is CONDITIONAL on the
  simulation agent's STAY verdict; if it moves/retires instead, say where.
- (`#` column: optionally flip `15` → `✅` per the house DONE convention — operator's call.)

### 3b. steward/.claude/subject-routing.yaml — DOES NOT EXIST (ls-verified)

No routing yaml exists anywhere under `steward/` (census-only registry,
SESSION-STATE.md L49). The gate should NOT create one as part of this island —
the census row + gospel concern-routing section carry the routing. If a
steward routing yaml ever materializes, its `steward/node` entry must route
`ARCHITECTURE` questions → `steward/node/CLAUDE.md` + NODE-ARC, and
`P2P-COMPUTE-FOOTPRINT` → per the simulation verdict. Record only; no action.

## 4. Verification checklist for the gate (Phase 4)

- [ ] `ls steward/node/CLAUDE.md` → EXISTS (NODE-GOSPEL placed)
- [ ] `ls genesis/docs/content/elohim-protocol/history/2026-06-11-elohim-node-architecture-founding-arc.md` → EXISTS (NODE-ARC placed) + INDEX.md row present
- [ ] README edits R1-R14 applied; `grep -n 'P2P-DATAPLANE\|REACH.md\|0\.53\|ARCHITECTURE\.md' steward/node/README.md` → only the CLAUDE.md pointer remains (R14), zero broken refs
- [ ] cite-gen --verify on gospel + README + arc
- [ ] COORD-1: session 5's gate skips E1 (README:53 already repaired)
- [ ] COORD-2: 5B §F3 struck (citer retired) — or applied as written if retirement declined
- [ ] main.rs:9 repair A1 applied WITH the retirement commit, not before
- [ ] Census row 15 flipped per §3a; simulation verdict folded in first
