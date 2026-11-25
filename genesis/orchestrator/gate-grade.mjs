// gate-grade.mjs: the Attest-grade station as a SHADOW (spec
// genesis/docs/superpowers/specs/2026-11-24-verdict-is-an-attestation-on-an-artifact-design.md,
// sections 4a, 5 and 8). Beside a gate's real reuse-or-run decision it says what evidence this
// device holds for the gate's closure and how that evidence would read under the PROPOSED graded
// table. The table is a proposal the steward has not ruled; nothing here is read by a decision.
//
// What is counted, and from what. An independent execution is one admitted device holding a green
// for THIS project and THIS closure, in a record this device read through a verifying reader:
//   - this device: a note `brit-build-ref build find` verified, signed by this workspace's key;
//   - a sibling: a `gate-verdict@1` row from `epr peer verdicts`, which admits by the roster and
//     reads only what arrived inside that device's own signed channel head.
// Standing is assigned by the reader a record came through, never by a field inside the record.
// A device counts once however many records it holds. A sponsored runner's result (a harvested
// `ci:<pipeline>` note, whatever key signed it) is a carrier's result: it counts zero and is listed
// apart. A record for another closure or project, a record whose closure cannot be read, a note
// brit could not verify, a signer that resolves to no admitted device, and an unsigned enrichment
// are each listed and count for nothing. A red of any provenance is shown and never netted out.
//
// The words on the line are "would suffice under the proposed table" or "would not". A grade is a
// reading, not a verdict, and a count of executions is not a count of witnesses.

export const GRADE_ENRICHMENT_MEASURE = 'attestation-enrichment@1';

/** `GATE_GRADE=off|shadow`, default shadow. There is no `on`: any other value reads as shadow. */
export function gradeMode(env = process.env) {
  return env?.GATE_GRADE === 'off' ? 'off' : 'shadow';
}

// The rows of the proposed table (spec section 5) a push can be, strictest last. The release row
// is a taking, not a push, and is the station Attest-release.
export const GRADE_CLASSES = Object.freeze(['documentation', 'tooling', 'library-crate', 'serving-path', 'governance']);

const CLASS_LABEL = Object.freeze({
  documentation: 'documentation only',
  tooling: 'test or tooling code',
  'library-crate': 'a library crate',
  'serving-path': 'a serving path',
  governance: 'a gate\'s own judge or recipe',
});

// A serving path, as the pre-push T2 receipt leg names them.
const SERVING_PREFIXES = Object.freeze([
  'elohim/elohim-storage/src/p2p/',
  'elohim/elohim-storage/src/sync/',
  'elohim/elohim-storage/src/reconcile/',
  'elohim/elohim-storage/src/p2p_iroh/',
  'doorway/doorway-service/src/',
]);

// Test and tooling trees: scripts, hooks, harnesses and their fixtures; never a crate's sources.
const TOOLING_PREFIXES = Object.freeze([
  'genesis/orchestrator/', 'genesis/agentic/', 'genesis/a2o/', 'scripts/', '.claude/', '.codex/',
  '.agents/', '.husky/', '.epr-meta/',
]);
const TOOLING_EXTENSIONS = /\.(mjs|cjs|js|ts|py|sh|bash|feature|json|jsonl|yaml|yml)$/;
// Generated ledgers no recipe judges by: the habit register's projection.
const LEDGER_PATHS = Object.freeze(['genesis/manifests/habits.yaml']);

// A recipe is part of a gate's own judge: the manifest that declares it and the justfile that runs it.
const isRecipePath = path => /(^|\/)(build-manifest\.json|justfile|[^/]+\.just)$/.test(path);

function classOfPath(path, isGovernance) {
  if (isGovernance(path) || isRecipePath(path)) return 'governance';
  if (SERVING_PREFIXES.some(prefix => path.startsWith(prefix))) return 'serving-path';
  if (/\.rs$/.test(path) || /(^|\/)Cargo\.(toml|lock)$/.test(path)) return 'library-crate';
  if (/\.md$/.test(path) || LEDGER_PATHS.includes(path)) return 'documentation';
  if (TOOLING_PREFIXES.some(prefix => path.startsWith(prefix)) && TOOLING_EXTENSIONS.test(path)) return 'tooling';
  return null;
}

/**
 * The class of a change, from the paths the push touches since its integration base. The strictest
 * row any path falls in is the class. A range that could not be read, an empty range, or any path
 * no row's definition places is `undetermined`, with the reason: a class is never guessed.
 */
export function classifyChange(range, { isGovernance = () => false } = {}) {
  if (!range || !range.base) return { cls: null, why: 'no integration base was read' };
  const files = Array.isArray(range.files) ? range.files : [];
  if (files.length === 0) return { cls: null, why: 'the push touches no path since its integration base' };
  let strictest = -1;
  for (const path of files) {
    const cls = classOfPath(String(path), isGovernance);
    if (!cls) return { cls: null, why: `no row places ${path}` };
    strictest = Math.max(strictest, GRADE_CLASSES.indexOf(cls));
  }
  return { cls: GRADE_CLASSES[strictest], label: CLASS_LABEL[GRADE_CLASSES[strictest]], paths: files.length };
}

