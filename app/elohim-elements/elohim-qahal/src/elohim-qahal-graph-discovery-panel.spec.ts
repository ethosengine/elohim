import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalGraphDiscoveryPanel as ElohimQahalGraphDiscoveryPanelClass } from './elohim-qahal-graph-discovery-panel.js';
import type {
  DiscoverySuggestion,
  ElohimQahalGraphDiscoveryPanel,
} from './elohim-qahal-graph-discovery-panel.js';

const SAMPLE_SUGGESTIONS: DiscoverySuggestion[] = [
  { id: 'qahal-sunrise', kind: 'adjacent-qahal', label: 'Sunrise Collective', relevance: 0.91 },
  {
    id: 'qahal-mountain',
    kind: 'federation-candidate',
    label: 'Mountain Node',
    relevance: 0.75,
  },
  {
    id: 'peer-jordi',
    kind: 'contributor-presence',
    label: 'Jordi (contributor at Sunrise)',
    relevance: 0.62,
  },
];

describe('<elohim-qahal-graph-discovery-panel>', () => {
  it('renders an h2 title', async () => {
    const el = await fixture<ElohimQahalGraphDiscoveryPanel>(html`
      <elohim-qahal-graph-discovery-panel
        .suggestions=${SAMPLE_SUGGESTIONS}
      ></elohim-qahal-graph-discovery-panel>
    `);
    const h2 = el.shadowRoot!.querySelector('h2');
    expect(h2).to.exist;
    expect(h2!.textContent).to.include('Graph Discovery');
  });

  it('renders one list item per suggestion', async () => {
    const el = await fixture<ElohimQahalGraphDiscoveryPanel>(html`
      <elohim-qahal-graph-discovery-panel
        .suggestions=${SAMPLE_SUGGESTIONS}
      ></elohim-qahal-graph-discovery-panel>
    `);
    const items = el.shadowRoot!.querySelectorAll('.suggestion-item');
    expect(items).to.have.lengthOf(3);
  });

  it('renders kind label for each suggestion', async () => {
    const el = await fixture<ElohimQahalGraphDiscoveryPanel>(html`
      <elohim-qahal-graph-discovery-panel
        .suggestions=${SAMPLE_SUGGESTIONS}
      ></elohim-qahal-graph-discovery-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('Adjacent Qahal');
    expect(text).to.include('Federation candidate');
    expect(text).to.include('Contributor presence');
  });

  it('renders suggestion labels', async () => {
    const el = await fixture<ElohimQahalGraphDiscoveryPanel>(html`
      <elohim-qahal-graph-discovery-panel
        .suggestions=${SAMPLE_SUGGESTIONS}
      ></elohim-qahal-graph-discovery-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('Sunrise Collective');
    expect(text).to.include('Mountain Node');
    expect(text).to.include('Jordi (contributor at Sunrise)');
  });

  it('renders relevance as a percentage', async () => {
    const el = await fixture<ElohimQahalGraphDiscoveryPanel>(html`
      <elohim-qahal-graph-discovery-panel
        .suggestions=${SAMPLE_SUGGESTIONS}
      ></elohim-qahal-graph-discovery-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('91%');
    expect(text).to.include('75%');
    expect(text).to.include('62%');
  });

  it('renders a .stub-note explaining the MVP deferral', async () => {
    const el = await fixture<ElohimQahalGraphDiscoveryPanel>(html`
      <elohim-qahal-graph-discovery-panel
        .suggestions=${SAMPLE_SUGGESTIONS}
      ></elohim-qahal-graph-discovery-panel>
    `);
    const note = el.shadowRoot!.querySelector('.stub-note');
    expect(note).to.exist;
    expect(note!.textContent).to.include('Visual stub at MVP');
    expect(note!.textContent).to.include('graph-walk discovery');
  });

  it('renders empty state without crash when suggestions is empty', async () => {
    const el = await fixture<ElohimQahalGraphDiscoveryPanel>(html`
      <elohim-qahal-graph-discovery-panel .suggestions=${[]}></elohim-qahal-graph-discovery-panel>
    `);
    const items = el.shadowRoot!.querySelectorAll('.suggestion-item');
    expect(items).to.have.lengthOf(0);
    const emptyNote = el.shadowRoot!.querySelector('.empty-note');
    expect(emptyNote).to.exist;
  });

  it('marks each item with data-kind attribute', async () => {
    const el = await fixture<ElohimQahalGraphDiscoveryPanel>(html`
      <elohim-qahal-graph-discovery-panel
        .suggestions=${SAMPLE_SUGGESTIONS}
      ></elohim-qahal-graph-discovery-panel>
    `);
    const adjacentItem = el.shadowRoot!.querySelector('[data-kind="adjacent-qahal"]');
    const fedItem = el.shadowRoot!.querySelector('[data-kind="federation-candidate"]');
    expect(adjacentItem).to.exist;
    expect(fedItem).to.exist;
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalGraphDiscoveryPanel>(html`
      <elohim-qahal-graph-discovery-panel
        .suggestions=${SAMPLE_SUGGESTIONS}
      ></elohim-qahal-graph-discovery-panel>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-graph-discovery-panel> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (panel is still; no motion to gate)', () => {
    const cssText = (
      ElohimQahalGraphDiscoveryPanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalGraphDiscoveryPanel>(html`
      <elohim-qahal-graph-discovery-panel
        .suggestions=${SAMPLE_SUGGESTIONS}
      ></elohim-qahal-graph-discovery-panel>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-graph-discovery-panel> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalGraphDiscoveryPanel>(
      'he-IL',
      html`
        <elohim-qahal-graph-discovery-panel
          .suggestions=${SAMPLE_SUGGESTIONS}
        ></elohim-qahal-graph-discovery-panel>
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
      ElohimQahalGraphDiscoveryPanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
