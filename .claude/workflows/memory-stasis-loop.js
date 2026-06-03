export const meta = {
  name: 'memory-stasis-loop',
  description: 'Drive the WHOLE memory discipline toward stasis in ONE loop: compaction debt + un-captured backlog + decompose-due + anti-dump equilibrium + MAP path-currency + roadmap-currency. Each round measure the full scoreboard, dispatch the EQUIPPED agent for the highest-leverage pressure with a broad goal, re-measure, repeat until every discipline hits stasis. Loop length tracks the REAL outstanding workload, not a guessed round-count.',
  phases: [{ title: 'Loop' }],
}

// STASIS = the FULL discipline at equilibrium, not just compaction:
//   (1) compaction — no content debt, every spec captured, context-coverage in band
//   (2) NO DUMPS — pressure dirs / archive / stale shifts all clean (the cardinal rule; a dump
//       is the one thing that cannot graduate to EPR, so it is fixed FIRST)
//   (3) decompose-due — landed plans dissolved to zero residue
//   (4) PATH currency — MAP.md (the walk) current with the architecture seeds (the graph)
//   (5) ROADMAP currency — the vision x readiness roadmap current with the gap-ledger x cluster-state
// One loop, every discipline. It never pre-guesses length: dispatch until the numbers stop falling
// (convergence) or every dimension hits stasis. Deterministic measurement; agents only for judgment residue.

const AUDIT = 'python3 .claude/scripts/memory-kit/placement-audit.py'
const ROUND_CAP = 10 // backstop only; the real stop is stasis / convergence

const MEASURE = {
  type: 'object',
  properties: {
    pressure_total: { type: 'number' },   // compaction debt: NEEDS-TRIAGE+MEM-UNLINKED+CLAIMED-ONLY+REGRESSED+SUPERSEDED+UNKNOWN
    uncaptured: { type: 'number' },        // un-reviewed specs/plans (decompose-coverage)
    decompose_due: { type: 'number' },     // landed plans past-due to decompose-self (headline `decompose:`)
    dumps: { type: 'number' },             // anti-dump: NO-EXIT + DRIFT-DEAD + DUMP + archive files + shifts past the ~14d budget
    path_drift: { type: 'number' },        // architecture seeds changed since MAP.md update (headline `path:`)
    roadmap_stale: { type: 'boolean' },    // roadmap artifact stale vs the gap-ledger x cluster-state (headline `roadmap:`)
    memkit_overflow: { type: 'boolean' },  // memory-kit process-artifact tier over budget / loose-unfiled (headline `memkit:`)
    mempalace_stale: { type: 'boolean' },  // MemPalace semantic index behind the cleaned surface (headline `mempalace:`)
    cites_legacy: { type: 'number' },      // legacy doc path-cites to migrate to content-addressed envelopes (audit CITE-FORMAT-CANDIDATE)
    open_gaps: { type: 'number' },
    claimed_gaps: { type: 'number' },
    pressure_dirs_empty: { type: 'boolean' },
    stasis_score: { type: 'number' },
    at_stasis: { type: 'boolean' },        // compaction context-coverage within band
    dominant: { type: 'string', enum: ['needs-triage', 'mem-unlinked', 'superseded', 'claimed', 'regression', 'none'] },
  },
  required: ['pressure_total', 'uncaptured', 'decompose_due', 'dumps', 'path_drift', 'roadmap_stale', 'memkit_overflow', 'mempalace_stale', 'cites_legacy', 'pressure_dirs_empty', 'stasis_score', 'at_stasis', 'dominant'],
}

