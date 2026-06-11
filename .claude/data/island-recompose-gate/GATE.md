# Six-pillar island recompose — CONSOLIDATED OPERATOR GATE

All six pillar sessions (recipe: `genesis/data/timeline/backlog/pillar-island-recompose-recipe.md`)
completed Phases 0–2 + verification on `feat/frontend-eyes-sprint`, 2026-06-11. Every placement is
committed and cite-verify green. **Nothing has been retired.** This file is the one gate: approve →
execute §Retirement below; reject any pillar → its placed docs stand (additive), only the listed
soften-edits apply.

## Placement commits (all on feat/frontend-eyes-sprint, not pushed)

| # | Pillar | Commit | Placed |
|---|--------|--------|--------|
| 1 | qahal | `bf4367e98` | history ×1, backlog ×2, gospel drift-repairs (no seed — all truth already homed) |
| 2 | elohim-pillar + shell | `949c01fc0` | history ×1, shell sub-manifest (census row 9), heavy gospel repairs (no seed) |
| 3 | storage | `c285a8299` | history ×1, backlog ×1, strand origin-note, gospel rails (no seed) |
| 4 | doorway | `8aba9c198` | **seed `doorway-two-axis-scaling`**, history ×2, backlog ×1, strand append |
| 5A | holochain dna/ | `55d0d2399` | **seed `dna-upgrade-governance`**, history ×3, backlog ×2, gospel rails |
| 5B | holochain docs/ | `45bc1bbf1` | history ×2, README reconciled in place, strand extension, ops residue, socat rationale |
| 6 | steward/node | `8704914b1` | **gospel `steward-node-gospel` CREATED** (census row 15), history ×1, README repairs ×14 (no backlog) |
| — | coordinator | `20f7aa9f6`, `2fbb0a04d` | INDEX rows ×3 (lamad/qahal), orphaned triage closures |

All 13 fan-out history records carry INDEX.md rows. Deprecation ledger: 0 open.

## Retirement list (operator approval → `git rm`, 26 files)

```
app/elohim-app/src/app/qahal/QAHAL_API_SPECIFICATION_v1.0.md
app/elohim-app/src/app/elohim/ELOHIM_PROTOCOL_ARCHITECTURE.md
app/elohim-app/src/app/elohim/ARCHITECTURE.md
elohim/elohim-storage/P2P-ARCHITECTURE.md
elohim/elohim-storage/EDGE-ARCHITECTURE.md
elohim/elohim-storage/REACH.md
doorway/doorway-service/ARCHITECTURE.md
doorway/doorway-service/FEDERATION.md
doorway/doorway-service/SCALING.md
doorway/doorway-service/REACH.md
doorway/doorway-service/RECOVERY-PROTOCOL.md
doorway/doorway-service/RECOVERY-SPRINT-PLAN.md
doorway/doorway-service/EDGE-DESIGN.md
elohim/holochain/dna/LINK_ARCHITECTURE.md
elohim/holochain/dna/NETWORK_UPGRADES.md
elohim/holochain/dna/SCHEMA_VERSIONS.md
elohim/holochain/docs/ARCHITECTURE.md
elohim/holochain/docs/ARCHITECTURE-GAP.md
elohim/holochain/docs/P2P-DATAPLANE.md
elohim/holochain/docs/COMMUNITY-COMPUTE.md
elohim/holochain/docs/SYNC-ENGINE.md
elohim/holochain/docs/DEPLOYMENT-RUNTIMES.md
elohim/holochain/docs/DEVELOPMENT.md
elohim/holochain/docs/REACH.md
elohim/holochain/docs/claude.md
steward/node/ARCHITECTURE.md
```

NOT retiring: `elohim/holochain/docs/README.md` (reconciled in place), `steward/node/README.md`
(reconciled), `steward/node/simulation/*` (stays — dated-but-honest analysis, gospel routes to it).

## Sequencing constraints

1. **qahal + elohim islands retire in ONE commit** — they cross-point at each other
   (QAHAL_API_SPECIFICATION:580 ↔ ELOHIM_PROTOCOL_ARCHITECTURE:179); separate commits open a
   dead-pointer window.
2. **Per-pillar after-retirement steps**: apply that pillar's deferred ref-repairs (this dir,
   per-pillar files — exact old/new pairs), then flip `docs_island` lines in the pillar's
   subject-routing.yaml + census rows (4, 5, 9, 10, 11, 12, 15 → done/✅), then
   `cite-gen --verify` + `locus-drift.py` stasis check.
3. **Cross-island conditionals**: holochain-docs §F3 and steward COORD strikes assume sibling
   retirement — if all 26 retire together, the conditional fragments are moot (verify per file
   notes).
4. claude.md retirement closes the tracked sibling at
   `genesis/data/timeline/backlog/deprecation-devfile-start-doorway-dead-command-retire.md:74-78`
   — mention in the retire commit body.

## Known flaws to fix BEFORE applying deferred repairs

- **doorway/deferred-ref-repairs.md §1.1/§1.2**: ~~repoint "Reference Documentation" at the hub-edge
  spec (`2026-05-08-doorway-hub-edge-design.md`) — that spec was DELETED in `53190a234`. Rework
  those two blocks (suggested target: doorway gospel + the two history arcs + the two-axis seed).~~
  **RESOLVED** (commit `docs(gate): rework doorway ref-repair blocks 1.1/1.2`): replaced dead
  hub-edge spec pointer with the two-axis-scaling seed, two history arcs, resilience canon dir,
  recovery-phase-2-revised spec, and access-tier-patterns — all verified to exist.
- **NEW ROT found in §1.1/§1.2 sweep**: `doorway/deferred-ref-repairs.md §3.3` NEW block also
  points to the deleted hub-edge spec (`2026-05-08-doorway-hub-edge-design.md`). §3.3 targets
  `elohim/elohim-hub/README.md` — the repair should route to `doorway/CLAUDE.md` for doorway-side
  framing instead. Fix before executing the gate (out of scope for this commit — operator action).
- If any pillar's retirement is REJECTED: that pillar's history-record prose/derived_from uses
  anticipated-retirement phrasing ("retired to git 2026-06-11") — one-line softens needed
  (qahal session staged the exact edit; others note it in their state files).

## Operator items beyond the gate (surfaced by the fan-out, not gate-blocking)

1. **Root CLAUDE.md libp2p gotcha is stale**: "libp2p 0.53 API (steward/node)" — build graph
   shows BOTH steward/node and elohim-storage resolve libp2p **0.54.1**. Managed-surface repair
   (+ same staleness in the `libp2p-transport` skill). Five sessions inherited this as fact
   before session 6 falsified it.
2. **Deprecation-sentinel hardening** (39 of 62 ledger entries are self-capture false positives;
   triage agents flagged 3 converging guards): (a) skip pure-read cmds (sed -n/cat/grep of
   source paths); (b) exclude `genesis/docs/content/elohim-protocol/history/**` prose +
   `chore(deprecation)` commit subjects; (c) exclude the ledger file itself from fingerprintable
   output.
3. **Reach reconciliation pressure is now fully mapped**: `reach-vocabulary-frontend-strand.md`
   carries storage + doorway + holochain-docs origin strands + the steward sixth site; the
   three-vocabulary reconciliation (roadmap item 13) now has its complete evidence file.
4. Open questions live in the placed docs (each pillar's final report enumerated them).
