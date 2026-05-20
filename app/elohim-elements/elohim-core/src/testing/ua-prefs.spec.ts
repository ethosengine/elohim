import { expect } from '@open-wc/testing';
import {
  setMediaQuery,
  clearMediaQueries,
  effectiveStimulusCeiling,
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
