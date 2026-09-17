/* eslint-disable @typescript-eslint/promise-function-async, sonarjs/no-clear-text-protocols */
import { strict as assert } from 'node:assert';
import { mkdtempSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, it } from 'node:test';

import {
  awaitCanonicalManifestoCandidate,
  classifyFirstPartyHttpError,
  httpOriginForWebSocketUrl,
  isOptionalNavigationCancellation,
  persistRealAppPhaseArtifacts,
  publicDoorwayUrl,
  publicDoorwayPorts,
  requiredRequestRoutingFailure,
  selectCanonicalManifestoCandidate,
  unexpectedOptionalNegatives,
} from '../real-app-network.js';

const OWNED_HOSTNAME = 'elohim.local';
const OWNED_ORIGIN = 'http://elohim.local:8889';
const OWNED_TOPOLOGY = {
  selectedOrigin: OWNED_ORIGIN,
  publicHostname: OWNED_HOSTNAME,
  ownedPorts: ['8888', '8889'],
};

void describe('public doorway browser routing', () => {
  void it('keeps the selected socket leg while using the declared public hostname', () => {
    assert.equal(publicDoorwayUrl('http://localhost:8889', OWNED_HOSTNAME), OWNED_ORIGIN);
  });

  void it('derives immutable ports from canonical fixture topology keys', () => {
    assert.deepEqual(
      publicDoorwayPorts({
        doorways: {
          alpha: { url: 'http://localhost:8888' },
          apex: { url: 'http://localhost:8889' },
        },
      }),
      ['8888', '8889']
    );
  });

  void it('rejects a withdrawn localhost leg absent from survivor membership', () => {
    const common = OWNED_TOPOLOGY;
    assert.match(
      requiredRequestRoutingFailure({
        ...common,
        requestUrl: 'http://localhost:8888/db/content',
      }) ?? '',
      /bypassed selected doorway/
    );
    assert.match(
      requiredRequestRoutingFailure({
        ...common,
        requestUrl: 'http://elohim.local:8888/main.js',
      }) ?? '',
      /bypassed selected doorway/
    );
    assert.match(
      requiredRequestRoutingFailure({
        ...common,
        requestUrl: 'https://doorway-alpha.elohim.host/db/content/elohim-host-landing',
      }) ?? '',
      /bypassed selected doorway/
    );
    assert.match(
      requiredRequestRoutingFailure({
        ...common,
        requestUrl: 'https://doorway-alpha.elohim.host/epr-head/elohim-protocol',
      }) ?? '',
      /bypassed selected doorway/
    );
    assert.match(
      requiredRequestRoutingFailure({
        ...common,
        requestUrl: 'https://doorway-alpha.elohim.host/api/v1/federation/doorways',
      }) ?? '',
      /bypassed selected doorway/
    );
    assert.equal(
      requiredRequestRoutingFailure({
        ...common,
        requestUrl: `${OWNED_ORIGIN}/db/content`,
      }),
      undefined
    );
    assert.equal(
      requiredRequestRoutingFailure({
        ...common,
        requestUrl: 'https://www.youtube.com/embed/demo',
      }),
      undefined
    );
    assert.equal(
      requiredRequestRoutingFailure({
        ...common,
        requestUrl: 'https://cdn.example.org/player.js',
      }),
      undefined
    );
    assert.equal(
      requiredRequestRoutingFailure({
        ...common,
        requestUrl: 'https://cdn.example.org/player.css',
      }),
      undefined
    );
    assert.equal(
      requiredRequestRoutingFailure({
        ...common,
        requestUrl: 'https://api.example.org/api/player',
      }),
      undefined
    );
  });

  void it('closes the origin-escape gap: ANY resource on elohim.host or a subdomain must route through the selected origin, not just /db/, /epr-head/, /api/', () => {
    const common = OWNED_TOPOLOGY;

    // Root document (no path prefix at all) on an unowned elohim.host origin.
    assert.match(
      requiredRequestRoutingFailure({
        ...common,
        requestUrl: 'https://doorway-alpha.elohim.host/',
      }) ?? '',
      /bypassed selected doorway/
    );

    // A static asset — never matches /db/, /epr-head/, /api/ — on the same
    // unowned elohim.host origin.
    assert.match(
      requiredRequestRoutingFailure({
        ...common,
        requestUrl: 'https://doorway-alpha.elohim.host/main.js',
      }) ?? '',
      /bypassed selected doorway/
    );

    // A lookalike host is never treated as elohim.host: no dot precedes the
    // suffix, so a naive string-suffix/substring check would wrongly match.
    assert.equal(
      requiredRequestRoutingFailure({
        ...common,
        requestUrl: 'https://notelohim.host.example/',
      }),
      undefined
    );
    assert.equal(
      requiredRequestRoutingFailure({
        ...common,
        requestUrl: 'https://evil-elohim.host/',
      }),
      undefined
    );

    // The selected origin itself always passes, even though it isn't
    // literally under elohim.host in this fixture's local topology.
    assert.equal(
      requiredRequestRoutingFailure({
        ...common,
        requestUrl: `${OWNED_ORIGIN}/`,
      }),
      undefined
    );

    // A genuine elohim.host origin that IS the currently-selected owned
    // origin passes for any path, including one that would otherwise be
    // unguarded (root document).
    assert.equal(
      requiredRequestRoutingFailure({
        selectedOrigin: 'https://doorway-alpha.elohim.host',
        publicHostname: 'doorway-alpha.elohim.host',
        ownedPorts: [],
        requestUrl: 'https://doorway-alpha.elohim.host/',
      }),
      undefined
    );
  });
});

