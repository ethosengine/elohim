import { expect } from '@open-wc/testing';
import { UNKNOWN_CERTAINTY } from './certainty.js';
import type { ContentCertainty, CertaintyState } from './certainty.js';

describe('ContentCertainty / UNKNOWN_CERTAINTY', () => {
  it('defaults to state=unknown with no richness fields populated', () => {
    expect(UNKNOWN_CERTAINTY.state).to.equal('unknown');
    expect(UNKNOWN_CERTAINTY.freshness).to.be.undefined;
    expect(UNKNOWN_CERTAINTY.attestationCount).to.be.undefined;
  });

  it('is shape-valid against the ContentCertainty type', () => {
    const c: ContentCertainty = UNKNOWN_CERTAINTY;
    expect(c).to.exist;
  });

  it('enumerates the six CertaintyState values', () => {
    const states: CertaintyState[] = [
      'canonical',
      'partial',
      'stale',
      'contested',
      'unreachable',
      'unknown',
    ];
    expect(states).to.have.lengthOf(6);
  });
});
