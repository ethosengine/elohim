import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimImagodeiIntrospectionPanel as ElohimImagodeiIntrospectionPanelClass } from './elohim-imagodei-introspection-panel.js';
import type {
  AffordanceTrace,
  ElohimImagodeiIntrospectionPanel,
  SubjectSetting,
} from './elohim-imagodei-introspection-panel.js';

const SAMPLE_SETTINGS: SubjectSetting[] = [
  { name: 'Power-user view', value: 'simple', source: 'self' },
  { name: 'External link visibility', value: 'full', source: 'steward' },
  { name: 'Notification volume/style', value: 'gentle', source: 'self' },
];

const SAMPLE_AFFORDANCES: AffordanceTrace[] = [
  { name: 'edit-settings', visible: true, whyTrace: 'viewerCanEdit=true; standing=contributor' },
  {
    name: 'steward-configure',
    visible: false,
    whyTrace: 'viewerId===targetId; self-configuration mode',
  },
];

describe('<elohim-imagodei-introspection-panel>', () => {
  it('renders the subject id in the heading', async () => {
    const el = await fixture<ElohimImagodeiIntrospectionPanel>(html`
      <elohim-imagodei-introspection-panel
        subject-id="user-alice"
        subject-tier="contributor"
        .subjectSettings=${SAMPLE_SETTINGS}
        .sampleAffordances=${SAMPLE_AFFORDANCES}
      ></elohim-imagodei-introspection-panel>
    `);
    expect(el.shadowRoot!.textContent).to.include('user-alice');
  });

  it('renders the capability tier', async () => {
    const el = await fixture<ElohimImagodeiIntrospectionPanel>(html`
      <elohim-imagodei-introspection-panel
        subject-id="user-alice"
        subject-tier="contributor"
        .subjectSettings=${SAMPLE_SETTINGS}
        .sampleAffordances=${SAMPLE_AFFORDANCES}
      ></elohim-imagodei-introspection-panel>
    `);
    expect(el.shadowRoot!.textContent).to.include('contributor');
  });

  it('renders the settings table with all setting names', async () => {
    const el = await fixture<ElohimImagodeiIntrospectionPanel>(html`
      <elohim-imagodei-introspection-panel
        subject-id="user-alice"
        subject-tier="contributor"
        .subjectSettings=${SAMPLE_SETTINGS}
        .sampleAffordances=${SAMPLE_AFFORDANCES}
      ></elohim-imagodei-introspection-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('Power-user view');
    expect(text).to.include('External link visibility');
    expect(text).to.include('Notification volume/style');
  });

  it('renders the affordance trace table', async () => {
    const el = await fixture<ElohimImagodeiIntrospectionPanel>(html`
      <elohim-imagodei-introspection-panel
        subject-id="user-alice"
        subject-tier="contributor"
        .subjectSettings=${SAMPLE_SETTINGS}
        .sampleAffordances=${SAMPLE_AFFORDANCES}
      ></elohim-imagodei-introspection-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('edit-settings');
    expect(text).to.include('steward-configure');
    expect(text).to.include('viewerCanEdit=true');
  });

  it('shows visible/hidden status for affordances', async () => {
    const el = await fixture<ElohimImagodeiIntrospectionPanel>(html`
      <elohim-imagodei-introspection-panel
        subject-id="user-alice"
        subject-tier="contributor"
        .subjectSettings=${SAMPLE_SETTINGS}
        .sampleAffordances=${SAMPLE_AFFORDANCES}
      ></elohim-imagodei-introspection-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('visible');
    expect(text).to.include('hidden');
  });

  it('shows the protected-tier marker when subjectProtectedTier is set', async () => {
    const el = await fixture<ElohimImagodeiIntrospectionPanel>(html`
      <elohim-imagodei-introspection-panel
        subject-id="user-alice"
        subject-tier="child"
        subject-protected-tier="child"
        .subjectSettings=${SAMPLE_SETTINGS}
        .sampleAffordances=${SAMPLE_AFFORDANCES}
      ></elohim-imagodei-introspection-panel>
    `);
    const marker = el.shadowRoot!.querySelector('elohim-imagodei-protected-tier-marker');
    expect(marker).to.exist;
  });

  it('does NOT show protected-tier marker when subjectProtectedTier is not set', async () => {
    const el = await fixture<ElohimImagodeiIntrospectionPanel>(html`
      <elohim-imagodei-introspection-panel
        subject-id="user-alice"
        subject-tier="contributor"
        .subjectSettings=${SAMPLE_SETTINGS}
        .sampleAffordances=${SAMPLE_AFFORDANCES}
      ></elohim-imagodei-introspection-panel>
    `);
    const marker = el.shadowRoot!.querySelector('elohim-imagodei-protected-tier-marker');
    expect(marker).to.be.null;
  });

  it('renders the stub note', async () => {
    const el = await fixture<ElohimImagodeiIntrospectionPanel>(html`
      <elohim-imagodei-introspection-panel
        subject-id="user-alice"
        subject-tier="contributor"
        .subjectSettings=${SAMPLE_SETTINGS}
        .sampleAffordances=${SAMPLE_AFFORDANCES}
      ></elohim-imagodei-introspection-panel>
    `);
    const stub = el.shadowRoot!.querySelector('.stub-note');
    expect(stub).to.exist;
    expect(stub!.textContent).to.include('Visual stub at MVP');
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimImagodeiIntrospectionPanel>(html`
      <elohim-imagodei-introspection-panel
        subject-id="user-alice"
        subject-tier="contributor"
        .subjectSettings=${SAMPLE_SETTINGS}
        .sampleAffordances=${SAMPLE_AFFORDANCES}
      ></elohim-imagodei-introspection-panel>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-imagodei-introspection-panel> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (panel is still; no motion to gate)', () => {
    const cssText = (
      ElohimImagodeiIntrospectionPanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimImagodeiIntrospectionPanel>(html`
      <elohim-imagodei-introspection-panel
        subject-id="user-alice"
        subject-tier="contributor"
        .subjectSettings=${SAMPLE_SETTINGS}
        .sampleAffordances=${SAMPLE_AFFORDANCES}
      ></elohim-imagodei-introspection-panel>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-imagodei-introspection-panel> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimImagodeiIntrospectionPanel>(
      'he-IL',
      html`
        <elohim-imagodei-introspection-panel
          subject-id="user-alice"
          subject-tier="contributor"
          .subjectSettings=${SAMPLE_SETTINGS}
          .sampleAffordances=${SAMPLE_AFFORDANCES}
        ></elohim-imagodei-introspection-panel>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const h2 = el.shadowRoot!.querySelector('h2')!;
    const rect = h2.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimImagodeiIntrospectionPanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
