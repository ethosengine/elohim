import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import type { ElohimGateFeedbackTrigger } from './elohim-gate-feedback-trigger.js';
import { ElohimGateFeedbackTrigger as TriggerClass } from './elohim-gate-feedback-trigger.js';
import { requiresLogicalProperties } from './testing/i18n.js';

describe('<elohim-gate-feedback-trigger>', () => {
  it('is defined in the custom element registry', () => {
    expect(customElements.get('elohim-gate-feedback-trigger')).to.equal(TriggerClass);
  });

  it('renders the trigger button', async () => {
    const el = await fixture<ElohimGateFeedbackTrigger>(
      html`<elohim-gate-feedback-trigger></elohim-gate-feedback-trigger>`,
    );
    const btn = el.shadowRoot?.querySelector('[data-testid="feedback-trigger-btn"]');
    expect(btn).to.exist;
  });

  it('opens dropdown menu when trigger is clicked', async () => {
    const el = await fixture<ElohimGateFeedbackTrigger>(
      html`<elohim-gate-feedback-trigger></elohim-gate-feedback-trigger>`,
    );

    const btn = el.shadowRoot?.querySelector<HTMLButtonElement>('[data-testid="feedback-trigger-btn"]');
    btn?.click();
    await elementUpdated(el);

    const menu = el.shadowRoot?.querySelector('[data-testid="feedback-trigger-menu"]');
    expect(menu).to.exist;
  });

  it('closes menu and opens modal when menu item is clicked', async () => {
    const el = await fixture<ElohimGateFeedbackTrigger>(
      html`<elohim-gate-feedback-trigger></elohim-gate-feedback-trigger>`,
    );

    const triggerBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="feedback-trigger-btn"]',
    );
    triggerBtn?.click();
    await elementUpdated(el);

    const flagItem = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="feedback-menu-item-flag"]',
    );
    flagItem?.click();
    await elementUpdated(el);

    // Menu closed
    expect(el.shadowRoot?.querySelector('[data-testid="feedback-trigger-menu"]')).to.be.null;

    // Modal open
    const modal = el.shadowRoot?.querySelector('[data-testid="feedback-modal-panel"]');
    expect(modal).to.exist;
  });

  it('modal title matches selected feedback type', async () => {
    const el = await fixture<ElohimGateFeedbackTrigger>(
      html`<elohim-gate-feedback-trigger></elohim-gate-feedback-trigger>`,
    );

    const triggerBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="feedback-trigger-btn"]',
    );
    triggerBtn?.click();
    await elementUpdated(el);

    const challengeItem = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="feedback-menu-item-challenge"]',
    );
    challengeItem?.click();
    await elementUpdated(el);

    const title = el.shadowRoot?.querySelector('[data-testid="feedback-modal-title"]');
    expect(title?.textContent).to.contain('Challenge Content');
  });

  it('submit button is disabled when textarea is empty', async () => {
    const el = await fixture<ElohimGateFeedbackTrigger>(
      html`<elohim-gate-feedback-trigger></elohim-gate-feedback-trigger>`,
    );

    const triggerBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="feedback-trigger-btn"]',
    );
    triggerBtn?.click();
    await elementUpdated(el);

    const feedbackItem = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="feedback-menu-item-feedback"]',
    );
    feedbackItem?.click();
    await elementUpdated(el);

    const submitBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="feedback-modal-submit"]',
    );
    expect(submitBtn?.hasAttribute('disabled')).to.be.true;
  });

  it('closes modal when close button is clicked', async () => {
    const el = await fixture<ElohimGateFeedbackTrigger>(
      html`<elohim-gate-feedback-trigger></elohim-gate-feedback-trigger>`,
    );

    const triggerBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="feedback-trigger-btn"]',
    );
    triggerBtn?.click();
    await elementUpdated(el);

    const feedbackItem = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="feedback-menu-item-feedback"]',
    );
    feedbackItem?.click();
    await elementUpdated(el);

    const closeBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="feedback-modal-close"]',
    );
    closeBtn?.click();
    await elementUpdated(el);

    expect(el.shadowRoot?.querySelector('[data-testid="feedback-modal-panel"]')).to.be.null;
  });

  it('dispatches feedback-submit event', async () => {
    const el = await fixture<ElohimGateFeedbackTrigger>(
      html`<elohim-gate-feedback-trigger content-id="test-123"></elohim-gate-feedback-trigger>`,
    );

    let submitted: unknown = null;
    el.addEventListener('feedback-submit', (e: Event) => {
      submitted = (e as CustomEvent).detail;
    });

    const triggerBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="feedback-trigger-btn"]',
    );
    triggerBtn?.click();
    await elementUpdated(el);

    const feedbackItem = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="feedback-menu-item-feedback"]',
    );
    feedbackItem?.click();
    await elementUpdated(el);

    const textarea = el.shadowRoot?.querySelector<HTMLTextAreaElement>(
      '[data-testid="feedback-modal-textarea"]',
    );
    if (textarea) {
      textarea.value = 'This is my feedback text';
      textarea.dispatchEvent(new Event('input'));
    }
    await elementUpdated(el);

    const submitBtn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="feedback-modal-submit"]',
    );
    submitBtn?.click();

    expect(submitted).to.deep.include({ type: 'feedback', contentId: 'test-123' });
  });

  it('trigger button has aria-label', async () => {
    const el = await fixture<ElohimGateFeedbackTrigger>(
      html`<elohim-gate-feedback-trigger></elohim-gate-feedback-trigger>`,
    );
    const btn = el.shadowRoot?.querySelector('[data-testid="feedback-trigger-btn"]');
    expect(btn?.getAttribute('aria-label')).to.equal('Governance feedback');
  });

  it('passes axe-core a11y scan', async () => {
    const el = await fixture<ElohimGateFeedbackTrigger>(
      html`<elohim-gate-feedback-trigger></elohim-gate-feedback-trigger>`,
    );
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-gate-feedback-trigger> — i18n gate', () => {
  it('uses only logical CSS properties', () => {
    const cssText = (TriggerClass as { styles: { cssText: string } }).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-gate-feedback-trigger> — ua-prefs gate', () => {
  it('forced-colors overrides present', () => {
    const cssText = (TriggerClass as { styles: { cssText: string } }).styles.cssText;
    expect(cssText).to.contain('forced-colors: active');
  });

  it('touch targets meet 44px minimum', () => {
    const cssText = (TriggerClass as { styles: { cssText: string } }).styles.cssText;
    expect(cssText).to.contain('44px');
  });
});