void describe('httpOriginForWebSocketUrl + websocket origin-escape', () => {
  void it('maps ws/wss to the http/https counterpart, keeping host/path/query', () => {
    assert.equal(
      httpOriginForWebSocketUrl('ws://elohim.local:8889/signal'),
      `${OWNED_ORIGIN}/signal`
    );
    assert.equal(
      httpOriginForWebSocketUrl('wss://doorway-alpha.elohim.host/signal?x=1'),
      'https://doorway-alpha.elohim.host/signal?x=1'
    );
  });

  void it('a wss escape to an unowned elohim.host origin is a routing failure once mapped', () => {
    const common = OWNED_TOPOLOGY;
    assert.match(
      requiredRequestRoutingFailure({
        ...common,
        requestUrl: httpOriginForWebSocketUrl('wss://doorway-alpha.elohim.host/signal'),
      }) ?? '',
      /bypassed selected doorway/
    );
    assert.equal(
      requiredRequestRoutingFailure({
        ...common,
        requestUrl: httpOriginForWebSocketUrl('ws://elohim.local:8889/signal'),
      }),
      undefined
    );
  });
});

const DISCOVERY_PATH = '/api/v1/federation/doorways';
const CONSTITUTION_MISS = '404 /db/content/constitution';
const THEOLOGY_MISS = '404 /db/content/theology';
const REQUIRED_FAILURE = 'required-failure';
const ABORTED = 'net::ERR_ABORTED';

void describe('selectCanonicalManifestoCandidate', () => {
  void it('selects the unique canonical content renderer when two markdown containers exist', () => {
    assert.equal(
      selectCanonicalManifestoCandidate([
        '',
        'Elohim Protocol\nExecutive Summary\nsubstantive body\nLove as Technology',
      ]),
      1
    );
  });

  void it('rejects missing and duplicate canonical renderers', () => {
    assert.throws(() => selectCanonicalManifestoCandidate(['', 'unrelated']));
    assert.throws(() =>
      selectCanonicalManifestoCandidate([
        'Executive Summary Love as Technology',
        'Executive Summary Love as Technology',
      ])
    );
  });
});

