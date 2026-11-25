import assert from 'node:assert/strict';
import test from 'node:test';

import {
  classifyChange, countExecutions, enrichmentsIn, gradeLine, gradeMode, gradeOf, gradeStep, rawModes, wouldSuffice,
} from './gate-grade.mjs';
import { isSiblingGovernancePath } from './gate-evidence.mjs';

const KEY = 'k'.repeat(64);
const OTHER = 'o'.repeat(64);
const SELF = 'a'.repeat(64);
const BASE = 'b'.repeat(40);
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
// A range of ordinary files (mode 100644 on both sides), unless a mode is given per path.
const rangeOf = (files, modes = {}) => ({
  base: BASE, files, modes: Object.fromEntries(files.map(path => [path, modes[path] || ['100644', '100644']])),
});
const docs = rangeOf(['genesis/docs/notes.md']);
const lineOf = (records, range = docs, today = 'ran green') => {
  const grade = gradeOf({ range, isGovernance: isSiblingGovernancePath, project: 'fixture', key: KEY, selfId: SELF, ...records });
  return { grade, line: gradeLine('fixture', grade, today) };
};
const cls = (files, modes) => classifyChange(rangeOf(files, modes), { isGovernance: isSiblingGovernancePath }).cls;

test('GATE_GRADE is off or shadow, and there is no on', () => {
  assert.equal(gradeMode({}), 'shadow');
  assert.equal(gradeMode({ GATE_GRADE: 'shadow' }), 'shadow');
  assert.equal(gradeMode({ GATE_GRADE: 'off' }), 'off');
  assert.equal(gradeMode({ GATE_GRADE: 'on' }), 'shadow', 'on is not a mode: it reads as shadow');
});