const short = value => String(value ?? '').slice(0, 12) || 'unknown';
export const shortDevice = did => {
  const text = String(did ?? '');
  return text.startsWith('did:key:') && text.length > 24 ? `${text.slice(8, 18)}…${text.slice(-4)}` : (text || 'unknown');
};

const closureOf = record => (typeof record === 'string' && record.length > 0 ? record : null);

/**
 * Tally the records for one project and closure. Every input names the reader it came through:
 * `local` (brit-verified rows of this step), `unverifiable` (notes brit would not vouch for),
 * `siblings` (rows of `epr peer verdicts`), `enrichments` (flow-log rows of
 * `attestation-enrichment@1`) and `carriers` (harvested CI notes). Pure: it reads nothing.
 */
export function countExecutions({
  project, key, selfId = null, local = [], unverifiable = [], siblings = [], enrichments = [], carriers = [],
  unread = [],
}) {
  const devices = new Map(); // device id -> { who, green, red }
  const ignored = [];
  const disagreements = [];
  const carried = [];
  const device = (id, who) => {
    if (!devices.has(id)) devices.set(id, { who, green: false, red: false });
    return devices.get(id);
  };
  const verdict = (entry, success, where) => {
    if (success === true) entry.green = true;
    else {
      entry.red = true;
      disagreements.push(`${entry.who} red${where ? ` (${where})` : ''}`);
    }
  };

  for (const row of local) {
    const a = row?.attestation || {};
    const closure = closureOf(a.inputsHash);
    if (!closure) { ignored.push('a local note whose closure cannot be read'); continue; }
    if (closure !== key) { ignored.push(`a local note for another closure (${short(closure)})`); continue; }
    if (typeof a.success !== 'boolean') { ignored.push('a local note with no verdict'); continue; }
    if (!selfId) { ignored.push(`a local ${a.success ? 'green' : 'red'} whose signer cannot be compared (this workspace's key was not read)`); if (!a.success) disagreements.push('an unresolved signer red (local note)'); continue; }
    if (a.agentId !== selfId) {
      ignored.push(`a local ${a.success ? 'green' : 'red'} signed by ${short(a.agentId || 'unsigned')}, which resolves to no admitted device`);
      if (!a.success) disagreements.push(`signer ${short(a.agentId || 'unsigned')} red (local note, not an admitted device)`);
      continue;
    }
    verdict(device('self', 'this device'), a.success, null);
  }
  for (const note of unverifiable) {
    ignored.push(`an unverifiable note at ${String(note?.commit || '').slice(0, 9) || 'an unknown commit'} (${String(note?.reason || 'brit could not verify it').split('\n')[0]})`);
  }
  for (const row of siblings) {
    if (row?.source !== 'sibling') { ignored.push('a verdict that did not come through the sibling reader'); continue; }
    const closure = closureOf(row.closure);
    if (!closure) { ignored.push('a sibling verdict whose closure cannot be read'); continue; }
    if (closure !== key || row.project !== project) { ignored.push(`a sibling verdict for another closure or project (${row.project || '?'} ${short(closure)})`); continue; }
    if (typeof row.device !== 'string' || !row.device.startsWith('did:key:')) { ignored.push('a sibling verdict naming no device key'); continue; }
    if (typeof row.success !== 'boolean') { ignored.push(`a sibling verdict from ${shortDevice(row.device)} with no verdict`); continue; }
    verdict(device(row.device, `sibling ${shortDevice(row.device)}`), row.success, null);
  }
  for (const row of enrichments) {
    if (row?.kind !== 're-executed') { ignored.push(`an enrichment of kind ${row?.kind || 'unknown'}`); continue; }
    const closure = closureOf(row.closure);
    if (!closure) { ignored.push('an enrichment whose closure cannot be read'); continue; }
    if (closure !== key || row.project !== project) { ignored.push(`an enrichment for another closure or project (${row.project || '?'} ${short(closure)})`); continue; }
    // attestation-enrichment@1 is unsigned and stays on the device that wrote it (enrich.rs): no
    // reader can verify who re-executed, so a green counts for nothing. Its red is still shown.
    if (row.outcome === 'red') disagreements.push(`re-executed enrichment red (unsigned, ${shortDevice(row.device)})`);
    ignored.push(`an unsigned re-executed enrichment ${row.outcome === 'green' ? 'green' : row.outcome === 'red' ? 'red' : 'with no outcome'} (${shortDevice(row.device)})`);
  }
  for (const row of carriers) {
    const name = row?.name || 'carrier';
    if (row?.state !== 'verified' || typeof row.success !== 'boolean') {
      if (row?.state && row.state !== 'absent') ignored.push(`${name}'s result ${row.state}${row.reason ? ` (${String(row.reason).split('\n')[0]})` : ''}`);
      continue;
    }
    const sameKey = closureOf(row.closure) === key;
    carried.push(`${name} ${row.success ? 'green' : 'red'}${sameKey ? '' : ' at this commit (keyed on the commit tree, not this closure)'}`);
    if (!row.success) disagreements.push(`carrier ${name} red`);
  }

  const greens = [...devices.values()].filter(entry => entry.green);
  return {
    independent: greens.length,
    by: greens.map(entry => `${entry.who} green`),
    carried,
    ignored: [...new Set(ignored)],
    disagreements: [...new Set(disagreements)],
    unverifiable: unverifiable.length,
    unread: [...unread],
  };
}

