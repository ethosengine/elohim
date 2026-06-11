# Deferred ref-repairs inventory — dna/ island retirement (Phase-3, operator-gated)

Re-swept 2026-06-11 (this session): `grep -rn` for `LINK_ARCHITECTURE`,
`NETWORK_UPGRADES`, `SCHEMA_VERSIONS` repo-wide, excluding `.git`,
`node_modules`, `target`, `dist`, `.angular`. All repairs below PRESUPPOSE the
island files' retirement — apply at the operator gate, not before.

**Replacement targets legend:**
- `ARC` = `genesis/docs/content/elohim-protocol/history/2026-06-11-link-architecture-arc.md` — **PENDING placement**
- `NU-ARC` = `genesis/docs/content/elohim-protocol/history/2026-06-11-network-upgrades-stewarded-coordination-arc.md` — **PENDING placement**
- `SEED` = `genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md` — **PENDING placement**
- `SWEEP` = `genesis/data/timeline/backlog/deprecation-link-architecture-query-index-sweep.md` — **EXISTS** (verified)
- `RLD` = `genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md` — **EXISTS** (verified)

POST-PLACEMENT UPDATE (2026-06-11, commit 55d0d2399): ALL four recompose
targets (ARC, NU-ARC, SEED, + the museum record) are now PLACED and EXIST —
every `PENDING` mark below is satisfied. Re-confirm with `ls` at apply time as
usual, but no pair below points at a missing target anymore.

---

## A. LINK_ARCHITECTURE.md inbound

### A1. genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md

**L1257** — class: prose
- Current (fragment): `The LINK_ARCHITECTURE.md triage rule — "if it exists only for queries, use projection" — becomes elohim-assisted at fleet scale.`
- Proposed: `The link-type triage rule — "if it exists only for queries, use projection" (origin: the retired dna/LINK_ARCHITECTURE.md; history: genesis/docs/content/elohim-protocol/history/2026-06-11-link-architecture-arc.md) — becomes elohim-assisted at fleet scale.`
- Target: ARC — PENDING placement.

**L1271** — class: prose (two fragments on one line)
- Current (fragment 1): `Every new link type proposal must pass LINK_ARCHITECTURE.md triage: genuinely structural relationship vs. projection-candidate.`
- Proposed: `Every new link type proposal must pass link-type triage: genuinely structural relationship vs. projection-candidate.`
- Current (fragment 2): `The LINK_ARCHITECTURE.md explicitly lists ~50 \`*By{Attribute}\` variants as deprecation candidates; retiring those reclaims headroom`
- Proposed: `The retired LINK_ARCHITECTURE.md explicitly listed ~50 \`*By{Attribute}\` variants as deprecation candidates (live tracking: genesis/data/timeline/backlog/deprecation-link-architecture-query-index-sweep.md); retiring those reclaims headroom`
- Targets: SWEEP — EXISTS; (doc-name retained as history, no path link needed for fragment 1).

**L1538** — class: prose (action item: write the rename note into the doc)
- Current: `- \`elohim/holochain/dna/LINK_ARCHITECTURE.md\` — note the \`ContentToResource → EprToResource\` rename in the link-type history`
- Proposed: `- \`genesis/docs/content/elohim-protocol/history/2026-06-11-link-architecture-arc.md\` — note the \`ContentToResource → EprToResource\` rename in the link-type history (LINK_ARCHITECTURE.md retired 2026-06-11)`
- Target: ARC — PENDING placement. NOTE for arc author: carry the `ContentToResource → EprToResource` rename so this action item stays satisfiable.

**L2819** — class: prose
- Current (fragment): `- **LINK_ARCHITECTURE.md deprecation checklist is incomplete.** ~50 \`*By{Attribute}\` query-index link types violate`
- Proposed: `- **The \`*By{Attribute}\` query-index deprecation sweep is open** (tracked: genesis/data/timeline/backlog/deprecation-link-architecture-query-index-sweep.md; the originating LINK_ARCHITECTURE.md checklist retired into genesis/docs/content/elohim-protocol/history/2026-06-11-link-architecture-arc.md). ~50 \`*By{Attribute}\` query-index link types violate`
- Targets: SWEEP — EXISTS; ARC — PENDING placement.