// who drains what — equipped agents, BROAD goal, never step-by-step
const DISPATCH = {
  capture: { agentType: 'cartographer', goal: 'Decompose the un-captured (needs-agent prose) specs/plans into bounded, cited gap-items (5-15 each, citing source lines), so the budget reflects the REAL remaining work. Run `' + AUDIT + ' --coverage` for the queue; write each into .claude/memory-kit/gap-items/. Lower `uncaptured` toward 0.' },
  'needs-triage': { agentType: 'librarian', goal: 'Classify NEEDS-TRIAGE docs (give a status + a place in the graph) and link UNLINKED memory; drive those numbers down. Your call which to act on.' },
  'mem-unlinked': { agentType: 'librarian', goal: 'Give unlinked memory entries a `cites:` to the system they describe, or let them go (forget). Lower MEM-UNLINKED.' },
  superseded: { agentType: 'historian', goal: 'Distill SUPERSEDED/abandoned docs into curated history records (gotcha + present-as-pointer + bidirectional canonical link) and retire the bodies to git. Empty the SUPERSEDED slots.' },
  claimed: { agentType: 'cartographer', goal: 'Rank the CLAIMED-ONLY gaps for verification (ci-investigator); do NOT trust checked boxes. Order by leverage toward lowering the CLAIMED count.' },
  regression: { agentType: 'cartographer', goal: 'Surface the REGRESSED items as the rework queue (highest priority); rank them for a fix sprint.' },
  'decompose-due': { agentType: 'librarian', goal: 'A landed/superseded plan in an ACTIVE home is past-due to DECOMPOSE-SELF (headline `decompose:`). Dissolve it to ZERO residue per the compaction-loop spec + PLACEMENT.md: durable truth -> canonical seed, lesson -> curated history (present-as-pointer), unfinished -> backlog, body -> git. No dumping ground. Lower the decompose-due count.' },
  dumps: { agentType: 'historian', goal: 'A DUMP is forming (EQUILIBRIUM: NO-EXIT / DRIFT-DEAD / archive non-empty / shifts past the ~14-day budget). This is the CARDINAL violation — a dump cannot graduate to EPR. Decompose it to zero residue NOW: distill any lesson to curated history, retire the body to git, clear the dump. Restore structural equilibrium.' },
  'map-drift': { agentType: 'librarian', goal: 'Architecture seed(s) changed since MAP.md was last updated (headline `path:`) — the WALK is stale vs the GRAPH. Reconcile genesis/docs/content/elohim-protocol/architecture/MAP.md: update the affected domain stanza + the gap ledger so the new-developer spine still resolves. Keep INDEX (graph) and MAP (walk) consistent. Lower path-drift to 0.' },
  'roadmap-stale': { agentType: 'cartographer', goal: 'The roadmap is stale vs the gap-ledger x cluster-state x vision (headline `roadmap:`). Regenerate genesis/data/timeline/roadmap/vision-readiness-sprint-roadmap.md: re-rank by vision x readiness from the live gap-item states + cluster-state availability + the household-living-core gospel; refresh the single highest-leverage next move. The roadmap is a maintained readout, never a snapshot.' },
  memkit: { agentType: 'librarian', goal: 'The memory-kit PROCESS-ARTIFACT tier is over budget or has loose unfiled reports (headline `memkit:`). These do NOT decompose into documentation — they follow COMET retention. Run `python3 .claude/scripts/memory-kit/memkit-retention.py --apply`: file loose reports into their dated cycle, compact the tail to one-line _digest.md, memorialize the core into TRAJECTORY.md (the permanent spine). Restore the comet shape; the date-stamps preserve the trajectory, bodies stay in git.' },
  cites: { agentType: 'librarian', goal: 'Un-sealed cite debt remains — docs authored this sprint whose cites are still plain paths (audit CITE-FORMAT-CANDIDATE; headline `cites:`). Run the deterministic born-linked sweep: `python3 .claude/scripts/memory-kit/cite-gen.py --seal-all` (assigns id: slugs + converts legacy doc-cites to `<slug> | desc | fingerprint` envelopes + verifies, idempotent, ~0.1s when clean). Content-addressed cites survive file moves — this is what makes relocations free (held/ moves never break a link). If the sweep reports `✍ N cite(s) on the title-default desc`, author the relationship hints — dispatch the corpus-describe workflow or run `cite-describe.py <doc> \'{"<ref>":"<hint>"}\'` per doc — that is the progressive-discovery payload. Lower `cites_legacy` toward 0.' },
  mempalace: { agentType: 'librarian', goal: 'The MemPalace semantic index is behind the cleaned surface (headline `mempalace:`) — the front-link would recall a stale view. Run `python3 .claude/scripts/memory-kit/mempalace-currency.py --remine`: sync (prune deleted/moved drawers) + mine the cleaned durable surface (canonical seeds + curated history + working memory + stories) + record the mine timestamp. NEVER mine the transient pile / raw code / junk drawer — index only the clean surface. Restore index freshness.' },
}

