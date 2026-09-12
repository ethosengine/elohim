export const meta = {
  name: 'recall-standing-reader',
  description: 'Weekly standing reader: sample the recall journey across every reader tier the actor sidecar has seen in the last 30 days, over the six-question bank, then have each sample judged by a seat of a DIFFERENT tier — so the recall-reaches-authority habit\'s rolling-window bound (recall-journey-window-ceiling) has journeys to read.',
  phases: [
    { title: 'Discover' },
    { title: 'Sample' },
    { title: 'Judge' },
  ],
}

// governed-discovery station 3, task 3.4. Drives ONLY the two verbs task 3.2 landed
// (elohim/eprfs/epr-cli/src/flow/memory/recall/sample.rs, commit 3d5b4cab6) — `sample` and
// `judge` — never a hand-rolled fold. Per (question, readerTier) pair this workflow runs
// EXACTLY one `sample` and one `judge`, which together write:
//   - one FlowEvent  (the sampled journey, folded by `sample`)
//   - one Verdict    (the second seat's ruling, folded by `judge`, as a `verdict` note on the
//                      recall contract)
//   - FIVE folds: `sample` folds recall-metered-bytes@1, recall-screens-to-shape@1,
//     recall-unmetered-bytes@1 and recall-not-reached@1 (four, read straight off sample.rs's own
//     `note::observe` call sites); `judge` folds recall-mistaken-assertions@1 (one). 4 + 1 = 5
//     per pair. It was FOUR until the 2026-09-12 station 3 fix round: a journey that never
//     reached authority folded nothing the rolling-window bound consumes, so a measured MISS read
//     as a clean journey. `recall-not-reached@1` is folded on EVERY journey — 1 on the miss path,
//     0 on the located path — and `recall-journey-window-ceiling@1` now consumes it.
// The dry run (one tier x one question, confirming 1 FlowEvent / 1 Verdict / 5 folds) is run by
// the controller after this script is reported, not by this workflow.

const EPR = process.env.EPR_BIN || 'epr'

// The six Intents in the bank — .epr-meta/elohim/algorithms/recall-questions.json — named
// verbatim; the workflow never invents a question id, it only points `sample` at the bank's own.
const QUESTIONS = ['q-remine', 'q-corrections', 'q-top-red', 'q-hook-binary', 'q-body-scan', 'q-journey-folds']

// Used ONLY when the actor sidecar (.eprfs/status/actors.jsonl) carries no claim in the last 30
// days, or is absent — the brief's own fallback triple.
const FALLBACK_TIERS = ['claude-haiku-4-5', 'claude-sonnet-5', 'claude-opus-5']

const TIERS_SCHEMA = {
  type: 'object',
  properties: {
    tiers: { type: 'array', items: { type: 'string' } },
    usedFallback: { type: 'boolean' },
    reason: { type: 'string' },
  },
  required: ['tiers', 'usedFallback', 'reason'],
}

// Maps a tier string (e.g. "claude-sonnet-5") to the short alias the Agent/agent() model
// override accepts. An unrecognized tier omits the override — the agent then inherits the
// session's resolved model, which is reported, never silently misreported as the tier it ran at.
function shortModel(tier) {
  const t = tier.toLowerCase()
  if (t.includes('haiku')) return 'haiku'
  if (t.includes('opus')) return 'opus'
  if (t.includes('sonnet')) return 'sonnet'
  if (t.includes('fable')) return 'fable'
  return undefined
}

// The judge seat's tier: the FIRST entry in `pool` that differs from `readerTier` — a rotation
// by one when `pool` has 2+ distinct tiers, and a safe fall-through to the fallback triple when
// the discovered set collapsed to a single tier (so a genuinely different-tier seat still judges).
function pickJudgeTier(readerTier, pool) {
  const distinct = pool.filter((t) => t !== readerTier)
  if (distinct.length) return distinct[0]
  const fromFallback = FALLBACK_TIERS.filter((t) => t !== readerTier)
  return fromFallback[0] || readerTier
}

phase('Discover')
const discovered = await agent(
  'From the repository root (/projects/elohim), determine the reader tiers seen in the last 30 days via the actor ' +
    'sidecar the recall lens resolver reads: .eprfs/status/actors.jsonl (JSON Lines; each line is ' +
    '{"cid": "...", "record": {"kind": "claim", "claimed": "agent:<role>@<model>", "session": "...", "claimedAt": "<RFC3339>"}}). ' +
    "Compute a 30-days-ago UTC cutoff (e.g. `date -u -d '30 days ago' +%Y-%m-%dT%H:%M:%S` on GNU date, " +
    "`date -u -v-30d +%Y-%m-%dT%H:%M:%S` on BSD/macOS date). If the file exists, select every line whose " +
    'record.claimedAt is >= that cutoff (ISO-8601 UTC timestamps compare correctly as strings), take the substring ' +
    'after the LAST "@" in record.claimed as that claim\'s model tier, and collect the sorted distinct set. Use jq if ' +
    'present, else python3 json, else plain grep/sed — whichever is on PATH. Do not edit anything; do not use ' +
    'Date.now/new Date in your own reasoning, only the shell `date` command above.\n' +
    `If the file is absent, unreadable, or the distinct set is empty, return tiers=${JSON.stringify(FALLBACK_TIERS)}, ` +
    'usedFallback=true, reason="sidecar empty/absent — fallback triple used" (or name why). Otherwise usedFallback=false ' +
    'and reason names how many claims and which tiers were found.',
  { label: 'discover-tiers', schema: TIERS_SCHEMA, model: 'haiku' },
)
const TIERS = discovered.tiers && discovered.tiers.length ? discovered.tiers : FALLBACK_TIERS
log(
  `reader tiers for this run: ${TIERS.join(', ')} ` +
    `(${discovered.usedFallback ? 'FALLBACK — ' : 'from the 30-day actor sidecar — '}${discovered.reason})`,
)

