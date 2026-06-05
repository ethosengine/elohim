import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import type { ElohimContentAnalytics } from './elohim-content-analytics.js';
import { ElohimContentAnalytics as AnalyticsClass } from './elohim-content-analytics.js';
import type { ContentAnalyticsLoader, ContentAnalyticsMetrics } from './elohim-content-analytics.js';
import { requiresLogicalProperties } from './testing/i18n.js';
import {
  assertThemeContrast,
  axeScanStrict,
  themeFixture,
  type ThemeCell,
} from './testing/theme-contrast.js';

const FIXTURE_METRICS: ContentAnalyticsMetrics = {
  viewCount: 123,
  completionCount: 45,
  completionRate: 37,
};

describe('<elohim-content-analytics>', () => {
  it('is defined in the custom element registry', () => {
    expect(customElements.get('elohim-content-analytics')).to.equal(AnalyticsClass);
  });

  it('renders the container and title', async () => {
    const el = await fixture<ElohimContentAnalytics>(
      html`<elohim-content-analytics></elohim-content-analytics>`,
    );
    const container = el.shadowRoot?.querySelector('[data-testid="content-analytics"]');
    expect(container).to.exist;
  });

  it('shows loading state while fetching', async () => {
    let resolve!: (m: ContentAnalyticsMetrics | null) => void;
    const loader: ContentAnalyticsLoader = {
      load: () => new Promise(r => { resolve = r; }),
    };

    const el = await fixture<ElohimContentAnalytics>(
      html`<elohim-content-analytics
        content-id="test-node"
        .loader=${loader}
      ></elohim-content-analytics>`,
    );

    // Wait for loadAnalytics() to set isLoading=true and Lit to re-render
    await elementUpdated(el);

    const loading = el.shadowRoot?.querySelector('[data-testid="analytics-loading"]');
    expect(loading).to.exist;
    expect(loading?.textContent).to.contain('Loading');

    // Resolve to clean up
    resolve(null);
  });

  it('displays metrics when loader resolves', async () => {
    const loader: ContentAnalyticsLoader = {
      load: async () => FIXTURE_METRICS,
    };

    const el = await fixture<ElohimContentAnalytics>(
      html`<elohim-content-analytics
        content-id="test-node"
        .loader=${loader}
      ></elohim-content-analytics>`,
    );

    // Wait for first render + updated() to fire loadAnalytics()
    await elementUpdated(el);
    // Wait for the loader's async resolve and the subsequent Lit re-render
    await new Promise(r => setTimeout(r, 20));
    await elementUpdated(el);

    const views = el.shadowRoot?.querySelector('[data-testid="analytics-views"]');
    expect(views?.textContent).to.contain('123');

    const completions = el.shadowRoot?.querySelector('[data-testid="analytics-completions"]');
    expect(completions?.textContent).to.contain('45');

    const rate = el.shadowRoot?.querySelector('[data-testid="analytics-completion-rate"]');
    expect(rate?.textContent).to.contain('37');
  });

  it('shows error state when loader returns null', async () => {
    const loader: ContentAnalyticsLoader = {
      load: async () => null,
    };

    const el = await fixture<ElohimContentAnalytics>(
      html`<elohim-content-analytics
        content-id="test-node"
        .loader=${loader}
      ></elohim-content-analytics>`,
    );

    await new Promise(r => setTimeout(r, 10));
    await elementUpdated(el);

    const error = el.shadowRoot?.querySelector('[data-testid="analytics-error"]');
    expect(error).to.exist;
    expect(error?.textContent).to.contain('unavailable');
  });

  it('shows error state when loader rejects', async () => {
    const loader: ContentAnalyticsLoader = {
      load: async () => { throw new Error('network failure'); },
    };

    const el = await fixture<ElohimContentAnalytics>(
      html`<elohim-content-analytics
        content-id="test-node"
        .loader=${loader}
      ></elohim-content-analytics>`,
    );

    await new Promise(r => setTimeout(r, 10));
    await elementUpdated(el);

    const error = el.shadowRoot?.querySelector('[data-testid="analytics-error"]');
    expect(error).to.exist;
  });

  it('renders pre-loaded metrics when metrics prop is set (no loader call)', async () => {
    let loaderCalled = false;
    const loader: ContentAnalyticsLoader = {
      load: async () => { loaderCalled = true; return null; },
    };

    const el = await fixture<ElohimContentAnalytics>(
      html`<elohim-content-analytics
        content-id="test-node"
        .loader=${loader}
        .metrics=${FIXTURE_METRICS}
      ></elohim-content-analytics>`,
    );

    await elementUpdated(el);

    expect(loaderCalled).to.be.false;
    const views = el.shadowRoot?.querySelector('[data-testid="analytics-views"]');
    expect(views?.textContent).to.contain('123');
  });

  it('dispatches analytics-loaded event on successful load', async () => {
    // Use a deferred loader so we can attach the event listener before load resolves
    let loaderResolve!: (m: ContentAnalyticsMetrics | null) => void;
    const loader: ContentAnalyticsLoader = {
      load: () => new Promise(r => { loaderResolve = r; }),
    };

    const el = await fixture<ElohimContentAnalytics>(
      html`<elohim-content-analytics
        content-id="test-node"
        .loader=${loader}
      ></elohim-content-analytics>`,
    );

    // Attach event listener before resolving the loader
    let loadedMetrics: unknown = null;
    el.addEventListener('analytics-loaded', (e: Event) => {
      loadedMetrics = (e as CustomEvent).detail;
    });

    // Now resolve the loader
    loaderResolve(FIXTURE_METRICS);

    await new Promise(r => setTimeout(r, 20));
    await elementUpdated(el);

    expect(loadedMetrics).to.deep.equal(FIXTURE_METRICS);
  });

  it('dispatches analytics-error event on failure', async () => {
    // Use a deferred loader so we can attach the event listener before load resolves
    let loaderResolve!: (m: ContentAnalyticsMetrics | null) => void;
    const loader: ContentAnalyticsLoader = {
      load: () => new Promise(r => { loaderResolve = r; }),
    };

    const el = await fixture<ElohimContentAnalytics>(
      html`<elohim-content-analytics
        content-id="test-node"
        .loader=${loader}
      ></elohim-content-analytics>`,
    );

    let errorFired = false;
    el.addEventListener('analytics-error', () => { errorFired = true; });

    // Resolve with null to trigger error
    loaderResolve(null);

    await new Promise(r => setTimeout(r, 20));
    await elementUpdated(el);

    expect(errorFired).to.be.true;
  });

  it('shows the note text', async () => {
    const el = await fixture<ElohimContentAnalytics>(
      html`<elohim-content-analytics
        .metrics=${FIXTURE_METRICS}
      ></elohim-content-analytics>`,
    );

    const note = el.shadowRoot?.querySelector('[part="note"]');
    expect(note?.textContent).to.contain('protocol-native');
  });

  it('passes axe-core a11y scan in metrics state', async () => {
    const el = await fixture<ElohimContentAnalytics>(
      html`<elohim-content-analytics
        .metrics=${FIXTURE_METRICS}
      ></elohim-content-analytics>`,
    );
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe-core a11y scan in loading state', async () => {
    let resolve!: (m: ContentAnalyticsMetrics | null) => void;
    const loader: ContentAnalyticsLoader = {
      load: () => new Promise(r => { resolve = r; }),
    };

    const el = await fixture<ElohimContentAnalytics>(
      html`<elohim-content-analytics
        content-id="node"
        .loader=${loader}
      ></elohim-content-analytics>`,
    );

    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
    resolve(null);
  });
});

