import { strict as assert } from 'node:assert';
import { mkdtemp, readFile, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, it } from 'node:test';
import { pathToFileURL } from 'node:url';

import { parseArgs, runLook } from '../look.js';

describe('parseArgs', () => {
  it('parses a bare url', () => {
    const o = parseArgs(['https://example.test/path']);
    assert.equal(o.url, 'https://example.test/path');
    assert.equal(o.as, undefined);
  });

  it('parses all flags', () => {
    const o = parseArgs([
      'https://example.test',
      '--as',
      'Matthew',
      '--doorway',
      'https://doorway.test',
      '--wait-testid',
      'app-root',
      '--out',
      'my-slug',
      '--viewport',
      '800x600',
    ]);
    assert.equal(o.as, 'Matthew');
    assert.equal(o.doorway, 'https://doorway.test');
    assert.equal(o.waitTestid, 'app-root');
    assert.equal(o.out, 'my-slug');
    assert.deepEqual(o.viewport, { width: 800, height: 600 });
  });

  it('throws when url is missing', () => {
    assert.throws(() => parseArgs([]), /Usage: look/);
    assert.throws(() => parseArgs(['--as', 'Matthew']), /Usage: look/);
  });

  it('throws on a bad --viewport', () => {
    assert.throws(() => parseArgs(['https://x.test', '--viewport', 'huge']), /--viewport expects WxH/);
  });

  it('throws on an unknown flag', () => {
    assert.throws(() => parseArgs(['https://x.test', '--nope', 'v']), /Unknown flag: --nope/);
  });
});

describe('runLook (file:// hermetic render)', () => {
  it('renders a local file to shot.png + capture.json', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'look-test-'));
    const html = join(dir, 'page.html');
    await writeFile(
      html,
      '<!doctype html><title>Look Smoke</title><h1 data-testid="probe">rendered</h1>',
    );

    const result = await runLook({ url: pathToFileURL(html).href, out: 'unit-smoke' });

    assert.equal(result.ok, true);
    assert.equal(result.title, 'Look Smoke');
    assert.equal(result.as, null);
    assert.deepEqual(result.pageErrors, []);
    // Files exist and are non-empty.
    assert.ok((await stat(result.shotPath)).size > 0, 'shot.png written');
    const capture = JSON.parse(await readFile(result.capturePath, 'utf8'));
    assert.equal(capture.ok, true);
    assert.equal(capture.title, 'Look Smoke');
  });
});