**L2827** — class: prose
- Current (fragment): `LINK_ARCHITECTURE.md updates the deprecation checklist to show closure.`
- Proposed: `Closure is recorded in genesis/data/timeline/backlog/deprecation-link-architecture-query-index-sweep.md (the retired LINK_ARCHITECTURE.md checklist's live successor).`
- Target: SWEEP — EXISTS.
- DO NOT rename the backfill title fragment `**Backfill 3 — LINK_ARCHITECTURE deprecation sweep.**` on the same line — SWEEP quotes that exact name (its ~L67/L69); the name is historical.

**L2835** — class: prose (touch-list item)
- Current: `- \`elohim/holochain/dna/LINK_ARCHITECTURE.md\` — close deprecation checklist; update the 256-cap accounting`
- Proposed: `- \`genesis/data/timeline/backlog/deprecation-link-architecture-query-index-sweep.md\` — record checklist closure + 256-cap accounting (LINK_ARCHITECTURE.md retired into genesis/docs/content/elohim-protocol/history/2026-06-11-link-architecture-arc.md)`
- Targets: SWEEP — EXISTS; ARC — PENDING placement.

### A2. genesis/docs/superpowers/plans/2026-05-24-records-lifecycle-phase2-findings-synthesis.md

**L43** — class: prose (table cell)
- Current (fragment): `LINK_ARCHITECTURE.md deprecation checklist incomplete; ~50 \`*By*\` query-index links unretired`
- Proposed: `\`*By*\` query-index deprecation sweep open (genesis/data/timeline/backlog/deprecation-link-architecture-query-index-sweep.md); ~50 links unretired`
- Target: SWEEP — EXISTS.

**L165** — class: prose
- Current: `       complete LINK_ARCHITECTURE.md deprecation checklist;`
- Proposed: `       complete the *By* query-index deprecation sweep (genesis/data/timeline/backlog/deprecation-link-architecture-query-index-sweep.md);`
- Target: SWEEP — EXISTS.

**L294** — class: prose
- Current (fragment): `Gaps 11, 12, and the LINK_ARCHITECTURE.md deprecation are not records-lifecycle work per se`
- Proposed: `Gaps 11, 12, and the *By* query-index deprecation sweep (genesis/data/timeline/backlog/deprecation-link-architecture-query-index-sweep.md) are not records-lifecycle work per se`
- Target: SWEEP — EXISTS.

### A3. genesis/docs/superpowers/plans/2026-05-11-tiered-quilt-wave-0-substrate-cleanup.md

**L173** — class: prose (plan modify-list item; the doc:line target dies at retirement)
- Current: `- Modify: \`elohim/holochain/dna/LINK_ARCHITECTURE.md:175\``
- Proposed: `- Modify: ~~\`elohim/holochain/dna/LINK_ARCHITECTURE.md:175\`~~ (doc retired 2026-06-11; its *By* listing is tracked in genesis/data/timeline/backlog/deprecation-link-architecture-query-index-sweep.md, history in genesis/docs/content/elohim-protocol/history/2026-06-11-link-architecture-arc.md)`
- Targets: SWEEP — EXISTS; ARC — PENDING placement.
- Context note (verified): LINK_ARCHITECTURE.md:175 sits in the `*By*` deprecation-candidates code block (`EventByAction`/`EventByLamadType`/`ResourceBySpec` listing).

### A4. genesis/docs/superpowers/plans/2026-05-24-records-lifecycle-part-d-substrate-gaps-plan.md

**L23** — class: prose
- Current (fragment): `LINK_ARCHITECTURE.md deprecation checklist is incomplete.`
- Proposed: `The *By* query-index deprecation sweep is open (genesis/data/timeline/backlog/deprecation-link-architecture-query-index-sweep.md).`
- Target: SWEEP — EXISTS.

**L115** — class: prose
- Current (fragment): `\`LINK_ARCHITECTURE.md\` deprecation checklist (incomplete);`
- Proposed: `the *By* query-index sweep entry genesis/data/timeline/backlog/deprecation-link-architecture-query-index-sweep.md (open);`
- Target: SWEEP — EXISTS.

**L125** — class: prose (backfill task title — name is quoted by SWEEP ~L67; keep the name)
- Current (fragment): `3. **LINK_ARCHITECTURE.md deprecation sweep**: formally retire the ~50 \`*By{Attribute}\` query-index link types`
- Proposed: `3. **LINK_ARCHITECTURE.md deprecation sweep** (name historical — doc retired 2026-06-11 into genesis/docs/content/elohim-protocol/history/2026-06-11-link-architecture-arc.md; live tracking genesis/data/timeline/backlog/deprecation-link-architecture-query-index-sweep.md): formally retire the ~50 \`*By{Attribute}\` query-index link types`
- Targets: ARC — PENDING placement; SWEEP — EXISTS.

