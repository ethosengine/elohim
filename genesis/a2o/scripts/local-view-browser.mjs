/** Actual production build and native node; no mocked projection payloads. */
import assert from 'node:assert/strict';
import { mkdir, mkdtemp, cp, readFile, writeFile } from 'node:fs/promises';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { createHash } from 'node:crypto';
import { chromium } from 'playwright';

const repo = resolve(fileURLToPath(new URL('../../..', import.meta.url)));
const reportRoot = join(repo, 'genesis/a2o/reports/recovery/local-views');
export async function runLocalViewBrowser(kind = 'orchard') {
  // The native reader and launcher are a separate port. Never build or select a stale pool binary.
  const binary = process.env.ELOHIM_NODE_BIN;
  if (!binary) throw new Error('Set ELOHIM_NODE_BIN to a built node with --local-view support');
  const { accessServer, pinBuild } = await import('../../../app/scripts/local-view-launcher.mjs');
  const started = Date.now();
  const stage = name =>
    console.log(`${kind}: ${name} (${Math.round((Date.now() - started) / 1000)}s)`);
  await mkdir(reportRoot, { recursive: true });
  const report = await mkdtemp(join(reportRoot, `${kind}-`));
  const orchard = kind === 'orchard' || kind === 'orchard-composition';
  const composed = kind === 'rakia' || kind === 'orchard-composition';
  const root = orchard
    ? join(report, 'orchard')
    : resolve(process.env.LOCAL_VIEW_TEST_SOURCE_ROOT ?? repo);
  if (orchard)
    await cp(join(repo, 'elohim/eprfs/epr-cli/tests/fixtures/views/orchard'), root, {
      recursive: true,
    });
  const declaration =
    kind === 'orchard-composition'
      ? 'composition.json'
      : kind === 'rakia'
        ? 'elohim/rakia/work-composition/.epr-meta/progress.json'
        : orchard
          ? 'board.view.json'
          : 'elohim/elohim-storage/.epr-meta/idle-is-free.view.json';
  const before = await readFile(join(root, declaration));
  const workflowPath = join(root, '.eprfs/status/flows.jsonl');
  const workflowBefore = await readFile(workflowPath).catch(error => {
    if (error.code === 'ENOENT') return null;
    throw error;
  });
  const sourcePath = orchard ? 'plots/north.md' : undefined;
  const sourceBefore = sourcePath ? await readFile(join(root, sourcePath), 'utf8') : '';
  const build = join(report, 'build');
  const manifest = await pinBuild(
    process.env.LOCAL_VIEW_TEST_BUILD ?? join(repo, 'app/elohim-app/dist/elohim-app/browser'),
    build
  );
  const token = 'local-view-browser-test-token-with-thirty-two-bytes';
  const config = join(report, 'reader.json');
  await writeFile(
    config,
    JSON.stringify({
      root,
      store: join(report, 'store'),
      views: { [kind]: declaration },
      listen: '127.0.0.1:14301',
    })
  );
  const child = spawn(binary, ['--local-view', config], {
    env: { ...process.env, LOCAL_VIEW_READER_TOKEN: token },
    stdio: ['ignore', 'ignore', 'pipe'],
  });
  let nativeErrors = '';
  child.stderr.on('data', b => {
    nativeErrors += b;
  });
  const relay = process.env.LOCAL_VIEW_RELAY_BIN
    ? spawn(process.env.LOCAL_VIEW_RELAY_BIN, ['--local-view-relay', '127.0.0.1:14302'], {
        env: { ...process.env, LOCAL_VIEW_READER_URL: 'http://127.0.0.1:14301' },
        stdio: 'ignore',
      })
    : undefined;
  const upstream = relay ? 'http://127.0.0.1:14302' : 'http://127.0.0.1:14301';
  const origin = 'http://localhost:14300';
  const server = accessServer({
    origin,
    upstream,
    token,
    capability: 'test-launch-capability',
    viewId: kind,
    build,
  });
  await new Promise(resolve => server.listen(14300, resolve));
  let context;
  const errors = [];
  let captures = 0;
  const captureStarts = new WeakMap();
  const captureResponses = [];
  const captureBudgetMs = 60000;
  const profile = join(report, 'browser');
  const launch = () =>
    chromium.launchPersistentContext(profile, {
      headless: true,
      args: [
        '--no-sandbox',
        '--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE localhost, EXCLUDE 127.0.0.1',
      ],
      viewport: { width: 1440, height: 1000 },
    });
  try {
    for (let i = 0; i < 100; i++) {
      if (child.exitCode !== null) throw Error(nativeErrors);
      try {
        const r = await fetch('http://127.0.0.1:14301/api/local-views', {
          headers: { Authorization: `Bearer ${token}` },
        });
        if (r.ok) break;
      } catch {}
      await new Promise(r => setTimeout(r, 100));
    }
    context = await launch();
    let page = await context.newPage();
    page.on('pageerror', error => errors.push(error.message));

    page.on('request', req => {
      if (req.url().endsWith('/capture')) {
        captures++;
        captureStarts.set(req, Date.now());
      }
    });
    page.on('response', response => {
      const start = captureStarts.get(response.request());
      if (start !== undefined)
        captureResponses.push({ status: response.status(), milliseconds: Date.now() - start });
    });
    await page.goto(`${origin}/avodah/views/${kind}#capability=test-launch-capability`);
    stage('opened');
    let progressCount = 0;
    if (composed) {
      await page.getByTestId('work-progress').waitFor({ timeout: 240000 });
      progressCount = await page.getByTestId('progress-card').count();
      assert.ok(progressCount > 0);
      assert.match(await page.getByTestId('work-progress').innerText(), /Baseline established/);
      assert.match(
        await page.getByTestId('work-progress').innerText(),
        /Structured evidence unavailable/
      );
      const claimSummary = page
        .locator('[data-testid=progress-card] summary')
        .filter({ hasText: 'authored claims' })
        .first();
      await claimSummary.click();
      const claim = page.locator('.claim').filter({ visible: true }).first();
      assert.ok((await claim.innerText()).length > 30, 'complete authored claim is visible');
      await claim.getByRole('button').click();
      assert.ok((await page.getByTestId('source-content').innerText()).length > 30);
      await page.getByTestId('tab-Progress').click();
      await page.locator('.linked-work button').first().click();
      await page.getByTestId('subject-detail').waitFor();
      assert.match(await page.getByTestId('subject-detail').innerText(), /Native evidence/);
      await page.getByRole('button', { name: 'Close subject details' }).click();
      await page.getByTestId('progress-search').fill('not-a-real-subject');
      await page.waitForFunction(() =>
        document.querySelector('[data-testid=progress-count]')?.textContent.includes('0 visible')
      );
      await page.getByTestId('progress-search').fill('');
      await page.evaluate(() => window.scrollTo(0, 0));
      for (const theme of ['light', 'dark']) {
        await page.evaluate(t => document.body.setAttribute('data-theme', t), theme);
        await page.screenshot({ path: join(report, `progress-${theme}.png`) });
      }
      await page.getByTestId('tab-Board').click();
    }
    await page.getByTestId('visible-count').waitFor({ timeout: 240000 });
    stage('reading rendered');
    assert.equal(new URL(page.url()).hash, '');
    assert.equal(captures, 1);
    const nativeHeaders = { Authorization: `Bearer ${token}` };
    const history = await (
      await fetch(`http://127.0.0.1:14301/api/local-views/${kind}/history`, {
        headers: nativeHeaders,
      })
    ).json();
    const bundle = await (
      await fetch(
        `http://127.0.0.1:14301/api/local-views/${kind}/snapshots/${history[0].snapshotCid}`,
        { headers: nativeHeaders }
      )
    ).json();
    if (relay) {
      for (const [path, expected] of [
        [`/${kind}/history`, history],
        [`/${kind}/snapshots/${history[0].snapshotCid}`, bundle],
      ]) {
        const relayed = await fetch(`${upstream}/api/local-views${path}`, {
          headers: nativeHeaders,
        });
        assert.deepEqual(await relayed.json(), expected, 'doorway and direct projection equality');
        assert.match(relayed.headers.get('cache-control'), /no-store/);
      }
      assert.equal((await fetch(`${upstream}/api/local-views/${kind}/history`)).status, 404);
      assert.equal(
        (
          await fetch(`${upstream}/api/local-views/${kind}/history?root=/etc`, {
            headers: nativeHeaders,
          })
        ).status,
        404
      );
    }
    const projection = JSON.parse(bundle.envelope).projection;
    if (kind === 'rakia' && process.env.LOCAL_VIEW_TEST_SOURCE_SNAPSHOT)
      assert.equal(
        history[0].snapshotCid,
        process.env.LOCAL_VIEW_TEST_SOURCE_SNAPSHOT,
        'isolated source bytes reproduce the real repository snapshot'
      );
    const sourceBeforeHashes = new Map();
    for (const path of Object.keys(projection.sources))
      sourceBeforeHashes.set(path, await readFile(join(root, path)));
    const count = await page.locator('[data-subject]').count();
    assert.ok(count > 0, 'real declared subjects render');
    const declarationValue = JSON.parse(
      Buffer.from(bundle.objects[projection.declaration_cid], 'base64').toString()
    );
    const declaredColumns = composed
      ? projection.composition.groups.find(g => g.role === 'work').presentation.columns
      : declarationValue.presentation.columns;
    assert.deepEqual(
      (
        await page
          .locator('[data-status]')
          .evaluateAll(nodes => nodes.map(node => node.getAttribute('aria-label')))
      ).slice(0, declaredColumns.length),
      declaredColumns.map(column => column.label)
    );
    if (kind === 'orchard') {
      assert.equal(await page.locator('[data-subject]').getAttribute('data-subject'), 'north-bed');
      await page.getByTestId('status-filter').selectOption('harvested');
      assert.match(await page.getByTestId('visible-count').innerText(), /^0 visible/);
      await page.getByTestId('status-filter').selectOption('');
    }
    await page.getByLabel('Density').selectOption({ label: 'Compact' });
    assert.ok((await page.getByTestId('declared-view').getAttribute('class')).includes('dense'));
    await page.getByLabel('Density').selectOption({ label: 'Comfortable' });
    await page.getByTestId('view-search').fill('not-a-real-subject');
    assert.match(await page.getByTestId('visible-count').innerText(), /^0 visible/);
    await page.getByTestId('tab-Burndown').click();
    assert.match(
      await page.getByTestId('burndown').innerText(),
      new RegExp(`${count} baseline subjects`)
    );
    await page.getByTestId('tab-Board').click();
    await page.getByTestId('view-search').fill('');
    const card = page.locator('[data-subject]').first();
    await card.focus();
    await page.keyboard.press('Enter');
    await page.getByTestId('subject-detail').waitFor();
    await page.getByTestId('exact-source').click();
    const capturedSource = await page.getByTestId('source-content').innerText();
    assert.ok(capturedSource.startsWith('---'));
    if (sourcePath) assert.equal(capturedSource, sourceBefore);
    assert.equal(captures, 1, 'navigation does not capture');
    await page.getByTestId('refresh').click();
    await page.waitForFunction(
      () => document.querySelector('[data-testid=observation]')?.children.length === 2,
      undefined,
      { timeout: 180000 }
    );
    assert.equal(captures, 2);
    assert.equal(captureResponses.length, 2);
    for (const response of captureResponses) {
      assert.equal(response.status, 200, 'each capture succeeds before rendering retained history');
      assert.ok(
        response.milliseconds < captureBudgetMs,
        'capture exceeds the 60 second proxy budget'
      );
    }
    stage('refreshed');
    if (composed) {
      await page.getByTestId('tab-Progress').click();
      await page.getByTestId('comparison').selectOption({ index: 1 });
      assert.match(await page.getByTestId('work-progress').innerText(), /Unchanged reading/);
      await page.getByTestId('tab-Board').click();
      const collection = page.getByTestId('work-collection');
      await collection.selectOption({ index: 0 });
      assert.equal(await page.locator('[data-subject]').count(), progressCount);
      await page.getByTestId('tab-Burndown').click();
      assert.match(
        await page.getByTestId('burndown').innerText(),
        new RegExp(`${count} baseline subjects`)
      );
    }
    await page.getByTestId('observation').selectOption({ index: 0 });
    assert.equal(captures, 2, 'history navigation does not capture');
    await page.getByTestId('tab-Board').click();
    for (const theme of ['light', 'dark']) {
      await page.evaluate(theme => document.body.setAttribute('data-theme', theme), theme);
      await page.screenshot({ path: join(report, `${theme}.png`), fullPage: true });
    }
    const capturedObservationLabel = await page
      .getByTestId('observation')
      .locator('option')
      .first()
      .innerText();
    await page.getByTestId('keep-offline').click();
    await Promise.race([
      page.getByTestId('offline-ready').waitFor({ timeout: 60000 }),
      page
        .getByRole('alert')
        .waitFor({ timeout: 60000 })
        .then(async () => {
          throw Error(await page.getByRole('alert').innerText());
        }),
    ]);
    assert.match(await page.getByTestId('offline-ready').innerText(), /#2/);
    stage('retained offline');
    // Aborted replacement must leave the committed generation and its build intact.
    await page.evaluate(() => {
      const original = IDBObjectStore.prototype.put;
      window.restoreOfflinePut = () => {
        IDBObjectStore.prototype.put = original;
      };
      IDBObjectStore.prototype.put = function (...args) {
        const request = original.apply(this, args);
        if (this.name === 'copies') this.transaction.abort();
        return request;
      };
    });
    await page.getByTestId('keep-offline').click();
    await page.getByRole('alert').waitFor({ timeout: 180000 });
    assert.match(await page.getByTestId('offline-ready').innerText(), /#2/);
    assert.equal(captures, 2, 'retention does not capture');
    await page.evaluate(() => {
      window.restoreOfflinePut();
      const original = Cache.prototype.put;
      window.restoreCachePut = () => {
        Cache.prototype.put = original;
      };
      Cache.prototype.put = async function () {
        throw new DOMException('Storage exhausted', 'QuotaExceededError');
      };
    });
    await page.getByTestId('keep-offline').click();
    await page.waitForFunction(() =>
      document.querySelector('[role=alert]')?.textContent.includes('QuotaExceededError')
    );
    await page.evaluate(() => window.restoreCachePut());
    await page.evaluate(() => {
      const original = window.fetch;
      window.restoreAssetFetch = () => {
        window.fetch = original;
      };
      window.fetch = async (...args) => {
        if (args[1]?.headers?.['X-Local-View-Pin']) return new Response('corrupt build bytes');
        return original(...args);
      };
    });
    await page.getByTestId('keep-offline').click();
    await page.waitForFunction(() =>
      document.querySelector('[role=alert]')?.textContent.includes('verification failed')
    );
    await page.evaluate(() => window.restoreAssetFetch());
    assert.match(await page.getByTestId('offline-ready').innerText(), /#2/);
    // A failed new capture must not hide the last usable server history or touch retention.
    await page.evaluate(() => {
      const original = window.fetch;
      window.restoreProjectionFetch = () => {
        window.fetch = original;
      };
      window.fetch = async (...args) => {
        if (String(args[0]).endsWith('/capture')) throw new Error('Reader disconnected');
        return original(...args);
      };
    });
    await page.getByTestId('refresh').click();
    await page.waitForFunction(() =>
      document.querySelector('[role=alert]')?.textContent.includes('Reader disconnected')
    );
    await page.evaluate(() => window.restoreProjectionFetch());
    assert.equal(await page.locator('[data-subject]').count(), count);

    assert.deepEqual(
      await readFile(join(root, declaration)),
      before,
      'viewing never changes declaration'
    );
    if (sourcePath)
      assert.equal(
        await readFile(join(root, sourcePath), 'utf8'),
        sourceBefore,
        'viewing never changes source'
      );
    for (const [path, bytes] of sourceBeforeHashes)
      assert.equal(
        createHash('sha256')
          .update(await readFile(join(root, path)))
          .digest('hex'),
        createHash('sha256').update(bytes).digest('hex'),
        `source unchanged: ${path}`
      );
    assert.deepEqual(
      await readFile(workflowPath).catch(error => {
        if (error.code === 'ENOENT') return null;
        throw error;
      }),
      workflowBefore,
      'no workflow acts'
    );
    await context.close();
    stage('replacement failures preserved previous copy');
    context = undefined;
    await new Promise(resolve => server.close(resolve));
    const exited = once(child, 'exit');
    child.kill('SIGTERM');
    await exited;
    if (relay) {
      const stopped = once(relay, 'exit');
      relay.kill('SIGTERM');
      await stopped;
    }
    // New browser process, same profile, no serving processes and no networking.
    context = await launch();
    await context.setOffline(true);
    page = await context.newPage();
    await page.goto(`${origin}/avodah/views/${kind}`);
    if (composed) {
      await page.getByTestId('work-progress').waitFor({ timeout: 60000 });
      assert.equal(await page.getByTestId('progress-card').count(), progressCount);
      await page.getByTestId('tab-Board').click();
    }
    await page.getByTestId('visible-count').waitFor({ timeout: 60000 });
    assert.equal(await page.locator('[data-subject]').count(), count);
    assert.match(await page.getByTestId('view-state').innerText(), /Offline/);
    assert.equal(
      await page.getByTestId('observation').locator('option').first().innerText(),
      capturedObservationLabel
    );
    await page.getByTestId('tab-Burndown').click();
    await page.getByTestId('burndown').waitFor();
    assert.equal(await page.locator('tbody tr').count(), 2);
    await page.getByTestId('tab-Board').click();
    await page.locator('[data-subject]').first().click();
    await page.getByTestId('exact-source').click();
    assert.equal(await page.getByTestId('source-content').innerText(), capturedSource);
    await page.screenshot({ path: join(report, 'cold-offline.png'), fullPage: true });
    await page.getByTestId('forget-offline').click();
    await page.getByTestId('offline-ready').waitFor({ state: 'detached' });
    const remainingCopies = await page.evaluate(
      () =>
        new Promise((resolve, reject) => {
          const request = indexedDB.open('declared-local-views', 1);
          request.onerror = () => reject(request.error);
          request.onsuccess = () => {
            const db = request.result;
            const count = db.transaction('copies').objectStore('copies').count();
            count.onsuccess = () => {
              db.close();
              resolve(count.result);
            };
            count.onerror = () => {
              db.close();
              reject(count.error);
            };
          };
        })
    );
    assert.equal(remainingCopies, 0);
    const receipt = {
      kind,
      sourceRoot: root,
      sourceSnapshot: kind === 'rakia' ? process.env.LOCAL_VIEW_TEST_SOURCE_SNAPSHOT : undefined,
      build: manifest.id,
      doorwayEquivalent: Boolean(relay),
      subjects: count,
      progressSubjects: progressCount,
      compositionRetained: composed,
      groups: projection.composition?.groups.map(group => ({
        id: group.id,
        subjects: projection.subjects.filter(subject => subject.fields.$group === group.id).length,
      })),
      authoredClaims: projection.subjects.reduce(
        (total, subject) => total + (subject.fields.$claims?.length ?? 0),
        0
      ),
      resolvedLinks: projection.composition?.links.filter(link => link.state === 'resolved').length,
      observations: 2,
      captures,
      captureResponses,
      captureBudgetMs,
      coldRestart: true,
      servingProcessesStopped: true,
      networkDisabled: true,
      interruptedSavePreserved: true,
      storageExhaustionPreserved: true,
      failedRefreshPreserved: true,
      corruptReplacementPreserved: true,
      observationTimePreserved: true,
      exactSourcePreserved: true,
      sourceFilesUnchanged: true,
      workflowActsUnchanged: true,
      forgotten: true,
      sourceSha256: createHash('sha256').update(capturedSource).digest('hex'),
      pageErrors: errors,
    };
    await writeFile(join(report, 'receipt.json'), JSON.stringify(receipt, null, 2));
    console.log(`Browser receipt: ${report}`);
    assert.deepEqual(errors, []);
    stage('cold offline verified');
    return report;
  } catch (error) {
    if (context) {
      const page = context.pages().at(-1);
      if (page) {
        await page.screenshot({ path: join(report, 'failure.png'), fullPage: true });
        await writeFile(
          join(report, 'failure.txt'),
          (await page.locator('body').innerText()) + '\n' + errors.join('\n')
        );
      }
    }
    throw error;
  } finally {
    await context?.close();
    server.close();
    if (child.exitCode === null) child.kill('SIGTERM');
    if (relay?.exitCode === null) relay.kill('SIGTERM');
  }
}
if (process.argv[1] === fileURLToPath(import.meta.url)) await runLocalViewBrowser(process.argv[2]);
