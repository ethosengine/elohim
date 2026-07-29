/**
 * Pure classification for the AgentPeerBinding provenance inventory.
 *
 * `get_agent_peer_bindings(anchor)` traverses an AgentToPeerBinding forward
 * link.  A binding is therefore correctly anchored only when the conductor
 * represented by that anchor belongs to the same human named in `agentCid`.
 * This module intentionally does not decide which side is authoritative; it
 * only makes the disagreement inspectable for the architecture ruling.
 */

export interface AgentPeerBindingView {
  actionHash: string;
  peerId: string;
  agentCid: string;
  validFromMicros: number;
  validUntilMicros: number | null;
  deviceArchetype: string;
  supersededBy: string | null;
}

export interface BindingAnchor {
  conductorUrl: string;
  anchorAgentPubKey: string;
  /** Null when the URL has no `elohim-<human>-<environment>` identity. */
  expectedHumanId: string | null;
}

export interface ObservedBinding extends AgentPeerBindingView, BindingAnchor {
  classification: 'correctly-anchored' | 'mis-provenanced' | 'unattributed-anchor';
  /** Correctly anchored action hashes for the same human/device/peer tuple. */
  contradictedBy: string[];
}

export interface ProvenanceInventory {
  rows: ObservedBinding[];
  summary: {
    observedBindings: number;
    correctlyAnchored: number;
    misProvenanced: number;
    unattributedAnchors: number;
    contradictedMisProvenanced: number;
  };
}

export interface AnchorBindings extends BindingAnchor {
  bindings: AgentPeerBindingView[];
}

function bindingIdentity(binding: AgentPeerBindingView): string {
  return [binding.agentCid, binding.peerId, binding.deviceArchetype].join('\u0000');
}

/**
 * Classify a snapshot returned by every reachable conductor.
 *
 * A contradiction means the same `(agentCid, peerId, deviceArchetype)` is
 * present under its expected human's anchor as well as under a different
 * anchor.  It is evidence for the data-reconciliation decision, not an
 * instruction to supersede or remove either row.
 */
export function inventoryBindingProvenance(
  anchors: AnchorBindings[],
): ProvenanceInventory {
  const correctlyAnchored = new Map<string, string[]>();

  for (const anchor of anchors) {
    if (!anchor.expectedHumanId) continue;
    for (const binding of anchor.bindings) {
      if (binding.agentCid !== anchor.expectedHumanId) continue;
      const identity = bindingIdentity(binding);
      const actions = correctlyAnchored.get(identity) ?? [];
      actions.push(binding.actionHash);
      correctlyAnchored.set(identity, actions);
    }
  }

  const rows = anchors.flatMap(anchor =>
    anchor.bindings.map(binding => {
      const classification: ObservedBinding['classification'] = !anchor.expectedHumanId ? 'unattributed-anchor'
        : binding.agentCid === anchor.expectedHumanId ? 'correctly-anchored'
        : 'mis-provenanced';
      const contradictedBy = classification === 'mis-provenanced'
        ? correctlyAnchored.get(bindingIdentity(binding)) ?? []
        : [];

      return { ...anchor, ...binding, classification, contradictedBy };
    }),
  );

  const misProvenanced = rows.filter(row => row.classification === 'mis-provenanced');
  return {
    rows,
    summary: {
      observedBindings: rows.length,
      correctlyAnchored: rows.filter(row => row.classification === 'correctly-anchored').length,
      misProvenanced: misProvenanced.length,
      unattributedAnchors: rows.filter(row => row.classification === 'unattributed-anchor').length,
      contradictedMisProvenanced: misProvenanced.filter(row => row.contradictedBy.length > 0).length,
    },
  };
}
