import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import {
  type AppRun,
  appRunGraph,
  type DeliveryData,
  orchestratorRunGraph,
  renderDeliveryPage,
} from '../lib/delivery-view.js';

const stage = (name: string, result: string, minutes: number) => ({
  name,
  result,
  durationMs: minutes * 60_000,
});

const unsplit: AppRun = {
  number: 1726,
  result: 'FAILURE',
  minutes: 129.5,
  split: 'stage',
  phases: { readiness: null, publish: null, verify: null, converge: null },
  publishVerifyMin: 122.5,
  delivered: 0,
  stages: [
    stage('Build App', 'SUCCESS', 0.9),
    stage('Publish and Verify App Delivery', 'FAILURE', 122.5),
  ],
};

const phased: AppRun = {
  ...unsplit,
  number: 1730,
  result: 'SUCCESS',
  split: 'phases',
  phases: { readiness: 0.2, publish: 3.1, verify: 1.4, converge: null },
  delivered: 1,
  stages: [
    stage('Build App', 'SUCCESS', 0.9),
    stage('Publish and Verify App Delivery', 'SUCCESS', 4.8),
  ],
};

void describe('delivery-view', () => {
  void it('gives the readiness case its own node even when the build predates the phase split', () => {
    const html = appRunGraph(unsplit);
    assert.match(html, /data-node="readiness"/);
    assert.match(html, /UNSPLIT/);
    assert.match(html, /tier T4 · cost unmeasured/);
    // The stage itself still renders, with its wall clock as its cost.
    assert.match(
      html,
      /Publish and Verify App Delivery<\/a><\/b><br>FAILURE<br>tier T4 · cost 122.5m/
    );
    assert.match(html, /job\/elohim\/job\/dev\/1726\/console/);
  });

  void it('expands a phased build into readiness, publish and verify nodes with their minutes', () => {
    const html = appRunGraph(phased);
    assert.match(html, /data-node="readiness"[^]*cost 0.2m/);
    assert.match(html, /data-node="publish"[^]*cost 3.1m/);
    assert.match(html, /data-node="verify" data-result="PASSED"/);
    assert.doesNotMatch(html, /data-node="converge"/, 'a phase with no case is not invented');
    assert.doesNotMatch(html, /Publish and Verify App Delivery/, 'the phases replace the stage');
  });

  void it('draws an orchestrator run as its node then one node per planned pipeline', () => {
    const html = orchestratorRunGraph({
      number: 1904,
      result: 'FAILURE',
      minutes: 7.8,
      planned: ['elohim', 'elohim-edge'],
      outcome: { elohim: 'NOT_DISPATCHED', 'elohim-edge': 'SUCCESS' },
      delivered: false,
      timedOut: false,
    });
    assert.match(html, /data-node="orchestrator-1904"/);
    assert.match(html, /href="#app-elohim"/, 'the app node links to its stage graph');
    assert.match(html, /actual-build-graph\.json/);
  });

  void it('names an absent reading instead of rendering it as health', () => {
    const data: DeliveryData = {
      generatedAt: '2026-09-24T00:00:00.000Z',
      habit: { data: null, command: 'epr flow walk x --json', error: 'spawn epr ENOENT' },
      orchestrator: {
        data: null,
        command: 'node delivery-series.mjs',
        error: 'Jenkins unreadable',
      },
      app: {
        data: { pipelines: { elohim: { runs: [unsplit], considered: 1, delivered: 0 } } },
        command: 'node … --stages',
      },
    };
    const html = renderDeliveryPage(data);
    assert.match(html, /Habit walk NOT MEASURED: spawn epr ENOENT/);
    assert.match(html, /Orchestrator series NOT MEASURED/);
    assert.match(html, /∞ \(nothing delivered\)/);
    assert.match(html, /data-node="readiness"/);
  });
});
