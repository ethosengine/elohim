import { strict as assert } from 'node:assert';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, it } from 'node:test';

import {
  observePhase,
  loadAuthorReceipt,
  parseArgs,
  type ExpectedAuthority,
  type ObserverConfig,
  type RawObservation,
  type Requester,
} from '../doorway-wan-chaos-observer.js';

const A = {
  actionHash: 'action-a',
  blobHash: 'blob-a',
  version: 'version-a',
  entryScript: 'main-a.js',
};
const B = {
  actionHash: 'action-b',
  blobHash: 'blob-b',
  version: 'version-b',
  entryScript: 'main-b.js',
};
const TEMP_PREFIX = join(tmpdir(), 'doorway-observer-');
const RECEIPT = 'receipt.json';
const ETHOSENGINE = 'https://192.0.2.10';
const SHEM = 'https://192.0.2.11';
const NOT_MEASURED = 'NOT MEASURED';
const CONTENT_ID = 'governed-site';

function authorReceipt(expected: ExpectedAuthority, suffix = expected.actionHash) {
  return {
    path: `/author-receipt-${suffix}.json`,
    sha256: `sha256-${suffix}`,
    receipt: {
      schema: 'doorway-chaos-author-receipt/v1' as const,
      source: 'fixture-author-operation' as const,
      receiptId: `receipt-${suffix}`,
      authorId: 'human-matthew-manager',
      authoredAt: new Date().toISOString(),
      contentId: CONTENT_ID,
      authority: expected,
    },
  };
}

function response(text: string, remoteAddress = '192.0.2.10'): RawObservation {
  return {
    status: 200,
    text,
    headers: { 'x-doorway-id': 'remote-claim' },
    startedAt: '2026-09-14T00:00:00.000Z',
    endedAt: '2026-09-14T00:00:00.010Z',
    remoteAddress,
  };
}

function requesterFor(expected: ExpectedAuthority, unavailable = new Set<string>()): Requester {
  return async target => {
    const key = target.address ?? (expected.actionHash === A.actionHash ? ETHOSENGINE : SHEM);
    if (unavailable.has(key)) {
      return await Promise.resolve({ ...response('', key), status: 0, error: 'unavailable' });
    }
    if (target.path.includes('/head?'))
      return await Promise.resolve(
        response(JSON.stringify({ headActionHash: expected.actionHash }), key)
      );
    if (target.path.includes('/db/content/'))
      return await Promise.resolve(response(JSON.stringify({ blobHash: expected.blobHash }), key));
    if (target.path.includes('/version.json?'))
      return await Promise.resolve(response(JSON.stringify({ commit: expected.version }), key));
    return await Promise.resolve(
      response(`<app-root></app-root><script src="${expected.entryScript}"></script>`, key)
    );
  };
}

function config(
  reportPath: string,
  phase: ObserverConfig['phase'],
  expected: ExpectedAuthority
): ObserverConfig {
  const boundaryAt = new Date().toISOString();
  return {
    phase,
    publicName: 'elohim.example',
    contentId: CONTENT_ID,
    expected,
    authorReceipt: authorReceipt(expected),
    boundaryAt,
    faultOnset: phase === 'fault' ? new Date(Date.parse(boundaryAt) - 1).toISOString() : undefined,
    recoveryOnset: phase === 'recovery' ? boundaryAt : undefined,
    maxEventObservationDelayMs: 1_000,
    timeoutMs: 100,
    windowMs: 2,
    cadenceMs: 1,
    maxGapMs: 1_000,
    legs: [
      { name: 'ethosengine', address: ETHOSENGINE },
      { name: 'shem', address: SHEM },
    ],
    reportPath,
  };
}

