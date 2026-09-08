import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { test } from 'node:test';
import { promisify } from 'node:util';

const execute = promisify(execFile);
const contentType = 'Content-Type';
const expectedHead = 'sha256-test';
const sameOriginRedirect = 'same-origin-redirect';
const ssrWithoutBootstrap = 'ssr-without-bootstrap';
for (const mode of [
  'valid',
  sameOriginRedirect,
  'cross-page',
  'cross-head',
  'cross-version',
  'cross-asset',
  'browser-redirect',
  'throw',
  'stale-version',
  'missing-asset',
  'missing-version',
  'empty-root',
  ssrWithoutBootstrap,
]) {
  void test(`served shell: ${mode}`, async () => {
    const server = createServer((request, response) => {
      const url = request.url ?? '/';
      const redirectTarget =
        mode === 'cross-page'
          ? '/'
          : mode === 'cross-head'
            ? '/apps/sha256-test/index.html'
            : mode === 'cross-version'
              ? '/version.json'
              : mode === 'cross-asset'
                ? '/main-TEST.js'
                : '';
      if (url === redirectTarget && request.headers.host?.startsWith('127.0.0.1')) {
        response
          .writeHead(302, {
            Location: `http://localhost:${request.headers.host.split(':')[1]}${url}`,
          })
          .end();
        return;
      }
      if (mode === sameOriginRedirect && url === '/') {
        response.writeHead(302, { Location: '/index.html' }).end();
        return;
      }
      const isHead = url.startsWith('/apps/sha256-test/');
      const path = isHead ? url.replace('/apps/sha256-test', '') : url;
      if (path === '/version.json') {
        if (mode === 'missing-version') {
          response.writeHead(404).end();
          return;
        }
        response.setHeader(contentType, 'application/json');
        response.end(
          JSON.stringify({ version: mode === 'stale-version' && !isHead ? 'old' : 'new' })
        );
      } else if (path === '/main-TEST.js') {
        if (mode === 'missing-asset') {
          response.writeHead(404).end();
          return;
        }
        response.setHeader(contentType, 'application/javascript');
        response.end(
          mode === 'browser-redirect'
            ? 'document.querySelector("app-root").textContent="Ready"; document.querySelector("app-root").setAttribute("data-app-ready","true"); if(location.hostname==="127.0.0.1") location.href="http://localhost:"+location.port+"/index.html"'
            : mode === 'throw'
              ? 'throw new Error("startup broke")'
              : mode === 'empty-root' || mode === ssrWithoutBootstrap
                ? ''
                : 'document.querySelector("app-root").textContent="Ready"; document.querySelector("app-root").setAttribute("data-app-ready","true")'
        );
      } else if (path === '/' || path === '/index.html') {
        response.setHeader(contentType, 'text/html');
        response.end(
          `<app-root>${mode === ssrWithoutBootstrap ? 'Rendered on server' : ''}</app-root><script src="/main-TEST.js"></script>`
        );
      } else response.writeHead(404).end();
    });
    await new Promise<void>(done => server.listen(0, '127.0.0.1', done));
    const address = server.address();
    assert.ok(address && typeof address !== 'string');
    const artifacts = await mkdtemp(`${tmpdir()}/served-shell-`);
    try {
      const run = execute(
        'bash',
        [
          resolve('../../scripts/ci/verify-served-shell.sh'),
          `http://127.0.0.1:${address.port}`,
          '/',
          'test-app',
          expectedHead,
        ],
        {
          env: {
            ...process.env,
            CI: 'false',
            JENKINS_URL: '',
            DEADLINE_SECS: '0',
            BOOT_TIMEOUT_MS: '1500',
            SHELL_REPORT_DIR: artifacts,
          },
        }
      );
      if (mode === 'valid' || mode === sameOriginRedirect)
        assert.match((await run).stdout, /boots at head/);
      else await assert.rejects(run);
    } finally {
      await new Promise<void>((done, reject) =>
        server.close(error => (error ? reject(error) : done()))
      );
      await rm(artifacts, { recursive: true, force: true });
    }
  });
}

