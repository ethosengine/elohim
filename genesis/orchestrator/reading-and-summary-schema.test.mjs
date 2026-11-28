import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import test from 'node:test';
import Ajv2020 from 'ajv/dist/2020.js';
// Decision 10 of the lifecycle design's section 16, not yet ruled.
// true: any kept record holds the unit against compaction, watch and closure.
// false: kept records remain, but hold nothing. No modelled event releases the hold.
const KEPT_REPORT_HOLDS_A_CLAIM_OPEN = true;
const root = resolve(import.meta.dirname, '../..');
const sprint = join(root, 'genesis/docs/superpowers/sprints/2026-11-27-feedback-contract/fc4');
const load = path => JSON.parse(readFileSync(path, 'utf8'));
const fixturePath = (group, kind, name = '') => join(sprint, 'fixtures', group, kind, name);
const names = (group, kind) => readdirSync(fixturePath(group, kind)).sort();
const schemas = () => ({
  reading: load(join(sprint, 'term-reading.draft.schema.json')),
  summary: load(join(sprint, 'claim-summary.draft.schema.json')),
});
const mapping = () => load(join(sprint, 'reading-to-state.json'));
const cases = () => load(join(sprint, 'precedence-cases.json'));
function compile() {
  const ajv = new Ajv2020({ strict: true, allErrors: true });
  ajv.addFormat('date', { type: 'string', validate: value => {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return false;
    const [y, m, d] = value.split('-').map(Number);
    const days = [31, y % 4 === 0 && (y % 100 !== 0 || y % 400 === 0) ? 29 : 28,
      31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    return m >= 1 && m <= 12 && d >= 1 && d <= days[m - 1];
  }});
  return Object.fromEntries(Object.entries(schemas()).map(([kind, schema]) => [kind, ajv.compile(schema)]));
}
function declaredRequired(node, path) {
  if (!node || typeof node !== 'object') return;
  for (const key of node.required ?? []) {
    assert.ok(Object.hasOwn(node.properties ?? {}, key), `${path}: required ${key} lacks adjacent properties`);
  }
  for (const [key, value] of Object.entries(node)) declaredRequired(value, `${path}/${key}`);
}
// False/absent facts do not establish a predicate. No lifecycle transition is inferred.
export function read(f) {
  if (!f.failed) return null;
  if (f.recurred && !f.curedAgain) return 'reopened';
  if (f.pinLag && !f.laterHead) return 'stale-emitter';
  if (!f.keptRecordHolds && f.curePresent && (f.coveredRecords ||
      (f.aggregates && f.currentCureCountedAgainst && f.differentCure))) return 'cure-available';
  if (!f.keptRecordHolds && f.aggregates && !f.retireMet && !f.note && !f.currentCureCountedAgainst) return 'on-watch';
  if (!f.keptRecordHolds && (f.retireMet || f.note) && !f.currentCureCountedAgainst) return 'retired';
  if (f.superseded) return 'superseded';
  if (f.horizonNoCure) return 'review-due';
  if (f.unknownRelease) return 'unrecorded';
  return 'open';
}
// R1–R4: event order is time; numeric releases abbreviate MAJOR.0.0 in lives.
export function start(keptReportHoldsOpen = KEPT_REPORT_HOLDS_A_CLAIM_OPEN) {
  return { keptReportHoldsOpen, tick: 0, session: 0, emitter: null, head: null, landed: 0, known: true,
    records: [], observations: [], aggregates: [], cures: [], declaration: [], notes: [], current: null, currentAt: 0, currentSession: 0,
    condition: null, holding: false, watch: 0, superseded: false, horizon: false };
}
// SemVer precedence ignores build metadata; numeric identifiers compare without precision loss.
const compareRelease = (a, b) => {
  const parts = v => String(typeof v === 'number' ? `${v}.0.0` : v).split('+')[0].split('-');
  const [ac, ...ap] = parts(a), [bc, ...bp] = parts(b);
  const cmp = (x, y) => x < y ? -1 : x > y ? 1 : 0;
  const aa = ac.split('.'), bb = bc.split('.');
  for (let i = 0; i < 3; i++) {
    const order = cmp(BigInt(aa[i]), BigInt(bb[i]));
    if (order) return order;
  }
  if (!ap.length || !bp.length) return cmp(!ap.length, !bp.length);
  const ax = ap.join('-').split('.'), bx = bp.join('-').split('.');
  for (let i = 0; i < Math.max(ax.length, bx.length); i++) {
    if (ax[i] === undefined || bx[i] === undefined) return cmp(ax.length, bx.length);
    const an = /^\d+$/.test(ax[i]), bn = /^\d+$/.test(bx[i]);
    const order = an && bn ? cmp(BigInt(ax[i]), BigInt(bx[i])) :
      an !== bn ? (an ? -1 : 1) : cmp(ax[i], bx[i]);
    if (order) return order;
  }
  return 0;
};
const latestEntry = u => [...u.declaration].sort((a, b) => compareRelease(a.release, b.release)).at(-1);
const latestCured = (u, release) => u.declaration
  .filter(c => c.state === 'cured' && compareRelease(c.release, release) <= 0)
  .sort((a, b) => compareRelease(a.release, b.release)).at(-1);
const releasePresent = u => latestEntry(u)?.state === 'cured' ? latestEntry(u) : undefined;
const present = u => releasePresent(u) ??
  (u.holding ? u.cures.filter(c => c.kind === 'condition').at(-1) : undefined);
// R10 (D4): release evidence takes priority over compacted condition cures; failures use the head declaration, including curing releases never taken.
const recurrences = u => {
  const observations = u.observations.filter(o => !o.passed).flatMap(o => {
    const release = latestCured(u, o.release);
    const conditions = u.cures.filter(c => c.kind === 'condition' &&
      c.compactedAt < o.at);
    return (release ? [release] : conditions).map(c => ({ o, c }));
  });
  // R18: withdrawal invalidates the current cure without creating a session record.
  // Keep that counter-evidence when a later cure ends reopening (R13/R14).
  const withdrawals = u.declaration.filter(e => e.state === 'withdrawn').flatMap(e => {
    const c = latestCured(u, e.release);
    return c && c.id === u.current ? [{ o: null, c }] : [];
  });
  return [...observations, ...withdrawals];
};
export function facts(u) {
  const failures = u.observations.filter(o => !o.passed), last = failures.at(-1);
  const recurrence = recurrences(u), note = u.notes.find(n => n.cure === u.current);
  const cured = ({ o, c }) => c.kind === 'condition'
    ? Boolean(releasePresent(u)) || u.cures.some(n => n.kind === 'condition' && n.at > o.at) // R10 (D4)
    : u.declaration.some(n => n.state === 'cured' && compareRelease(n.release, c.release) > 0); // R4, R10: condition instances count too.
  const retireMet = u.aggregates.length > 0 && u.aggregates.every(a => a.participant !== null &&
    u.observations.some(o => o.passed && o.participant === a.participant && o.session > u.currentSession &&
      u.watch > o.at && !failures.some(f => f.at > o.at))); // R6
  return {
    failed: failures.length > 0, coveredRecords: u.records.some(r => r.kind === 'covered'),
    curePresent: Boolean(present(u)), differentCure: Boolean(present(u) && present(u).id !== u.current), // R12 (D5)
    aggregates: u.aggregates.length > 0, retireMet,
    note: Boolean(note), recurred: recurrence.length > 0,
    curedAgain: recurrence.length > 0 && recurrence.every(cured),
    currentCureCountedAgainst: recurrence.some(({ c }) => c.id === u.current),
    keptRecordHolds: u.keptReportHoldsOpen && u.records.some(r => r.kind === 'kept'), // R1, R12–R14
    pinLag: Boolean(last && u.head !== null && compareRelease(last.release, u.head) < 0 && last.afterHead && last.at > u.landed),
    laterHead: Boolean(last && u.observations.some(o => o.release === u.head && o.at > last.at)),
    unknownRelease: !u.known, horizonNoCure: u.known && u.horizon && !present(u), superseded: u.superseded,
  }; // R9–R16
}
export function apply(unit, event) {
  const u = structuredClone(unit), e = event;
  const reading = read(facts(u));
  if (e.type === 'retire') { // R5, R7, R8: refused acts leave the entire unit unchanged.
    if (!['cure-available', 'retired'].includes(reading) && !(reading === 'on-watch' && e.ruling)) return u;
    if (reading === 'retired' && facts(u).note) return u;
  }
  u.tick++;
  const condition = holds => { // R4: each false-to-true edge is a distinct cure.
    if (holds && !u.holding) u.cures.push({ id: `condition:${u.tick}`, kind: 'condition', at: u.tick });
    u.holding = holds;
  };
  switch (e.type) {
    case 'fail':
    case 'pass': { // R1, R4: observations survive compaction; only covered records are replaced.
      if (e.newSession !== false) u.session++;
      if (e.clears) u.condition = e.clears;
      const o = { at: u.tick, session: u.session, release: e.release ?? u.head,
        participant: e.participant ?? null, record: e.record, passed: e.type === 'pass', afterHead: e.afterHead ?? false };
      u.observations.push(o);
      if (!o.passed) {
        u.emitter ??= o.release;
        const coveredBefore = u.observations.slice(0, -1).some(x =>
          !x.passed && x.record === 'covered' && x.session === u.session);
        if (e.record === 'kept') u.records.push({ session: u.session, kind: 'kept', participant: o.participant, at: o.at });
        else if (!coveredBefore) u.records.push({ session: u.session, kind: e.record, participant: o.participant });
      }
      if (u.condition === 'observed') condition(o.passed);
      break;
    }
    case 'commit': { // R1: retain the same occurrence, without another observation.
      const record = e.session === undefined ? u.records.findLast(r => r.kind === 'covered') :
        u.records.find(r => r.session === e.session && r.kind === 'covered') ?? u.records.find(r => r.session === e.session);
      if (record?.kind === 'kept') return unit;
      if (!record) throw new Error('not modelled: commitment without a covered session record');
      record.kind = 'kept';
      break;
    }
    case 'release': // R4: replace the declaration, including on downgrade.
      u.head = e.release; u.landed = u.tick;
      u.declaration = e.entries.filter(([release]) => compareRelease(release, e.release) <= 0).map(([release, state]) =>
        ({ id: `release:${release}`, kind: 'release', release, state }));
      break;
    case 'condition': // R4: non-observed conditions are explicit events.
      assert.notEqual(e.name, 'observed'); u.condition = e.name; condition(e.holds); break;
    case 'retire':
      if (reading === 'cure-available') { // R2, R3, R5 (D5): move aggregates, never notes.
        const cure = present(u);
        if (cure.kind === 'condition') cure.compactedAt ??= u.tick;
        u.current = cure.id; u.currentAt = u.tick; u.currentSession = u.session;
        for (const aggregate of u.aggregates) aggregate.cure = u.current;
        const participants = new Set(u.records.filter(r => r.kind === 'covered').map(r => r.participant));
        for (const participant of participants)
          u.aggregates.push({ cure: u.current, participant, at: u.tick, session: u.session });
        u.records = u.records.filter(r => r.kind === 'kept');
      } else u.notes.push({ cure: u.current, at: u.tick }); // R3, R7, R8
      break;
    case 'watch': u.watch = u.tick; break; // R6: an elapsed period, not a clock.
    case 'supersede': u.superseded = true; break; // R15
    case 'horizon': u.horizon = true; break; // R15
    case 'known': u.known = e.value; break; // R15
    default: throw new Error(`Unknown event ${e.type}`);
  }
  return u;
}
const summaryState = (reading, f) => {
  const state = reading === null || f.unknownRelease ? null : mapping()[reading].state;
  return state && typeof state === 'object' ? (f.note ? state.then : state.untilNoteWritten) : state;
}; // R17: unknown emitters send nothing; retired sends cured until the current cure's note exists.
const errors = {
  reading: {
    'reading-outside-nine': ['enum', '/reading'],
    'unknown-field': ['additionalProperties', '', 'unknown'],
    'claim-id-grammar': ['pattern', '/claim'],
    'emitter-characters': ['pattern', '/emitter'],
    'claim-id-too-long': ['maxLength', '/claim'],
    'emitter-too-long': ['maxLength', '/emitter'],
    'emitter-wrong-type': ['type', '/emitter'],
    'missing-claim': ['required', '', 'claim'],
    'missing-reading': ['required', '', 'reading'],
  },
  summary: {
    'state-outside-list': ['enum', '/state'],
    'class-outside-list': ['enum', '/class'],
    'unknown-field': ['additionalProperties', '', 'unknown'],
    'negative-count': ['minimum', '/sessions'],
    'fractional-count': ['type', '/bytes'],
    'timestamp-not-day': ['format', '/first-day'],
    'invalid-calendar-day': ['format', '/last-day'],
    'claim-id-grammar': ['pattern', '/claim'],
    'emitter-characters': ['pattern', '/emitter'],
    'private-message': ['additionalProperties', '', 'message'],
    'private-about': ['additionalProperties', '', 'about'],
    'severity-outside-list': ['enum', '/severity'],
    'polarity-outside-list': ['enum', '/polarity'],
    'missing-class': ['required', '', 'class'],
    'open-dropped': ['not', ''],
    'open-cure': ['not', ''],
    'open-bytes': ['not', ''],
    'severity-error': ['enum', '/severity'],
    'missing-claim': ['required', '', 'claim'],
    'missing-emitter': ['required', '', 'emitter'],
    'missing-polarity': ['required', '', 'polarity'],
    'missing-severity': ['required', '', 'severity'],
    'missing-state': ['required', '', 'state'],
  },
};
test('both schemas compile in strict mode', () => {
  for (const [kind, schema] of Object.entries(schemas())) declaredRequired(schema, kind);
  assert.doesNotThrow(compile, `schemas must exist and compile at ${sprint}`);
});
test('every accepted reading and summary fixture passes its schema', () => {
  for (const [kind, validate] of Object.entries(compile())) {
    const files = names('accepted', kind);
    assert.ok(files.length >= 3, `insufficient fixtures at ${fixturePath('accepted', kind)}`);
    for (const name of files) {
      assert.equal(validate(load(fixturePath('accepted', kind, name))), true,
        `${kind}/${name}: ${JSON.stringify(validate.errors)}`);
    }
  }
});
test('every refused fixture fails exactly once for its named reason', () => {
  for (const [kind, validate] of Object.entries(compile())) {
    for (const name of names('refused', kind)) {
      const ok = validate(load(fixturePath('refused', kind, name)));
      const actual = validate.errors ?? [];
      const want = errors[kind][name.replace(/\.json$/, '')];
      const message = `${kind}/${name}: ${JSON.stringify(actual)}`;
      assert.ok(want, `no expected reason for ${kind}/${name}`);
      assert.equal(ok, false, message);
      assert.equal(actual.length, 1, message);
      assert.equal(actual[0].keyword, want[0], message);
      assert.equal(actual[0].instancePath, want[1], message);
      if (want[2]) assert.equal(actual[0].params.additionalProperty ?? actual[0].params.missingProperty, want[2], message);
    }
  }
});
test('each refused directory and the refusal table name exactly the same files', () => {
  for (const kind of Object.keys(errors)) {
    assert.deepEqual(names('refused', kind), Object.keys(errors[kind]).map(name => `${name}.json`).sort(),
      fixturePath('refused', kind));
  }
});
test('the nine readings agree with the mapping and its states and display words', () => {
  const tableStates = {open:'open', 'stale-emitter':null, unrecorded:null,
    'cure-available':'open', 'on-watch':'cured', retired:{untilNoteWritten:'cured',then:'retired'},
    reopened:'reopened', 'review-due':'open', superseded:null};
  const schema = schemas();
  const entries = mapping();
  assert.deepEqual([...schema.reading.properties.reading.enum].sort(), Object.keys(tableStates).sort());
  assert.deepEqual(Object.keys(entries).sort(), Object.keys(tableStates).sort());
  for (const [reading, entry] of Object.entries(entries)) {
    const values = entry.state && typeof entry.state === 'object' ? Object.values(entry.state) : [entry.state];
    for (const state of values) {
      assert.ok(state === null || schema.summary.properties.state.enum.includes(state), `${reading}: unknown state ${state}`);
    }
    const shown = { 'stale-emitter': 'behind', 'cure-available': 'cure available', 'on-watch': 'on watch' }[reading] ?? reading;
    assert.equal(entry.shownAs, shown, `${reading}: shownAs`);
    assert.deepEqual(entry.state, tableStates[reading], `${reading}: state differs from R17`);
    assert.equal(entry.unknownEmitterState, null, `${reading}: R17 unknown emitter`);
  }
});
test('every precedence case uses declared facts and returns its expected reading', () => {
  const vocabulary = new Set(Object.keys(load(join(sprint, 'facts.json')).facts));
  const readings = schemas().reading.properties.reading.enum;
  const seen = new Set();
  const entries = cases();
  assert.ok(entries.length > 0, `no cases at ${join(sprint, 'precedence-cases.json')}`);
  for (const c of entries) {
    assert.equal(Object.hasOwn(c, 'open'), false, `case ${c.name}: open field is forbidden`);
    assert.ok(!seen.has(c.name), `case ${c.name}: duplicate name`);
    seen.add(c.name);
    assert.doesNotMatch(`${c.rule ?? ''} ${c.because ?? ''}`, /D\d/, `case ${c.name}: working name`);
    assert.ok(/\bR(?:[1-9]|1[0-8])\b/.test(c.because ?? ''), `case ${c.name}: missing citation`);
    assert.ok(c.facts && typeof c.facts === 'object' && !Array.isArray(c.facts), `case ${c.name}: malformed facts`);
    for (const [key, value] of Object.entries(c.facts)) {
      assert.ok(vocabulary.has(key) && typeof value === 'boolean', `case ${c.name}: invalid fact ${key}`);
    }
    assert.ok(c.expect === null || readings.includes(c.expect), `case ${c.name}: unknown expectation`);
    assert.equal(read(c.facts), c.expect, `case ${c.name}`);
  }
});
test('every combination of the declared facts gives exactly one reading, or none only when nothing failed', () => {
  const names = Object.keys(load(join(sprint, 'facts.json')).facts);
  const readings = new Set(schemas().reading.properties.reading.enum);
  assert.ok(names.includes('failed') && names.length <= 16, `facts.json must declare failed and at most 16 facts, has ${names.length}`);
  const reached = new Set();
  for (let bits = 0; bits < 2 ** names.length; bits++) {
    const facts = Object.fromEntries(names.map((name, i) => [name, Boolean(bits & (1 << i))]));
    const reading = read(facts);
    if (!facts.failed) assert.equal(reading, null, `nothing failed, yet ${JSON.stringify(facts)} reads ${reading}`);
    else assert.ok(readings.has(reading), `no reading for ${JSON.stringify(facts)}`);
    if (reading) reached.add(reading);
  }
  assert.deepEqual([...reached].sort(), [...readings].sort(), 'every one of the nine readings must be reachable');
});
test('the cases include every step of one unit in order', () => {
  assert.deepEqual(cases().filter(c => c.sequence === 'one-unit').map(c => c.step),
    [1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 'one-unit must contain all ten steps in order');
});
test('the story uses only declared reading and state words', () => {
  const storyPath = join(root, 'genesis/a2o/features/devflow/feedback-drains-when-cured.feature');
  const story = readFileSync(storyPath, 'utf8');
  const words = new Set();
  const stateWords = new Set();
  const glossary = story.match(/gives each claim a READING:([\s\S]*?)\. "Behind"/);
  assert.ok(glossary, `${storyPath}: reading glossary not found`);
  for (const m of glossary[1].matchAll(/"([^"]+)"/g)) words.add(m[1]);
  for (const line of story.split('\n')) {
    if (/judge.*reads|claim.*read "/.test(line)) {
      for (const m of line.matchAll(/(?:reads(?: the claim)?|read) "([^"]+)"/g)) words.add(m[1]);
    }
    if (/summary saying/.test(line)) {
      const m = line.match(/claim (?:was|is) ([a-z-]+)/);
      assert.ok(m, `${storyPath}: unparsed summary state: ${line.trim()}`);
      stateWords.add(m[1]);
    }
  }
  const shown = Object.values(mapping()).map(entry => entry.shownAs);
  const states = schemas().summary.properties.state.enum;
  for (const word of words) assert.ok(shown.includes(word), `${storyPath}: unknown reading ${word}`);
  for (const word of stateWords) assert.ok(states.includes(word), `${storyPath}: unknown state ${word}`);
});
// R1–R16: each life asserts both projections after every event, independently of static cases.
const lives = load(join(sprint, 'lives.json'));
for (const life of lives) test(`life: ${life.name}`, () => {
  let unit = start(life.decision10 === 'machine-may-close' ? false : KEPT_REPORT_HOLDS_A_CLAIM_OPEN);
  for (const [index, step] of life.steps.entries()) {
    const label = `${life.name} step ${index + 1} (${step.event.type}): ${step.rule}`;
    assert.equal(Object.hasOwn(step, 'open'), false, `${label}: ${step.open}`);
    assert.doesNotMatch(`${step.rule ?? ''} ${step.because ?? ''}`, /D\d/, `${label}: working name`);
    assert.match(step.rule, /\bR(?:[1-9]|1[0-8])\b/, label);
    const before = structuredClone(unit);
    const next = apply(unit, step.event);
    assert.deepEqual(unit, before, `${label}: apply mutated its input (R1)`);
    assert.deepEqual(next, apply(unit, step.event), `${label}: apply is not deterministic (R1)`);
    if (step.event.type === 'retire') { // R1, R2, R5, R7, R8
      const prior = read(facts(before));
      if ((prior === 'retired' && facts(before).note) ||
          (!['cure-available', 'retired'].includes(prior) && !(prior === 'on-watch' && step.event.ruling)))
        assert.deepEqual(next, before, `${label}: refused retire changed the unit`);
      assert.deepEqual(next.records.filter(r => r.kind === 'kept'),
        before.records.filter(r => r.kind === 'kept'), `${label}: kept records changed`);
      assert.deepEqual(next.aggregates.slice(0, before.aggregates.length),
        prior === 'cure-available' ? before.aggregates.map(a => ({ ...a, cure: next.current })) : before.aggregates,
        `${label}: R5 (D5) preserves existing aggregates except their cure`);
      assert.deepEqual(next.notes.slice(0, before.notes.length), before.notes, `${label}: old notes changed`);
    }
    if (step.event.type === 'fail') {
      const kept = next.records.filter(r => r.kind === 'kept');
      const beforeKept = before.records.filter(r => r.kind === 'kept');
      assert.deepEqual(kept.slice(0, beforeKept.length), beforeKept, `${label}: existing kept records changed`);
      assert.equal(kept.length, beforeKept.length + Number(step.event.record === 'kept'),
        `${label}: every kept failure adds exactly one record`);
      if (step.event.record === 'kept') assert.deepEqual(next.records.filter(r => r.kind === 'covered'),
        before.records.filter(r => r.kind === 'covered'), `${label}: kept failure changed covered records`);
    }
    unit = next;
    const f = facts(unit), reading = read(f);
    assert.deepEqual(Object.keys(f).sort(), Object.keys(load(join(sprint, 'facts.json')).facts).sort(), label);
    if (step.stock) for (const [key, value] of Object.entries(step.stock))
      assert.deepEqual(unit[key], value, `${label}: ${key}`);
    assert.equal(reading, step.reading, label);
    assert.equal(summaryState(reading, f), step.state, label);
  }
});
test('f-only-kept remains open under both decision-10 postures', () => {
  const life = lives.find(l => l.name === 'f-only-kept');
  for (const posture of [true, false]) {
    let unit = start(posture);
    for (const step of life.steps) {
      unit = apply(unit, step.event);
      assert.equal(read(facts(unit)), 'open');
      assert.equal(summaryState(read(facts(unit)), facts(unit)), 'open');
    }
  }
});
test('commit without a covered record is not modelled, including after compaction', () => {
  let compacted = apply(start(), { type: 'fail', release: 0, record: 'covered' });
  compacted = apply(compacted, { type: 'release', release: 1, entries: [[1, 'cured']] });
  compacted = apply(compacted, { type: 'retire' });
  for (const unit of [start(), compacted])
    for (const event of [{ type: 'commit' }, { type: 'commit', session: 1 }, { type: 'commit', session: 99 }])
      assert.throws(() => apply(unit, event), /^Error: not modelled:/);
  const kept = apply(apply(start(), { type: 'fail', release: 0, record: 'covered' }), { type: 'commit', session: 1 });
  assert.deepEqual(apply(kept, { type: 'commit', session: 1 }), kept);
});
test('commitment lives follow each decision-10 posture and change only the record kind', () => {
  for (const life of lives.filter(l => l.name.startsWith('a-commitment-'))) for (const posture of [true, false]) {
    let unit = start(posture);
    const expectedLife = lives.find(l => l.name === life.name.replace(/-if-decision-10-lets-the-machine-close$/, '') + (posture ? '' : '-if-decision-10-lets-the-machine-close')) ?? life;
    for (const step of expectedLife.steps) {
      const next = apply(unit, step.event);
      if (step.event.type === 'commit') {
        const expected = structuredClone(unit);
        expected.tick++;
        expected.records.findLast(r => r.kind === 'covered').kind = 'kept';
        assert.deepEqual(next, expected, life.name);
      }
      unit = next;
      assert.equal(read(facts(unit)), step.reading, `${life.name}: posture ${posture}`);
      assert.equal(summaryState(read(facts(unit)), facts(unit)), step.state, life.name);
    }
  }
});
test('R9–R16: scripted lives reach all nine readings', () => {
  const seen = new Set();
  for (const life of lives) {
    let unit = start(life.decision10 === 'machine-may-close' ? false : KEPT_REPORT_HOLDS_A_CLAIM_OPEN);
    for (const step of life.steps) {
      assert.equal(Object.hasOwn(step, 'open'), false, `${life.name}: ${step.open}`);
      unit = apply(unit, step.event);
      const reading = read(facts(unit));
      if (reading !== null) seen.add(reading);
    }
  }
  assert.deepEqual([...seen].sort(), [...schemas().reading.properties.reading.enum].sort());
});
test('R9–R16: facts declare exactly the predicates read uses, with rule citations', () => {
  const declared = load(join(sprint, 'facts.json')).facts;
  const used = [...new Set([...read.toString().matchAll(/f\.(\w+)/g)].map(m => m[1]))];
  assert.deepEqual(Object.keys(declared).sort(), used.sort());
  for (const text of Object.values(declared)) assert.match(text, /\bR(?:[1-9]|1[0-8])\b/);
});
// Each mutant must disagree with a fixed scripted expectation, including retained stocks.
test('thirty-four rule mutations are killed by scripted lives', () => {
  const source = readFileSync(import.meta.filename, 'utf8');
  const model = source.slice(source.indexOf('const KEPT_REPORT_HOLDS_A_CLAIM_OPEN'), source.indexOf('const errors ='))
    .replace(/const root =[\s\S]*?(?=export function read)/, '')
    .replaceAll('export function ', 'function ');
  const mutations = [
    ['decision10-a-second-branch-only', 'if (!f.keptRecordHolds && f.curePresent && (f.coveredRecords ||\n      (f.aggregates && f.currentCureCountedAgainst && f.differentCure)))', 'if (f.curePresent && (f.coveredRecords || (f.aggregates && f.currentCureCountedAgainst && f.differentCure && !f.keptRecordHolds)))', ['same-session-covered-then-kept']],
    ['decision10-b-watch-unguarded', '!f.keptRecordHolds && f.aggregates', 'f.aggregates', ['a-persons-report-arrives-during-the-watch']],
    ['decision10-c-retired-unguarded', '!f.keptRecordHolds && (f.retireMet', '(f.retireMet', ['a-persons-report-arrives-during-the-watch']],
    ['decision10-d-commitment-ignored', "u.records.some(r => r.kind === 'kept')", "u.records.some(r => r.kind === 'kept' && r.at !== undefined)", ['a-commitment-keeps-one-of-two-records']],
    ['decision10-e-counted-against-only', "u.records.some(r => r.kind === 'kept')", "recurrence.some(({ o }) => o && u.records.some(r => r.kind === 'kept' && r.session === o.session))", ['a-persons-report-before-the-cure-beside-a-covered-record']],
    ['decision10-f-kept-suppresses-covered', "!x.passed && x.record === 'covered' && x.session", '!x.passed && x.session', ['reviewer3-X2-kept-then-covered-same-session-if-decision-10-lets-the-machine-close']],
    ['decision10-g-commit-ignores-session', 'e.session === undefined ?', 'true ?', ['named-commit-keeps-older-participant-if-decision-10-lets-the-machine-close']],
    ['decision10-h-commit-default-oldest', "u.records.findLast(r => r.kind === 'covered')", "u.records.find(r => r.kind === 'covered')", ['unnamed-commit-keeps-newest-participant-if-decision-10-lets-the-machine-close']],
    ['r1-commit-adds-record', "record.kind = 'kept';", "u.records.push({ ...record, kind: 'kept' });",
      ['a-commitment-makes-the-only-record-kept']],
    ['r1-commit-no-op', "record.kind = 'kept';", '',
      ['a-commitment-tied-while-the-cure-waits']],
    ['r1-second-failure-adds-record', '!coveredBefore', 'true',
      ['mutation-g-one-record-per-session']],
    ['r1-kept-never-added', "if (e.record === 'kept')", "if (e.record === 'kept') return u; else if (false)",
      ['same-session-covered-then-kept']],
    ['r1-two-kept-collapse', "if (e.record === 'kept')", "if (e.record === 'kept' && u.records.some(r => r.kind === 'kept' && r.session === u.session)) return u; else if (e.record === 'kept')",
      ['two-kept-reports-in-one-session']],
    ['r1-kept-replaces-covered', "if (e.record === 'kept')", "if (e.record === 'kept' && u.records.some(r => r.kind === 'covered' && r.session === u.session)) u.records.splice(u.records.findIndex(r => r.kind === 'covered' && r.session === u.session), 1); if (e.record === 'kept')",
      ['same-session-covered-then-kept']],
    ['r1-kept-dropped-after-compaction', "if (e.record === 'kept')", "if (e.record === 'kept' && coveredBefore && !u.records.some(r => r.kind === 'covered' && r.session === u.session)) return u; else if (e.record === 'kept')",
      ['kept-failure-in-an-already-compacted-session']],
    ['decision10-machine-closes-ordinary-lives', 'const KEPT_REPORT_HOLDS_A_CLAIM_OPEN = true;',
      'const KEPT_REPORT_HOLDS_A_CLAIM_OPEN = false;',
      ['c-false-retired', 'reviewer-B5-kept-during-watch-from-head', 'reviewer-C1-kept-reopen-during-watch-then-watch-met']],
    ['d4-condition-also-counted-against', '(release ? [release] : conditions)',
      '(release ? [release, ...conditions] : conditions)', ['release-cure-qualifies-before-the-condition-if-decision-10-lets-the-machine-close']],
    ['future-entries-read', '.filter(([release]) => compareRelease(release, e.release) <= 0)', '',
      ['entries-newer-than-the-head-are-ignored']],
    ['d4-release-does-not-end-condition-reopening', 'Boolean(releasePresent(u)) || ', ''],
    ['d5-no-aggregate-branch', '(f.aggregates && f.currentCureCountedAgainst && f.differentCure)', 'false'],
    ['d5-moves-notes', 'for (const aggregate of u.aggregates) aggregate.cure = u.current;',
      'for (const aggregate of [...u.aggregates, ...u.notes]) aggregate.cure = u.current;'],
    ['a', 'a.participant !== null &&', 'true &&'],
    ['b', 'last.afterHead && last.at > u.landed', 'true'],
    ['c', 'u.known && u.horizon && !present(u)', 'u.known && u.horizon'],
    ['d', "u.records.filter(r => r.kind === 'covered').map", 'u.records.map'],
    ['e', "!(reading === 'on-watch' && e.ruling)", '!e.ruling'],
    ['f-current-cure-only', 'recurrence.some(({ c }) => c.id === u.current)', 'recurrence.length > 0'],
    ['h-head-bound', 'u.declaration = e.entries.filter(([release]) => compareRelease(release, e.release) <= 0).map',
      'u.declaration = (e.entries.length ? e.entries.filter(([release]) => compareRelease(release, e.release) <= 0) : u.declaration.map(c => [c.release, c.state])).map'],
    ['i-release-precedence', "releasePresent(u) ??\n  (u.holding ? u.cures.filter(c => c.kind === 'condition').at(-1) : undefined)",
      "u.holding ? u.cures.filter(c => c.kind === 'condition').at(-1) : releasePresent(u)"],
    ['r18-dropped', "e.state === 'withdrawn'", 'false'],
    ['r4-any-cured', "latestEntry(u)?.state === 'cured'", "u.declaration.some(e => e.state === 'cured')"],
    ['r10-withdrawn-cures-again', "n.state === 'cured' &&", 'true &&'],
    ['j-quiet-watch', '!failures.some(f => f.at > o.at)', 'true'],
    ['g', '!coveredBefore', 'true'],
  ];
  for (const [name, from, to, required = []] of mutations) {
    assert.ok(model.includes(from), `mutation ${name}: missing target`);
    const m = new Function('assert', 'mapping', model.replace(from, to) +
      ';return {start,apply,facts,read,summaryState};')(assert, mapping);
    const killed = lives.filter(life => {
      let u = m.start(life.decision10 === 'machine-may-close' ? false : undefined);
      return life.steps.some(step => {
        u = m.apply(u, step.event);
        const f = m.facts(u), r = m.read(f);
        return !Object.hasOwn(step, 'open') && (r !== step.reading || (!name.startsWith('decision10-') &&
          (m.summaryState(r, f) !== step.state || Object.entries(step.stock ?? {}).some(([k, v]) => JSON.stringify(u[k]) !== JSON.stringify(v)))));
      });
    });
    assert.ok(killed.length, `mutation ${name} survived`);
    if (/^decision10-[a-h]-/.test(name)) console.log(`${name}: READING killed by ${required.join(', ')}`);
    for (const life of required) assert.ok(killed.some(l => l.name === life),
      `mutation ${name} must be killed by ${life}`);
  }
});
