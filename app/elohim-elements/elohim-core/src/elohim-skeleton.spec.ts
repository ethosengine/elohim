import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import type { ElohimSkeleton } from './elohim-skeleton.js';
import { ElohimSkeleton as ElohimSkeletonClass } from './elohim-skeleton.js';
import { clearMediaQueries, measureLuminanceChanges } from './testing/ua-prefs.js';
import { requiresLogicalProperties } from './testing/i18n.js';

describe('<elohim-skeleton>', () => {
  it('is defined in the custom element registry', () => {
    expect(customElements.get('elohim-skeleton')).to.equal(ElohimSkeletonClass);
  });

  it('renders with default width, height, and radius', async () => {
    const el = await fixture<ElohimSkeleton>(html`<elohim-skeleton></elohim-skeleton>`);
    expect(el).to.exist;
    expect(el.width).to.equal('100%');
    expect(el.height).to.equal('1rem');
    expect(el.radius).to.equal('var(--elohim-radius-sm, 4px)');
  });

  it('applies width and height as inline styles after first update', async () => {
    const el = await fixture<ElohimSkeleton>(html`
      <elohim-skeleton width="200px" height="2rem"></elohim-skeleton>
    `);
    expect(el.style.width).to.equal('200px');
    expect(el.style.height).to.equal('2rem');
  });

  it('updates inline style when width property changes', async () => {
    const el = await fixture<ElohimSkeleton>(html`<elohim-skeleton></elohim-skeleton>`);
    el.width = '320px';
    await elementUpdated(el);
    expect(el.style.width).to.equal('320px');
  });

  it('updates inline style when height property changes', async () => {
    const el = await fixture<ElohimSkeleton>(html`<elohim-skeleton></elohim-skeleton>`);
    el.height = '3rem';
    await elementUpdated(el);
    expect(el.style.height).to.equal('3rem');
  });

  it('updates --elohim-skeleton-radius custom property when radius changes', async () => {
    const el = await fixture<ElohimSkeleton>(html`<elohim-skeleton></elohim-skeleton>`);
    el.radius = '8px';
    await elementUpdated(el);
    expect(el.style.getPropertyValue('--elohim-skeleton-radius')).to.equal('8px');
  });

  it('has shadowRoot', async () => {
    const el = await fixture<ElohimSkeleton>(html`<elohim-skeleton></elohim-skeleton>`);
    expect(el.shadowRoot).to.exist;
  });

  it('passes axe-core a11y scan in default state', async () => {
    const el = await fixture<ElohimSkeleton>(html`<elohim-skeleton></elohim-skeleton>`);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe-core a11y scan with explicit dimensions', async () => {
    const el = await fixture<ElohimSkeleton>(html`
      <elohim-skeleton width="200px" height="48px"></elohim-skeleton>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-skeleton> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS gates shimmer animation behind prefers-reduced-motion and update: fast', () => {
    // setMediaQuery patches JS matchMedia only, not CSS @media re-evaluation.
    // Assert structural contract: animation MUST live inside the media query guard
    // so reduced-motion and e-paper displays receive a static placeholder.
    const cssText = (
      ElohimSkeletonClass as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.contain('prefers-reduced-motion: no-preference');
    expect(cssText).to.contain('update: fast');
  });

  it('animation declaration only appears inside the motion/update guard', () => {
    const cssText = (
      ElohimSkeletonClass as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const animationIdx = cssText.indexOf('animation:');
    const mediaIdx = cssText.indexOf('@media (prefers-reduced-motion: no-preference)');
    // animation: must appear AFTER the @media rule opening
    expect(animationIdx).to.be.greaterThan(mediaIdx);
  });

  it('forced-colors overrides use CSS system colors (Canvas, ButtonFace, ButtonText)', () => {
    const cssText = (
      ElohimSkeletonClass as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.contain('forced-colors: active');
    // At minimum one system color is referenced inside the forced-colors block
    const forcedIdx = cssText.indexOf('forced-colors: active');
    const afterForced = cssText.slice(forcedIdx);
    const hasSystemColor =
      afterForced.includes('ButtonFace') ||
      afterForced.includes('Canvas') ||
      afterForced.includes('ButtonText');
    expect(hasSystemColor).to.be.true;
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimSkeleton>(html`
      <elohim-skeleton width="200px" height="200px"></elohim-skeleton>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-skeleton> — i18n precondition gate', () => {
  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimSkeletonClass as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
