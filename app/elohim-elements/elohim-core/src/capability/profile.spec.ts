import { expect } from '@open-wc/testing';
import { DEFAULT_PROFILE } from './profile.js';
import type { CapabilityProfile } from './profile.js';

describe('CapabilityProfile / DEFAULT_PROFILE', () => {
  it('has Sabbath defaults: stimulus=still, textuality=textual', () => {
    expect(DEFAULT_PROFILE.stimulus).to.equal('still');
    expect(DEFAULT_PROFILE.textuality).to.equal('textual');
  });

  it('defaults theme/contrast/locale to auto', () => {
    expect(DEFAULT_PROFILE.theme).to.equal('auto');
    expect(DEFAULT_PROFILE.contrast).to.equal('auto');
    expect(DEFAULT_PROFILE.locale).to.equal('auto');
  });

  it('defaults lens to standard for adult-pilot baseline', () => {
    expect(DEFAULT_PROFILE.lens).to.equal('standard');
  });

  it('has an unstewarded pilot lock by default', () => {
    expect(DEFAULT_PROFILE.lock.kind).to.equal('pilot');
    expect(DEFAULT_PROFILE.origin).to.equal('pilot');
    expect(DEFAULT_PROFILE.standings).to.deep.equal([]);
  });

  it('is shape-valid against the CapabilityProfile type', () => {
    // Compile-time check: if the type and the constant disagree, tsc fails the build.
    const profile: CapabilityProfile = DEFAULT_PROFILE;
    expect(profile).to.exist;
  });
});
