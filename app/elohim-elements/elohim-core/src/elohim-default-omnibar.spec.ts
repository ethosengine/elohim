import { expect } from '@open-wc/testing';
import { html } from 'lit';

import './register.js';
import type { ElohimDefaultOmnibar } from './elohim-default-omnibar.js';
import { ElohimDefaultOmnibar as OmnibarClass } from './elohim-default-omnibar.js';
import {
  assertThemeContrast,
  assertThemeReactivity,
  axeScanStrict,
  themeFixture,
  type ThemeCell,
} from './testing/theme-contrast.js';

describe('<elohim-default-omnibar>', () => {
  it('is defined in the custom element registry', () => {
    expect(customElements.get('elohim-default-omnibar')).to.equal(OmnibarClass);
  });
});

describe('<elohim-default-omnibar> — theme-contrast gate', () => {
  const CELLS: ThemeCell[] = ['system-light', 'system-dark', 'tokens-light', 'tokens-dark'];

  for (const cell of CELLS) {
    it(`anonymous omnibar (sign-in link) passes contrast in ${cell}`, async () => {
      const { el } = await themeFixture<ElohimDefaultOmnibar>(
        html`
          <elohim-default-omnibar></elohim-default-omnibar>
        `,
        cell
      );
      await el.updateComplete;
      assertThemeContrast(el);
      await axeScanStrict(el);
    });
  }

  it('omnibar reacts to the theme (frozen-chain canary)', async () => {
    await assertThemeReactivity<ElohimDefaultOmnibar>(
      () =>
        html`
          <elohim-default-omnibar></elohim-default-omnibar>
        `
    );
  });
});
