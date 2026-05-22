import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalSocialComputePanel as ElohimQahalSocialComputePanelClass } from './elohim-qahal-social-compute-panel.js';
import type {
  ComputeTopology,
  ElohimQahalSocialComputePanel,
} from './elohim-qahal-social-compute-panel.js';

const SAMPLE_TOPOLOGY: ComputeTopology = {
  qahalId: 'bay-area-dawn-runners',
  selfHub: {
    hubId: 'hub-001',
    status: 'healthy',
    allocatedGB: 4,
    lastSeen: 'now',
  },
  stewardsForUs: [
    {
      stewardId: 'peer-alice',
      stewardName: 'Alice',
      health: 'healthy',
      percent: 100,
      lastSync: '17m ago',
      replicating: 'household state + care-economy ledger',
    },
    {
      stewardId: 'peer-bob',
      stewardName: 'Bob',
      health: 'degraded',
      percent: 72,
      lastSync: '2h ago',
      replicating: 'household state',
    },
    {
      stewardId: 'peer-carol',
      stewardName: 'Carol',
      health: 'down',
      percent: 0,
      lastSync: '1d ago',
      replicating: 'household state',
    },
  ],
  weStewardFor: [
    { stewardId: 'peer-delta', description: 'Sunrise Collective household state' },
    { stewardId: 'peer-echo', description: 'Mountain Node care-economy ledger' },
  ],
  recoveryReadiness: {
    ready: true,
    shamirThreshold: '2/3',
    lastDrill: '2026-04-15',
    nextDrill: '2026-07-15',
  },
};

