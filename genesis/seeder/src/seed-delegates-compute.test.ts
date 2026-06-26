import { describe, it, expect } from 'vitest';
import { assertBoundedMinimum } from './seed-delegates-compute.js';

describe('assertBoundedMinimum (spec §14 minimum-bounds guard)', () => {
  it('rejects epr_scope ["*"] with omitted rate', () => {
    expect(() => assertBoundedMinimum({ epr_scope: ['*'], reach_ceiling: 'commons', rotation_ttl_days: 30 } as any))
      .toThrow(/rate_per_hour/i);
  });
  it('rejects epr_scope ["*"] with omitted ttl', () => {
    expect(() => assertBoundedMinimum({ epr_scope: ['*'], reach_ceiling: 'commons', rate_per_hour: 60 } as any))
      .toThrow(/rotation_ttl_days/i);
  });
  it('rejects reach_ceiling outside {commons,community} without acknowledgement', () => {
    expect(() => assertBoundedMinimum({ epr_scope: ['epr:x'], reach_ceiling: 'public', rate_per_hour: 60, rotation_ttl_days: 30 }))
      .toThrow(/reach_elevation_acknowledged/i);
  });
  it('accepts community ceiling without acknowledgement', () => {
    expect(() => assertBoundedMinimum({ epr_scope: ['epr:x'], reach_ceiling: 'community', rate_per_hour: 60, rotation_ttl_days: 30 }))
      .not.toThrow();
  });
  it('accepts a bounded wildcard contract (finite rate + ttl + commons)', () => {
    expect(() => assertBoundedMinimum({ epr_scope: ['*'], reach_ceiling: 'commons', rate_per_hour: 60, rotation_ttl_days: 30 }))
      .not.toThrow();
  });
});
