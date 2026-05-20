import { expect } from '@open-wc/testing';
import { parseStandingRequirement, satisfiesRequirement } from './standing.js';

describe('Standing DSL', () => {
  describe('parseStandingRequirement', () => {
    it('parses a single Standing as one AND-group containing one OR-clause', () => {
      const parsed = parseStandingRequirement(['pilot']);
      expect(parsed).to.deep.equal([['pilot']]);
    });

    it('parses an OR string into a single AND-group with multiple OR-clauses', () => {
      const parsed = parseStandingRequirement(['pilot | steward']);
      expect(parsed).to.deep.equal([['pilot', 'steward']]);
    });

    it('parses array of Standings as multiple AND-groups (AND semantics)', () => {
      const parsed = parseStandingRequirement(['pilot', 'contributor']);
      expect(parsed).to.deep.equal([['pilot'], ['contributor']]);
    });

    it('parses mixed: array entries are AND, | within entry is OR', () => {
      const parsed = parseStandingRequirement(['pilot | steward', 'contributor']);
      expect(parsed).to.deep.equal([['pilot', 'steward'], ['contributor']]);
    });

    it('trims whitespace around | tokens', () => {
      const parsed = parseStandingRequirement(['pilot|steward', 'pilot |  contributor']);
      expect(parsed).to.deep.equal([
        ['pilot', 'steward'],
        ['pilot', 'contributor'],
      ]);
    });
  });

  describe('satisfiesRequirement', () => {
    it('passes when viewer holds the single required Standing', () => {
      expect(satisfiesRequirement(['pilot'], ['pilot'])).to.be.true;
    });

    it('fails when viewer lacks the required Standing', () => {
      expect(satisfiesRequirement(['steward'], ['pilot'])).to.be.false;
    });

    it('passes for OR: viewer holds one of several alternatives', () => {
      expect(satisfiesRequirement(['steward'], ['pilot | steward'])).to.be.true;
    });

    it('passes for AND: viewer holds all required Standings', () => {
      expect(satisfiesRequirement(['pilot', 'contributor'], ['pilot', 'contributor'])).to.be.true;
    });

    it('fails AND when viewer lacks one part', () => {
      expect(satisfiesRequirement(['pilot'], ['pilot', 'contributor'])).to.be.false;
    });

    it('handles mixed: AND of (pilot OR steward) AND contributor', () => {
      expect(satisfiesRequirement(['steward', 'contributor'], ['pilot | steward', 'contributor']))
        .to.be.true;
      expect(satisfiesRequirement(['steward'], ['pilot | steward', 'contributor'])).to.be.false;
    });

    it('returns true for empty requirement (vacuous)', () => {
      expect(satisfiesRequirement(['pilot'], [])).to.be.true;
    });
  });
});
