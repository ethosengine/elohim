// device-notes-publish: two devices publish divergent build notes to one forge without either push
// being rejected, a mirror only fast-forwards, and the legacy shared ref is never written.
// Every repository here is a scratch repository under the OS temp dir; git runs with no global or
// system config and no inherited GIT_* variables (a pre-push hook exports them).

import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { after, test } from 'node:test';
import assert from 'node:assert/strict';

import { destinationOf, devicePrefix, noteRetentionDays, patternRefspec, plan, publish, retentionLine } from './device-notes-publish.mjs';

const SHEM = 'did:key:z6Mkm5deEA87WrC3PDDGjkZwnWhUGhwHS43UinXhy5N6bowj';
const OTHER = 'did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK';
const STEP_REF = 'refs/notes/brit/build/gate%3Aelohim-storage';

const scratch = mkdtempSync(join(tmpdir(), 'device-notes-'));
after(() => rmSync(scratch, { recursive: true, force: true }));

const gitConfig = join(scratch, 'gitconfig');
writeFileSync(gitConfig, '[init]\n\tdefaultBranch = dev\n[protocol "file"]\n\tallow = always\n');
const ENV = Object.fromEntries(Object.entries(process.env).filter(([k]) => !k.startsWith('GIT_')));
Object.assign(ENV, {
  GIT_CONFIG_GLOBAL: gitConfig, GIT_CONFIG_NOSYSTEM: '1',
  GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@t', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@t',
  EPR_BIN: join(scratch, 'no-epr'), DEVICE_NOTES_DID: '', DEVICE_NOTES_PUSH: '',
});

function git(cwd, ...args) {
  const r = spawnSync('git', args, { cwd, env: ENV, encoding: 'utf8' });
  if (r.status !== 0) throw new Error(`git ${args.join(' ')} (exit ${r.status}): ${r.stderr}`);
  return r.stdout.trim();
}
const refAt = (repo, ref) => {
  const r = spawnSync('git', ['rev-parse', '--verify', '-q', ref], { cwd: repo, env: ENV, encoding: 'utf8' });
  return r.status === 0 ? r.stdout.trim() : null;
};
const note = (repo, body, ref = STEP_REF) => git(repo, 'notes', '--ref', ref, 'add', '-f', '-m', body, 'HEAD');

let n = 0;
/** A bare forge seeded with one commit and a legacy shared notes ref, plus clones as devices. */
function world(...devices) {
  const dir = join(scratch, `w${n++}`);
  const forge = join(dir, 'forge.git');
  const seed = join(dir, 'seed');
  git(scratch, 'init', '--bare', forge);
  git(scratch, 'init', seed);
  writeFileSync(join(seed, 'a.txt'), 'a\n');
  git(seed, 'add', 'a.txt');
  git(seed, 'commit', '-m', 'seed');
  note(seed, '{"legacy":true}');
  git(seed, 'push', forge, 'dev:refs/heads/dev', `${STEP_REF}:${STEP_REF}`);
  const legacy = refAt(forge, STEP_REF);
  const clones = devices.map(name => {
    const repo = join(dir, name);
    git(scratch, 'clone', '-q', forge, repo);
    return repo;
  });
  return { forge, legacy, clones };
}

function run(root, extra = {}) {
  const lines = [];
  const code = publish({ root, env: ENV, log: line => lines.push(line), ...extra });
  return { code, lines, text: lines.join('\n') };
}

test('the destination is the DID in brit ref segment encoding, under devices/, and a valid ref', () => {
  const prefix = devicePrefix(SHEM);
  assert.equal(prefix, 'refs/notes/brit/devices/did%3Akey%3Az6Mkm5deEA87WrC3PDDGjkZwnWhUGhwHS43UinXhy5N6bowj/build/');
  assert.equal(destinationOf(STEP_REF, SHEM), `${prefix}gate%3Aelohim-storage`);
  assert.equal(patternRefspec(SHEM), `refs/notes/brit/build/*:${prefix}*`);
  const r = spawnSync('git', ['check-ref-format', `${prefix}gate%3Aelohim-storage`], { env: ENV });
  assert.equal(r.status, 0);
  assert.equal(decodeURIComponent(prefix.split('/')[4]), SHEM, 'the ref name gives back the exact DID');
  for (const bad of ['', 'z6Mk', 'did:key:', 'did:web:a/b', 'did:key:a b', 'did:KEY:x']) {
    assert.throws(() => devicePrefix(bad), /not a device DID/, bad);
  }
  assert.throws(() => destinationOf('refs/notes/brit/validate/x', SHEM), /not a build-notes ref/);
});