phase('Loop')
let prevRemaining = Infinity
let dry = 0
let round = 0
const history = []

while (round < ROUND_CAP) {
  round++

  // 1. MEASURE (deterministic, cheap) — an agent runs the tools and returns the FULL scoreboard
  const m = await agent(
    `Run, from /projects/elohim, and return the numbers as the schema. Edit NOTHING.\n` +
    `  ${AUDIT} --ledger --json   -> pressure_total = sum of rows whose state is one of NEEDS-TRIAGE, MEM-UNLINKED, CLAIMED-ONLY, REGRESSED, SUPERSEDED, UNKNOWN-STATUS; dominant = the largest of those classes ('none' if pressure_total is 0); open_gaps/claimed_gaps from "DECOMPOSED GAPS".\n` +
    `  ${AUDIT} --coverage --json -> uncaptured.\n` +
    `  ${AUDIT} --headline        -> decompose_due = the number on the \`decompose:\` line; path_drift = the "N seeds changed since MAP update" on the \`path:\` line; roadmap_stale = true unless the \`roadmap:\` line reads current/fresh; memkit_overflow = true unless the \`memkit:\` line reads "comet shape ✅"; mempalace_stale = true unless the \`mempalace:\` line reads "fresh ✅".\n` +
    `  ${AUDIT}                    -> STRUCTURAL EQUILIBRIUM section: dumps = NO-EXIT + DRIFT-DEAD + DUMP + archive(_retired) file count; pressure_dirs_empty = true iff every pressure dir shows 0 docs.\n` +
    `  python3 .claude/scripts/memory-kit/cite-propagate.py 2>/dev/null | grep -oE "stamped: [0-9]+" ; python3 .claude/scripts/memory-kit/memory-coherence-audit.py 2>/dev/null | grep -oE "format-candidate \\(cites_legacy\\): [0-9]+"  -> cites_legacy = the format-candidate count (legacy doc-cites to migrate to envelopes).\n` +
    `  Also run: find .claude/shifts -name '*.md' -mtime +14 2>/dev/null | wc -l  -> ADD that count to dumps (stale shift narration past the ~14-day budget is a dump).\n` +
    `  ${AUDIT} --stasis --json   -> stasis_score (composite context-coverage) and at_stasis (within +-margin band AND hard dims pass).\n` +
    `Return only the measured numbers.`,
    { label: `measure:r${round}`, phase: 'Loop', schema: MEASURE, model: 'haiku' },
  )

  const remaining = m.pressure_total + m.uncaptured + m.decompose_due + m.dumps + m.path_drift + (m.roadmap_stale ? 1 : 0) + (m.memkit_overflow ? 1 : 0) + (m.mempalace_stale ? 1 : 0)
  history.push({ round, remaining, stasis_score: m.stasis_score, uncaptured: m.uncaptured, pressure: m.pressure_total, decompose_due: m.decompose_due, dumps: m.dumps, path_drift: m.path_drift, roadmap_stale: m.roadmap_stale, memkit_overflow: m.memkit_overflow, mempalace_stale: m.mempalace_stale })
  log(`round ${round}: coverage=${(m.stasis_score * 100).toFixed(1)}% · pressure=${m.pressure_total} · uncaptured=${m.uncaptured} · decompose-due=${m.decompose_due} · dumps=${m.dumps} · path-drift=${m.path_drift} · roadmap-stale=${m.roadmap_stale} · memkit-overflow=${m.memkit_overflow} · mempalace-stale=${m.mempalace_stale}`)

  // 2. STASIS? "done" = EVERY discipline at equilibrium: compaction in band + captured + no dumps +
  //    decompose-due drained + MAP current + roadmap current + memory-kit comet-shaped + index fresh.
  if (m.at_stasis && m.uncaptured === 0 && m.decompose_due === 0 && m.dumps === 0 && m.path_drift === 0 && !m.roadmap_stale && !m.memkit_overflow && !m.mempalace_stale) {
    log(`STASIS reached at round ${round}: all disciplines at equilibrium (compaction ${(m.stasis_score * 100).toFixed(1)}%, no dumps, MAP + roadmap current, capture complete).`)
    break
  }
  // 3. CONVERGENCE? (numbers stopped falling — diminishing returns / operator-or-env residual)
  if (remaining >= prevRemaining) {
    dry++
    if (dry >= 2) {
      log(`Convergence: remaining (${remaining}) has not fallen for 2 rounds — stopping. Residual needs operator judgment / verification (ci-investigator) / blocked-by-env.`)
      break
    }
  } else {
    dry = 0
  }
  prevRemaining = remaining

  // 4. DISPATCH the highest-leverage pressure. Priority: DUMPS first (cardinal: no dumps, ever) ->
  //    capture (can't drain what isn't surfaced) -> decompose-due -> compaction debt -> path -> roadmap -> claimed.
  const which =
    m.dumps > 0 ? 'dumps' :
    m.uncaptured > 0 ? 'capture' :
    m.decompose_due > 0 ? 'decompose-due' :
    (m.pressure_total > 0 && m.dominant !== 'none') ? m.dominant :
    m.path_drift > 0 ? 'map-drift' :
    m.roadmap_stale ? 'roadmap-stale' :
    m.memkit_overflow ? 'memkit' :
    m.mempalace_stale ? 'mempalace' :
    m.cites_legacy > 0 ? 'cites' :
    'claimed'
  const d = DISPATCH[which] || DISPATCH.capture
  log(`round ${round}: dispatching ${d.agentType} for "${which}" (highest-leverage drain).`)
  await agent(
    `${d.goal}\n\nYou are draining the unified memory-stasis budget toward 0; this is round ${round}. Use the deterministic tools ` +
    `(\`${AUDIT} --ledger / --coverage / --focus / --headline\`, decompose.py, spec-coherence-index.py) per ` +
    `.claude/scripts/memory-kit/CLAUDE.md, genesis/docs/PLACEMENT.md, and the compaction-loop spec ` +
    `(genesis/docs/superpowers/specs/2026-06-02-spec-plan-compaction-loop-design.md). Lower YOUR pressure number; how is your judgment. ` +
    `Cardinal rule: NO DUMPING GROUNDS — decompose to zero residue, curated history is present-as-pointer. ` +
    `Do not touch BLOCKED-BY-ENV work (it can't be validated). When done, the next round re-measures.`,
    { label: `drain:r${round}:${which}`, phase: 'Loop', agentType: d.agentType },
  )
}

