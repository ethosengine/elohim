/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/peer-capacity-view.schema.json -- DO NOT EDIT */

/**
 * Per-peer-device storage capacity rollup: raw hardware, free-for-self, pledged-per-tier (with multi-reach dedup at actual-bytes level), constitutional ratio compliance. Source of truth: computed projection from infrastructure:system-sample (raw capacity), Mishpat::Commitment entries (pledges), peer_blob_inventory (actually held). Operational Category C — no persisted entity; recomputable.
 */
export interface PeerCapacityView {
  peerCid: string;
  /**
   * ISO 8601.
   */
  computedAt: string;
  totalRawBytes: number;
  pledges: {
    dwellingBytes: number;
    collectiveBytes: number;
    commonsBytes: number;
    /**
     * Sum of per-tier pledges. May exceed totalRawBytes when commitments overlap on shards.
     */
    totalPledgedBytes: number;
    pledgesByRecipient?: {
      tier: 'dwelling' | 'collective' | 'commons';
      recipientHubId: string;
      commitmentCid: string;
      capacityBytes: number;
      /**
       * Only present for dwelling tier.
       */
      providerRole?: 'steward_mutual' | 'collective_steward';
    }[];
  };
  actuallyHeld: {
    /**
     * Distinct shard bytes held; shard satisfying multiple commitments counts once.
     */
    uniqueShardBytes: number;
    /**
     * totalRawBytes - uniqueShardBytes. Negative when peer is over-fetching.
     */
    freeBytesRemaining: number;
    fragmentationEstimate: number;
  };
  ratioCompliance: {
    effectiveRatios: Record<string, unknown>;
    currentRatios: Record<string, unknown>;
    compliantWithDonut: boolean;
    violations: {
      tier: 'commons' | 'dwelling' | 'collective' | 'free';
      violationKind: 'below_floor' | 'above_ceiling' | 'below_manifest_target' | 'above_manifest_target';
      currentPct: number;
      boundPct: number;
    }[];
  };
}
