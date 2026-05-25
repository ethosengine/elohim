import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import type { ElohimEprRelationshipsPanel } from './elohim-epr-relationships-panel.js';
import { ElohimEprRelationshipsPanel as ElohimEprRelationshipsPanelClass } from './elohim-epr-relationships-panel.js';
import { requiresLogicalProperties } from './testing/i18n.js';

const SAMPLE_RELATIONSHIPS = [
  { type: 'PREREQUISITE', target: 'epr:foundations-intro' },
  { type: 'TEACHES', target: 'epr:advanced-patterns' },
  { type: 'REFERENCES', target: 'epr:external-ref' },
];

describe('<elohim-epr-relationships-panel>', () => {
  it('is defined in the custom element registry', () => {
    expect(customElements.get('elohim-epr-relationships-panel')).to.equal(
      ElohimEprRelationshipsPanelClass,
    );
  });

  it('renders nothing when relationships array is empty', async () => {
    const el = await fixture<ElohimEprRelationshipsPanel>(
      html`<elohim-epr-relationships-panel></elohim-epr-relationships-panel>`,
    );
    expect(el.shadowRoot?.querySelector('[data-testid="epr-relationships-panel"]')).to.be.null;
  });

  it('renders groups when relationships are provided', async () => {
    const el = await fixture<ElohimEprRelationshipsPanel>(
      html`<elohim-epr-relationships-panel
        .relationships=${SAMPLE_RELATIONSHIPS}
      ></elohim-epr-relationships-panel>`,
    );
    const panel = el.shadowRoot?.querySelector('[data-testid="epr-relationships-panel"]');
    expect(panel).to.exist;
    const groups = el.shadowRoot?.querySelectorAll('[data-testid="epr-rel-group"]');
    expect(groups?.length).to.equal(3);
  });

  it('orders known relationship types before unknown ones', async () => {
    const rels = [
      { type: 'UNKNOWN_TYPE', target: 'epr:a' },
      { type: 'PREREQUISITE', target: 'epr:b' },
      { type: 'TEACHES', target: 'epr:c' },
    ];
    const el = await fixture<ElohimEprRelationshipsPanel>(
      html`<elohim-epr-relationships-panel
        .relationships=${rels}
      ></elohim-epr-relationships-panel>`,
    );
    const groups = el.shadowRoot?.querySelectorAll('[data-testid="epr-rel-group"]');
    expect(groups?.[0]?.getAttribute('data-type')).to.equal('PREREQUISITE');
    expect(groups?.[1]?.getAttribute('data-type')).to.equal('TEACHES');
    expect(groups?.[2]?.getAttribute('data-type')).to.equal('UNKNOWN_TYPE');
  });

  it('renders relationship cards with correct target labels', async () => {
    const el = await fixture<ElohimEprRelationshipsPanel>(
      html`<elohim-epr-relationships-panel
        .relationships=${[{ type: 'PREREQUISITE', target: 'epr:intro' }]}
      ></elohim-epr-relationships-panel>`,
    );
    const card = el.shadowRoot?.querySelector('[data-testid="epr-relationship-card"]');
    expect(card?.textContent).to.contain('epr:intro');
  });

  it('dispatches epr-navigate event when card is clicked', async () => {
    const el = await fixture<ElohimEprRelationshipsPanel>(
      html`<elohim-epr-relationships-panel
        .relationships=${[{ type: 'PREREQUISITE', target: 'epr:intro' }]}
      ></elohim-epr-relationships-panel>`,
    );

    let navigatedTarget: string | null = null;
    el.addEventListener('epr-navigate', (e: Event) => {
      navigatedTarget = (e as CustomEvent<{ target: string }>).detail.target;
    });

    const card = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="epr-relationship-card"]',
    );
    card?.click();

    expect(navigatedTarget).to.equal('epr:intro');
  });

  it('calls navigator callback instead of dispatching event when navigator is set', async () => {
    let callbackTarget: string | null = null;
    const navigator = (t: string) => { callbackTarget = t; };

    const el = await fixture<ElohimEprRelationshipsPanel>(
      html`<elohim-epr-relationships-panel
        .relationships=${[{ type: 'PREREQUISITE', target: 'epr:intro' }]}
        .navigator=${navigator}
      ></elohim-epr-relationships-panel>`,
    );

    const card = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="epr-relationship-card"]',
    );
    card?.click();

    expect(callbackTarget).to.equal('epr:intro');
  });

  it('reacts to relationships property changes', async () => {
    const el = await fixture<ElohimEprRelationshipsPanel>(
      html`<elohim-epr-relationships-panel></elohim-epr-relationships-panel>`,
    );
    expect(el.shadowRoot?.querySelector('[data-testid="epr-relationships-panel"]')).to.be.null;

    el.relationships = SAMPLE_RELATIONSHIPS;
    await elementUpdated(el);

    expect(el.shadowRoot?.querySelector('[data-testid="epr-relationships-panel"]')).to.exist;
  });

  it('passes axe-core a11y scan with relationships', async () => {
    const el = await fixture<ElohimEprRelationshipsPanel>(
      html`<elohim-epr-relationships-panel
        .relationships=${SAMPLE_RELATIONSHIPS}
      ></elohim-epr-relationships-panel>`,
    );
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-epr-relationships-panel> — i18n precondition gate', () => {
  it('uses only logical CSS properties (no physical margin/padding)', () => {
    const cssText = (
      ElohimEprRelationshipsPanelClass as { styles: { cssText: string } }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-epr-relationships-panel> — ua-prefs precondition gate', () => {
  it('CSS does not contain transitions or animations outside media guards', () => {
    const cssText = (
      ElohimEprRelationshipsPanelClass as { styles: { cssText: string } }
    ).styles.cssText;
    // No animations declared at all — still mode
    expect(cssText).to.not.contain('animation:');
    expect(cssText).to.not.contain('@keyframes');
  });

  it('forced-colors overrides use CSS system colors', () => {
    const cssText = (
      ElohimEprRelationshipsPanelClass as { styles: { cssText: string } }
    ).styles.cssText;
    expect(cssText).to.contain('forced-colors: active');
    const forcedIdx = cssText.indexOf('forced-colors: active');
    const afterForced = cssText.slice(forcedIdx);
    const hasSystemColor =
      afterForced.includes('ButtonFace') ||
      afterForced.includes('Canvas') ||
      afterForced.includes('CanvasText') ||
      afterForced.includes('Highlight');
    expect(hasSystemColor).to.be.true;
  });
});
