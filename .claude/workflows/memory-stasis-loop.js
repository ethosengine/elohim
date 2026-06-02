export const meta = {
  name: 'memory-stasis-loop',
  description: 'Drive the memory surface toward stasis: each round measure the budget + the un-captured backlog (decompose-coverage), dispatch the EQUIPPED agent for the highest-leverage pressure with a broad goal, re-measure, repeat until every metric hits 0. Loop length is proportional to the REAL outstanding workload — it is not a guessed round-count.',
  phases: [{ title: 'Loop' }],
}

// STASIS = no content debt + no un-captured backlog + pressure dirs empty.
// The loop never pre-guesses how long it runs: it keeps dispatching until the numbers
// stop falling (convergence) or hit zero, so effort tracks the actual workload.
// Deterministic measurement every round; agents only for the residue that needs judgment.

const AUDIT = 'python3 .claude/scripts/memory-kit/placement-audit.py'
const ROUND_CAP = 8 // backstop only; the real stop is stasis / convergence

const MEASURE = {
  type: 'object',
  properties: {
    pressure_total: { type: 'number' }, // NEEDS-TRIAGE+MEM-UNLINKED+CLAIMED-ONLY+REGRESSED+SUPERSEDED+UNKNOWN
    uncaptured: { type: 'number' }, // un-reviewed specs/plans (decompose-coverage)
    open_gaps: { type: 'number' },
    claimed_gaps: { type: 'number' },
    pressure_dirs_empty: { type: 'boolean' },
    stasis_score: { type: 'number' },
    at_stasis: { type: 'boolean' },
    dominant: { type: 'string', enum: ['needs-triage', 'mem-unlinked', 'superseded', 'claimed', 'regression', 'none'] },
  },
  required: ['pressure_total', 'uncaptured', 'pressure_dirs_empty', 'stasis_score', 'at_stasis', 'dominant'],
}

// who drains what — equipped agents, BROAD goal, never step-by-step
const DISPATCH = {
  capture: { agentType: 'cartographer', goal: 'Decompose the un-captured (needs-agent prose) specs/plans into bounded, cited gap-items (5-15 each, citing source lines), so the budget reflects the REAL remaining work. Run `' + AUDIT + ' --coverage` for the queue; write each into .claude/memory-kit/gap-items/. Lower `uncaptured` toward 0.' },
  'needs-triage': { agentType: 'librarian', goal: 'Classify NEEDS-TRIAGE docs (give a status + a place in the graph) and link UNLINKED memory; drive those numbers down. Your call which to act on.' },
  'mem-unlinked': { agentType: 'librarian', goal: 'Give unlinked memory entries a `cites:` to the system they describe, or let them go (forget). Lower MEM-UNLINKED.' },
  superseded: { agentType: 'historian', goal: 'Distill SUPERSEDED/abandoned docs into history records (gotcha + pointer + bidirectional canonical link) and retire the bodies. Empty the SUPERSEDED slots.' },
  claimed: { agentType: 'cartographer', goal: 'Rank the CLAIMED-ONLY gaps for verification (ci-investigator); do NOT trust checked boxes. Order by leverage toward lowering the CLAIMED count.' },
  regression: { agentType: 'cartographer', goal: 'Surface the REGRESSED items as the rework queue (highest priority); rank them for a fix sprint.' },
}

phase('Loop')
let prevRemaining = Infinity
let dry = 0
let round = 0
const history = []

