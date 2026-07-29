import { describe, expect, it } from 'vitest';

import {
  inventoryBindingProvenance,
  type AnchorBindings,
} from '../agent-peer-binding-provenance.js';

const binding = (actionHash: string, agentCid: string, peerId = 'peer-1') => ({
  actionHash,
  agentCid,
  peerId,
  validFromMicros: 1,
  validUntilMicros: null,
  deviceArchetype: 'desktop',
  supersededBy: null,
});

describe('inventoryBindingProvenance', () => {
  it('identifies Adam-anchored rows for another human and their correct counterpart', () => {
    const anchors: AnchorBindings[] = [
      {
        conductorUrl: 'ws://elohim-adam-alpha:4445',
        anchorAgentPubKey: 'adam-key',
        expectedHumanId: 'human-adam-firstman',
        bindings: [binding('wrong-matthew', 'human-matthew-manager')],
      },
      {
        conductorUrl: 'ws://elohim-matthew-alpha:4445',
        anchorAgentPubKey: 'matthew-key',
        expectedHumanId: 'human-matthew-manager',
        bindings: [binding('right-matthew', 'human-matthew-manager')],
      },
    ];

    const inventory = inventoryBindingProvenance(anchors);

    expect(inventory.summary).toEqual({
      observedBindings: 2,
      correctlyAnchored: 1,
      misProvenanced: 1,
      unattributedAnchors: 0,
      contradictedMisProvenanced: 1,
    });
    expect(inventory.rows[0]).toMatchObject({
      classification: 'mis-provenanced',
      contradictedBy: ['right-matthew'],
    });
  });

  it('keeps unknown URL anchors visible without asserting they are wrong', () => {
    const inventory = inventoryBindingProvenance([{
      conductorUrl: 'ws://localhost:4445',
      anchorAgentPubKey: 'local-key',
      expectedHumanId: null,
      bindings: [binding('local', 'human-adam-firstman')],
    }]);

    expect(inventory.summary.unattributedAnchors).toBe(1);
    expect(inventory.rows[0].classification).toBe('unattributed-anchor');
  });
});
