/* Generated from protocol schema: views/node-shape-view.schema.json -- DO NOT EDIT */

/**
 * Durable node shape published by elohim-node at boot. Source of truth: node-registry DNA NodeRegistration entry; this view projects from stewarded_nodes SQLite table.
 */
export interface NodeShapeView {
  nodeId: string;
  hostname: string;
  deviceArchetypeId: string;
  householdId: string;
  role: 'edge' | 'archival' | 'inference' | 'doorway';
  capabilityLevel: number;
  committed: {
    cpuCores: number;
    memoryGb: number;
    storageTb: number;
    bandwidthMbps?: number;
    maxCustodyGb?: number;
    canSteward?: boolean;
    canInfer?: boolean;
    canDoorway?: boolean;
  };
  stewardTier?: 'caretaker' | 'guardian' | 'steward' | 'pioneer';
  custodianOptIn?: boolean;
  region?: string | null;
  signature: string;
  signedAt: string;
  dhtAnchorHash?: string | null;
}
