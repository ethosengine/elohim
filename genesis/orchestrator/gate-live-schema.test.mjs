import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import Ajv2020 from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

import { createRecognizer } from './gate-failure-lines.mjs';
import { appendEvent, closeRun, openRun, readRun } from './gate-live.mjs';

// Ajv2020 + strict/allErrors follows schemas/scripts/test-replicates-content-schema.mjs.
const schemaRoot = new URL('../../elohim/sdk/schemas/v1/', import.meta.url);
const load = name => JSON.parse(readFileSync(new URL(name, schemaRoot), 'utf8'));
const schema = load('gate-live-event.schema.json');
const ajv = new Ajv2020({ strict: true, allErrors: true });
addFormats(ajv);
ajv.addKeyword('tsType'); // The shared Unanswered envelope's codegen annotation.
ajv.addSchema(load('objects/unanswered.schema.json'),
  new URL('objects/unanswered.schema.json', schema.$id).href);
const validate = ajv.compile(schema);

const at = '2026-10-26T12:00:00.000Z';
const event = (kind, fields = {}) => ({ v: 1, run: '20261026T120000Z-abcdef-123456', seq: 1, at, kind, ...fields });
const diagnostic = {
  class: 'test', fingerprint: '012345abcdef', locus: { test: 'accepts a local gate' },
  nth: 1, excerpt: 'test accepts a local gate ... FAILED',
};
const failure = event('failure', { project: 'orchestrator', failure: diagnostic });
const verdict = event('gate-verdict', { project: 'orchestrator', verdict: { exit: 0, seconds: 0 } });
const derivation = { method: `bafyrei${'a'.repeat(52)}`, input: `bafyrei${'b'.repeat(52)}`, hit: false };
const unanswered = {
  why: 'interrupted', comeBack: { when: 'on-event', event: 'gate-restarted' },
  meanwhile: { local: 'use-held', what: 'the device-local run journal' }, escalation: null,
};

function accepts(value) {
  assert.equal(validate(value), true, JSON.stringify(validate.errors, null, 2));
}