describe('<elohim-content-analytics> — i18n gate', () => {
  it('uses only logical CSS properties', () => {
    const cssText = (AnalyticsClass as { styles: { cssText: string } }).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-content-analytics> — ua-prefs gate', () => {
  it('no animations declared without media guard (still mode default)', () => {
    const cssText = (AnalyticsClass as { styles: { cssText: string } }).styles.cssText;
    expect(cssText).to.not.contain('@keyframes');
  });

  it('forced-colors overrides present', () => {
    const cssText = (AnalyticsClass as { styles: { cssText: string } }).styles.cssText;
    expect(cssText).to.contain('forced-colors: active');
    const idx = cssText.indexOf('forced-colors: active');
    const after = cssText.slice(idx);
    expect(after.includes('Canvas') || after.includes('CanvasText') || after.includes('GrayText')).to.be.true;
  });
});

describe('<elohim-content-analytics> — theme-contrast gate', () => {
  // tokens cells: no shipped binding for this element yet — system cells only
  // (theme-authority spec §4.1). The blank-slate contract per scheme.
  const CELLS: ThemeCell[] = ['system-light', 'system-dark'];

  for (const cell of CELLS) {
    it(`passes contrast in ${cell}`, async () => {
      const { el } = await themeFixture<ElohimContentAnalytics>(
        html`<elohim-content-analytics .metrics=${FIXTURE_METRICS}></elohim-content-analytics>`,
        cell,
      );
      await el.updateComplete;
      assertThemeContrast(el);
      await axeScanStrict(el);
    });
  }
});
