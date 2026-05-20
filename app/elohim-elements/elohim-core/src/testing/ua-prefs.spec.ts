import { expect, fixture as fxFixture, html as fxHtml } from '@open-wc/testing';
import { LitElement } from 'lit';
import {
  setMediaQuery,
  clearMediaQueries,
  effectiveStimulusCeiling,
  measureLuminanceChanges,
} from './ua-prefs.js';

describe('ua-prefs harness', () => {
  afterEach(() => {
    clearMediaQueries();
  });

  describe('setMediaQuery', () => {
    it('forces prefers-reduced-motion to active when set to reduce', () => {
      setMediaQuery('prefers-reduced-motion', 'reduce');
      expect(matchMedia('(prefers-reduced-motion: reduce)').matches).to.be.true;
    });

    it('forces prefers-color-scheme', () => {
      setMediaQuery('prefers-color-scheme', 'dark');
      expect(matchMedia('(prefers-color-scheme: dark)').matches).to.be.true;
    });

    it('forces update: slow (e-paper simulation)', () => {
      setMediaQuery('update', 'slow');
      expect(matchMedia('(update: slow)').matches).to.be.true;
    });

    it('clearMediaQueries undoes overrides', () => {
      setMediaQuery('prefers-reduced-motion', 'reduce');
      clearMediaQueries();
      expect(matchMedia('(prefers-reduced-motion: reduce)').matches).to.be.false;
    });
  });

  describe('effectiveStimulusCeiling', () => {
    it('returns still when prefers-reduced-motion is reduce', () => {
      setMediaQuery('prefers-reduced-motion', 'reduce');
      expect(effectiveStimulusCeiling()).to.equal('still');
    });

    it('returns still when update is slow (e-paper)', () => {
      setMediaQuery('update', 'slow');
      expect(effectiveStimulusCeiling()).to.equal('still');
    });

    it('returns lively when no OS constraint applies', () => {
      expect(effectiveStimulusCeiling()).to.equal('lively');
    });
  });
});

class StableThing extends LitElement {
  override render() {
    return fxHtml`<div style="background: #fff; width: 50px; height: 50px;"></div>`;
  }
}
customElements.define('stable-thing', StableThing);

describe('photosensitive flash analyzer', () => {
  it('reports zero high-luminance changes for a still element', async () => {
    const el = await fxFixture<StableThing>(fxHtml`<stable-thing></stable-thing>`);
    const result = await measureLuminanceChanges(el, { sampleMs: 1000, sampleHz: 30 });
    expect(result.flashHz).to.be.lessThan(3);
    expect(result.exceedsThreshold).to.be.false;
  });

  // Note: we don't test the positive case in unit tests because it's timing-sensitive.
  // The presence of the API is what we're locking in.
});
