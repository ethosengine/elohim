import { expect, fixture, html } from '@open-wc/testing';
import { LitElement } from 'lit';
import { provide } from '@lit/context';
import { property } from 'lit/decorators.js';

import { CapabilityAwareElement } from './mixin.js';
import { capabilityProfileContext } from './context.js';
import { DEFAULT_PROFILE } from './profile.js';
import type { CapabilityProfile } from './profile.js';

class ContextProvider extends LitElement {
  @provide({ context: capabilityProfileContext })
  @property({ attribute: false })
  profile: CapabilityProfile = DEFAULT_PROFILE;

  override render() {
    return html`<slot></slot>`;
  }
}
customElements.define('ctx-provider', ContextProvider);

class CapAwareThing extends CapabilityAwareElement(LitElement) {
  override render() {
    return html`<span data-lens=${this.profile.lens}></span>`;
  }
}
customElements.define('cap-aware-thing', CapAwareThing);

describe('CapabilityAwareElement mixin', () => {
  it('exposes a profile property that defaults to DEFAULT_PROFILE when no provider', async () => {
    const el = await fixture<CapAwareThing>(html`<cap-aware-thing></cap-aware-thing>`);
    expect(el.profile).to.deep.equal(DEFAULT_PROFILE);
  });

  it('receives profile from a provider in the DOM tree', async () => {
    const provider = await fixture<ContextProvider>(html`
      <ctx-provider>
        <cap-aware-thing></cap-aware-thing>
      </ctx-provider>
    `);
    const el = provider.querySelector<CapAwareThing>('cap-aware-thing')!;
    await el.updateComplete;
    expect(el.profile.lens).to.equal('standard');
  });

  it('re-renders when the provider updates profile', async () => {
    const provider = await fixture<ContextProvider>(html`
      <ctx-provider>
        <cap-aware-thing></cap-aware-thing>
      </ctx-provider>
    `);
    const el = provider.querySelector<CapAwareThing>('cap-aware-thing')!;
    await el.updateComplete;
    provider.profile = { ...DEFAULT_PROFILE, lens: 'detail' };
    await el.updateComplete;
    const span = el.shadowRoot!.querySelector('span')!;
    expect(span.getAttribute('data-lens')).to.equal('detail');
  });
});
