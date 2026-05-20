import { expect, fixture, html } from '@open-wc/testing';
import { LitElement } from 'lit';
import { axeScan, expectKeyboardFocusable } from './a11y.js';

class FocusableThing extends LitElement {
  static override readonly shadowRootOptions: ShadowRootInit = {
    ...LitElement.shadowRootOptions,
    delegatesFocus: true,
  };
  override render() {
    return html`
      <button>x</button>
    `;
  }
}
customElements.define('focusable-thing', FocusableThing);

class NotFocusableThing extends LitElement {
  override render() {
    return html`
      <span>x</span>
    `;
  }
}
customElements.define('not-focusable-thing', NotFocusableThing);

describe('a11y harness', () => {
  it('axeScan returns no violations for a clean element', async () => {
    const el = await fixture(html`
      <button>Save</button>
    `);
    const { violations } = await axeScan(el);
    expect(violations).to.have.lengthOf(0);
  });

  it('expectKeyboardFocusable passes when the element receives focus', async () => {
    const el = await fixture<FocusableThing>(html`
      <focusable-thing></focusable-thing>
    `);
    await expectKeyboardFocusable(el);
  });

  it('expectKeyboardFocusable throws when the element cannot receive focus', async () => {
    const el = await fixture<NotFocusableThing>(html`
      <not-focusable-thing></not-focusable-thing>
    `);
    let threw = false;
    try {
      await expectKeyboardFocusable(el);
    } catch {
      threw = true;
    }
    expect(threw).to.be.true;
  });
});
