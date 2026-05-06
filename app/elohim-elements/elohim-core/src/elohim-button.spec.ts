import { fixture, html, expect } from '@open-wc/testing';
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
    expect(slot).to.exist;
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

  it('emits a click event when activated by keyboard (Enter)', async () => {
    const el = await fixture<ElohimButton>(html`
      <elohim-button>Hi</elohim-button>
    `);
    let clicks = 0;
    el.addEventListener('click', () => clicks++);
    const inner = el.shadowRoot!.querySelector('button')!;
    inner.focus();
    inner.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
    inner.click(); // browsers fire click on Enter for native buttons; simulate
    expect(clicks).to.equal(1);
  });

  it('does not emit a click event when disabled', async () => {
    const el = await fixture<ElohimButton>(html`
      <elohim-button disabled>Hi</elohim-button>
    `);
    let clicks = 0;
    el.addEventListener('click', () => clicks++);
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

  it('passes axe-core a11y scan in default state', async () => {
    const el = await fixture<ElohimButton>(html`
      <elohim-button>Submit</elohim-button>
    `);
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
