import assert from 'node:assert/strict';
import test from 'node:test';

import {
  classifyChange, countExecutions, enrichmentsIn, gradeLine, gradeMode, gradeOf, gradeStep, wouldSuffice,
} from './gate-grade.mjs';

const KEY = 'k'.repeat(64);
const OTHER = 'o'.repeat(64);
const SELF = 'a'.repeat(64);
const SIBLING = 'did:key:z6MkpZRhMwSiblingDeviceKeyOfTheSteward0001';
const SIBLING_2 = 'did:key:z6MkgHjuSecondSiblingDeviceKeyOfSteward0002';

const local = (success, { agentId = SELF, inputsHash = KEY, commit = 'c'.repeat(40) } = {}) => ({
  commit, attestation: { agentId, success, inputsHash, builtAt: '2026-11-25T00:00:00Z' },
});
const sibling = (success, { device = SIBLING, closure = KEY, project = 'fixture', source = 'sibling' } = {}) => ({
  source, device, project, closure, success, commit: 'c'.repeat(40), seconds: 12,
});
const jenkins = (success, { closure = 't'.repeat(40), state = 'verified' } = {}) => ({
  name: 'jenkins', state, success, closure,
});
const tally = records => countExecutions({ project: 'fixture', key: KEY, selfId: SELF, ...records });
const docs = { base: 'b'.repeat(40), files: ['genesis/docs/notes.md'] };
const lineOf = (records, range = docs, today = 'ran green') => {
  const grade = gradeOf({ range, isGovernance: () => false, project: 'fixture', key: KEY, selfId: SELF, ...records });
  return { grade, line: gradeLine('fixture', grade, today) };
};

test('GATE_GRADE is off or shadow, and there is no on', () => {
  assert.equal(gradeMode({}), 'shadow');
  assert.equal(gradeMode({ GATE_GRADE: 'shadow' }), 'shadow');
  assert.equal(gradeMode({ GATE_GRADE: 'off' }), 'off');
  assert.equal(gradeMode({ GATE_GRADE: 'on' }), 'shadow', 'on is not a mode: it reads as shadow');
});

test('two records from one device count once', () => {
  const own = tally({ local: [local(true), local(true, { commit: 'd'.repeat(40) })] });
  assert.equal(own.independent, 1);
  assert.deepEqual(own.by, ['this device green']);

  const twice = tally({ siblings: [sibling(true), sibling(true)] });
  assert.equal(twice.independent, 1, 'one sibling device, two verdicts');

  const both = tally({ local: [local(true)], siblings: [sibling(true), sibling(true, { device: SIBLING_2 })] });
  assert.equal(both.independent, 3, 'this device and two different sibling devices');
  assert.equal(both.by.length, 3);
});