// One (question, readerTier) pair per journey; the judge seat rotates to a different tier.
const pairs = []
for (const question of QUESTIONS) {
  for (const readerTier of TIERS) {
    const judgeTier = pickJudgeTier(readerTier, TIERS)
    pairs.push({
      question,
      readerTier,
      judgeTier,
      session: `standing-reader-${question}-${readerTier}`,
    })
  }
}
log(`${pairs.length} (question x reader-tier) pairs to sample and judge this run`)
// Name the cap on tier-fidelity out loud: a tier string shortModel() cannot map runs at the
// inherited session model, which is a silent substitution unless it is logged per pair.
for (const pair of pairs) {
  for (const tier of [pair.readerTier, pair.judgeTier]) {
    if (!shortModel(tier)) log(`unrecognized tier ${tier}: pair ${pair.session} runs at the inherited model`)
  }
}

const SAMPLE_SCHEMA = {
  type: 'object',
  properties: {
    eventCid: { type: 'string' },
    assertion: { type: 'string' },
    reached: { type: 'boolean' },
    readAccount: { type: 'string' },
  },
  required: ['eventCid', 'assertion', 'reached', 'readAccount'],
}

const JUDGE_SCHEMA = {
  type: 'object',
  properties: {
    mistaken: { type: 'number' },
    verdictNoteCid: { type: 'string' },
  },
  required: ['mistaken', 'verdictNoteCid'],
}

// pipeline: each pair flows through Sample then Judge independently — a slow judge on pair A
// never blocks pair B's sample. Both stages receive the ORIGINAL pair as their second argument.
const results = await pipeline(
  pairs,
  (pair) =>
    agent(
      'You are a FRESH, context-reset reader with no memory of this repository beyond what this one command shows ' +
        'you. You are ENTRY-ONLY: never grep/find/cat a repository file directly and never open a file on your own ' +
        'initiative — the recall executor is the sole legitimate path to evidence, and for this journey that is a ' +
        'single command, which itself opens, reads and finishes the ceremony for you. Run exactly, from the ' +
        `repository root (/projects/elohim): \`${EPR} flow memory recall sample --question ${pair.question} ` +
        `--reader agent:reader@${pair.readerTier} --session ${pair.session} --json\`. Parse its JSON stdout and ` +
        'return eventCid = event.cid, assertion = question.assertion, reached = the reached boolean, and ' +
        'readAccount = a 1-2 sentence account of what read.path/read.lines showed, from the view\'s own fields only ' +
        '(first_screen/read/location) — never by opening that file yourself.',
      { label: `sample:${pair.question}:${pair.readerTier}`, phase: 'Sample', schema: SAMPLE_SCHEMA, model: shortModel(pair.readerTier) },
    ),
  (sample, pair) => {
    if (!sample) return null
    return agent(
      'You are the SECOND SEAT: a different reader tier from whoever ran this journey, ruling on an ALREADY-RECORDED ' +
        `FlowEvent — you do not re-run the journey. A reader sampled recall question "${pair.question}" and it folded ` +
        `as FlowEvent ${sample.eventCid}. The question's reached_when.assertion is: "${sample.assertion}". The ` +
        `reader's own account of what it read: "${sample.readAccount}". The journey's own reached flag: ${sample.reached}. ` +
        "Count MISTAKEN assertions only: distinct claims in the reader's account that the assertion CONTRADICTS — 0 when " +
        'nothing the reader said is wrong. An OMISSION (a claim in the assertion the account never mentions) is not a ' +
        'mistake: name omissions in your reason, never in the count. Then run exactly, from the repository ' +
        `root (/projects/elohim): \`${EPR} flow memory recall judge --event ${sample.eventCid} ` +
        `--as agent:judge@${pair.judgeTier} --mistaken <your count> --reason "<one-line reason>" --json\`, substituting ` +
        'your count and reason. Return mistaken = the count you used and verdictNoteCid = the JSON verdict.note field.',
      { label: `judge:${pair.question}:${pair.judgeTier}`, phase: 'Judge', schema: JUDGE_SCHEMA, model: shortModel(pair.judgeTier) },
    )
  },
)

const completed = results.filter(Boolean)
const totalMistaken = completed.reduce((sum, r) => sum + (r.mistaken || 0), 0)
log(`${completed.length}/${pairs.length} pairs completed; ${totalMistaken} total mistaken assertions across the run`)

return {
  tiers: TIERS,
  tiersUsedFallback: discovered.usedFallback,
  pairsAttempted: pairs.length,
  pairsCompleted: completed.length,
  totalMistakenAssertions: totalMistaken,
  perPair: pairs.map((pair, i) => ({
    question: pair.question,
    readerTier: pair.readerTier,
    judgeTier: pair.judgeTier,
    outcome: results[i],
  })),
  note:
    'One sample + one judge per (question, reader-tier) pair; 1 FlowEvent + 1 Verdict + 5 folds ' +
    '(4 from sample, 1 from judge) per completed pair. Feeds the recall-journey-window-ceiling ' +
    'rolling-window bound this habit reads — it needs 3+ journeys in the window to report a rate ' +
    'rather than `skipped`.',
}