describe('<elohim-qahal-social-compute-panel>', () => {
  it('renders selfHub status + allocation', async () => {
    const el = await fixture<ElohimQahalSocialComputePanel>(html`
      <elohim-qahal-social-compute-panel
        .topology=${SAMPLE_TOPOLOGY}
      ></elohim-qahal-social-compute-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('healthy');
    expect(text).to.include('4 GB');
    expect(text).to.include('now');
  });

  it('renders stewardsForUs with health indicators', async () => {
    const el = await fixture<ElohimQahalSocialComputePanel>(html`
      <elohim-qahal-social-compute-panel
        .topology=${SAMPLE_TOPOLOGY}
      ></elohim-qahal-social-compute-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('Alice');
    expect(text).to.include('Bob');
    expect(text).to.include('Carol');

    const healthyMarker = el.shadowRoot!.querySelector('.health-marker[data-health="healthy"]');
    const degradedMarker = el.shadowRoot!.querySelector('.health-marker[data-health="degraded"]');
    const downMarker = el.shadowRoot!.querySelector('.health-marker[data-health="down"]');
    expect(healthyMarker).to.exist;
    expect(healthyMarker!.textContent!.trim()).to.equal('●');
    expect(degradedMarker).to.exist;
    expect(degradedMarker!.textContent!.trim()).to.equal('◐');
    expect(downMarker).to.exist;
    expect(downMarker!.textContent!.trim()).to.equal('○');
  });

  it('renders weStewardFor as reciprocal-trust list', async () => {
    const el = await fixture<ElohimQahalSocialComputePanel>(html`
      <elohim-qahal-social-compute-panel
        .topology=${SAMPLE_TOPOLOGY}
      ></elohim-qahal-social-compute-panel>
    `);
    const items = el.shadowRoot!.querySelectorAll('.reciprocal-item');
    expect(items).to.have.lengthOf(2);
    expect(items[0].textContent).to.include('peer-delta');
    expect(items[0].textContent).to.include('Sunrise Collective household state');
    expect(items[1].textContent).to.include('peer-echo');
    expect(items[1].textContent).to.include('Mountain Node care-economy ledger');
  });

  it('renders recovery readiness with Shamir threshold', async () => {
    const el = await fixture<ElohimQahalSocialComputePanel>(html`
      <elohim-qahal-social-compute-panel
        .topology=${SAMPLE_TOPOLOGY}
      ></elohim-qahal-social-compute-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('Recovery readiness');
    expect(text).to.include('Shamir threshold');
    expect(text).to.include('2/3');
  });

  it('renders ready=true as "✓ ready"', async () => {
    const el = await fixture<ElohimQahalSocialComputePanel>(html`
      <elohim-qahal-social-compute-panel
        .topology=${SAMPLE_TOPOLOGY}
      ></elohim-qahal-social-compute-panel>
    `);
    const readiness = el.shadowRoot!.querySelector('.readiness');
    expect(readiness).to.exist;
    expect(readiness!.textContent).to.include('✓ ready');
  });

  it('renders ready=false as "⚠ not ready"', async () => {
    const notReadyTopology: ComputeTopology = {
      ...SAMPLE_TOPOLOGY,
      recoveryReadiness: { ...SAMPLE_TOPOLOGY.recoveryReadiness, ready: false },
    };
    const el = await fixture<ElohimQahalSocialComputePanel>(html`
      <elohim-qahal-social-compute-panel
        .topology=${notReadyTopology}
      ></elohim-qahal-social-compute-panel>
    `);
    const readiness = el.shadowRoot!.querySelector('.readiness');
    expect(readiness).to.exist;
    expect(readiness!.textContent).to.include('⚠ not ready');
  });

  it('renders active-ratio header with checkmark only when all stewards are healthy', async () => {
    // SAMPLE_TOPOLOGY has mixed health — no checkmark
    const el = await fixture<ElohimQahalSocialComputePanel>(html`
      <elohim-qahal-social-compute-panel
        .topology=${SAMPLE_TOPOLOGY}
      ></elohim-qahal-social-compute-panel>
    `);
    const ratio = el.shadowRoot!.querySelector('.active-ratio');
    expect(ratio).to.exist;
    expect(ratio!.textContent).to.include('1-of-3 active');
    expect(ratio!.textContent).to.not.include('✓');

    // All-healthy topology should include checkmark
    const allHealthy: ComputeTopology = {
      ...SAMPLE_TOPOLOGY,
      stewardsForUs: SAMPLE_TOPOLOGY.stewardsForUs.map(s => ({ ...s, health: 'healthy' as const })),
    };
    const el2 = await fixture<ElohimQahalSocialComputePanel>(html`
      <elohim-qahal-social-compute-panel
        .topology=${allHealthy}
      ></elohim-qahal-social-compute-panel>
    `);
    const ratio2 = el2.shadowRoot!.querySelector('.active-ratio');
    expect(ratio2).to.exist;
    expect(ratio2!.textContent).to.include('3-of-3 active');
    expect(ratio2!.textContent).to.include('✓');
  });

  it('renders empty stewardsForUs without crash — header present, no list items', async () => {
    const emptyTopology: ComputeTopology = { ...SAMPLE_TOPOLOGY, stewardsForUs: [] };
    const el = await fixture<ElohimQahalSocialComputePanel>(html`
      <elohim-qahal-social-compute-panel
        .topology=${emptyTopology}
      ></elohim-qahal-social-compute-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('Compute-stewards FOR us');
    const items = el.shadowRoot!.querySelectorAll('.steward-item');
    expect(items).to.have.lengthOf(0);
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalSocialComputePanel>(html`
      <elohim-qahal-social-compute-panel
        .topology=${SAMPLE_TOPOLOGY}
      ></elohim-qahal-social-compute-panel>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-social-compute-panel> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (panel is still; no motion to gate)', () => {
    const cssText = (
      ElohimQahalSocialComputePanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalSocialComputePanel>(html`
      <elohim-qahal-social-compute-panel
        .topology=${SAMPLE_TOPOLOGY}
      ></elohim-qahal-social-compute-panel>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-social-compute-panel> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalSocialComputePanel>(
      'he-IL',
      html`
        <elohim-qahal-social-compute-panel
          .topology=${SAMPLE_TOPOLOGY}
        ></elohim-qahal-social-compute-panel>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const section = el.shadowRoot!.querySelector('section')!;
    const rect = section.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimQahalSocialComputePanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
