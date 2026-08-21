import { describe, it, expect } from 'vitest';
import {
  activationDecision,
  buildCoStewardCommitmentBody,
  distributionLadderRung,
  stewardAgentKeysByHousehold,
  DEFAULT_COMMONS_EPR_ID,
  DEFAULT_CO_STEWARD_HUMAN_ID,
  type HouseholdCard,
} from '../seed-household-costeward.js';

const BASE = {
  providerAgentKey: 'uhCAkJESSICAliveAgentKeyFromHerOwnPodAuthMe',
  contentId: DEFAULT_COMMONS_EPR_ID,
  reach: 'commons',
  householdId: 'household-dowell',
  coStewardHumanId: DEFAULT_CO_STEWARD_HUMAN_ID,
};

describe('buildCoStewardCommitmentBody', () => {
  it('authors the exact shape the replication fold classifies as Commons tier', () => {
    const body = buildCoStewardCommitmentBody(BASE);
    // classify() reads `action` alone to decide the commons tier.
    expect(body.action).toBe('replicates-commons');
    // parse_replicates_commons mirrors the EPR head_ref onto `recipient`, which
    // is what makes commitment_refs_covering able to scope this pledge per-CID.
    expect(body.receiver).toBe(DEFAULT_COMMONS_EPR_ID);
    expect(body.provider).toBe(BASE.providerAgentKey);
  });

  it('classifies the resource as a LIST, not a scalar', () => {
    // The commitment_backed_collectives scope test membership-checks the parsed
    // list in Rust; a scalar "content:commons" is silently missed (U1, the dark
    // card of 2026-06-19).
    const body = buildCoStewardCommitmentBody(BASE);
    expect(Array.isArray(body.resourceClassifiedAs)).toBe(true);
    expect(body.resourceClassifiedAs).toEqual(['content:commons']);
  });

  it('follows the row reach rather than hard-coding commons', () => {
    const body = buildCoStewardCommitmentBody({ ...BASE, reach: 'public' });
    expect(body.resourceClassifiedAs).toEqual(['content:public']);
  });

  it('is content-addressed, so a re-seed recognizes its own row', () => {
    expect(buildCoStewardCommitmentBody(BASE).id).toBe(buildCoStewardCommitmentBody(BASE).id);
  });

  it('mints a DIFFERENT id for a different co-steward, so drift cannot hide', () => {
    const other = buildCoStewardCommitmentBody({ ...BASE, providerAgentKey: 'uhCAkSOMEONEelse' });
    expect(other.id).not.toBe(buildCoStewardCommitmentBody(BASE).id);
  });

  it('scopes the pledge to the household', () => {
    expect(buildCoStewardCommitmentBody(BASE).inScopeOf).toEqual(['household-dowell']);
  });

  it('carries the head_ref under the DHT payload spelling the fold reads', () => {
    expect(buildCoStewardCommitmentBody(BASE).metadata['head_ref']).toBe(DEFAULT_COMMONS_EPR_ID);
    expect(buildCoStewardCommitmentBody(BASE).metadata['variant']).toBe('content');
  });

  it('omits a byte figure it could not measure ("absent beats fabricated")', () => {
    expect(buildCoStewardCommitmentBody(BASE).metadata).not.toHaveProperty('coStewardedBytes');
    expect(buildCoStewardCommitmentBody({ ...BASE, bytes: 4096 }).metadata['coStewardedBytes']).toBe(
      4096,
    );
  });
});

describe('distributionLadderRung', () => {
  it('treats a measured card with ZERO stewards as still needing a round', () => {
    // The regression this guards: a mesh whose conductor identities were
    // regenerated keeps shard_locations naming the PREVIOUS generation's agent
    // keys. The manifest exists (measured) but nothing joins `humans`, so both
    // folds are zero. Reading `distributionState` alone would call that done.
    const card: HouseholdCard = { distributionState: 'measured', stewardingCollectives: 0 };
    expect(distributionLadderRung(card)).toBe('redistribute');
  });

  it('is satisfied only by a non-zero steward count', () => {
    expect(distributionLadderRung({ stewardingCollectives: 1 })).toBe('satisfied');
    expect(distributionLadderRung({ distributionState: 'unmeasured' })).toBe('redistribute');
    expect(distributionLadderRung({})).toBe('redistribute');
  });
});

describe('stewardAgentKeysByHousehold', () => {
  const humans = [
    { id: 'human-matthew-manager', agentPubKey: 'uhCAkMATTHEW', householdId: 'household-dowell' },
    { id: 'human-jessica-spouse', agentPubKey: 'uhCAkJESSICAstale', householdId: 'household-dowell' },
    { id: 'human-gertrude-grandma', agentPubKey: 'uhCAkGERTRUDE', householdId: 'household-gertrude' },
    { id: 'human-adam-firstman', agentPubKey: null, householdId: 'household-eden' },
  ];

  it('represents each household exactly once, so the lever cannot inflate diversity', () => {
    const keys = stewardAgentKeysByHousehold(humans);
    expect(keys).toEqual(['uhCAkMATTHEW', 'uhCAkGERTRUDE']);
  });

  it('drops a human with no agent key rather than naming an unjoinable steward', () => {
    expect(stewardAgentKeysByHousehold(humans)).not.toContain(null);
    expect(stewardAgentKeysByHousehold([{ id: 'x', agentPubKey: 'uhCAkX', householdId: null }])).toEqual(
      [],
    );
  });

  it('prefers the LIVE pod key over the stale one cached on the row', () => {
    // Measured on the household mesh 2026-08-21: a content.db outliving its
    // conductor keeps a stale key on the canonical human-* row while the live
    // key lands on a separate agent:<key> row. shard_locations carries the LIVE
    // key, so the stale one joins nothing.
    const live = new Map([['human-matthew-manager', 'uhCAkMATTHEWlive']]);
    expect(stewardAgentKeysByHousehold(humans, live)[0]).toBe('uhCAkMATTHEWlive');
  });
});

describe('activationDecision', () => {
  it('recognizes an already-active row from a plain read, with no write', () => {
    // A redundant active→active PATCH still routes through the conductor write
    // path and gets shed 503 {"status":"catching-up"} under seed back-pressure.
    expect(activationDecision('active')).toBe('skip-active');
  });

  it('activates a proposed row (POST inserts `proposed` unconditionally)', () => {
    expect(activationDecision('proposed')).toBe('activate');
  });

  it('falls TOWARD the write on an unknown state, never silently skips', () => {
    expect(activationDecision('')).toBe('activate');
    expect(activationDecision(null)).toBe('missing');
    expect(activationDecision(undefined)).toBe('missing');
  });
});