test('this device\'s own green is said apart and is not an independent execution', () => {
  const { grade, line } = lineOf({ local: [local(true), local(true, { commit: 'd'.repeat(40) })] });
  assert.equal(grade.tally.independent, 0, 'two own records are still zero independent executions');
  assert.equal(grade.tally.own, 'green');
  assert.match(line, /executions for this closure: own: green; independent, signature-checked: 0 \(no reader checks a sibling's signature yet\); sibling claims, signature unchecked: 0; carrier: none/);
  assert.equal(gradeStep('fixture', grade), 'fixture:grade:documentation:n0c0:yes');
  assert.equal(lineOf({}).grade.tally.own, 'none');
  assert.equal(lineOf({ local: [local(false)] }).grade.tally.own, 'red');
});

test('a sibling row is a signature-unchecked claim, one per device, and never suffices', () => {
  const twice = tally({ siblings: [sibling(true), sibling(true)] });
  assert.equal(twice.claims, 1, 'one sibling device, two rows');
  assert.equal(twice.independent, 0);
  const two = tally({ siblings: [sibling(true), sibling(true, { device: SIBLING_2 })] });
  assert.equal(two.claims, 2);
  assert.equal(two.independent, 0, 'no claim becomes a checked execution');

  const { grade, line } = lineOf({ siblings: [sibling(true), sibling(true, { device: SIBLING_2 })] });
  assert.match(line, /own: none; independent, signature-checked: 0 .*; sibling claims, signature unchecked: 2 \(sibling z6MkpZRhMw…0001 green, sibling z6MkgHjuSe…0002 green\)/);
  assert.equal(grade.would.suffice, 'no');
  assert.match(line, /would suffice: no \(a sibling's claim is signature-unchecked and does not count/);
  assert.equal(gradeStep('fixture', grade), 'fixture:grade:documentation:n0c2:no');
  // Tooling too: a claim alone never reaches yes on any row.
  assert.equal(lineOf({ siblings: [sibling(true)] }, rangeOf(['genesis/a2o/scripts/look.ts'])).grade.would.suffice, 'no');
});

test('a carrier\'s green counts zero and is listed apart, whoever signed it', () => {
  const alone = lineOf({ carriers: [jenkins(true)] });
  assert.equal(alone.grade.tally.independent, 0);
  assert.equal(alone.grade.tally.claims, 0);
  assert.match(alone.line, /carrier: jenkins green at this commit \(keyed on the commit tree, not this closure\)/);
  assert.match(alone.line, /would suffice: no \(a carrier's result counts 0 executions/);
  // The harvested note is signed by this device's own key; even keyed on this closure it is a
  // carrier's result, because standing comes from the reader it came through.
  const sameKey = tally({ carriers: [{ ...jenkins(true, { closure: KEY }), agentId: SELF }] });
  assert.equal(sameKey.own, 'none');
  assert.equal(sameKey.claims, 0);
  assert.deepEqual(sameKey.carried, ['jenkins green']);
  // The station's fixture: a sibling's green beside a runner's green is one claim, not two.
  assert.equal(tally({ siblings: [sibling(true)], carriers: [jenkins(true)] }).claims, 1);
});

test('a red beside greens is shown, never netted out, and would-suffice is no', () => {
  const { grade, line } = lineOf({ local: [local(true)], siblings: [sibling(false)] });
  assert.equal(grade.tally.own, 'green', 'the green is still said; the red is not subtracted from it');
  assert.match(line, /disagreements: sibling z6MkpZRhMw…0001 red/);
  assert.match(line, /would suffice: no \(a disagreement is on record\)/);
  const sameDevice = lineOf({ local: [local(true), local(false)] });
  assert.match(sameDevice.line, /own: green and red; .*disagreements: this device red/);
  assert.equal(sameDevice.grade.would.suffice, 'no');
  const carrierRed = lineOf({ local: [local(true)], carriers: [jenkins(false)] });
  assert.match(carrierRed.line, /disagreements: carrier jenkins red/);
  assert.equal(carrierRed.grade.would.suffice, 'no');
  // A red outranks unexamined: a disagreement is a definite no on a library row too.
  assert.equal(lineOf({ local: [local(true)], siblings: [sibling(false)] }, rangeOf(['elohim/eprfs/epr-cli/src/a.rs'])).grade.would.suffice, 'no');
  const agreeing = lineOf({ local: [local(true)], siblings: [sibling(true)] });
  assert.match(agreeing.line, /disagreements: none/);
  assert.equal(agreeing.grade.would.suffice, 'yes');
  assert.match(agreeing.line, /would suffice: yes \(row: this device's own green for this key\)/);
});

test('a record for another closure or project, or with no readable closure, counts for nothing and is listed', () => {
  const result = tally({
    local: [local(true, { inputsHash: OTHER }), local(true, { inputsHash: null })],
    siblings: [sibling(true, { closure: OTHER }), sibling(true, { project: 'elsewhere' }), sibling(true, { closure: '' })],
  });
  assert.equal(result.own, 'none');
  assert.equal(result.claims, 0);
  assert.deepEqual(result.by, []);
  assert.ok(result.ignored.some(text => /a local note for another closure/.test(text)));
  assert.ok(result.ignored.some(text => /a local note whose closure cannot be read/.test(text)));
  assert.ok(result.ignored.some(text => /a sibling verdict for another closure or project/.test(text)));
  assert.ok(result.ignored.some(text => /a sibling verdict whose closure cannot be read/.test(text)));
  assert.deepEqual(tally({ siblings: [sibling(false, { closure: OTHER })] }).disagreements, []);
});

test('an unverifiable signer is listed and not counted', () => {
  const { grade, line } = lineOf({
    local: [local(true, { agentId: 'f'.repeat(64) })],
    unverifiable: [{ commit: 'e'.repeat(40), reason: 'signature does not verify' }],
    siblings: [sibling(true, { device: 'service:jenkins' }), sibling(true, { source: 'elsewhere' })],
  });
  assert.equal(grade.tally.own, 'none');
  assert.equal(grade.tally.claims, 0);
  assert.match(line, /not counted: .*a local green signed by ffffffffffff, which resolves to no admitted device/);
  assert.match(line, /an unverifiable note at eeeeeeeee \(signature does not verify\)/);
  assert.match(line, /a sibling verdict naming no device key/);
  assert.match(line, /a verdict that did not come through the sibling reader/);
  assert.equal(grade.would.suffice, 'no');
  assert.match(grade.would.why, /unverifiable note/);
  const noKey = countExecutions({ project: 'fixture', key: KEY, selfId: null, local: [local(true)] });
  assert.equal(noKey.own, 'none', 'with no workspace key read, a local row is nobody\'s');
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
  assert.equal(result.claims, 0, 'the enrichment adds no execution and no claim');
  assert.ok(result.ignored.some(entry => /an unsigned re-executed enrichment green/.test(entry)));
  assert.ok(result.disagreements.some(entry => /re-executed enrichment red \(unsigned/.test(entry)));
});

test('the class comes from the paths the push touches, strictest row first', () => {
  assert.equal(cls(['genesis/docs/a.md', 'README.md']), 'documentation');
  assert.equal(cls(['genesis/docs/a.md', 'genesis/a2o/scripts/look.ts']), 'tooling');
  assert.equal(cls(['genesis/docs/a.md', 'elohim/eprfs/epr-cli/src/flow/enrich.rs']), 'library-crate');
  assert.equal(cls(['elohim/eprfs/epr-cli/src/flow/enrich.rs', 'doorway/doorway-service/src/server/http.rs']), 'serving-path');
  assert.equal(cls(['doorway/doorway-service/src/server/http.rs', 'genesis/orchestrator/gate-runner.mjs']), 'governance');
  assert.equal(cls(['.epr-meta/elohim/packages/agentdocs/elohim-root-gospel.json', 'CLAUDE.md', 'genesis/manifests/habits.yaml']), 'tooling');
  assert.equal(cls(['genesis/manifests/habits.yaml', 'genesis/orchestrator/.epr-meta/push-gate-pays-once.habit.md']), 'documentation');
  assert.equal(cls(['genesis/manifests/cluster-state.yaml']), null, 'a manifest no row names is not guessed');
});

test('everything that selects, runs, keys, records or enforces a gate is governance', () => {
  for (const path of [
    '.husky/pre-push.bash', '.husky/pre-push', '.husky/hooks-path.mjs',
    'genesis/orchestrator/run-local-gate.sh', 'genesis/orchestrator/gate-steps.sh', 'genesis/orchestrator/cargo-lint.mjs',
    'genesis/orchestrator/gate-runner.mjs', 'genesis/orchestrator/gate-evidence.mjs', 'genesis/orchestrator/gate-grade.mjs',
    'genesis/orchestrator/gate-grade.test.mjs', 'genesis/orchestrator/gate-input-audit.mjs',
    'genesis/orchestrator/handover-record.mjs', 'genesis/orchestrator/device-notes-publish.mjs',
    'genesis/orchestrator/attested-pins.mjs', 'genesis/orchestrator/doc-cut-guard.mjs',
    'genesis/orchestrator/suite-reads.json', 'genesis/agentic/pool-policy.json', 'genesis/agentic/bin/pool-lib.sh',
    'elohim/eprfs/build-manifest.json', 'build-manifest.json', 'justfile', 'elohim/eprfs/justfile',
    'elohim/epr/rust-toolchain.toml', 'rust-toolchain',
    'genesis/docs/superpowers/specs/2026-11-24-verdict-is-an-attestation-on-an-artifact-design.md',
  ]) {
    assert.equal(cls([path]), 'governance', path);
    // And beside a document it still decides the class.
    assert.equal(cls(['README.md', path]), 'governance', `README.md + ${path}`);
  }
  const governance = lineOf({ local: [local(true)], siblings: [sibling(true)] }, rangeOf(['.husky/pre-push.bash']), 'reuse own');
  assert.equal(governance.grade.would.suffice, 'no');
  assert.match(governance.line, /class a gate's own judge or recipe; /);
  assert.match(governance.line, /\| today: reuse own$/);
  assert.equal(gradeStep('fixture', governance.grade), 'fixture:grade:governance:n0c1:no');
});

test('a gitlink is governance and a symlink is undetermined, whatever the filename says', () => {
  assert.equal(cls(['elohim/brit'], { 'elohim/brit': ['160000', '160000'] }), 'governance');
  assert.equal(cls(['notes.md'], { 'notes.md': ['100644', '160000'] }), 'governance', 'a document name that became a gitlink');
  assert.equal(cls(['notes.md'], { 'notes.md': ['160000', '000000'] }), 'governance', 'a removed gitlink');
  const linked = classifyChange(rangeOf(['notes.md'], { 'notes.md': ['000000', '120000'] }), { isGovernance: isSiblingGovernancePath });
  assert.equal(linked.cls, null);
  assert.match(linked.why, /notes\.md is a symlink, never classified by its name/);
  // A path whose mode was not read is not classified by its name either.
  const unread = classifyChange({ base: BASE, files: ['notes.md'], modes: null }, { isGovernance: isSiblingGovernancePath });
  assert.equal(unread.cls, null);
  assert.match(unread.why, /the git mode of notes\.md was not read/);
  assert.equal(classifyChange({ base: BASE, files: ['notes.md'], modes: {} }, {}).cls, null);

  assert.deepEqual(rawModes(`:100644 100644 ${'1'.repeat(40)} ${'2'.repeat(40)} M\0a.md\0:000000 160000 ${'0'.repeat(40)} ${'3'.repeat(40)} A\0elohim/brit\0`),
    { 'a.md': ['100644', '100644'], 'elohim/brit': ['000000', '160000'] });
  assert.equal(rawModes('a.md\0'), null, 'a name list is not a mode reading');
  assert.deepEqual(rawModes(''), {});
});

test('a row that asks for evidence not examined reads unexamined, never yes, and the row kind keeps it', () => {
  const records = { local: [local(true)], siblings: [sibling(true)] };
  const library = lineOf(records, rangeOf(['elohim/eprfs/epr-cli/src/flow/enrich.rs']));
  assert.equal(library.grade.would.suffice, 'unexamined');
  assert.match(library.line, /would suffice: unexamined: the crate's own key \(the row reads a green at that key; only the project key was read\) \|/);
  assert.equal(gradeStep('fixture', library.grade), 'fixture:grade:library-crate:n0c1:na');

  const serving = lineOf(records, rangeOf(['doorway/doorway-service/src/server/http.rs']));
  assert.equal(serving.grade.would.suffice, 'unexamined');
  assert.match(serving.line, /would suffice: unexamined: the household witness the row requires at dev and main \|/);
  assert.equal(gradeStep('fixture', serving.grade), 'fixture:grade:serving-path:n0c1:na');
  for (const { line } of [library, serving]) assert.doesNotMatch(line, /would suffice: yes/);
});

test('class undetermined prints no would-suffice', () => {
  const unplaced = lineOf({ local: [local(true)] }, rangeOf(['genesis/docs/a.md', 'app/elohim-app/src/app/app.component.ts']));
  assert.equal(unplaced.grade.would, null);
  assert.match(unplaced.line, /class: undetermined \(no row places app\/elohim-app\/src\/app\/app\.component\.ts\)/);
  assert.doesNotMatch(unplaced.line, /would suffice/);
  assert.match(unplaced.line, /own: green/, 'what is held is still printed');
  assert.equal(gradeStep('fixture', unplaced.grade), 'fixture:grade:undetermined:n0c0:na');
  for (const range of [null, { base: null, files: ['a.md'] }, { base: BASE, files: [] }]) {
    const { grade, line } = lineOf({ local: [local(true)] }, range);
    assert.equal(grade.would, null);
    assert.match(line, /class: undetermined \(/);
    assert.doesNotMatch(line, /would suffice/);
  }
  assert.equal(wouldSuffice(null, tally({ local: [local(true)] })), null);
});

test('a label grants nothing: every line is marked shadow and proposed, and none says trusted or verified', () => {
  const every = [
    lineOf({ local: [local(true)] }).line,
    lineOf({ siblings: [sibling(true)] }).line,
    lineOf({ carriers: [jenkins(true)] }).line,
    lineOf({ local: [local(true)] }, rangeOf(['justfile'])).line,
    lineOf({ local: [local(true)] }, rangeOf(['elohim/eprfs/epr-cli/src/a.rs'])).line,
    lineOf({ local: [local(true, { agentId: 'f'.repeat(64) })], unverifiable: [{ commit: 'e'.repeat(40), reason: 'bad' }] }).line,
  ];
  for (const line of every) {
    assert.match(line, /^\[gate\] fixture grade \(shadow, proposed table\): /);
    assert.doesNotMatch(line, /\btrusted\b|\bverified\b|\bapproved\b|\baccepted\b/);
  }
});
