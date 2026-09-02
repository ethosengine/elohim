import { describe, expect, it } from 'vitest';

import { buildSpoolCustodyInput } from '../seed-spool-custody.js';

describe('buildSpoolCustodyInput', () => {
  it('is deterministic and matches the Rust spool custody id vector', () => {
    const bounds = {
      maxBytes: 64 << 20,
      atomsPerHour: 120,
      retentionDays: 90,
    };

    const input = buildSpoolCustodyInput({
      providerAgent: 'uhCAkA',
      receiverAgent: 'uhCAkB',
      collectiveCid: 'collective:uhCkkHousehold',
      bounds,
    });

    expect(input).toEqual({
      id: 'custody-spool-4c267bbf6ea97775',
      action: 'custody-spool',
      provider: 'uhCAkA',
      receiver: 'uhCAkB',
      resource_classified_as: ['spool:witness:uhCAkB'],
      resource_quantity_value: 64 << 20,
      resource_quantity_unit: 'B',
      in_scope_of: ['collective:uhCkkHousehold'],
      note: "spool custody: uhCAkA holds uhCAkB's witnesses",
      metadata_json: JSON.stringify({
        seedGeneration: 'spool-custody',
        kind: 'custody-spool',
        bounds,
      }),
    });
    expect(
      buildSpoolCustodyInput({
        providerAgent: 'uhCAkA',
        receiverAgent: 'uhCAkB',
        collectiveCid: 'collective:uhCkkHousehold',
        bounds,
      }),
    ).toEqual(input);
  });
});
