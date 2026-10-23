import { expect, fixture, html } from '@open-wc/testing';

import './register.js';
import type { ElohimResiliencyBadge } from './elohim-resiliency-badge.js';

const anchor = (el: Element) => el.shadowRoot!.querySelector('a')!;

describe('<elohim-resiliency-badge>', () => {
  it('is defined', () => {
    expect(customElements.get('elohim-resiliency-badge')).to.exist;
  });

  it('shows the given headline verbatim as the link text', async () => {
    const el = await fixture<ElohimResiliencyBadge>(html`
      <elohim-resiliency-badge href="/epr/abc" headline="Safe in 3 homes"></elohim-resiliency-badge>
    `);
    expect(anchor(el).textContent!.trim()).to.equal('Safe in 3 homes');
    expect(anchor(el).getAttribute('href')).to.equal('/epr/abc');
    expect(anchor(el).getAttribute('part')).to.equal('badge');
    expect(el.getAttribute('data-state')).to.equal('known');
  });

  it('unknown: no headline shows only the unknown-state sentence', async () => {
    const el = await fixture<ElohimResiliencyBadge>(html`
      <elohim-resiliency-badge href="/epr/abc"></elohim-resiliency-badge>
    `);
    expect(anchor(el).textContent!.trim()).to.equal('Held by households · see where it lives');
    expect(el.getAttribute('data-state')).to.equal('unknown');
  });

  it('has no wording logic of its own: figures are never turned into sentences', async () => {
    const el = await fixture<ElohimResiliencyBadge>(html`
      <elohim-resiliency-badge href="/epr/abc" .holders=${3} .target=${3}></elohim-resiliency-badge>
    `);
    expect(anchor(el).textContent!.trim()).to.equal('Held by households · see where it lives');
  });

  it('explicit state is reflected for theming', async () => {
    const el = await fixture<ElohimResiliencyBadge>(html`
      <elohim-resiliency-badge
        href="/x"
        state="thin"
        headline="Held by 1 of 3"
      ></elohim-resiliency-badge>
    `);
    expect(el.getAttribute('data-state')).to.equal('thin');
  });

  it('does not fetch anything', async () => {
    const realFetch = globalThis.fetch;
    let called = false;
    globalThis.fetch = (() => {
      called = true;
      return Promise.reject(new Error('no'));
    }) as typeof fetch;
    try {
      await fixture<ElohimResiliencyBadge>(html`
        <elohim-resiliency-badge href="/x" content-id="abc"></elohim-resiliency-badge>
      `);
    } finally {
      globalThis.fetch = realFetch;
    }
    expect(called).to.equal(false);
  });
});