// final measurement so the return reflects reality after the last drain
const finalCov = await agent(
  `Run from /projects/elohim: ${AUDIT} --stasis --json, ${AUDIT} --coverage --json, and ${AUDIT} --headline. Return the MEASURE schema ` +
  `(pressure_total, uncaptured, decompose_due, dumps, path_drift, roadmap_stale, open_gaps, claimed_gaps, pressure_dirs_empty, stasis_score, at_stasis, dominant). Edit nothing.`,
  { label: 'measure:final', phase: 'Loop', schema: MEASURE, model: 'haiku' },
)

const reached = finalCov.at_stasis && finalCov.uncaptured === 0 && finalCov.decompose_due === 0 && finalCov.dumps === 0 && finalCov.path_drift === 0 && !finalCov.roadmap_stale && !finalCov.memkit_overflow && !finalCov.mempalace_stale

return {
  rounds: round,
  reached_stasis: reached,
  final_score: finalCov.stasis_score,
  final: { uncaptured: finalCov.uncaptured, decompose_due: finalCov.decompose_due, dumps: finalCov.dumps, path_drift: finalCov.path_drift, roadmap_stale: finalCov.roadmap_stale, pressure: finalCov.pressure_total },
  history,
  note: 'One loop, every discipline. It drains compaction debt, un-captured prose, decompose-due plans, forming dumps (cardinal — fixed first), MAP path-drift, and roadmap staleness until all hit equilibrium or stop falling. Residual OPEN gaps are the implementation backlog for /plan; CLAIMED gaps await ci-investigator; blocked-by-env is held, not failed.',
}