**L131** — class: prose (touch-list item)
- Current: `  - \`elohim/holochain/dna/LINK_ARCHITECTURE.md\` — update deprecation checklist`
- Proposed: `  - \`genesis/data/timeline/backlog/deprecation-link-architecture-query-index-sweep.md\` — record checklist closure (LINK_ARCHITECTURE.md retired)`
- Target: SWEEP — EXISTS.

### A5. genesis/plans/2026-03-06-rust-architect-agent-plan.md

**L310** — class: prose (reference list; path ALREADY stale — pre-`elohim/` move spelling)
- Current: `- \`holochain/dna/LINK_ARCHITECTURE.md\` (link design patterns)`
- Proposed: `- \`genesis/docs/content/elohim-protocol/history/2026-06-11-link-architecture-arc.md\` (link design patterns — the retired dna/LINK_ARCHITECTURE.md)`
- Target: ARC — PENDING placement.

### A6. app/elohim-app/src/app/imagodei/services/doorway-registry.service.ts

**L375** — class: code-comment
- Current: `   * Note: Per LINK_ARCHITECTURE.md, "get all doorways" is a query candidate`
- Proposed: `   * Note: Per the link-type triage rule (query-only workloads belong in SQL projection — genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md), "get all doorways" is a query candidate`
- Target: RLD — EXISTS.

### A7. .claude/agents/rust-architect.md

**L563** — class: prose (agent definition reference list; check managed-surface registry before editing)
- Current: `- \`elohim/holochain/dna/LINK_ARCHITECTURE.md\` (link design patterns)`
- Proposed: `- \`genesis/docs/content/elohim-protocol/history/2026-06-11-link-architecture-arc.md\` (link design patterns — retired dna/LINK_ARCHITECTURE.md) and the link-budget rail in \`elohim/holochain/dna/CLAUDE.md\``
- Target: ARC — PENDING placement.
- Sibling artifact `.claude/memory-kit/2026-06-02/rewrites/rust-architect.proposed.md:544` repeats the same line — dated generated proposal, no-action (see §D).

### A8. genesis/data/timeline/backlog/deprecation-link-architecture-query-index-sweep.md (NEW since prior inventory)

**L18** — class: cites:-entry (PLAIN PATH; tooling seals — do not hand-write slugs/fingerprints)
- Current: `  - elohim/holochain/dna/LINK_ARCHITECTURE.md`
- Proposed: `  - genesis/docs/content/elohim-protocol/history/2026-06-11-link-architecture-arc.md`
- Target: ARC — PENDING placement.

**L28** — class: prose (records what the sentinel captured; keep name, mark retired)
- Current (fragment): `A checklist line in \`elohim/holochain/dna/LINK_ARCHITECTURE.md\` (§"Query-Only`
- Proposed: `A checklist line in the now-retired \`elohim/holochain/dna/LINK_ARCHITECTURE.md\` (history: genesis/docs/content/elohim-protocol/history/2026-06-11-link-architecture-arc.md) (§"Query-Only`
- Target: ARC — PENDING placement.

**L60-63** — class: prose (touch-list item; the entry itself says the retirement "should carry the closure note" — honor that)
- Current: `- \`elohim/holochain/dna/LINK_ARCHITECTURE.md\` — close the deprecation checklist\n  and update the 256-cap accounting (this doc is itself slated for retirement in\n  the holochain \`dna/\` island recompose; whichever lands first should carry the\n  closure note).`
- Proposed: `- ~~\`elohim/holochain/dna/LINK_ARCHITECTURE.md\`~~ — RETIRED 2026-06-11 in the dna/\n  island recompose (history: genesis/docs/content/elohim-protocol/history/2026-06-11-link-architecture-arc.md);\n  per this entry's own rule the retirement carries the checklist-closure note. The\n  underlying \`*By*\` sweep stays open HERE; 256-cap accounting lives in the\n  records-lifecycle design (~L1271).`
- Target: ARC — PENDING placement. NOTE for arc author: include an explicit closure note for the `- [ ] Mark ~50 query-only links as DEPRECATED` checklist line (sentinel fingerprint 4b3ce06c317d points at this entry).