void describe('doorway WAN chaos observer', () => {
  void it('refuses a response that nominates markers other than the external oracle', async () => {
    const reportPath = join(mkdtempSync(TEMP_PREFIX), RECEIPT);
    const report = await observePhase(config(reportPath, 'baseline', A), requesterFor(B));
    assert.equal(report.phases.baseline?.verdict, 'REFUSED');
    assert.deepEqual(report.phases.baseline?.expected, A);
    assert.deepEqual(report.phases.baseline?.ordinary.observed, B);
  });

  void it('makes every probe fresh and keeps declared, kernel, and reported attribution separate', async () => {
    const reportPath = join(mkdtempSync(TEMP_PREFIX), RECEIPT);
    const paths: string[] = [];
    const base = requesterFor(A);
    const requester: Requester = async target => {
      paths.push(target.path);
      return await base(target);
    };
    const report = await observePhase(config(reportPath, 'baseline', A), requester);
    assert.ok(paths.every(path => path.includes('chaos_observer=')));
    const leg = report.phases.baseline?.legs[0];
    assert.equal(leg?.attribution.declaredAddress, ETHOSENGINE);
    assert.equal(leg?.attribution.kernelVerifiedRemoteAddress, ETHOSENGINE);
    assert.equal(leg?.attribution.remotelyReportedDoorway, 'remote-claim');
  });

  void it('does not call DNS success automatic selection without verified withdrawal and a survivor', async () => {
    const reportPath = join(mkdtempSync(TEMP_PREFIX), RECEIPT);
    await observePhase(config(reportPath, 'baseline', A), requesterFor(A));
    await new Promise(resolve => setTimeout(resolve, 2));
    const fault = config(reportPath, 'fault', B);
    fault.withdrawnLeg = 'ethosengine';
    const afterFault = await observePhase(fault, requesterFor(B, new Set([ETHOSENGINE])));
    assert.equal(afterFault.claims.survivesViaRemainingDeclaredEntrance, 'PERMITTED');
    assert.equal(afterFault.claims.sameHostnameSelectsAfterWithdrawal, 'PERMITTED');
    assert.equal(afterFault.claims.dnsMembershipWithdrawal, NOT_MEASURED);

    const withoutMarker = config(join(mkdtempSync(TEMP_PREFIX), RECEIPT), 'fault', B);
    const dnsOnly = await observePhase(withoutMarker, requesterFor(B));
    assert.equal(dnsOnly.claims.sameHostnameSelectsAfterWithdrawal, NOT_MEASURED);
  });

  void it('requires ordinary DNS to use the later-withdrawn leg at baseline', async () => {
    const reportPath = join(mkdtempSync(TEMP_PREFIX), RECEIPT);
    const shemAtBaseline: Requester = async target =>
      await requesterFor(A)({ ...target, address: target.address ?? SHEM });
    await observePhase(config(reportPath, 'baseline', A), shemAtBaseline);
    await new Promise(resolve => setTimeout(resolve, 2));
    const fault = config(reportPath, 'fault', B);
    fault.withdrawnLeg = 'ethosengine';
    const afterFault = await observePhase(fault, requesterFor(B, new Set([ETHOSENGINE])));
    assert.equal(afterFault.claims.sameHostnameSelectsAfterWithdrawal, 'REFUSED');
  });

  void it('refuses exact authority markers when the public root does not bind the expected entry', async () => {
    const reportPath = join(mkdtempSync(TEMP_PREFIX), RECEIPT);
    const base = requesterFor(A);
    const missingEntry: Requester = async target =>
      target.path.startsWith('/?')
        ? response('<app-root></app-root>', ETHOSENGINE)
        : await base(target);
    const report = await observePhase(config(reportPath, 'baseline', A), missingEntry);
    assert.equal(report.phases.baseline?.verdict, 'REFUSED');
  });

  void it('requires a fixture-author receipt instead of accepting SUT-shaped marker arguments', () => {
    assert.throws(
      () =>
        parseArgs([
          '--phase',
          'baseline',
          '--public-name',
          'elohim.example',
          '--content-id',
          CONTENT_ID,
          '--expected-action',
          A.actionHash,
        ]),
      /missing --author-receipt/
    );
    const path = join(mkdtempSync(TEMP_PREFIX), 'sut-shaped.json');
    writeFileSync(path, JSON.stringify(A));
    assert.throws(() => loadAuthorReceipt(path), /not a fixture-author operation receipt/);
  });

  void it('refuses a pinned leg whose socket peer differs from its declared address', async () => {
    const reportPath = join(mkdtempSync(TEMP_PREFIX), RECEIPT);
    const base = requesterFor(A);
    const misdirected: Requester = async target => {
      const observed = await base(target);
      return target.address === ETHOSENGINE ? { ...observed, remoteAddress: SHEM } : observed;
    };
    const report = await observePhase(config(reportPath, 'baseline', A), misdirected);
    assert.equal(report.phases.baseline?.legs[0].verdict, 'REFUSED');
    assert.equal(report.phases.baseline?.verdict, 'REFUSED');
  });

  void it('refuses an otherwise-perfect phase that begins before its declared boundary', async () => {
    const reportPath = join(mkdtempSync(TEMP_PREFIX), RECEIPT);
    const futureBoundary = config(reportPath, 'baseline', A);
    futureBoundary.boundaryAt = new Date(Date.now() + 100).toISOString();
    const report = await observePhase(futureBoundary, requesterFor(A));
    assert.equal(report.phases.baseline?.continuity.verdict, 'REFUSED');
    assert.equal(report.phases.baseline?.verdict, 'REFUSED');
  });

  void it('refuses an otherwise-perfect phase whose real sample cadence exceeds its bound', async () => {
    const reportPath = join(mkdtempSync(TEMP_PREFIX), RECEIPT);
    const slow: Requester = async target => {
      await new Promise(resolve => setTimeout(resolve, 2));
      return await requesterFor(A)(target);
    };
    const tightCadence = config(reportPath, 'baseline', A);
    tightCadence.maxGapMs = 2;
    const report = await observePhase(tightCadence, slow);
    assert.ok((report.phases.baseline?.continuity.largestSampleGapMs ?? 0) > 2);
    assert.equal(report.phases.baseline?.continuity.verdict, 'REFUSED');
    assert.equal(report.phases.baseline?.verdict, 'REFUSED');
  });

  void it('refuses a perfect fault window that starts after the fault observation SLO', async () => {
    const reportPath = join(mkdtempSync(TEMP_PREFIX), RECEIPT);
    await observePhase(config(reportPath, 'baseline', A), requesterFor(A));
    await new Promise(resolve => setTimeout(resolve, 2));
    const fault = config(reportPath, 'fault', B);
    fault.withdrawnLeg = 'ethosengine';
    fault.maxEventObservationDelayMs = 1;
    await new Promise(resolve => setTimeout(resolve, 3));
    fault.boundaryAt = new Date().toISOString();
    const report = await observePhase(fault, requesterFor(B, new Set([ETHOSENGINE])));
    assert.equal(report.phases.fault?.verdict, 'REFUSED');
  });

  void it('measures the event SLO to a complete successful sample, not its early start', async () => {
    const reportPath = join(mkdtempSync(TEMP_PREFIX), RECEIPT);
    await observePhase(config(reportPath, 'baseline', A), requesterFor(A));
    await new Promise(resolve => setTimeout(resolve, 2));
    const fault = config(reportPath, 'fault', B);
    fault.withdrawnLeg = 'ethosengine';
    fault.maxEventObservationDelayMs = 10;
    const base = requesterFor(B, new Set([ETHOSENGINE]));
    const slow: Requester = async target => {
      await new Promise(resolve => setTimeout(resolve, 2));
      return await base(target);
    };
    const report = await observePhase(fault, slow);
    assert.ok((report.phases.fault?.continuity.eventToFirstSampleMs ?? 99) <= 10);
    assert.ok((report.phases.fault?.continuity.eventToFirstCompleteSampleMs ?? 0) > 10);
    assert.equal(report.phases.fault?.verdict, 'REFUSED');
  });

  void it('rejects non-finite timing bounds before observation', async () => {
    const reportPath = join(mkdtempSync(TEMP_PREFIX), RECEIPT);
    const invalidGap = config(reportPath, 'baseline', A);
    invalidGap.maxGapMs = Number.NaN;
    await assert.rejects(observePhase(invalidGap, requesterFor(A)), /--max-gap-ms/);

    const invalidEvent = config(reportPath, 'fault', B);
    invalidEvent.maxEventObservationDelayMs = Number.POSITIVE_INFINITY;
    await assert.rejects(
      observePhase(invalidEvent, requesterFor(B)),
      /--max-event-observation-delay-ms/
    );
  });

  void it('refuses authority B when its author receipt predates fault onset', async () => {
    const reportPath = join(mkdtempSync(TEMP_PREFIX), RECEIPT);
    await observePhase(config(reportPath, 'baseline', A), requesterFor(A));
    await new Promise(resolve => setTimeout(resolve, 2));
    const fault = config(reportPath, 'fault', B);
    fault.withdrawnLeg = 'ethosengine';
    fault.authorReceipt.receipt.authoredAt = new Date(
      Date.parse(fault.faultOnset as string) - 1
    ).toISOString();
    const report = await observePhase(fault, requesterFor(B, new Set([ETHOSENGINE])));
    assert.equal(report.phases.fault?.verdict, 'REFUSED');
  });

  void it('cannot pass an unknown or out-of-order phase', async () => {
    const reportPath = join(mkdtempSync(TEMP_PREFIX), RECEIPT);
    const fault = await observePhase(config(reportPath, 'fault', B), requesterFor(B));
    assert.equal(fault.phases.fault?.verdict, NOT_MEASURED);

    const baseline = config(reportPath, 'baseline', A);
    baseline.boundaryAt = new Date().toISOString();
    await observePhase(baseline, requesterFor(A));
    const recovery = config(reportPath, 'recovery', B);
    recovery.boundaryAt = new Date(Date.parse(baseline.boundaryAt) - 1).toISOString();
    const reversed = await observePhase(recovery, requesterFor(B));
    assert.equal(reversed.phases.recovery?.verdict, 'REFUSED');
  });

  void it('permits authority continuity only after A-to-B, split B, and recovered B all agree', async () => {
    const reportPath = join(mkdtempSync(TEMP_PREFIX), RECEIPT);
    await observePhase(config(reportPath, 'baseline', A), requesterFor(A));
    await new Promise(resolve => setTimeout(resolve, 2));
    const fault = config(reportPath, 'fault', B);
    fault.withdrawnLeg = 'ethosengine';
    await observePhase(fault, requesterFor(B, new Set([ETHOSENGINE])));
    await new Promise(resolve => setTimeout(resolve, 2));
    const recovery = config(reportPath, 'recovery', B);
    recovery.authorReceipt = fault.authorReceipt;
    const complete = await observePhase(recovery, requesterFor(B));
    assert.equal(complete.claims.currentAuthorityDuringSplitAndRecovery, 'PERMITTED');
    assert.equal(complete.claims.sameHostnameSelectsAfterWithdrawal, 'PERMITTED');
    assert.match(readFileSync(reportPath, 'utf8'), /history-receipt-not-protocol-truth/);
  });

  void it('refuses reused A provenance and a recovery receipt that is not fault B', async () => {
    const reusedPath = join(mkdtempSync(TEMP_PREFIX), RECEIPT);
    await observePhase(config(reusedPath, 'baseline', A), requesterFor(A));
    await new Promise(resolve => setTimeout(resolve, 2));
    const reusedFault = config(reusedPath, 'fault', A);
    reusedFault.withdrawnLeg = 'ethosengine';
    const reused = await observePhase(reusedFault, requesterFor(A, new Set([ETHOSENGINE])));
    assert.equal(reused.phases.fault?.verdict, 'REFUSED');

    const changedPath = join(mkdtempSync(TEMP_PREFIX), RECEIPT);
    await observePhase(config(changedPath, 'baseline', A), requesterFor(A));
    await new Promise(resolve => setTimeout(resolve, 2));
    const fault = config(changedPath, 'fault', B);
    fault.withdrawnLeg = 'ethosengine';
    await observePhase(fault, requesterFor(B, new Set([ETHOSENGINE])));
    await new Promise(resolve => setTimeout(resolve, 2));
    const changedRecovery = config(changedPath, 'recovery', B);
    changedRecovery.authorReceipt = authorReceipt(B, 'different-b-receipt');
    const changed = await observePhase(changedRecovery, requesterFor(B));
    assert.equal(changed.phases.recovery?.verdict, 'REFUSED');
  });

  void it('refuses aggregate continuity when recovery changes only B entry script', async () => {
    const reportPath = join(mkdtempSync(TEMP_PREFIX), RECEIPT);
    await observePhase(config(reportPath, 'baseline', A), requesterFor(A));
    await new Promise(resolve => setTimeout(resolve, 2));
    const fault = config(reportPath, 'fault', B);
    fault.withdrawnLeg = 'ethosengine';
    await observePhase(fault, requesterFor(B, new Set([ETHOSENGINE])));
    await new Promise(resolve => setTimeout(resolve, 2));
    const changedEntry = { ...B, entryScript: 'other-main-b.js' };
    const recovery = config(reportPath, 'recovery', changedEntry);
    recovery.authorReceipt = authorReceipt(changedEntry, B.actionHash);
    const complete = await observePhase(recovery, requesterFor(changedEntry));
    assert.equal(complete.claims.currentAuthorityDuringSplitAndRecovery, 'REFUSED');
  });
});
