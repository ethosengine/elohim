import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import type { ElohimFeedbackMechanismGateway } from './elohim-feedback-mechanism-gateway.js';
import { ElohimFeedbackMechanismGateway as GatewayClass } from './elohim-feedback-mechanism-gateway.js';
import { requiresLogicalProperties } from './testing/i18n.js';
import {
  assertThemeContrast,
  axeScanStrict,
  themeFixture,
  type ThemeCell,
} from './testing/theme-contrast.js';

describe('<elohim-feedback-mechanism-gateway>', () => {
  it('is defined in the custom element registry', () => {
    expect(customElements.get('elohim-feedback-mechanism-gateway')).to.equal(GatewayClass);
  });

  it('shows loading state when loading=true', async () => {
    const el = await fixture<ElohimFeedbackMechanismGateway>(html`
      <elohim-feedback-mechanism-gateway loading></elohim-feedback-mechanism-gateway>
    `);
    const loading = el.shadowRoot?.querySelector('[data-testid="gateway-loading"]');
    expect(loading).to.exist;
    expect(loading?.textContent).to.contain('Loading governance');
  });

  it('shows loading state when selection is null', async () => {
    const el = await fixture<ElohimFeedbackMechanismGateway>(html`
      <elohim-feedback-mechanism-gateway></elohim-feedback-mechanism-gateway>
    `);
    const loading = el.shadowRoot?.querySelector('[data-testid="gateway-loading"]');
    expect(loading).to.exist;
  });

  it('renders reactions slot for level 1 angular selection', async () => {
    const el = await fixture<ElohimFeedbackMechanismGateway>(html`
      <elohim-feedback-mechanism-gateway .selection=${{ level: 1, renderTarget: 'angular' }}>
        <div slot="reactions" data-testid="reactions-slot-content">Reactions</div>
      </elohim-feedback-mechanism-gateway>
    `);

    const slot = el.shadowRoot?.querySelector('slot[name="reactions"]');
    expect(slot).to.exist;
  });

  it('renders context-menu slot for level 0 angular selection', async () => {
    const el = await fixture<ElohimFeedbackMechanismGateway>(html`
      <elohim-feedback-mechanism-gateway .selection=${{ level: 0, renderTarget: 'angular' }}>
        <div slot="context-menu">Menu</div>
      </elohim-feedback-mechanism-gateway>
    `);
    const slot = el.shadowRoot?.querySelector('slot[name="context-menu"]');
    expect(slot).to.exist;
  });

  it('renders psephos slot for psephos render target', async () => {
    const el = await fixture<ElohimFeedbackMechanismGateway>(html`
      <elohim-feedback-mechanism-gateway .selection=${{ level: 3, renderTarget: 'psephos' }}>
        <div slot="psephos">Ballot</div>
      </elohim-feedback-mechanism-gateway>
    `);
    const slot = el.shadowRoot?.querySelector('slot[name="psephos"]');
    expect(slot).to.exist;
  });

  it('shows sensemaking badge when readyForSensemaking=true', async () => {
    const el = await fixture<ElohimFeedbackMechanismGateway>(html`
      <elohim-feedback-mechanism-gateway
        .selection=${{ level: 1, renderTarget: 'angular' }}
        .accumulationStatus=${{
          readyForSensemaking: true,
          controversyDetected: false,
          settled: false,
        }}
      ></elohim-feedback-mechanism-gateway>
    `);
    const badge = el.shadowRoot?.querySelector('[data-testid="sensemaking-link"]');
    expect(badge).to.exist;
  });

  it('dispatches sensemaking-navigate event when sensemaking badge is clicked', async () => {
    const el = await fixture<ElohimFeedbackMechanismGateway>(html`
      <elohim-feedback-mechanism-gateway
        entity-type="content"
        entity-id="test-id"
        .selection=${{ level: 1, renderTarget: 'angular' }}
        .accumulationStatus=${{
          readyForSensemaking: true,
          controversyDetected: false,
          settled: false,
        }}
      ></elohim-feedback-mechanism-gateway>
    `);

    let navigated = false;
    el.addEventListener('sensemaking-navigate', () => {
      navigated = true;
    });

    const badge = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="sensemaking-link"]'
    );
    badge?.click();

    expect(navigated).to.be.true;
  });

  it('calls onNavigateToSensemaking callback when provided', async () => {
    let called = false;
    const callback = () => {
      called = true;
    };

    const el = await fixture<ElohimFeedbackMechanismGateway>(html`
      <elohim-feedback-mechanism-gateway
        .selection=${{ level: 1, renderTarget: 'angular' }}
        .accumulationStatus=${{
          readyForSensemaking: true,
          controversyDetected: false,
          settled: false,
        }}
        .onNavigateToSensemaking=${callback}
      ></elohim-feedback-mechanism-gateway>
    `);

    const badge = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="sensemaking-link"]'
    );
    badge?.click();

    expect(called).to.be.true;
  });

  it('passes axe-core a11y scan in loading state', async () => {
    const el = await fixture<ElohimFeedbackMechanismGateway>(html`
      <elohim-feedback-mechanism-gateway loading></elohim-feedback-mechanism-gateway>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-feedback-mechanism-gateway> — i18n gate', () => {
  it('uses only logical CSS properties', () => {
    const cssText = (GatewayClass as { styles: { cssText: string } }).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-feedback-mechanism-gateway> — ua-prefs gate', () => {
  it('forced-colors overrides present', () => {
    const cssText = (GatewayClass as { styles: { cssText: string } }).styles.cssText;
    expect(cssText).to.contain('forced-colors: active');
  });
});

describe('<elohim-feedback-mechanism-gateway> — theme-contrast gate', () => {
  // tokens cells: no shipped binding for this element yet — system cells only
  // (theme-authority spec §4.1). The blank-slate contract per scheme.
  const CELLS: ThemeCell[] = ['system-light', 'system-dark'];

  // All three badge states (sensemaking / controversy / settled) render at once
  // when every accumulation flag is set — cheapest path to the richest text.
  const selection = { level: 1, renderTarget: 'angular' } as const;
  const accumulationStatus = {
    readyForSensemaking: true,
    controversyDetected: true,
    settled: true,
  } as const;

  for (const cell of CELLS) {
    it(`passes contrast in ${cell}`, async () => {
      const { el } = await themeFixture<ElohimFeedbackMechanismGateway>(
        html`
          <elohim-feedback-mechanism-gateway
            entity-type="content"
            entity-id="test-id"
            .selection=${selection}
            .accumulationStatus=${accumulationStatus}
          ></elohim-feedback-mechanism-gateway>
        `,
        cell
      );
      await el.updateComplete;
      assertThemeContrast(el);
      await axeScanStrict(el);
    });
  }
});