void describe('awaitCanonicalManifestoCandidate', () => {
  void it('waits through an empty renderer until one substantive canonical renderer settles', async () => {
    const manifesto = `Executive Summary ${'content '.repeat(140)} Love as Technology`;
    const readings = [
      ['', ''],
      ['', manifesto],
      ['', manifesto],
    ];
    const result = await awaitCanonicalManifestoCandidate(
      () => Promise.resolve(readings.shift() ?? ['', manifesto]),
      Date.now() + 1_000,
      () => Promise.resolve()
    );
    assert.equal(result.index, 1);
    assert.equal(result.text, manifesto);
  });

  void it('does not accept a late duplicate canonical renderer', async () => {
    const manifesto = `Executive Summary ${'content '.repeat(140)} Love as Technology`;
    const readings = [
      [manifesto, ''],
      [manifesto, manifesto],
    ];
    let now = 0;
    await assert.rejects(
      awaitCanonicalManifestoCandidate(
        () => Promise.resolve(readings.shift() ?? [manifesto, manifesto]),
        2,
        () => {
          now += 1;
          return Promise.resolve();
        },
        () => now
      )
    );
  });
});

void describe('persistRealAppPhaseArtifacts', () => {
  void it('keeps one run intact when a later run writes the same phase', () => {
    const reportsRoot = mkdtempSync(join(tmpdir(), 'real-app-artifacts-'));
    const first = persistRealAppPhaseArtifacts({
      runId: 'run-one',
      phase: 'baseline',
      screenshot: Buffer.from('first-png'),
      receipt: { run: 1 },
      reportsRoot,
    });
    persistRealAppPhaseArtifacts({
      runId: 'run-two',
      phase: 'baseline',
      screenshot: Buffer.from('second-png'),
      receipt: { run: 2 },
      reportsRoot,
    });
    assert.equal(readFileSync(first.screenshotPath, 'utf8'), 'first-png');
    const firstReceipt = JSON.parse(readFileSync(first.receiptPath, 'utf8')) as {
      run: number;
      artifactPaths: typeof first;
    };
    assert.equal(firstReceipt.run, 1);
    assert.deepEqual(firstReceipt.artifactPaths, first);
  });
});

void describe('isOptionalNavigationCancellation', () => {
  void it('allows the exact known discovery cancellation observed at baseline', () => {
    assert.equal(
      isOptionalNavigationCancellation({
        path: DISCOVERY_PATH,
        errorText: ABORTED,
      }),
      true
    );
  });
  void it('keeps a critical asset abort and a discovery failure outside navigation fatal', () => {
    assert.equal(
      isOptionalNavigationCancellation({
        path: '/main.js',
        errorText: ABORTED,
      }),
      false
    );
    assert.equal(
      isOptionalNavigationCancellation({
        path: DISCOVERY_PATH,
        errorText: 'net::ERR_FAILED',
      }),
      false
    );
    assert.equal(
      isOptionalNavigationCancellation({
        path: DISCOVERY_PATH,
        errorText: 'net::ERR_ABORTED_CRITICAL',
      }),
      false
    );
  });
});

void describe('classifyFirstPartyHttpError', () => {
  void it('allows only contract-grounded discovery misses and the private love-map refusal', () => {
    assert.equal(
      classifyFirstPartyHttpError({ path: '/epr-head/value-scanner-epic', status: 404 }),
      'expected-optional-negative'
    );
    assert.equal(
      classifyFirstPartyHttpError({ path: '/db/content/love-map-matthew-jessica', status: 403 }),
      'expected-optional-negative'
    );
  });

  void it('keeps required assets, changed statuses, and unknown optional-looking paths fatal', () => {
    assert.equal(classifyFirstPartyHttpError({ path: '/main.js', status: 404 }), REQUIRED_FAILURE);
    assert.equal(
      classifyFirstPartyHttpError({ path: '/epr-head/value-scanner-epic', status: 500 }),
      REQUIRED_FAILURE
    );
    assert.equal(
      classifyFirstPartyHttpError({ path: '/epr-head/new-related-card', status: 404 }),
      REQUIRED_FAILURE
    );
  });

  void it('fails a later phase that introduces a new optional negative', () => {
    const baseline = new Set([CONSTITUTION_MISS]);
    const later = new Set([CONSTITUTION_MISS, THEOLOGY_MISS]);
    assert.deepEqual(unexpectedOptionalNegatives(baseline, later), [THEOLOGY_MISS]);
  });

  void it('accepts a later phase where a baseline optional negative becomes healthy', () => {
    assert.deepEqual(unexpectedOptionalNegatives(new Set([CONSTITUTION_MISS]), new Set()), []);
  });
});
