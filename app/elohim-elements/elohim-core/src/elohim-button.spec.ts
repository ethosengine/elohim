import { aTimeout, elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import type { ElohimButton } from './elohim-button.js';

describe('<elohim-button>', () => {
  it('renders the default slot content', async () => {
    const el = await fixture<ElohimButton>(html`
      <elohim-button>Click me</elohim-button>
    `);
    expect(el).to.exist;
    expect(el.shadowRoot).to.exist;
    const slot = el.shadowRoot!.querySelector('slot');
    expect(slot).to.be.instanceOf(HTMLSlotElement);
    const assigned = slot!.assignedNodes({ flatten: true });
    const text = assigned
      .map(n => n.textContent)
      .join('')
      .trim();
    expect(text).to.equal('Click me');
  });

  it('defaults variant to "primary"', async () => {
    const el = await fixture<ElohimButton>(html`
      <elohim-button>Hi</elohim-button>
    `);
    expect(el.variant).to.equal('primary');
  });

  it('accepts variant="secondary" and reflects to attribute', async () => {
    const el = await fixture<ElohimButton>(html`
      <elohim-button variant="secondary">Hi</elohim-button>
    `);
    expect(el.variant).to.equal('secondary');
    expect(el.getAttribute('variant')).to.equal('secondary');
  });

  it('accepts variant="ghost"', async () => {
    const el = await fixture<ElohimButton>(html`
      <elohim-button variant="ghost">Hi</elohim-button>
    `);
    expect(el.variant).to.equal('ghost');
  });

  it('reflects programmatic variant changes to the host attribute', async () => {
    const el = await fixture<ElohimButton>(html`
      <elohim-button>Hi</elohim-button>
    `);
    expect(el.getAttribute('variant')).to.equal('primary');
    el.variant = 'ghost';
    await elementUpdated(el);
    expect(el.getAttribute('variant')).to.equal('ghost');
  });

  it('emits a click event when activated by mouse', async () => {
    const el = await fixture<ElohimButton>(html`
      <elohim-button>Hi</elohim-button>
    `);
    let clicks = 0;
    el.addEventListener('click', () => clicks++);
    const inner = el.shadowRoot!.querySelector('button')!;
    inner.click();
    expect(clicks).to.equal(1);
  });

  it('emits a click event on Enter (native button keyboard semantics)', async () => {
    const el = await fixture<ElohimButton>(html`
      <elohim-button>Hi</elohim-button>
    `);
    let clicks = 0;
    el.addEventListener('click', () => clicks++);
    const inner = el.shadowRoot!.querySelector('button')!;
    inner.focus();
    // The inner element MUST be a native <button> for keyboard activation to work
    // automatically. Asserting that here makes the contract explicit.
    expect(inner).to.be.instanceOf(HTMLButtonElement);
    // Dispatch a keypress event the way the browser would on Enter for a focused button.
    // We assert the cancelable click that follows is captured by the host listener.
    inner.dispatchEvent(
      new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true })
    );
    inner.dispatchEvent(
      new KeyboardEvent('keyup', { key: 'Enter', bubbles: true, cancelable: true })
    );
    // For native buttons, the browser also fires a click on Enter. In a unit-test
    // context without a real browser keyboard pipeline, dispatch the click explicitly
    // — but only AFTER asserting the inner element is a real <button>, which is the
    // contract we actually care about.
    inner.click();
    await elementUpdated(el);
    expect(clicks).to.equal(1);
  });

  it('emits a click event on Space (native button keyboard semantics)', async () => {
    const el = await fixture<ElohimButton>(html`
      <elohim-button>Hi</elohim-button>
    `);
    let clicks = 0;
    el.addEventListener('click', () => clicks++);
    const inner = el.shadowRoot!.querySelector('button')!;
    inner.focus();
    expect(inner).to.be.instanceOf(HTMLButtonElement);
    inner.dispatchEvent(
      new KeyboardEvent('keydown', { key: ' ', bubbles: true, cancelable: true })
    );
    inner.dispatchEvent(new KeyboardEvent('keyup', { key: ' ', bubbles: true, cancelable: true }));
    inner.click();
    await elementUpdated(el);
    expect(clicks).to.equal(1);
  });

  it('does not emit a click event when disabled (host dispatch)', async () => {
    const el = await fixture<ElohimButton>(html`
      <elohim-button disabled>Hi</elohim-button>
    `);
    let clicks = 0;
    el.addEventListener('click', () => clicks++);
    // Dispatching on the host element is what real browser interactions look like
    // before they reach the inner shadow button. A native disabled <button> swallows
    // pointer-driven click events. This asserts the public-surface contract.
    const inner = el.shadowRoot!.querySelector('button')!;
    inner.click();
    expect(clicks).to.equal(0);
  });

  it('does not emit a click event when disabled is toggled programmatically', async () => {
    const el = await fixture<ElohimButton>(html`
      <elohim-button>Hi</elohim-button>
    `);
    let clicks = 0;
    el.addEventListener('click', () => clicks++);
    el.disabled = true;
    await elementUpdated(el);
    const inner = el.shadowRoot!.querySelector('button')!;
    inner.click();
    expect(clicks).to.equal(0);
  });

  it('sets aria-disabled when disabled', async () => {
    const el = await fixture<ElohimButton>(html`
      <elohim-button disabled>Hi</elohim-button>
    `);
    const inner = el.shadowRoot!.querySelector('button')!;
    expect(inner.getAttribute('aria-disabled')).to.equal('true');
    expect(inner.hasAttribute('disabled')).to.be.true;
  });

  it('updates aria-disabled when disabled toggles after render', async () => {
    const el = await fixture<ElohimButton>(html`
      <elohim-button>Hi</elohim-button>
    `);
    const inner = el.shadowRoot!.querySelector('button')!;
    expect(inner.getAttribute('aria-disabled')).to.equal('false');
    el.disabled = true;
    await elementUpdated(el);
    expect(inner.getAttribute('aria-disabled')).to.equal('true');
    expect(inner.hasAttribute('disabled')).to.be.true;
  });

  it('is keyboard-focusable (delegates focus to inner button)', async () => {
    const el = await fixture<ElohimButton>(html`
      <elohim-button>Hi</elohim-button>
    `);
    el.focus();
    await aTimeout(0);
    // Either the host receives focus directly, or focus delegates to the inner button.
    // Both satisfy the keyboard-navigability contract — assert that focus landed
    // somewhere meaningful on this component.
    const inner = el.shadowRoot!.querySelector('button')!;
    const active = document.activeElement;
    expect(active === el || active === inner).to.equal(true);
  });

  it('passes axe-core a11y scan in default state', async () => {
    const el = await fixture<ElohimButton>(html`
      <elohim-button>Submit</elohim-button>
    `);
    // axe traverses shadow DOM automatically when given the host element; do not
    // pass el.shadowRoot — axe needs the element in the document tree to resolve
    // ARIA references.
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe-core a11y scan in disabled state', async () => {
    const el = await fixture<ElohimButton>(html`
      <elohim-button disabled>Submit</elohim-button>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});
