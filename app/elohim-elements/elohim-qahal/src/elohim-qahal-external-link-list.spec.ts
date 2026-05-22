import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalExternalLinkList as ElohimQahalExternalLinkListClass } from './elohim-qahal-external-link-list.js';
import type {
  ExternalLink,
  ExternalLinkVisibilityMap,
  ElohimQahalExternalLinkList,
} from './elohim-qahal-external-link-list.js';

const SAMPLE_LINKS: ExternalLink[] = [
  { id: 'link-a', url: 'https://example.com/a', label: 'Example A' },
  { id: 'link-b', url: 'https://example.com/b', label: 'Example B' },
];

const VISIBILITY_MAP: ExternalLinkVisibilityMap = {
  child: 'hidden',
  visitor: 'filtered_via_co_steward',
  steward: 'full',
};

describe('<elohim-qahal-external-link-list>', () => {
  it('hides links when viewerTier maps to hidden', async () => {
    const el = await fixture<ElohimQahalExternalLinkList>(html`
      <elohim-qahal-external-link-list
        .links=${SAMPLE_LINKS}
        .visibilityMap=${VISIBILITY_MAP}
        viewer-tier="child"
      ></elohim-qahal-external-link-list>
    `);
    const anchors = el.shadowRoot!.querySelectorAll('a');
    expect(anchors).to.have.lengthOf(0);
    const note = el.shadowRoot!.querySelector('.hidden-note');
    expect(note).to.exist;
  });

  it('renders links fully when viewerTier maps to full', async () => {
    const el = await fixture<ElohimQahalExternalLinkList>(html`
      <elohim-qahal-external-link-list
        .links=${SAMPLE_LINKS}
        .visibilityMap=${VISIBILITY_MAP}
        viewer-tier="steward"
      ></elohim-qahal-external-link-list>
    `);
    const anchors = el.shadowRoot!.querySelectorAll('a');
    expect(anchors).to.have.lengthOf(2);
  });

  it('renders filtered-via-co-steward annotation when visibility is filtered_via_co_steward', async () => {
    const el = await fixture<ElohimQahalExternalLinkList>(html`
      <elohim-qahal-external-link-list
        .links=${SAMPLE_LINKS}
        .visibilityMap=${VISIBILITY_MAP}
        viewer-tier="visitor"
      ></elohim-qahal-external-link-list>
    `);
    const annotation = el.shadowRoot!.querySelector('.filtered-annotation');
    expect(annotation).to.exist;
    expect(annotation!.textContent).to.include('co-steward');
  });

  it('renders links inside details when filtered_via_co_steward', async () => {
    const el = await fixture<ElohimQahalExternalLinkList>(html`
      <elohim-qahal-external-link-list
        .links=${SAMPLE_LINKS}
        .visibilityMap=${VISIBILITY_MAP}
        viewer-tier="visitor"
      ></elohim-qahal-external-link-list>
    `);
    const details = el.shadowRoot!.querySelector('details');
    expect(details).to.exist;
    const anchors = details!.querySelectorAll('a');
    expect(anchors).to.have.lengthOf(2);
  });

  it('all visible links are rendered as anchors', async () => {
    const el = await fixture<ElohimQahalExternalLinkList>(html`
      <elohim-qahal-external-link-list
        .links=${SAMPLE_LINKS}
        .visibilityMap=${VISIBILITY_MAP}
        viewer-tier="steward"
      ></elohim-qahal-external-link-list>
    `);
    const anchors = el.shadowRoot!.querySelectorAll('a[href]');
    expect(anchors).to.have.lengthOf(2);
    expect(anchors[0].getAttribute('href')).to.equal('https://example.com/a');
    expect(anchors[1].getAttribute('href')).to.equal('https://example.com/b');
  });

  it('anchors have target="_blank" and rel="noopener noreferrer"', async () => {
    const el = await fixture<ElohimQahalExternalLinkList>(html`
      <elohim-qahal-external-link-list
        .links=${SAMPLE_LINKS}
        .visibilityMap=${VISIBILITY_MAP}
        viewer-tier="steward"
      ></elohim-qahal-external-link-list>
    `);
    const anchors = el.shadowRoot!.querySelectorAll('a');
    for (const anchor of Array.from(anchors)) {
      expect(anchor.getAttribute('target')).to.equal('_blank');
      expect(anchor.getAttribute('rel')).to.equal('noopener noreferrer');
    }
  });

  it('each rendered link has a ⤤ provenance marker', async () => {
    const el = await fixture<ElohimQahalExternalLinkList>(html`
      <elohim-qahal-external-link-list
        .links=${SAMPLE_LINKS}
        .visibilityMap=${VISIBILITY_MAP}
        viewer-tier="steward"
      ></elohim-qahal-external-link-list>
    `);
    const markers = el.shadowRoot!.querySelectorAll(
      'elohim-qahal-provenance-marker[category="external-hyperlink"]'
    );
    expect(markers).to.have.lengthOf(2);
    for (const marker of Array.from(markers)) {
      expect(marker.shadowRoot!.textContent).to.include('⤤');
    }
  });

  it('markers carry offline attribute when offline is true', async () => {
    const el = await fixture<ElohimQahalExternalLinkList>(html`
      <elohim-qahal-external-link-list
        .links=${SAMPLE_LINKS}
        .visibilityMap=${VISIBILITY_MAP}
        viewer-tier="steward"
        ?offline=${true}
      ></elohim-qahal-external-link-list>
    `);
    const markers = el.shadowRoot!.querySelectorAll('elohim-qahal-provenance-marker');
    expect(markers.length).to.be.greaterThan(0);
    for (const marker of Array.from(markers)) {
      expect(marker.hasAttribute('offline')).to.be.true;
    }
  });

  it('markers do not carry offline attribute when online', async () => {
    const el = await fixture<ElohimQahalExternalLinkList>(html`
      <elohim-qahal-external-link-list
        .links=${SAMPLE_LINKS}
        .visibilityMap=${VISIBILITY_MAP}
        viewer-tier="steward"
      ></elohim-qahal-external-link-list>
    `);
    const markers = el.shadowRoot!.querySelectorAll('elohim-qahal-provenance-marker');
    for (const marker of Array.from(markers)) {
      expect(marker.hasAttribute('offline')).to.be.false;
    }
  });

  it('renders empty state when links array is empty and tier is full', async () => {
    const el = await fixture<ElohimQahalExternalLinkList>(html`
      <elohim-qahal-external-link-list
        .links=${[]}
        .visibilityMap=${VISIBILITY_MAP}
        viewer-tier="steward"
      ></elohim-qahal-external-link-list>
    `);
    const note = el.shadowRoot!.querySelector('.empty-note');
    expect(note).to.exist;
  });

  it('passes axe accessibility audit (full visibility)', async () => {
    const el = await fixture<ElohimQahalExternalLinkList>(html`
      <elohim-qahal-external-link-list
        .links=${SAMPLE_LINKS}
        .visibilityMap=${VISIBILITY_MAP}
        viewer-tier="steward"
      ></elohim-qahal-external-link-list>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe accessibility audit (hidden)', async () => {
    const el = await fixture<ElohimQahalExternalLinkList>(html`
      <elohim-qahal-external-link-list
        .links=${SAMPLE_LINKS}
        .visibilityMap=${VISIBILITY_MAP}
        viewer-tier="child"
      ></elohim-qahal-external-link-list>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-external-link-list> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (link list is still; no motion to gate)', () => {
    const cssText = (
      ElohimQahalExternalLinkListClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalExternalLinkList>(html`
      <elohim-qahal-external-link-list
        .links=${SAMPLE_LINKS}
        .visibilityMap=${VISIBILITY_MAP}
        viewer-tier="steward"
      ></elohim-qahal-external-link-list>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-external-link-list> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalExternalLinkList>(
      'he-IL',
      html`
        <elohim-qahal-external-link-list
          .links=${SAMPLE_LINKS}
          .visibilityMap=${VISIBILITY_MAP}
          viewer-tier="steward"
        ></elohim-qahal-external-link-list>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const ul = el.shadowRoot!.querySelector('ul')!;
    const rect = ul.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimQahalExternalLinkListClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