**L104-105** — class: prose
- Current (fragment): `LINK_ARCHITECTURE.md island retirement may close the line incidentally — see\n\`genesis/data/timeline/backlog/pillar-island-recompose-recipe.md\`).`
- Proposed: `LINK_ARCHITECTURE.md island retirement (landed 2026-06-11; history: genesis/docs/content/elohim-protocol/history/2026-06-11-link-architecture-arc.md) closed the checklist line; the *By* sweep itself stays open here).`
- Target: ARC — PENDING placement.

**L113-114** — class: prose (verification clause)
- Current (fragment): `\`LINK_ARCHITECTURE.md\` checklist line closed (or the doc retired in the island\nrecompose).`
- Proposed: `the originating checklist line resolved by the doc's retirement in the island recompose (2026-06-11).`
- Target: none needed (statement of fact).

**L67, L69** — no-action: quotes of the backfill section NAME ("Backfill 3 — LINK_ARCHITECTURE deprecation sweep") in OTHER docs; the quoted names stay historical.

### A9. Phase-3 registry flips (census/registry class — flip at operator gate, NOT text repairs)

**genesis/data/timeline/backlog/subject-routing-locus-census.md L138**
- Current (fragment): `EDGE-DESIGN}, holochain {LINK_ARCHITECTURE}.`
- Flip: remove `holochain {LINK_ARCHITECTURE}` from the "Island docs still to route+retire" set (L136-138) once retirement lands; optionally append the dna/ recompose to the census's recomposed log (the lamad recompose precedent is recorded at ~L140-146 of the same file).

### A10. Already repaired / no-action (LINK_ARCHITECTURE)

- `elohim/holochain/tests/sweettest/src/tests/recovery_m4.rs:430` — **ALREADY REPAIRED 2026-06-11** (parent session). The corrected Scenario-7 doc-comment intentionally retains the doc name as history ("the previously cited \`LINK_ARCHITECTURE.md\` §3 'dual-anchor primacy' never existed in any version of that doc"). EXCLUDE from repairs.
- `genesis/data/timeline/backlog/pillar-island-recompose-recipe.md:198,206` — the recompose's own recipe/process doc; the parent session updates it as part of the gate ceremony. No independent repair.
- `.claude/memory-kit/2026-05-10/path-update-proposals.md:829-836`, `.claude/memory-kit/2026-05-13/agent-audit.md:360`, `.claude/memory-kit/2026-06-02/rewrites/rust-architect.proposed.md:544` — dated generated audit/proposal artifacts; no-action.
- `.claude/data/deprecations.jsonl:58` — machine ledger (fingerprint 4b3ce06c317d, status blocked, already points at SWEEP); no-action (the stasis sweep owns lifecycle).

---

## B. NETWORK_UPGRADES.md inbound

### B1. elohim/holochain/dna/elohim/dna.yaml

**L12** — class: code-comment (YAML comment; verified it does NOT affect DNA hash — but still retirement-presupposing → deferred)
- Current: `#   rna module (elohim/holochain/rna/) and NETWORK_UPGRADES.md.`
- Proposed: `#   rna module (elohim/holochain/rna/) and genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md.`
- Target: SEED — PENDING placement.

### B2. elohim/holochain/tests/manifest-hygiene/tests/manifest_hygiene.rs

**L170** — class: code-comment
- Current: `// is tracked separately (rna module, NETWORK_UPGRADES.md, future brainstorm).`
- Proposed: `// is tracked separately (rna module, genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md, future brainstorm).`
- Target: SEED — PENDING placement.

### B3. PASS-B (docs/ cluster recompose owns these files; exact pairs included for whoever applies)

**elohim/holochain/docs/README.md L118** — ALREADY-BROKEN relative path (`./dna/NETWORK_UPGRADES.md` resolves to `elohim/holochain/docs/dna/NETWORK_UPGRADES.md`, which does not exist; the real file is at `elohim/holochain/dna/NETWORK_UPGRADES.md`)
- Current: `| [dna/NETWORK_UPGRADES.md](./dna/NETWORK_UPGRADES.md) | DNA migration strategy |`
- Proposed: `| [DNA upgrade governance](../../../genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md) | DNA migration strategy |`
- Target: SEED — PENDING placement. (Relative path verified: elohim/holochain/docs/ → repo root is `../../../`.)

**elohim/holochain/docs/ARCHITECTURE.md L268** — same ALREADY-BROKEN relative path class
- Current: `- [dna/NETWORK_UPGRADES.md](./dna/NETWORK_UPGRADES.md) - DNA migration strategy`
- Proposed: `- [DNA upgrade governance](../../../genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md) - DNA migration strategy`
- Target: SEED — PENDING placement.

