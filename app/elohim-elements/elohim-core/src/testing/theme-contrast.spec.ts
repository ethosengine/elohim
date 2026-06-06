import { expect } from '@open-wc/testing';
import { css, html, LitElement } from 'lit';
import { customElement } from 'lit/decorators.js';

import {
  assertThemeContrast,
  assertThemeReactivity,
  contrastRatio,
  themeFixture,
  type ThemeCell,
} from './theme-contrast.js';

/** Deliberately failing element: light-gray text hardcoded on white. */
@customElement('tc-bad-contrast')
class TcBadContrast extends LitElement {
  static override readonly styles = css`
    :host {
      display: block;
      background: #fff;
    }

    p {
      color: #ccc; /* 1.6:1 on white — must be caught */
    }
  `;

  override render() {
    return html`
      <p>barely visible text</p>
    `;
  }
}

/** Honest blank-slate element: system pair, follows color-scheme. */
@customElement('tc-good-contrast')
class TcGoodContrast extends LitElement {
  static override readonly styles = css`
    :host {
      display: block;
      background: Canvas;
      color: CanvasText;
    }
  `;

  override render() {
    return html`
      <p>readable text</p>
    `;
  }
}

/** Theme-frozen element: same colors regardless of data-theme. */
@customElement('tc-frozen')
class TcFrozen extends LitElement {
  static override readonly styles = css`
    :host {
      display: block;
      background: #1e293b;
      color: #f1f5f9;
    }
  `;

  override render() {
    return html`
      <p>always dark</p>
    `;
  }
}

/** Token-reactive element: surface bound to the lamad palette. */
@customElement('tc-reactive')
class TcReactive extends LitElement {
  static override readonly styles = css`
    :host {
      display: block;
      background: var(--lamad-bg-secondary, Canvas);
      color: var(--lamad-text-primary, CanvasText);
    }
  `;

  override render() {
    return html`
      <p>themed text</p>
    `;
  }
}

describe('theme-contrast helper', () => {
  it('computes WCAG21 ratios from CSS color strings', () => {
    expect(contrastRatio('rgb(0, 0, 0)', 'rgb(255, 255, 255)')).to.be.closeTo(21, 0.1);
    expect(contrastRatio('rgb(241, 245, 249)', 'rgb(99, 102, 241)')).to.be.closeTo(4.08, 0.05);
  });

  it('flags a failing fg/bg pair with the offending ratio', async () => {
    const { el } = await themeFixture<TcBadContrast>(
      html`
        <tc-bad-contrast></tc-bad-contrast>
      `,
      'system-light'
    );
    await el.updateComplete;
    let thrown: Error | null = null;
    try {
      assertThemeContrast(el);
    } catch (e) {
      thrown = e as Error;
    }
    expect(thrown, 'expected contrast failure').to.not.be.null;
    expect(thrown!.message).to.contain('barely visible');
    expect(thrown!.message).to.match(/1\.6\d?:1/);
  });

  it('passes an honest system-color element in BOTH schemes', async () => {
    for (const cell of ['system-light', 'system-dark'] as ThemeCell[]) {
      const { el } = await themeFixture<TcGoodContrast>(
        html`
          <tc-good-contrast></tc-good-contrast>
        `,
        cell
      );
      await el.updateComplete;
      expect(() => assertThemeContrast(el), `cell ${cell}`).to.not.throw();
    }
  });

  it('tokens cells apply the real palette via documentElement[data-theme]', async () => {
    const { el } = await themeFixture<TcReactive>(
      html`
        <tc-reactive></tc-reactive>
      `,
      'tokens-dark'
    );
    await el.updateComplete;
    const bg = getComputedStyle(el).backgroundColor;
    expect(bg).to.equal('rgb(30, 41, 59)'); // --lamad-bg-secondary dark
  });

  it('assertThemeReactivity catches a frozen element', async () => {
    let thrown = false;
    try {
      await assertThemeReactivity<TcFrozen>(
        () => html`
          <tc-frozen></tc-frozen>
        `
      );
    } catch {
      thrown = true;
    }
    expect(thrown, 'frozen element must fail reactivity').to.be.true;
  });

  it('assertThemeReactivity passes a token-bound element', async () => {
    await assertThemeReactivity(
      () => html`
        <tc-reactive></tc-reactive>
      `
    );
  });
});
