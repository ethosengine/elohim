import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import type { ElohimReactionBar } from './elohim-reaction-bar.js';
import { ElohimReactionBar as ElohimReactionBarClass } from './elohim-reaction-bar.js';
import { requiresLogicalProperties } from './testing/i18n.js';
import {
  assertThemeContrast,
  axeScanStrict,
  themeFixture,
  type ThemeCell,
} from './testing/theme-contrast.js';

describe('<elohim-reaction-bar>', () => {
  it('is defined in the custom element registry', () => {
    expect(customElements.get('elohim-reaction-bar')).to.equal(ElohimReactionBarClass);
  });

  it('renders with default permitted reactions', async () => {
    const el = await fixture<ElohimReactionBar>(html`
      <elohim-reaction-bar></elohim-reaction-bar>
    `);
    const bar = el.shadowRoot?.querySelector('[data-testid="reaction-moved"]');
    expect(bar).to.exist;
  });

  it('respects custom constraints permittedTypes', async () => {
    const el = await fixture<ElohimReactionBar>(html`
      <elohim-reaction-bar
        .constraints=${{ permittedTypes: ['grateful', 'inspired'] }}
      ></elohim-reaction-bar>
    `);
    expect(el.shadowRoot?.querySelector('[data-testid="reaction-grateful"]')).to.exist;
    expect(el.shadowRoot?.querySelector('[data-testid="reaction-moved"]')).to.be.null;
  });

  it('shows reaction count badge when showCounts is true and count > 0', async () => {
    const el = await fixture<ElohimReactionBar>(html`
      <elohim-reaction-bar
        .counts=${[{ type: 'moved', count: 5 }]}
        .showCounts=${true}
      ></elohim-reaction-bar>
    `);
    const count = el.shadowRoot?.querySelector('.reaction-count');
    expect(count?.textContent?.trim()).to.equal('5');
  });

  it('hides label in compact mode', async () => {
    const el = await fixture<ElohimReactionBar>(html`
      <elohim-reaction-bar compact></elohim-reaction-bar>
    `);
    const label = el.shadowRoot?.querySelector('.reaction-label');
    expect(label).to.be.null;
  });

  it('dispatches reaction-submit event when a reaction button is clicked', async () => {
    const el = await fixture<ElohimReactionBar>(html`
      <elohim-reaction-bar entity-type="content" entity-id="test-123"></elohim-reaction-bar>
    `);

    let submitted: unknown = null;
    el.addEventListener('reaction-submit', (e: Event) => {
      submitted = (e as CustomEvent).detail;
    });

    const btn = el.shadowRoot?.querySelector<HTMLButtonElement>('[data-testid="reaction-moved"]');
    btn?.click();

    expect(submitted).to.deep.include({ type: 'moved' });
  });

  it('dispatches reaction-remove when a user reaction is clicked again (toggle)', async () => {
    const el = await fixture<ElohimReactionBar>(html`
      <elohim-reaction-bar entity-id="test-123" .userReactions=${['moved']}></elohim-reaction-bar>
    `);

    let removed: unknown = null;
    el.addEventListener('reaction-remove', (e: Event) => {
      removed = (e as CustomEvent).detail;
    });

    const btn = el.shadowRoot?.querySelector<HTMLButtonElement>('[data-testid="reaction-moved"]');
    btn?.click();

    expect(removed).to.deep.include({ type: 'moved' });
  });

  it('shows mediation dialog for mediated reactions', async () => {
    const el = await fixture<ElohimReactionBar>(html`
      <elohim-reaction-bar
        .constraints=${{
          permittedTypes: ['uncomfortable'],
          mediatedTypes: [
            {
              type: 'uncomfortable',
              constitutionalReasoning: 'Consider context carefully.',
              proceedBehavior: { visibleToOthers: true },
            },
          ],
        }}
      ></elohim-reaction-bar>
    `);

    const btn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="reaction-uncomfortable"]'
    );
    btn?.click();
    await elementUpdated(el);

    const dialog = el.shadowRoot?.querySelector('[data-testid="mediation-dialog"]');
    expect(dialog).to.exist;
    expect(dialog?.textContent).to.contain('Consider context carefully.');
  });

  it('closes mediation dialog on cancel', async () => {
    const el = await fixture<ElohimReactionBar>(html`
      <elohim-reaction-bar
        .constraints=${{
          permittedTypes: ['uncomfortable'],
          mediatedTypes: [
            {
              type: 'uncomfortable',
              constitutionalReasoning: 'Reflect.',
              proceedBehavior: { visibleToOthers: true },
            },
          ],
        }}
      ></elohim-reaction-bar>
    `);

    const btn = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="reaction-uncomfortable"]'
    );
    btn?.click();
    await elementUpdated(el);

    const cancel = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="mediation-cancel-btn"]'
    );
    cancel?.click();
    await elementUpdated(el);

    expect(el.shadowRoot?.querySelector('[data-testid="mediation-dialog"]')).to.be.null;
  });

  it('calls onReactionSubmit callback when provided', async () => {
    let callbackEvent: unknown = null;
    const onSubmit = (e: unknown) => {
      callbackEvent = e;
    };

    const el = await fixture<ElohimReactionBar>(html`
      <elohim-reaction-bar .onReactionSubmit=${onSubmit}></elohim-reaction-bar>
    `);

    const btn = el.shadowRoot?.querySelector<HTMLButtonElement>('[data-testid="reaction-moved"]');
    btn?.click();

    expect(callbackEvent).to.exist;
  });

  it('marks user reactions as aria-pressed=true', async () => {
    const el = await fixture<ElohimReactionBar>(html`
      <elohim-reaction-bar .userReactions=${['grateful']}></elohim-reaction-bar>
    `);
    const btn = el.shadowRoot?.querySelector('[data-testid="reaction-grateful"]');
    expect(btn?.getAttribute('aria-pressed')).to.equal('true');
  });

  it('marks non-user reactions as aria-pressed=false', async () => {
    const el = await fixture<ElohimReactionBar>(html`
      <elohim-reaction-bar></elohim-reaction-bar>
    `);
    const btn = el.shadowRoot?.querySelector('[data-testid="reaction-moved"]');
    expect(btn?.getAttribute('aria-pressed')).to.equal('false');
  });

  it('passes axe-core a11y scan', async () => {
    const el = await fixture<ElohimReactionBar>(html`
      <elohim-reaction-bar></elohim-reaction-bar>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('reaction buttons have touch-target minimum size in coarse-pointer CSS', () => {
    const cssText = (ElohimReactionBarClass as { styles: { cssText: string } }).styles.cssText;
    expect(cssText).to.contain('pointer: coarse');
    expect(cssText).to.contain('44px');
  });
});

describe('<elohim-reaction-bar> — i18n precondition gate', () => {
  it('uses only logical CSS properties', () => {
    const cssText = (ElohimReactionBarClass as { styles: { cssText: string } }).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-reaction-bar> — ua-prefs precondition gate', () => {
  it('no animations declared without media guard (still mode default)', () => {
    const cssText = (ElohimReactionBarClass as { styles: { cssText: string } }).styles.cssText;
    expect(cssText).to.not.contain('@keyframes');
    // transitions are not declared since default is still
    const transIdx = cssText.indexOf('transition:');
    expect(transIdx).to.equal(-1);
  });

  it('forced-colors overrides use CSS system colors', () => {
    const cssText = (ElohimReactionBarClass as { styles: { cssText: string } }).styles.cssText;
    expect(cssText).to.contain('forced-colors: active');
    const forcedIdx = cssText.indexOf('forced-colors: active');
    // eslint-disable-next-line unicorn/prefer-set-has -- string scan, not membership lookup
    const afterForced = cssText.slice(forcedIdx);
    const hasSystemColor =
      afterForced.includes('ButtonFace') ||
      afterForced.includes('Canvas') ||
      afterForced.includes('Highlight') ||
      afterForced.includes('HighlightText');
    expect(hasSystemColor).to.be.true;
  });
});

describe('<elohim-reaction-bar> — theme-contrast gate', () => {
  // tokens cells: no shipped binding for this element yet — system cells only
  // (theme-authority spec §4.1). The blank-slate contract per scheme.
  const CELLS: ThemeCell[] = ['system-light', 'system-dark'];

  for (const cell of CELLS) {
    it(`passes contrast in ${cell}`, async () => {
      // Richest text-visible fixture: default reactions rendered with their
      // labels plus a GrayText count badge ("5") via showCounts. Mediation
      // overlay stays closed (no mediated click), matching the default view.
      const { el } = await themeFixture<ElohimReactionBar>(
        html`
          <elohim-reaction-bar
            .counts=${[{ type: 'moved', count: 5 }]}
            .showCounts=${true}
          ></elohim-reaction-bar>
        `,
        cell
      );
      await el.updateComplete;
      assertThemeContrast(el);
      await axeScanStrict(el);
    });
  }
});