test('validates every event kind from the actual journal and recognizer writers', t => {
  const root = mkdtempSync(join(tmpdir(), 'gate-live-schema-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const now = () => new Date(at);
  const { path } = openRun({ root, now });
  appendEvent(path, { kind: 'gate-start', project: 'orchestrator' }, { now });
  const recognizer = createRecognizer({
    project: 'orchestrator',
    emit: value => appendEvent(path, value, { now }),
  });
  for (const line of [
    'running 1 test', 'test accepts_local ... FAILED', '---- accepts_local stdout ----',
    "thread 'accepts_local' panicked at src/gate.rs:42:9:", 'assertion failed', 'failures:',
    'src/check.ts(12,3): error TS2322: incompatible type',
    'Diff in /checkout/src/fmt.rs:3:', '/checkout/src/lint.ts',
    '  7:2  error  unexpected variable  no-unused-vars', 'No space left on device',
    'error: Recipe `test` failed on line 18 with exit code 101',
  ]) recognizer.push(line);
  recognizer.end();
  appendEvent(path, { kind: 'gate-verdict', project: 'orchestrator', verdict: {
    exit: 101, seconds: 12, failure_class: 'code', failures: recognizer.summary().failures,
  } }, { now });
  closeRun(path, 'failed', { now });
  const { events } = readRun(path);
  assert.deepEqual([...new Set(events.map(row => row.kind))].sort(), [
    'detail', 'failure', 'gate-start', 'gate-verdict', 'run-close', 'run-open', 'step-failed',
  ]);
  assert.deepEqual([...new Set(events.filter(row => row.failure).map(row => row.failure.class))].sort(),
    ['compile', 'env', 'format', 'lint', 'test']);
  for (const row of events) accepts(row);
});

const corpus = [
  ['candidate and session', event('run-open', { candidate: { sha: 'a'.repeat(40), target: 'refs/heads/dev' }, session: 'anon-123' })],
  ['null candidate and session', event('run-open', { candidate: { sha: null, target: null }, session: null })],
  ['attested gate', event('gate-start', { project: 'brit', attested: true })],
  ['test without a file', failure],
  ['empty locus', { ...failure, failure: { ...diagnostic, locus: {} } }],
  ['relative file', { ...failure, failure: { ...diagnostic, locus: { file: 'src/gate.rs', line: 42 } } }],
  ['detail without nth', event('detail', { project: 'orchestrator', failure: {
    class: 'test', fingerprint: diagnostic.fingerprint,
    locus: { file: '/checkout/src/gate.rs', line: 42, test: 'accepts_local' }, excerpt: 'assertion failed',
  } })],
  ['minimal verdict', verdict],
  ['style-floor verdict', { ...verdict, step: 'rust-style', verdict: { exit: 2, seconds: 0 } }],
  ['untapped verdict', { ...verdict, verdict: { exit: 1, seconds: 0.25, failure_class: 'unknown', failures: null } }],
  ['environment verdict', { ...verdict, verdict: { exit: 143, seconds: 2, failure_class: 'environment', failures: 0 } }],
  ['full verdict', { ...verdict, verdict: {
    exit: 1, seconds: 12, failure_class: 'code', failures: 200, overflow: 4, evidence: derivation.input,
  } }],
  ['passed verdict', { ...verdict, verdict: { exit: 0, seconds: 12, failure_class: null, failures: 0 } }],
  ['legacy reused verdict', { ...verdict, verdict: { ...verdict.verdict, reused: true } }],
  ['legacy deferred verdict', { ...verdict, verdict: { ...verdict.verdict, deferred: true } }],
  ['answered derivation', { ...verdict, status: 'Answered', derivation }],
  ['answered failure', { ...verdict, status: 'Answered', derivation, verdict: { exit: 1, seconds: 5 } }],
  ['derivation hit', { ...verdict, status: 'Answered', derivation: { ...derivation, hit: true }, verdict: { ...verdict.verdict, reused: true } }],
  ['interrupted Task', { ...verdict, status: 'Unanswered', derivation, unanswered, verdict: { exit: 143, seconds: 5 } }],
  ['deferred Task', { ...verdict, status: 'Unanswered', derivation, unanswered: { ...unanswered, why: 'deferred-to-ci', meanwhile: { local: 'run-here' } }, verdict: { ...verdict.verdict, deferred: true } }],
  ['escalated answer with retry time', { ...verdict, status: 'Unanswered', unanswered: {
    ...unanswered, comeBack: { when: 'at', epochSecs: 1793016000 },
    meanwhile: { local: 'nothing', because: 'the device is not carrying this Task' },
    escalation: { asked: 2, changed: 'nothing', whoCanBeAsked: ['sponsor'] },
  } }],
  ...['passed', 'failed', 'interrupted', 'gates-passed'].map(outcome =>
    [`run-close ${outcome}`, event('run-close', { outcome })]),
  ['gate loop with deferrals', event('run-close', { outcome: 'gates-passed', deferred: 2, unanswered })],
  ['interrupted run answer', event('run-close', { outcome: 'interrupted', unanswered })],
  ...[
    { label: 'mine', sessions: ['self'] }, { label: 'other', sessions: ['anon-123'] },
    { label: 'ambiguous', sessions: ['anon-123', 'self'] }, { label: 'unattributed', sessions: [] },
  ].map(attribution => [`attribution ${attribution.label}`, { ...failure, attribution }]),
];
for (const [name, value] of corpus) test(`accepts ${name}`, () => accepts(value));

const without = (value, key) => Object.fromEntries(Object.entries(value).filter(([name]) => name !== key));
const negatives = [
  ['unknown version', { ...failure, v: 2 }],
  ['unknown kind', { ...failure, kind: 'progress' }],
  ['missing sequence', without(failure, 'seq')],
  ['negative sequence', { ...failure, seq: -1 }],
  ['fractional sequence', { ...failure, seq: 1.5 }],
  ['bad timestamp', { ...failure, at: 'yesterday' }],
  ['invalid calendar date', { ...failure, at: '2026-02-30T12:00:00.000Z' }],
  ['non-UTC timestamp', { ...failure, at: '2026-10-26T12:00:00.000+01:00' }],
  ['empty run id', { ...failure, run: '' }],
  ['unknown event field', { ...failure, surprise: true }],
  ['wrong candidate type', event('run-open', { candidate: { sha: 1, target: null }, session: null })],
  ['missing candidate', event('run-open', { session: null })],
  ['start without project', event('gate-start')],
  ['wrong kind payload', event('gate-start', { project: 'x', failure: diagnostic })],
  ['missing diagnostic', without(failure, 'failure')],
  ['missing nth on failure', { ...failure, failure: without(diagnostic, 'nth') }],
  ['bad fingerprint', { ...failure, failure: { ...diagnostic, fingerprint: 'not-a-fingerprint' } }],
  ['unknown class', { ...failure, failure: { ...diagnostic, class: 'code' } }],
  ['line as string', { ...failure, failure: { ...diagnostic, locus: { file: 'x.rs', line: '2' } } }],
  ['excerpt as array', { ...failure, failure: { ...diagnostic, excerpt: ['failure'] } }],
  ['oversized excerpt', { ...failure, failure: { ...diagnostic, excerpt: 'x'.repeat(2049) } }],
  ['step-failed with success exit', event('step-failed', { project: 'x', step: 'test', exit: 0 })],
  ['step-failed without step', event('step-failed', { project: 'x', exit: 1 })],
  ['verdict without exit', { ...verdict, verdict: { seconds: 1 } }],
  ['negative duration', { ...verdict, verdict: { exit: 1, seconds: -1 } }],
  ['diagnostic class as verdict class', { ...verdict, verdict: { ...verdict.verdict, failure_class: 'test' } }],
  ['negative overflow', { ...verdict, verdict: { ...verdict.verdict, overflow: -1 } }],
  ['pending status', { ...verdict, status: 'Queued' }],
  ['unanswered without envelope', { ...verdict, status: 'Unanswered' }],
  ['answered with unanswered envelope', { ...verdict, status: 'Answered', unanswered }],
  ['unanswered without status', { ...verdict, unanswered }],
  ...['why', 'comeBack', 'meanwhile', 'escalation'].map(key =>
    [`incomplete Unanswered ${key}`, { ...verdict, status: 'Unanswered', unanswered: without(unanswered, key) }]),
  ['invented refusal shape', { ...verdict, status: 'Unanswered', unanswered: { reason: 'stopped', retry: 'later', fallback: 'local' } }],
  ['invalid meanwhile', { ...verdict, status: 'Unanswered', unanswered: { ...unanswered, meanwhile: { local: 'nothing' } } }],
  ['bare method digest', { ...verdict, derivation: { ...derivation, method: 'a'.repeat(64) } }],
  ['invalid input CID', { ...verdict, derivation: { ...derivation, input: 'sha256:abcd' } }],
  ['incomplete derivation', { ...verdict, derivation: without(derivation, 'hit') }],
  ['non-boolean derivation hit', { ...verdict, derivation: { ...derivation, hit: 'yes' } }],
  ['unknown closure outcome', event('run-close', { outcome: 'success' })],
  ['boolean deferred count', event('run-close', { outcome: 'gates-passed', deferred: true })],
  ['negative deferred count', event('run-close', { outcome: 'gates-passed', deferred: -1 })],
  ['fractional deferred count', event('run-close', { outcome: 'gates-passed', deferred: 1.5 })],
  ['legacy attribution label', { ...failure, attribution: { label: 'inherited', sessions: ['other'] } }],
  ['non-string session', { ...failure, attribution: { label: 'other', sessions: [123] } }],
  ['duplicate sessions', { ...failure, attribution: { label: 'ambiguous', sessions: ['self', 'self'] } }],
  ['ambiguous with one session', { ...failure, attribution: { label: 'ambiguous', sessions: ['self'] } }],
  ['unattributed with a session', { ...failure, attribution: { label: 'unattributed', sessions: ['self'] } }],
  ['mine without a session', { ...failure, attribution: { label: 'mine', sessions: [] } }],
];
for (const [name, value] of negatives) test(`rejects ${name}`, () => assert.equal(validate(value), false));