test('two devices publish divergent notes to one forge; neither is rejected; the legacy ref is untouched', () => {
  const { forge, legacy, clones: [a, b] } = world('shem', 'other');
  git(a, 'fetch', 'origin', `${STEP_REF}:${STEP_REF}`);
  git(b, 'fetch', 'origin', `${STEP_REF}:${STEP_REF}`);
  note(a, '{"device":"shem"}');
  note(b, '{"device":"other"}');
  note(a, '{"device":"shem","step":"eprfs"}', 'refs/notes/brit/build/gate%3Aeprfs');
  assert.notEqual(refAt(a, STEP_REF), refAt(b, STEP_REF), 'the two devices hold diverged notes');

  const ra = run(a, { did: SHEM });
  assert.equal(ra.code, 0, ra.text);
  assert.match(ra.text, /device-notes: new refs\/notes\/brit\/devices\/did%3Akey%3Az6Mkm5.*\/build\/gate%3Aelohim-storage/);
  const rb = run(b, { did: OTHER });
  assert.equal(rb.code, 0, rb.text);

  assert.equal(refAt(forge, destinationOf(STEP_REF, SHEM)), refAt(a, STEP_REF));
  assert.equal(refAt(forge, destinationOf('refs/notes/brit/build/gate%3Aeprfs', SHEM)), refAt(a, 'refs/notes/brit/build/gate%3Aeprfs'));
  assert.equal(refAt(forge, destinationOf(STEP_REF, OTHER)), refAt(b, STEP_REF));
  assert.equal(refAt(forge, STEP_REF), legacy, 'the legacy shared ref stays frozen');
  assert.equal(refAt(forge, 'refs/notes/brit/build/gate%3Aeprfs'), null, 'nothing new under the legacy prefix');
  // The control: the pre-R4 shared push is what diverged (row 3). Once one device has moved the
  // shared ref, the other device's push of the same ref is rejected as a non-fast-forward.
  git(a, 'push', 'origin', `${STEP_REF}:${STEP_REF}`);
  const shared = spawnSync('git', ['push', 'origin', `${STEP_REF}:${STEP_REF}`], { cwd: b, env: ENV, encoding: 'utf8' });
  assert.notEqual(shared.status, 0, 'the second device is rejected on the shared ref');
  assert.match(shared.stderr, /rejected|fetch first|non-fast-forward/);
  git(forge, 'update-ref', STEP_REF, legacy); // back to the frozen value for the checks below

  // A second publish with no change is up to date; a new note fast-forwards the device's mirror.
  const again = run(a, { did: SHEM });
  assert.equal(again.code, 0, again.text);
  assert.match(again.text, /up to date, 2 ref\(s\)/);
  const before = refAt(forge, destinationOf(STEP_REF, SHEM));
  git(a, 'commit', '--allow-empty', '-m', 'next');
  note(a, '{"device":"shem","n":2}');
  const ff = run(a, { did: SHEM });
  assert.equal(ff.code, 0, ff.text);
  assert.match(ff.text, new RegExp(`fast-forward \\S+gate%3Aelohim-storage ${before.slice(0, 12)}\\.\\.`));
  assert.equal(refAt(forge, destinationOf(STEP_REF, SHEM)), refAt(a, STEP_REF));
  assert.equal(refAt(forge, destinationOf(STEP_REF, OTHER)), refAt(b, STEP_REF), "the other device's mirror is not moved");
  assert.equal(refAt(forge, STEP_REF), legacy);
});

test('a rewound local ref is refused with a named cure, and the mirror does not move', () => {
  const { forge, legacy, clones: [a] } = world('shem');
  note(a, '{"n":1}');
  const first = refAt(a, STEP_REF);
  assert.equal(run(a, { did: SHEM }).code, 0);
  git(a, 'commit', '--allow-empty', '-m', 'next');
  note(a, '{"n":2}');
  assert.equal(run(a, { did: SHEM }).code, 0);
  const published = refAt(forge, destinationOf(STEP_REF, SHEM));

  git(a, 'update-ref', STEP_REF, first);
  const r = run(a, { did: SHEM });
  assert.equal(r.code, 2, r.text);
  assert.match(r.text, /REFUSED rewound \S+gate%3Aelohim-storage: forge \w{12}, local \w{12} — cure: .*git fetch origin \S+:refs\/notes\/brit\/build\/gate%3Aelohim-storage/);
  assert.match(r.text, /nothing pushed/);
  assert.equal(refAt(forge, destinationOf(STEP_REF, SHEM)), published);
  assert.equal(refAt(forge, STEP_REF), legacy);

  // The cure it names restores the local ref, and the next publish is up to date.
  git(a, 'fetch', 'origin', `${destinationOf(STEP_REF, SHEM)}:${STEP_REF}`);
  const cured = run(a, { did: SHEM });
  assert.equal(cured.code, 0, cured.text);
  assert.match(cured.text, /up to date/);
});

