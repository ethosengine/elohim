/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/portal-host-view.schema.json -- DO NOT EDIT */

/**
 * M5 ships only 'trusted'
 */
export type Reach = 'private' | 'self' | 'intimate' | 'trusted' | 'familiar' | 'community' | 'public' | 'commons';

/**
 * Source of truth: DHT (Notarized, Category A). PortalHost declares URLs authorized to render this human's auth portal. Anchored on the Human entry's ActionHash so portal hosts survive KeyRotation. M5 ships only reach=trusted.
 */
export interface PortalHostView {
  /**
   * ActionHash (base64url) of the Human entry this PortalHost anchors on
   */
  humanId: string;
  hostUrl: string;
  label?: string | null;
  addedAt: string;
  /**
   * Operational enrichment from libp2p; not part of notarized entry
   */
  lastReachableAt?: string | null;
  reach: Reach;
  /**
   * ActionHash (base64url) of the PortalHost entry; canonical PK
   */
  dhtAnchorHash: string;
}
