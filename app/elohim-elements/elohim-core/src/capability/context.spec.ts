import { expect, fixture, html } from '@open-wc/testing';
import { LitElement } from 'lit';
import { consume, provide } from '@lit/context';
import { property, state } from 'lit/decorators.js';

import { capabilityProfileContext, contentCertaintyContext } from './context.js';
import { DEFAULT_PROFILE } from './profile.js';
import { UNKNOWN_CERTAINTY } from './certainty.js';
import type { CapabilityProfile } from './profile.js';
import type { ContentCertainty } from './certainty.js';

class TestProvider extends LitElement {
  @provide({ context: capabilityProfileContext })
  @property({ attribute: false })
  profile: CapabilityProfile = DEFAULT_PROFILE;

  @provide({ context: contentCertaintyContext })
  @property({ attribute: false })
  certainty: ContentCertainty = UNKNOWN_CERTAINTY;

  override render() {
    return html`<slot></slot>`;
  }
}
customElements.define('test-provider', TestProvider);

class TestConsumer extends LitElement {
  @consume({ context: capabilityProfileContext, subscribe: true })
  @state()
  profile: CapabilityProfile = DEFAULT_PROFILE;

  @consume({ context: contentCertaintyContext, subscribe: true })
  @state()
  certainty: ContentCertainty = UNKNOWN_CERTAINTY;

  override render() {
    return html`<span data-lens=${this.profile.lens} data-state=${this.certainty.state}></span>`;
  }
}
customElements.define('test-consumer', TestConsumer);

describe('capability context', () => {
  it('propagates profile from provider to consumer', async () => {
    const el = await fixture<TestProvider>(html`
      <test-provider>
        <test-consumer></test-consumer>
      </test-provider>
    `);
    const consumer = el.querySelector<TestConsumer>('test-consumer')!;
    await consumer.updateComplete;
    const span = consumer.shadowRoot!.querySelector('span')!;
    expect(span.getAttribute('data-lens')).to.equal('standard');
  });

  it('propagates certainty from provider to consumer', async () => {
    const el = await fixture<TestProvider>(html`
      <test-provider>
        <test-consumer></test-consumer>
      </test-provider>
    `);
    const consumer = el.querySelector<TestConsumer>('test-consumer')!;
    await consumer.updateComplete;
    const span = consumer.shadowRoot!.querySelector('span')!;
    expect(span.getAttribute('data-state')).to.equal('unknown');
  });

  it('re-renders consumer when provider changes profile', async () => {
    const provider = await fixture<TestProvider>(html`
      <test-provider>
        <test-consumer></test-consumer>
      </test-provider>
    `);
    const consumer = provider.querySelector<TestConsumer>('test-consumer')!;
    await consumer.updateComplete;
    provider.profile = { ...DEFAULT_PROFILE, lens: 'minimal' };
    await consumer.updateComplete;
    const span = consumer.shadowRoot!.querySelector('span')!;
    expect(span.getAttribute('data-lens')).to.equal('minimal');
  });
});
