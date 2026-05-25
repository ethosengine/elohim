import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import type { ElohimGraduatedFeedback } from './elohim-graduated-feedback.js';
import { ElohimGraduatedFeedback as ElohimGraduatedFeedbackClass } from './elohim-graduated-feedback.js';
import { requiresLogicalProperties } from './testing/i18n.js';

describe('<elohim-graduated-feedback>', () => {
  it('is defined in the custom element registry', () => {
    expect(customElements.get('elohim-graduated-feedback')).to.equal(ElohimGraduatedFeedbackClass);
  });

  it('renders the scale label for the default context (usefulness)', async () => {
    const el = await fixture<ElohimGraduatedFeedback>(
      html`<elohim-graduated-feedback></elohim-graduated-feedback>`,
    );
    const label = el.shadowRoot?.querySelector('.feedback-label');
    expect(label?.textContent).to.contain('useful');
  });

  it('renders correct scale positions for accuracy context', async () => {
    const el = await fixture<ElohimGraduatedFeedback>(
      html`<elohim-graduated-feedback context="accuracy"></elohim-graduated-feedback>`,
    );
    const falseBtn = el.shadowRoot?.querySelector(
      '[data-testid="position-false"]',
    );
    expect(falseBtn).to.exist;
    const accurateBtn = el.shadowRoot?.querySelector(
      '[data-testid="position-accurate"]',
    );
    expect(accurateBtn).to.exist;
  });

  it('shows intensity slider after position selection', async () => {
    const el = await fixture<ElohimGraduatedFeedback>(
      html`<elohim-graduated-feedback></elohim-graduated-feedback>`,
    );

    // No intensity section before selection
    expect(el.shadowRoot?.querySelector('[data-testid="intensity-slider"]')).to.be.null;

    const usefulBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="position-useful"]',
    );
    usefulBtn?.click();
    await elementUpdated(el);

    expect(el.shadowRoot?.querySelector('[data-testid="intensity-slider"]')).to.exist;
  });

  it('shows submit button after position selection', async () => {
    const el = await fixture<ElohimGraduatedFeedback>(
      html`<elohim-graduated-feedback></elohim-graduated-feedback>`,
    );

    expect(el.shadowRoot?.querySelector('[data-testid="submit-feedback-btn"]')).to.be.null;

    const usefulBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="position-useful"]',
    );
    usefulBtn?.click();
    await elementUpdated(el);

    expect(el.shadowRoot?.querySelector('[data-testid="submit-feedback-btn"]')).to.exist;
  });

  it('requires reasoning for negative positions (index <= 0.25)', async () => {
    const el = await fixture<ElohimGraduatedFeedback>(
      html`<elohim-graduated-feedback></elohim-graduated-feedback>`,
    );

    // "Not Useful" is index 0
    const notUsefulBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="position-not-useful"]',
    );
    notUsefulBtn?.click();
    await elementUpdated(el);

    const textarea = el.shadowRoot?.querySelector('[data-testid="reasoning-textarea"]');
    expect(textarea).to.exist;

    const submitBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="submit-feedback-btn"]',
    );
    expect(submitBtn?.hasAttribute('disabled')).to.be.true;
  });

  it('dispatches feedback-submit event on submit', async () => {
    const el = await fixture<ElohimGraduatedFeedback>(
      html`<elohim-graduated-feedback></elohim-graduated-feedback>`,
    );

    let submitted: unknown = null;
    el.addEventListener('feedback-submit', (e: Event) => {
      submitted = (e as CustomEvent).detail;
    });

    const usefulBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="position-useful"]',
    );
    usefulBtn?.click();
    await elementUpdated(el);

    const submitBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="submit-feedback-btn"]',
    );
    submitBtn?.click();

    expect(submitted).to.deep.include({ context: 'usefulness', position: 'Useful' });
  });

  it('calls onSubmit callback when provided', async () => {
    let callbackData: unknown = null;
    const onSubmit = (data: unknown) => { callbackData = data; };

    const el = await fixture<ElohimGraduatedFeedback>(
      html`<elohim-graduated-feedback .onSubmit=${onSubmit}></elohim-graduated-feedback>`,
    );

    const usefulBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="position-useful"]',
    );
    usefulBtn?.click();
    await elementUpdated(el);

    const submitBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="submit-feedback-btn"]',
    );
    submitBtn?.click();

    expect(callbackData).to.exist;
  });

  it('shows community distribution bar when distribution + totalResponses are set', async () => {
    const el = await fixture<ElohimGraduatedFeedback>(
      html`<elohim-graduated-feedback
        .distribution=${{ Useful: 3, Transformative: 1 }}
        .totalResponses=${4}
        .showAggregates=${true}
      ></elohim-graduated-feedback>`,
    );
    expect(el.shadowRoot?.querySelector('.distribution-bar')).to.exist;
  });

  it('resets form on reset button click', async () => {
    const el = await fixture<ElohimGraduatedFeedback>(
      html`<elohim-graduated-feedback .hasSubmitted=${true}></elohim-graduated-feedback>`,
    );

    const usefulBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="position-useful"]',
    );
    usefulBtn?.click();
    await elementUpdated(el);

    const resetBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="reset-feedback-btn"]',
    );
    resetBtn?.click();
    await elementUpdated(el);

    // After reset, intensity slider gone, no selection
    expect(el.shadowRoot?.querySelector('[data-testid="intensity-slider"]')).to.be.null;
  });

  it('passes axe-core a11y scan in default state', async () => {
    const el = await fixture<ElohimGraduatedFeedback>(
      html`<elohim-graduated-feedback></elohim-graduated-feedback>`,
    );
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-graduated-feedback> — i18n precondition gate', () => {
  it('uses only logical CSS properties', () => {
    const cssText = (ElohimGraduatedFeedbackClass as { styles: { cssText: string } }).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-graduated-feedback> — ua-prefs precondition gate', () => {
  it('no animations declared without media guard', () => {
    const cssText = (ElohimGraduatedFeedbackClass as { styles: { cssText: string } }).styles.cssText;
    expect(cssText).to.not.contain('@keyframes');
  });

  it('forced-colors overrides present', () => {
    const cssText = (ElohimGraduatedFeedbackClass as { styles: { cssText: string } }).styles.cssText;
    expect(cssText).to.contain('forced-colors: active');
    const idx = cssText.indexOf('forced-colors: active');
    const after = cssText.slice(idx);
    expect(after.includes('ButtonFace') || after.includes('Canvas') || after.includes('Highlight')).to.be.true;
  });

  it('coarse-pointer touch targets present for position buttons', () => {
    const cssText = (ElohimGraduatedFeedbackClass as { styles: { cssText: string } }).styles.cssText;
    expect(cssText).to.contain('pointer: coarse');
  });
});
