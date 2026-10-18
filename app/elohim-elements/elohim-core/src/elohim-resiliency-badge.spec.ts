import { expect, fixture, html, waitUntil } from '@open-wc/testing';

import './register.js';
import type { ElohimResiliencyBadge } from './elohim-resiliency-badge.js';

const anchor = (el: Element) => el.shadowRoot!.querySelector('a')!;

describe('<elohim-resiliency-badge>', () => {
  const realFetch = globalThis.fetch;
  afterEach(() => {
    globalThis.fetch = realFetch;
  });

  it('is defined', () => {
    expect(customElements.get('elohim-resiliency-badge')).to.exist;
  });

  it('held: renders figures as link text', async () => {
    const el = await fixture<ElohimResiliencyBadge>(html`
      <elohim-resiliency-badge href="/epr/abc" .holders=${3} .target=${3}></elohim-resiliency-badge>
    `);
    expect(anchor(el).textContent!.trim()).to.equal('Held by 3 of 3 households');
    expect(anchor(el).getAttribute('href')).to.equal('/epr/abc');
    expect(anchor(el).getAttribute('part')).to.equal('badge');
    expect(el.getAttribute('data-state')).to.equal('held');
  });

  it('thin: holders below target', async () => {
    const el = await fixture<ElohimResiliencyBadge>(html`
      <elohim-resiliency-badge href="/epr/abc" .holders=${1} .target=${3}></elohim-resiliency-badge>
    `);
    expect(anchor(el).textContent!.trim()).to.equal('Held by 1 of 3 households');
    expect(el.getAttribute('data-state')).to.equal('thin');
  });

  it('zero holders reads as not yet held', async () => {
    const el = await fixture<ElohimResiliencyBadge>(html`
      <elohim-resiliency-badge href="/epr/abc" .holders=${0} .target=${3}></elohim-resiliency-badge>
    `);
    expect(anchor(el).textContent!.trim()).to.equal('Not yet held by any household');
    expect(el.getAttribute('data-state')).to.equal('thin');
  });

  it('unknown: no figures, no content-id', async () => {
    const el = await fixture<ElohimResiliencyBadge>(html`
      <elohim-resiliency-badge href="/epr/abc"></elohim-resiliency-badge>
    `);
    expect(anchor(el).textContent!.trim()).to.equal('Held by households · see where it lives');
    expect(el.getAttribute('data-state')).to.equal('unknown');
  });

  it('explicit state overrides the derived one', async () => {
    const el = await fixture<ElohimResiliencyBadge>(html`
      <elohim-resiliency-badge
        href="/x"
        state="thin"
        .holders=${3}
        .target=${3}
      ></elohim-resiliency-badge>
    `);
    expect(el.getAttribute('data-state')).to.equal('thin');
  });

  it('fetches figures from the resilience view when content-id is set', async () => {
    const urls: string[] = [];
    globalThis.fetch = (async (u: RequestInfo | URL) => {
      urls.push(String(u));
      return new Response(
        JSON.stringify({ feltStatus: { floor: { hasHouseholds: 2, wantsHouseholds: 3 } } }),
        { status: 200 }
      );
    }) as typeof fetch;
    const el = await fixture<ElohimResiliencyBadge>(html`
      <elohim-resiliency-badge href="/epr/abc" content-id="abc"></elohim-resiliency-badge>
    `);
    await waitUntil(() => el.getAttribute('data-state') === 'thin');
    expect(urls[0]).to.equal('/api/v1/resilience/abc/household');
    expect(anchor(el).textContent!.trim()).to.equal('Held by 2 of 3 households');
  });

  it('base-url overrides the origin', async () => {
    const urls: string[] = [];
    globalThis.fetch = (async (u: RequestInfo | URL) => {
      urls.push(String(u));
      return new Response('{}', { status: 200 });
    }) as typeof fetch;
    await fixture(html`
      <elohim-resiliency-badge
        content-id="abc"
        base-url="https://d.example/"
      ></elohim-resiliency-badge>
    `);
    await waitUntil(() => urls.length > 0);
    expect(urls[0]).to.equal('https://d.example/api/v1/resilience/abc/household');
  });

  it('fetch failure keeps the unknown state silently', async () => {
    globalThis.fetch = (async () => {
      throw new Error('offline');
    }) as typeof fetch;
    const el = await fixture<ElohimResiliencyBadge>(html`
      <elohim-resiliency-badge href="/epr/abc" content-id="abc"></elohim-resiliency-badge>
    `);
    await new Promise(r => setTimeout(r, 30));
    expect(el.getAttribute('data-state')).to.equal('unknown');
    expect(anchor(el).textContent!.trim()).to.equal('Held by households · see where it lives');
  });

  it('non-ok and unmeasured responses keep unknown', async () => {
    globalThis.fetch = (async () =>
      new Response('{"feltStatus":null}', { status: 200 })) as typeof fetch;
    const el = await fixture<ElohimResiliencyBadge>(html`
      <elohim-resiliency-badge content-id="abc"></elohim-resiliency-badge>
    `);
    await new Promise(r => setTimeout(r, 30));
    expect(el.getAttribute('data-state')).to.equal('unknown');
  });
});