test('a diverged local ref is refused, and one refusal holds back every other ref', () => {
  const { forge, clones: [a] } = world('shem');
  note(a, '{"n":1}');
  const base = refAt(a, STEP_REF);
  note(a, '{"n":2}');
  assert.equal(run(a, { did: SHEM }).code, 0);
  const published = refAt(forge, destinationOf(STEP_REF, SHEM));
  git(a, 'update-ref', STEP_REF, base);
  note(a, '{"n":"2-other"}');
  note(a, '{"other":1}', 'refs/notes/brit/build/gate%3Aeprfs');

  const r = run(a, { did: SHEM });
  assert.equal(r.code, 2, r.text);
  assert.match(r.text, /REFUSED diverged \S+gate%3Aelohim-storage: .*never merged or forced/);
  assert.equal(refAt(forge, destinationOf(STEP_REF, SHEM)), published);
  assert.equal(refAt(forge, destinationOf('refs/notes/brit/build/gate%3Aeprfs', SHEM)), null, 'the publish is all or nothing');
});

test("a forge tip this checkout has never seen is refused until it is fetched", () => {
  const { clones: [a, fresh] } = world('shem', 'shem-fresh');
  note(a, '{"from":"a"}');
  assert.equal(run(a, { did: SHEM }).code, 0);
  note(fresh, '{"from":"fresh"}');
  const r = run(fresh, { did: SHEM });
  assert.equal(r.code, 2, r.text);
  assert.match(r.text, /REFUSED unknown-forge-tip .*git fetch origin \S+:refs\/brit-mirror\//);
});

test('a dry run prints the refspecs and contacts no forge', () => {
  const { forge, clones: [a] } = world('shem');
  note(a, '{"n":1}');
  git(a, 'remote', 'set-url', 'origin', join(scratch, 'does-not-exist.git'));
  const r = run(a, { did: SHEM, dryRun: true });
  assert.equal(r.code, 0, r.text);
  assert.ok(r.lines.includes(`device-notes: refspec ${STEP_REF}:${destinationOf(STEP_REF, SHEM)}`), r.text);
  assert.match(r.text, /the forge was not read and nothing was pushed/);
  assert.equal(refAt(forge, destinationOf(STEP_REF, SHEM)), null);
});

test('no device identity refuses with the cure; a DID can come from DEVICE_NOTES_DID', () => {
  const { clones: [a] } = world('shem');
  note(a, '{"n":1}');
  const r = run(a);
  assert.equal(r.code, 1, r.text);
  assert.match(r.text, /REFUSED no device identity .*just status device/);
  const viaEnv = publish({ root: a, env: { ...ENV, DEVICE_NOTES_DID: SHEM }, dryRun: true, log: () => {} });
  assert.equal(viaEnv, 0);
});

test('plan never names a destination outside devices/', () => {
  const { actions } = plan({
    locals: [{ ref: STEP_REF, sha: 'a'.repeat(40) }], held: new Map(), did: SHEM, remote: 'origin',
    has: () => true, isAncestor: () => true,
  });
  assert.deepEqual(actions.map(a => a.dest), [destinationOf(STEP_REF, SHEM)]);
  assert.ok(actions.every(a => a.dest.startsWith('refs/notes/brit/devices/')));
});

const brit = spawnSync('brit', ['--version'], { encoding: 'utf8' });
test('DEVICE_NOTES_PUSH=brit publishes through brit push', { skip: brit.status !== 0 && 'brit not on PATH' }, () => {
  const { forge, legacy, clones: [a] } = world('shem');
  note(a, '{"n":1}');
  const r = run(a, { did: SHEM, env: { ...ENV, DEVICE_NOTES_PUSH: 'brit' } });
  assert.equal(r.code, 0, r.text);
  assert.equal(refAt(forge, destinationOf(STEP_REF, SHEM)), refAt(a, STEP_REF));
  assert.equal(refAt(forge, STEP_REF), legacy);
});

test('the note retention is a declared positive integer, printed as declared and not enforced', () => {
  const repo = join(import.meta.dirname, '..', '..');
  const days = noteRetentionDays(repo);
  assert.ok(Number.isInteger(days) && days > 0, `pool-policy ci_note_retention_days is ${days}`);
  assert.equal(retentionLine(days),
    `device-notes: retention ${days} days declared (pool-policy ci_note_retention_days), not yet enforced: no note expires`);
  assert.equal(noteRetentionDays(repo, { read: () => '{"ci_note_retention_days": 0}' }), null);
  assert.equal(noteRetentionDays(repo, { read: () => 'not json' }), null);
  assert.match(retentionLine(null), /retention undeclared/);
});