const customRoute = 'custom-route';
const browserMount = 'browser-mount';
const declarationTransient = 'declaration-transient';
const declarationChanged = 'declaration-changed';
const declarationDeadline = 'declaration-deadline';
const adoptionTransient = 'adoption-transient';
const renderedHeading = 'A living learning path';
const headingPresent = 'heading-present';
const renderedConcept = '/lamad/concept/elohim-host-landing';
for (const mode of [
  'current',
  'missing',
  'stale',
  'failed',
  'csr-fallback',
  'redirect',
  customRoute,
  browserMount,
  'declaration-missing',
  'declaration-wrong',
  'declaration-malformed',
  'declaration-unreachable',
  declarationTransient,
  declarationChanged,
  declarationDeadline,
  adoptionTransient,
  headingPresent,
  'heading-loading',
  'heading-state-only',
]) {
  void test(`SSR attestation: ${mode}`, async () => {
    let declarationReads = 0;
    let healthReads = 0;
    const server = createServer((request, response) => {
      if (request.url === '/db/content/test-app') {
        declarationReads++;
        if (
          mode === 'declaration-unreachable' ||
          (mode === declarationTransient && declarationReads === 1)
        ) {
          response.writeHead(503).end();
        } else if (mode === 'declaration-malformed') {
          response.end('not JSON');
        } else {
          response.setHeader(contentType, 'application/json');
          response.end(
            JSON.stringify({
              serverBlobHash:
                mode === 'declaration-missing' || mode === declarationDeadline
                  ? null
                  : mode === 'declaration-wrong' ||
                      (mode === declarationChanged && declarationReads > 1)
                    ? 'other'
                    : expectedHead,
            })
          );
        }
      } else if (request.url?.startsWith('/health')) {
        healthReads++;
        response.setHeader(contentType, 'application/json');
        response.end(
          JSON.stringify(
            mode === 'missing'
              ? {}
              : {
                  servedBundleHeads: [
                    {
                      slug: 'test-app',
                      serverBlobHash:
                        mode === 'stale' || (mode === adoptionTransient && healthReads === 1)
                          ? 'old'
                          : expectedHead,
                      status: mode === 'failed' ? 'failed' : 'current',
                    },
                  ],
                }
          )
        );
      } else {
        const correctRoute =
          ![customRoute, browserMount].includes(mode) || request.url === renderedConcept;
        if (mode !== 'csr-fallback' && correctRoute) response.setHeader('x-ssr-rendered', '1');
        if (mode === 'redirect')
          response.writeHead(302, { Location: 'https://other-doorway.invalid/' });
        response.end(
          mode === headingPresent
            ? `<app-root><h1>A living <span>learning path</span></h1></app-root>`
            : mode === 'heading-state-only'
              ? `<app-root>Loading</app-root><script type="application/json">{"title":"${renderedHeading}"}</script>`
              : '<app-root>Loading</app-root>'
        );
      }
    });
    await new Promise<void>(done => server.listen(0, '127.0.0.1', done));
    const address = server.address();
    assert.ok(address && typeof address !== 'string');
    try {
      const run = execute(
        'bash',
        [
          resolve('../../scripts/ci/verify-projected-head.sh'),
          `http://127.0.0.1:${address.port}`,
          'test-app',
          expectedHead,
          '',
          mode === customRoute ? renderedConcept : mode === browserMount ? '/lamad' : '/',
          mode.startsWith('heading-') ? renderedHeading : '',
        ],
        {
          env: {
            ...process.env,
            PROJHEAD_CONVERGE_WINDOW: mode === adoptionTransient ? '3' : '0',
            PROJHEAD_CONVERGE_INTERVAL: '1',
            PROJHEAD_DECLARE_WINDOW:
              mode === declarationTransient ? '3' : mode === declarationDeadline ? '1' : '0',
            PROJHEAD_DECLARE_INTERVAL: '1',
          },
        }
      );
      if (
        mode === 'current' ||
        mode === customRoute ||
        mode === declarationTransient ||
        mode === headingPresent
      )
        assert.match((await run).stdout, /projected head propagated/);
      else if (mode === adoptionTransient)
        assert.match((await run).stdout, /projected head converged/);
      else if (mode.startsWith('declaration-')) {
        await assert.rejects(run, (error: unknown) => {
          assert.match((error as { stderr: string }).stderr, /SSR declaration/);
          return true;
        });
        if (mode !== declarationChanged)
          assert.equal(healthReads, 0, 'no adoption without declaration');
        if (mode === declarationDeadline)
          assert.equal(declarationReads, 1, 'no new request after the declaration deadline');
      } else await assert.rejects(run);
    } finally {
      await new Promise<void>((done, reject) =>
        server.close(error => (error ? reject(error) : done()))
      );
    }
  });
}