### B4. No-action (NETWORK_UPGRADES)

- `elohim/holochain/dna/NETWORK_UPGRADES.md:261` — self-reference ("Document the migration path in NETWORK_UPGRADES.md"); dies with the file. NOTE for NU-ARC/SEED authors: the seed should re-home this instruction ("document the migration path in <seed>").
- `.claude/memory-kit/2026-06-0{1,2,3}/memory-coherence-audit.{md,json}` — dated generated snapshots citing memory `project_lineage_rna_upgrade_path` → NETWORK_UPGRADES.md. No-action. **Verified**: the LIVE memory dir (`/projects/.claude-config/projects/-projects-elohim/memory/`) has ZERO current citations of any island file (grep 2026-06-11), so no live memory repair is needed.
- `genesis/data/timeline/backlog/pillar-island-recompose-recipe.md:198` — recipe/process doc (see A10).

---

## C. SCHEMA_VERSIONS.md inbound

**Zero true inbound references — verified 2026-06-11.** Every repo match for
`SCHEMA_VERSIONS` is the unrelated Rust constant `SUPPORTED_SCHEMA_VERSIONS`
(`elohim/elohim-views/src/shared.rs:73` and its consumers in
`elohim/elohim-storage/src/{http.rs,services/response.rs,views_convert/inputs.rs}`,
plus prose mentions in `genesis/plans/2026-03-17-protocol-schema-contract-design.md:450`
and `.claude/skills/holochain-storage-api/SKILL.md:295`) — none reference the
island file. Only the recompose recipe (`pillar-island-recompose-recipe.md:198`)
names it. No repairs.

---

## D. OPTIONAL — adjacent dead-reference repair (not island-inbound, surfaced during sweep)

**elohim/holochain/tests/manifest-hygiene/README.md** — cites "wave-1 execution
plan §7" twice; the cited file `genesis/docs/plans/2026-04-21-rno-lessons-wave-1-execution-plan.md`
is NOT in the tree (verified: no `wave-1`/`wave1` file under genesis/; only the
sibling spec `genesis/docs/superpowers/specs/2026-04-21-bootstrap-steward-authority-frame-design.md`
exists).

**Header prose (~L4-5)** — class: prose
- Current: `workdir \`happ.yaml\`. Enforces the conventions laid out in wave-1 execution\nplan §7.`
- Proposed: `workdir \`happ.yaml\`. Enforces the manifest conventions now canonical in\ngenesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md (origin: wave-1 execution plan §7, no longer in tree).`
- Target: SEED — PENDING placement.

**Related section (last lines)** — class: prose
- Current: `- Plan: \`genesis/docs/plans/2026-04-21-rno-lessons-wave-1-execution-plan.md\` §7`
- Proposed: `- Governance: \`genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md\` (supersedes wave-1 execution plan §7, no longer in tree)`
- Target: SEED — PENDING placement.
- Also on this list (no change needed): the spec line cites `2026-04-21-bootstrap-steward-authority-frame-design.md` — EXISTS, healthy.

OPEN QUESTION (for D): the README's check #9 cites "wave-1 §1.2 Q2" inline
(~L31) — same dead plan. If the SEED absorbs the bootstrap-steward
progenitor rules, repoint that too; otherwise point at the bootstrap-steward
spec. Left out of the pair list pending the seed's final scope.

---

## Stats

- **Repair pairs (text edits): 27** fragments across **15 files**
  (A1: 7 fragments / A2: 3 / A3: 1 / A4: 4 / A5: 1 / A6: 1 / A7: 1 / A8: 6 / B1: 1 / B2: 1 / B3: 2 / D: 2... see sections; census flip counted separately).
- **Census/registry flips (Phase-3): 1** (subject-routing-locus-census.md L138).
- **No-action / already-done: 9 sites** (recovery_m4.rs done; NU self-ref; recipe doc; 6 memory-kit/ledger artifacts).
- **New since prior inventory: 1 live doc** (`deprecation-link-architecture-query-index-sweep.md`, 6 repair fragments incl. its `cites:` line) + confirmation that memory-kit artifacts and deprecations.jsonl carry mentions (all no-action).
- **Targets:** SWEEP + RLD EXIST; ARC / NU-ARC / SEED are PENDING placement (re-verify before applying — no pair above proposes a target that is neither existing nor in the planned recompose set).
- NU-ARC currently has zero pairs pointing at it (all NETWORK_UPGRADES prose repairs point at the SEED, which is the forward-looking policy home; the arc is history).
