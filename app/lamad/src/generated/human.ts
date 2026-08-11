/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/human.schema.json -- DO NOT EDIT */

/**
 * Visibility of this human's profile
 */
export type Reach = 'private' | 'self' | 'intimate' | 'trusted' | 'familiar' | 'community' | 'public' | 'commons';

/**
 * Source of truth: DHT (Notarized, Category A). Projection of the imagodei Human DHT entry. Rebuildable via signal replay on HumanCreated/HumanUpdated signals. dhtAnchorHash is null for pre-coherence rows (seeded before DHT was live).
 */
export interface HumanView {
  /**
   * Stable human identifier (UUID or agent-derived)
   */
  id: string;
  /**
   * Holochain AgentPubKey (base64url). None before coherence or for custodial-only rows.
   */
  agentPubKey?: string | null;
  displayName: string;
  bio?: string | null;
  /**
   * Tag-style affinity labels (parsed from JSON text in DB)
   */
  affinities: string[];
  profileReach: Reach;
  location?: string | null;
  profilePhotoUrl?: string | null;
  /**
   * Holochain hApp instance identifier
   */
  hAppId: string;
  createdAt: string;
  updatedAt: string;
  /**
   * ActionHash (base64url) of the Human entry in imagodei DNA. None for pre-coherence rows.
   */
  dhtAnchorHash?: string | null;
  /**
   * Household collective id (projection of collectives DHT entry with kind:household). None for humans outside a household grouping. Short-term: settable at create-time via the create-human input while the DHT humans-replayer is a stub; otherwise filled by the household_backfill startup pass.
   */
  householdId?: string | null;
}