/**
 * What the proposed table would say for this class and tally: `{ suffice: 'yes'|'no', why }`, or
 * null when the class is undetermined (no would-suffice is printed then). Row values are the
 * proposal's (N = 1 on every push row); nothing here is ruled.
 */
export function wouldSuffice(cls, tally) {
  if (!cls) return null;
  const no = why => ({ suffice: 'no', why });
  if (tally.disagreements.length > 0) return no('a disagreement is on record');
  if (tally.unverifiable > 0) return no('an unverifiable note under this step reads as a possible red');
  if (cls === 'governance') {
    return no('governance row: never skipped for a gate whose own judge or recipe changed, and which gates those are is not computed here, so every gate reads as changed');
  }
  if (tally.independent < 1) {
    return no(tally.carried.length > 0
      ? 'row needs 1; a carrier\'s result counts 0 executions and is not yet eligible to stand alone'
      : 'row needs 1; no admitted device holds a green for this closure');
  }
  const note = cls === 'library-crate' ? ' at the crate\'s own key; this is the project key'
    : cls === 'serving-path' ? ' for the gate; the household witness the row requires at dev and main is not read here'
      : '';
  return { suffice: 'yes', why: `row needs 1${note}` };
}

/** The one line. `today` is the real decision, already made and recorded: `reuse own`, `ran green`, … */
export function gradeLine(project, { change, tally, would }, today) {
  const parts = [];
  parts.push(change.cls ? `class ${change.label}` : `class: undetermined (${change.why})`);
  parts.push(`executions for this closure: ${tally.independent} independent${tally.by.length ? ` (${tally.by.join(', ')})` : ''}`
    + `, carrier: ${tally.carried.length ? tally.carried.join(', ') : 'none'}`);
  if (tally.ignored.length) parts.push(`not counted: ${tally.ignored.join(', ')}`);
  if (tally.unread.length) parts.push(`unread: ${tally.unread.join(', ')}`);
  parts.push(`disagreements: ${tally.disagreements.length ? tally.disagreements.join(', ') : 'none'}`);
  if (would) parts.push(`would suffice: ${would.suffice} (${would.why})`);
  return `[gate] ${project} grade (shadow, proposed table): ${parts.join('; ')} | today: ${today}`;
}

/** The reading row's step: the class, the count and the reading ride in the name (see gate-runner). */
export function gradeStep(project, { change, tally, would }) {
  return `${project}:grade:${change.cls || 'undetermined'}:n${tally.independent}:${would ? would.suffice : 'na'}`;
}

/** Compose class, tally and reading. Pure. */
export function gradeOf({ range, isGovernance, ...records }) {
  const change = classifyChange(range, { isGovernance });
  const tally = countExecutions(records);
  return { change, tally, would: wouldSuffice(change.cls, tally) };
}

/**
 * The `re-executed` enrichments in a flow log's text, as written by `epr flow enrich` on this
 * device. Only lines that name the measure are parsed; a line of any other shape is skipped. A row
 * that arrived from another device (a `via:` slot) is not one this writer makes and is dropped.
 */
export function enrichmentsIn(text) {
  const out = [];
  for (const line of String(text || '').split('\n')) {
    if (!line.includes(GRADE_ENRICHMENT_MEASURE)) continue;
    let slots;
    try { slots = JSON.parse(line)?.record?.classifiedAs; } catch { continue; }
    if (!Array.isArray(slots) || !slots.includes(`measure:${GRADE_ENRICHMENT_MEASURE}`)) continue;
    if (slots.some(slot => typeof slot === 'string' && slot.startsWith('via:'))) continue;
    const env = name => {
      const found = slots.filter(slot => typeof slot === 'string' && slot.startsWith(`env:${name}=`));
      return found.length === 1 ? found[0].slice(`env:${name}=`.length) : null;
    };
    out.push({ kind: env('kind'), outcome: env('outcome'), project: env('project'), closure: env('closure'), device: env('device') });
  }
  return out;
}