test('a carrier\'s green counts zero and is listed apart, whoever signed it', () => {
  const alone = lineOf({ carriers: [jenkins(true)] });
  assert.equal(alone.grade.tally.independent, 0);
  assert.match(alone.line, /executions for this closure: 0 independent, carrier: jenkins green at this commit \(keyed on the commit tree, not this closure\)/);
  assert.match(alone.line, /would suffice: no \(row needs 1; a carrier's result counts 0 executions/);

  // The harvested note is signed by this device's own key; even keyed on this closure it is a
  // carrier's result, because standing comes from the reader it came through.
  const sameKey = tally({ carriers: [{ ...jenkins(true, { closure: KEY }), agentId: SELF }] });
  assert.equal(sameKey.independent, 0);
  assert.deepEqual(sameKey.carried, ['jenkins green']);

  // The station's Verify fixture: a sibling's green and a sponsored runner's green print one.
  const withSibling = lineOf({ siblings: [sibling(true)], carriers: [jenkins(true)] });
  assert.equal(withSibling.grade.tally.independent, 1);
  assert.match(withSibling.line, /1 independent \(sibling z6MkpZRhMw…0001 green\), carrier: jenkins green/);
  assert.match(withSibling.line, /would suffice: yes \(row needs 1\)/);
});

test('a red beside greens is shown, never netted out, and would-suffice is no', () => {
  const { grade, line } = lineOf({ local: [local(true)], siblings: [sibling(false)] });
  assert.equal(grade.tally.independent, 1, 'the green is still counted; the red is not subtracted from it');
  assert.match(line, /disagreements: sibling z6MkpZRhMw…0001 red/);
  assert.match(line, /would suffice: no \(a disagreement is on record\)/);

  const sameDevice = lineOf({ local: [local(true), local(false)] });
  assert.match(sameDevice.line, /disagreements: this device red/);
  assert.equal(sameDevice.grade.would.suffice, 'no');

  const carrierRed = lineOf({ local: [local(true)], carriers: [jenkins(false)] });
  assert.match(carrierRed.line, /disagreements: carrier jenkins red/);
  assert.equal(carrierRed.grade.would.suffice, 'no');

  const agreeing = lineOf({ local: [local(true)], siblings: [sibling(true)] });
  assert.match(agreeing.line, /disagreements: none/);
  assert.equal(agreeing.grade.would.suffice, 'yes');
});

test('a record for another closure or project, or with no readable closure, counts for nothing and is listed', () => {
  const result = tally({
    local: [local(true, { inputsHash: OTHER }), local(true, { inputsHash: null })],
    siblings: [sibling(true, { closure: OTHER }), sibling(true, { project: 'elsewhere' }), sibling(true, { closure: '' })],
  });
  assert.equal(result.independent, 0);
  assert.deepEqual(result.by, []);
  assert.ok(result.ignored.some(text => /a local note for another closure/.test(text)));
  assert.ok(result.ignored.some(text => /a local note whose closure cannot be read/.test(text)));
  assert.ok(result.ignored.some(text => /a sibling verdict for another closure or project/.test(text)));
  assert.ok(result.ignored.some(text => /a sibling verdict whose closure cannot be read/.test(text)));
  // Its red is not this closure's red either.
  const otherRed = tally({ siblings: [sibling(false, { closure: OTHER })] });
  assert.deepEqual(otherRed.disagreements, []);
});

test('an unverifiable signer is listed and not counted', () => {
  const { grade, line } = lineOf({
    local: [local(true, { agentId: 'f'.repeat(64) })],
    unverifiable: [{ commit: 'e'.repeat(40), reason: 'signature does not verify' }],
    siblings: [sibling(true, { device: 'service:jenkins' }), sibling(true, { source: 'elsewhere' })],
  });
  assert.equal(grade.tally.independent, 0);
  assert.match(line, /not counted: .*a local green signed by ffffffffffff, which resolves to no admitted device/);
  assert.match(line, /an unverifiable note at eeeeeeeee \(signature does not verify\)/);
  assert.match(line, /a sibling verdict naming no device key/);
  assert.match(line, /a verdict that did not come through the sibling reader/);
  assert.equal(grade.would.suffice, 'no');
  assert.match(grade.would.why, /unverifiable note/);

  // With no workspace key read, this device's own rows cannot be told from anyone's: none counts.
  const noKey = countExecutions({ project: 'fixture', key: KEY, selfId: null, local: [local(true)] });
  assert.equal(noKey.independent, 0);
});

test('an unsigned enrichment is read, listed and not counted; its red is shown', () => {
  const slots = (outcome, extra = []) => JSON.stringify({ cid: 'bafy', record: { kind: 'event', classifiedAs: [
    'run:observation', '.', 'reason:re-executed', 'measure:attestation-enrichment@1', 'value:1',
    `env:closure=${KEY}`, `env:device=${SIBLING}`, 'env:kind=re-executed', `env:outcome=${outcome}`,
    'env:project=fixture', ...extra,
  ] } });
  const text = [
    slots('green'), slots('red'), slots('green', [`via:${SIBLING_2}`]), 'not json attestation-enrichment@1',
    JSON.stringify({ record: { classifiedAs: ['measure:gate-verdict@1', 'env:project=fixture'] } }),
  ].join('\n');
  const rows = enrichmentsIn(text);
  assert.equal(rows.length, 2, 'a crossed row and a malformed line are not enrichments this writer made');
  const result = tally({ local: [local(true)], enrichments: rows });
  assert.equal(result.independent, 1, 'the enrichment adds no execution');
  assert.ok(result.ignored.some(entry => /an unsigned re-executed enrichment green/.test(entry)));
  assert.ok(result.disagreements.some(entry => /re-executed enrichment red \(unsigned/.test(entry)));
});

test('the class comes from the paths the push touches, strictest row first', () => {
  const base = 'b'.repeat(40);
  const isGovernance = path => path === 'genesis/orchestrator/gate-runner.mjs';
  const cls = files => classifyChange({ base, files }, { isGovernance }).cls;
  assert.equal(cls(['genesis/docs/a.md', 'README.md']), 'documentation');
  assert.equal(cls(['genesis/docs/a.md', 'genesis/orchestrator/doc-cut-guard.mjs']), 'tooling');
  assert.equal(cls(['genesis/docs/a.md', 'elohim/eprfs/epr-cli/src/flow/enrich.rs']), 'library-crate');
  assert.equal(cls(['elohim/eprfs/epr-cli/src/flow/enrich.rs', 'doorway/doorway-service/src/server/http.rs']), 'serving-path');
  assert.equal(cls(['doorway/doorway-service/src/server/http.rs', 'genesis/orchestrator/gate-runner.mjs']), 'governance');
  assert.equal(cls(['elohim/eprfs/justfile']), 'governance', 'a recipe is the gate\'s own judge');
  assert.equal(cls(['elohim/eprfs/build-manifest.json']), 'governance');
});

test('class undetermined prints no would-suffice', () => {
  const unplaced = lineOf({ local: [local(true)] }, { base: 'b'.repeat(40), files: ['genesis/docs/a.md', 'app/elohim-app/src/app/app.component.ts'] });
  assert.equal(unplaced.grade.would, null);
  assert.match(unplaced.line, /class: undetermined \(no row places app\/elohim-app\/src\/app\/app\.component\.ts\)/);
  assert.doesNotMatch(unplaced.line, /would suffice/);
  assert.match(unplaced.line, /1 independent \(this device green\)/, 'the count is still printed');
  assert.equal(gradeStep('fixture', unplaced.grade), 'fixture:grade:undetermined:n1:na');

  for (const range of [null, { base: null, files: ['a.md'] }, { base: 'b'.repeat(40), files: [] }]) {
    const { grade, line } = lineOf({ local: [local(true)] }, range);
    assert.equal(grade.would, null);
    assert.match(line, /class: undetermined \(/);
    assert.doesNotMatch(line, /would suffice/);
  }
  assert.equal(wouldSuffice(null, tally({ local: [local(true)] })), null);
});

test('the governance row never reads as sufficient, and a label grants nothing', () => {
  const governance = lineOf({ local: [local(true)], siblings: [sibling(true)] },
    { base: 'b'.repeat(40), files: ['elohim/eprfs/justfile'] }, 'reuse own');
  assert.equal(governance.grade.would.suffice, 'no');
  assert.match(governance.line, /class a gate's own judge or recipe; /);
  assert.match(governance.line, /\| today: reuse own$/);
  assert.equal(gradeStep('fixture', governance.grade), 'fixture:grade:governance:n2:no');

  const every = [
    governance.line,
    lineOf({ local: [local(true)] }).line,
    lineOf({ carriers: [jenkins(true)] }).line,
    lineOf({ local: [local(true, { agentId: 'f'.repeat(64) })], unverifiable: [{ commit: 'e'.repeat(40), reason: 'bad' }] }).line,
  ];
  for (const line of every) {
    assert.match(line, /^\[gate\] fixture grade \(shadow, proposed table\): /);
    assert.doesNotMatch(line, /\btrusted\b|\bverified\b|\bapproved\b|\baccepted\b/);
  }
});
