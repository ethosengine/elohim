import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalStreamPanel as ElohimQahalStreamPanelClass } from './elohim-qahal-stream-panel.js';
import type { ElohimQahalStreamPanel, StreamEvent } from './elohim-qahal-stream-panel.js';

const SAMPLE_EVENTS: StreamEvent[] = [
  {
    id: 'evt-1',
    authorId: 'alice',
    timestamp: '2026-05-21T10:00:00Z',
    content: 'Alice made a contribution.',
    rea: { kind: 'care', tokens: 3 },
    acknowledgmentPending: false,
  },
  {
    id: 'evt-2',
    authorId: 'bob',
    timestamp: '2026-05-21T11:00:00Z',
    content: 'Bob added some presence.',
    rea: { kind: 'presence', tokens: 1 },
    acknowledgmentPending: true,
  },
  {
    id: 'evt-3',
    authorId: 'commons-elohim',
    timestamp: '2026-05-21T12:00:00Z',
    content: 'The commons are steady.',
  },
];

describe('<elohim-qahal-stream-panel>', () => {
  it('renders a "Stream" h2 heading', async () => {
    const el = await fixture<ElohimQahalStreamPanel>(html`
      <elohim-qahal-stream-panel .events=${SAMPLE_EVENTS}></elohim-qahal-stream-panel>
    `);
    const h2 = el.shadowRoot!.querySelector('h2');
    expect(h2).to.exist;
    expect(h2!.textContent!.trim()).to.equal('Stream');
  });

  it('renders one list item per event', async () => {
    const el = await fixture<ElohimQahalStreamPanel>(html`
      <elohim-qahal-stream-panel .events=${SAMPLE_EVENTS}></elohim-qahal-stream-panel>
    `);
    const items = el.shadowRoot!.querySelectorAll('.stream-item');
    expect(items).to.have.lengthOf(3);
  });

  it('renders event content text in each item', async () => {
    const el = await fixture<ElohimQahalStreamPanel>(html`
      <elohim-qahal-stream-panel .events=${SAMPLE_EVENTS}></elohim-qahal-stream-panel>
    `);
    const content = el.shadowRoot!.textContent ?? '';
    expect(content).to.include('Alice made a contribution.');
    expect(content).to.include('Bob added some presence.');
    expect(content).to.include('The commons are steady.');
  });

  it('renders timestamp in a <time> element', async () => {
    const el = await fixture<ElohimQahalStreamPanel>(html`
      <elohim-qahal-stream-panel .events=${SAMPLE_EVENTS}></elohim-qahal-stream-panel>
    `);
    const times = el.shadowRoot!.querySelectorAll('time');
    expect(times.length).to.be.greaterThan(0);
    expect(times[0].getAttribute('datetime')).to.equal('2026-05-21T10:00:00Z');
  });

  it('composes <elohim-qahal-imagodei-badge> for each event author', async () => {
    const el = await fixture<ElohimQahalStreamPanel>(html`
      <elohim-qahal-stream-panel .events=${SAMPLE_EVENTS}></elohim-qahal-stream-panel>
    `);
    const badges = el.shadowRoot!.querySelectorAll('elohim-qahal-imagodei-badge');
    expect(badges.length).to.equal(3);
  });

  it('composes <elohim-qahal-care-economy-marker> for events with rea', async () => {
    const el = await fixture<ElohimQahalStreamPanel>(html`
      <elohim-qahal-stream-panel .events=${SAMPLE_EVENTS}></elohim-qahal-stream-panel>
    `);
    const markers = el.shadowRoot!.querySelectorAll('elohim-qahal-care-economy-marker');
    // Two events have rea (evt-1 and evt-2)
    expect(markers.length).to.equal(2);
  });

  it('sets data-acknowledgment-pending attribute on pending items', async () => {
    const el = await fixture<ElohimQahalStreamPanel>(html`
      <elohim-qahal-stream-panel .events=${SAMPLE_EVENTS}></elohim-qahal-stream-panel>
    `);
    const items = el.shadowRoot!.querySelectorAll('.stream-item');
    // evt-2 is pending (index 1)
    expect(items[1].hasAttribute('data-acknowledgment-pending')).to.be.true;
    // evt-1 and evt-3 are not pending
    expect(items[0].hasAttribute('data-acknowledgment-pending')).to.be.false;
    expect(items[2].hasAttribute('data-acknowledgment-pending')).to.be.false;
  });

  it('sets data-commons-elohim attribute on commons-elohim authored events', async () => {
    const el = await fixture<ElohimQahalStreamPanel>(html`
      <elohim-qahal-stream-panel .events=${SAMPLE_EVENTS}></elohim-qahal-stream-panel>
    `);
    const items = el.shadowRoot!.querySelectorAll('.stream-item');
    // evt-3 is authored by commons-elohim (index 2)
    expect(items[2].hasAttribute('data-commons-elohim')).to.be.true;
    expect(items[0].hasAttribute('data-commons-elohim')).to.be.false;
    expect(items[1].hasAttribute('data-commons-elohim')).to.be.false;
  });

  it('renders empty state when events array is empty', async () => {
    const el = await fixture<ElohimQahalStreamPanel>(html`
      <elohim-qahal-stream-panel .events=${[]}></elohim-qahal-stream-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('No activity yet — the household is quiet.');
    const items = el.shadowRoot!.querySelectorAll('.stream-item');
    expect(items).to.have.lengthOf(0);
  });

  it('renders empty state by default when no events prop is set', async () => {
    const el = await fixture<ElohimQahalStreamPanel>(html`
      <elohim-qahal-stream-panel></elohim-qahal-stream-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('No activity yet — the household is quiet.');
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalStreamPanel>(html`
      <elohim-qahal-stream-panel .events=${SAMPLE_EVENTS}></elohim-qahal-stream-panel>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-stream-panel> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (panel is still; no motion to gate)', () => {
    const cssText = (
      ElohimQahalStreamPanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalStreamPanel>(html`
      <elohim-qahal-stream-panel .events=${SAMPLE_EVENTS}></elohim-qahal-stream-panel>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-stream-panel> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalStreamPanel>(
      'he-IL',
      html`
        <elohim-qahal-stream-panel .events=${SAMPLE_EVENTS}></elohim-qahal-stream-panel>
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
      ElohimQahalStreamPanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
