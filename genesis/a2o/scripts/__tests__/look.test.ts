/* eslint-disable @typescript-eslint/no-floating-promises -- node:test describe/it
   return promises that the test runner itself consumes; awaiting them is wrong. */
import { strict as assert } from 'node:assert';
import { mkdtemp, readFile, stat, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, it } from 'node:test';
import { pathToFileURL } from 'node:url';

import { parseArgs, runLook, type LookResult } from '../look.js';

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
      '--timeout',
      '45000',
    ]);
    assert.equal(o.as, 'Matthew');
    assert.equal(o.doorway, 'https://doorway.test');
    assert.equal(o.waitTestid, 'app-root');
    assert.equal(o.out, 'my-slug');
    assert.deepEqual(o.viewport, { width: 800, height: 600 });
    assert.equal(o.timeoutMs, 45000);
  });

  it('throws when url is missing', () => {
    assert.throws(() => parseArgs([]), /Usage: look/);
    assert.throws(() => parseArgs(['--as', 'Matthew']), /Usage: look/);
  });

  it('throws on a bad --viewport', () => {
    assert.throws(
      () => parseArgs(['https://x.test', '--viewport', 'huge']),
      /--viewport expects WxH/
    );
  });

  it('throws on an unknown flag', () => {
    assert.throws(() => parseArgs(['https://x.test', '--nope', 'v']), /Unknown flag: --nope/);
  });

  it('parses --timeout as timeoutMs', () => {
    const o = parseArgs(['https://example.test', '--timeout', '90000']);
    assert.equal(o.timeoutMs, 90000);
  });

  it('throws on --timeout below 1000', () => {
    assert.throws(
      () => parseArgs(['https://x.test', '--timeout', '500']),
      /--timeout expects milliseconds >= 1000/
    );
  });

  it('throws on --timeout non-numeric', () => {
    assert.throws(
      () => parseArgs(['https://x.test', '--timeout', 'forever']),
      /--timeout expects milliseconds >= 1000/
    );
  });
});

describe('runLook (file:// hermetic render)', () => {
  it('renders a local file to shot.png + capture.json', async () => {
    const dir = await mkdtemp(join(tmpdir(), 'look-test-'));
    const html = join(dir, 'page.html');
    await writeFile(
      html,
      '<!doctype html><title>Look Smoke</title><h1 data-testid="probe">rendered</h1>'
    );

    const result = await runLook({ url: pathToFileURL(html).href, out: 'unit-smoke' });

    assert.equal(result.ok, true);
    assert.equal(result.title, 'Look Smoke');
    assert.equal(result.as, null);
    assert.deepEqual(result.pageErrors, []);
    // No HTTP traffic at all → the additive httpErrors field is present and empty.
    // (A file:// miss surfaces as requestfailed, never as an HTTP 4xx response.)
    assert.deepEqual(result.httpErrors, []);
    // Files exist and are non-empty.
    assert.ok((await stat(result.shotPath)).size > 0, 'shot.png written');
    const capture = JSON.parse(await readFile(result.capturePath, 'utf8')) as LookResult;
    assert.equal(capture.ok, true);
    assert.equal(capture.title, 'Look Smoke');
    assert.deepEqual(capture.httpErrors, []);
  });
});

describe('runLook (localhost hermetic 404 capture)', () => {
  it('records {url, status} for HTTP responses >= 400 in httpErrors', async () => {
    // Hermetic localhost server: the page itself is a 200, its subresource 404s.
    const server = createServer((req, res) => {
      if (req.url === '/page.html') {
        res.writeHead(200, { 'content-type': 'text/html' });
        res.end('<!doctype html><title>404 Probe</title><img src="/missing.png">');
      } else {
        res.writeHead(404);
        res.end();
      }
    });
    await new Promise<void>(resolve => server.listen(0, '127.0.0.1', resolve));
    const { port } = server.address() as { port: number };

    try {
      const result = await runLook({
        url: `http://127.0.0.1:${port}/page.html`,
        out: 'unit-404',
      });

      assert.deepEqual(result.httpErrors, [
        { url: `http://127.0.0.1:${port}/missing.png`, status: 404 },
      ]);
      // Additive contract: a 4xx subresource is recorded, not promoted to failure.
      assert.equal(result.ok, true);
      const capture = JSON.parse(await readFile(result.capturePath, 'utf8')) as LookResult;
      assert.deepEqual(capture.httpErrors, result.httpErrors);
    } finally {
      await new Promise<void>((resolve, reject) =>
        server.close(err => (err ? reject(err) : resolve()))
      );
    }
  });
});