while (round < ROUND_CAP) {
  round++

  // 1. MEASURE (deterministic, cheap) — an agent runs the tools and returns the numbers
  const m = await agent(
    `Run, from /projects/elohim, and return the numbers as the schema:\n` +
    `  ${AUDIT} --ledger --json   → sum the rows whose state is one of NEEDS-TRIAGE, MEM-UNLINKED, CLAIMED-ONLY, REGRESSED, SUPERSEDED, UNKNOWN-STATUS = pressure_total; the largest of those classes = dominant (use 'none' if pressure_total is 0).\n` +
    `  ${AUDIT} --coverage --json → uncaptured.\n` +
    `  ${AUDIT}                    → read the STRUCTURAL EQUILIBRIUM section: pressure_dirs_empty = true iff every pressure dir shows 0 docs.\n` +
    `  ${AUDIT} --ledger          → from "DECOMPOSED GAPS": open_gaps, claimed_gaps.\n` +
    `  ${AUDIT} --stasis --json   → stasis_score (composite context-coverage) and at_stasis (score within the ±margin band AND hard dims pass).\n` +
    `Return only the measured numbers. Do not edit anything.`,
    { label: `measure:r${round}`, phase: 'Loop', schema: MEASURE, model: 'haiku' },
  )

  const remaining = m.pressure_total + m.uncaptured
  history.push({ round, remaining, stasis_score: m.stasis_score, uncaptured: m.uncaptured, pressure: m.pressure_total, open: m.open_gaps, claimed: m.claimed_gaps })
  log(`round ${round}: context-coverage=${(m.stasis_score * 100).toFixed(1)}% · uncaptured=${m.uncaptured} · pressure=${m.pressure_total} · open_gaps=${m.open_gaps} · claimed=${m.claimed_gaps}`)

  // 2. STASIS?  "done" = context-coverage score within the ±margin band (at_stasis) AND every spec/plan
  //    captured. at_stasis already folds in the hard dims (pressure dirs empty, no dumps).
  if (m.at_stasis && m.uncaptured === 0) {
    log(`STASIS reached at round ${round}: context-coverage ${(m.stasis_score * 100).toFixed(1)}% within target band and capture complete.`)
    break
  }
  // 3. CONVERGENCE?  (numbers stopped falling — diminishing returns)
  if (remaining >= prevRemaining) {
    dry++
    if (dry >= 2) {
      log(`Convergence: remaining (${remaining}) has not fallen for 2 rounds — stopping. Residual needs operator judgment / blocked.`)
      break
    }
  } else {
    dry = 0
  }
  prevRemaining = remaining

  // 4. DISPATCH the equipped agent for the highest-leverage pressure (un-captured first — you
  //    can't drain what you haven't surfaced). Broad goal, agent's judgment on the how.
  const which = m.uncaptured > 0 ? 'capture' : (m.dominant !== 'none' ? m.dominant : 'claimed')
  const d = DISPATCH[which] || DISPATCH.capture
  log(`round ${round}: dispatching ${d.agentType} for "${which}" (highest-leverage drain).`)
  await agent(
    `${d.goal}\n\nYou are draining the memory-stasis budget toward 0; this is round ${round}. Use the deterministic tools ` +
    `(\`${AUDIT} --ledger / --coverage / --focus\`, decompose.py, spec-coherence-index.py) per ` +
    `.claude/scripts/memory-kit/CLAUDE.md and genesis/docs/PLACEMENT.md. Lower YOUR pressure number; how is your judgment. ` +
    `Do not touch BLOCKED-BY-ENV work (it can't be validated). When done, the next round re-measures.`,
    { label: `drain:r${round}:${which}`, phase: 'Loop', agentType: d.agentType },
  )
}

// final measurement so the return reflects reality after the last drain
const finalCov = await agent(
  `Run \`${AUDIT} --stasis --json\` and \`${AUDIT} --coverage --json\` from /projects/elohim and return {pressure_total:0,uncaptured:<n>,pressure_dirs_empty:<bool>,stasis_score:<n>,at_stasis:<bool>,dominant:'none'}.`,
  { label: 'measure:final', phase: 'Loop', schema: MEASURE, model: 'haiku' },
)

return {
  rounds: round,
  reached_stasis: (finalCov.at_stasis && finalCov.uncaptured === 0),
  final_score: finalCov.stasis_score,
  final_uncaptured: finalCov.uncaptured,
  history,
  note: 'Loop length tracked the real backlog: it dispatched until uncaptured + pressure stopped falling. Un-captured (prose specs) is drained by agents; the resulting OPEN gaps are the implementation backlog for /plan; CLAIMED gaps await the verification index.',
}
