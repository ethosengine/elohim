import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { cpSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

import { Given, Then, When } from '@cucumber/cucumber';

interface Capture {
  snapshot: string;
  projection: {
    version: number;
    declaration_cid: string;
    subjects: { fields: Record<string, unknown>; readings: Record<string, unknown> }[];
    sources: Record<string, string>;
    composition?: { links: { state: string }[]; groups: { id: string }[] };
  };
}
const repo = resolve('../..');
const binary = process.env.EPR_BIN ?? 'epr';
const compositionPath = 'composition.json';
let orchard: string;
let store: string;
let captured: Capture[];
let baseline: Capture;
let revised: Capture;
let receipts: Record<string, unknown>[];
function run(args: string[]): Capture {
  return JSON.parse(
    execFileSync(binary, ['view', ...args, '--json'], {
      encoding: 'utf8',
      timeout: 240000,
      maxBuffer: 128 * 1024 * 1024,
    })
  ) as Capture;
}
function prepare(): void {
  const root = mkdtempSync(join(tmpdir(), 'work-composition-'));
  orchard = join(root, 'orchard');
  store = join(root, 'store');
  cpSync(join(repo, 'elohim/eprfs/epr-cli/tests/fixtures/views/orchard'), orchard, {
    recursive: true,
  });
}
Given('coding and orchard work compositions with related subject kinds', prepare);
When('the same native reader captures both compositions', { timeout: 600000 }, () => {
  captured = [
    run([compositionPath, '--root', orchard]),
    run(['elohim/rakia/work-composition/.epr-meta/progress.json', '--root', repo]),
  ];
});
Then('each composition retains its declarations, exact sources and explicit relationships', () => {
  for (const c of captured) {
    assert.equal(c.projection.version, 2);
    assert.ok(c.projection.composition?.links.some(l => l.state === 'resolved'));
    assert.ok(Object.keys(c.projection.sources).length > 2);
  }
  assert.ok(captured[1].projection.composition?.groups.some(g => g.id === 'habits'));
});
Then('authored claims remain claims even when their text says completed', () => {
  assert.ok(
    captured.every(c =>
      c.projection.subjects.some(
        s => Array.isArray(s.fields['$claims']) && s.fields['$claims'].length > 0
      )
    )
  );
});
Then('unavailable evidence never becomes acceptance', () => {
  for (const c of captured)
    for (const s of c.projection.subjects)
      for (const reading of Object.values(s.readings)) {
        const r = reading as { state: string; value?: unknown };
        if (r.state === 'unavailable') assert.equal(r.value, undefined);
      }
});
Given('a retained composition observation and a version one declared view', () => {
  prepare();
  baseline = run([compositionPath, '--root', orchard, '--save', store]);
});
When('a domain declaration changes and another observation is captured', () => {
  const p = join(orchard, compositionPath);
  const value = JSON.parse(readFileSync(p, 'utf8')) as { purpose: string };
  value.purpose = 'Changed orchard interpretation';
  writeFileSync(p, JSON.stringify(value));
  revised = run([compositionPath, '--root', orchard]);
});
Then('the earlier observation reopens with its original interpretation', () => {
  assert.notEqual(baseline.projection.declaration_cid, revised.projection.declaration_cid);
  assert.deepEqual(
    run(['--snapshot', baseline.snapshot, '--store', store]).projection,
    baseline.projection
  );
});
Then('version one views still reopen without migration', () => {
  const old = run(['view.json', '--root', orchard, '--save', store]);
  assert.deepEqual(run(['--snapshot', old.snapshot, '--store', store]).projection, old.projection);
});
Then('an executable declaration or a reference outside the repository is refused', () => {
  const p = join(orchard, compositionPath);
  const original = JSON.parse(readFileSync(p, 'utf8')) as Record<string, unknown>;
  for (const value of [
    { ...original, execute: 'run' },
    { ...original, appManifest: '../outside' },
  ]) {
    writeFileSync(p, JSON.stringify(value));
    assert.notEqual(
      spawnSync(binary, ['view', compositionPath, '--root', orchard], { encoding: 'utf8' }).status,
      0
    );
  }
});
Given('a pinned Avodah build reading a work composition', () => {
  readFileSync(join(repo, 'app/elohim-app/dist/elohim-app/browser/index.csr.html'));
});
When('I compare observations and inspect a linked work subject', { timeout: 1200000 }, async () => {
  const { runLocalViewBrowser } = await import('../../scripts/local-view-browser.mjs');
  receipts = [];
  for (const kind of ['orchard-composition', 'rakia']) {
    const path = await runLocalViewBrowser(kind);
    receipts.push(
      JSON.parse(readFileSync(join(path, 'receipt.json'), 'utf8')) as Record<string, unknown>
    );
  }
});
Then('I can inspect baseline, authored claims and evidence availability', () => {
  for (const r of receipts) assert.ok(Number(r.progressSubjects) > 0);
});
Then('the board and burndown use one selected subject kind', () => {
  for (const r of receipts) assert.ok(Number(r.subjects) > 0);
});
Then(
  'opening and refresh each capture of a repository-sized composition successfully within 60 seconds, the workspace proxy deadline',
  () => {
    for (const r of receipts) {
      const responses = r.captureResponses as { status: number; milliseconds: number }[];
      assert.equal(responses.length, 2);
      for (const response of responses) {
        assert.equal(response.status, 200);
        assert.ok(response.milliseconds < 60000);
      }
    }
  }
);
Then('offline retention includes the complete composition and sources', () => {
  for (const r of receipts)
    for (const key of [
      'compositionRetained',
      'coldRestart',
      'networkDisabled',
      'exactSourcePreserved',
      'sourceFilesUnchanged',
      'workflowActsUnchanged',
    ])
      assert.equal(r[key], true, key);
});
