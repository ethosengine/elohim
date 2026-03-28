/* Generated from protocol schema: views/economic-event-view.schema.json -- DO NOT EDIT */

/**
 * REA economic event as returned by the storage API. Category A (DHT-notarized). Source of truth: DHT anchor, projected to storage for query.
 */
export interface EconomicEventView {
  /**
   * Unique event identifier
   */
  id: string;
  /**
   * Application scope (e.g. lamad)
   */
  appId?: string;
  /**
   * REA economic action (use, consume, produce, transfer, cite)
   */
  action: string;
  /**
   * Agent providing the resource
   */
  provider: string;
  /**
   * Agent or content receiving the resource
   */
  receiver: string;
  /**
   * ResourceSpecification URI
   */
  resourceConformsTo?: string;
  /**
   * EconomicResource being acted upon
   */
  resourceInventoriedAs?: string;
  /**
   * Classification URIs for the resource
   */
  resourceClassifiedAs?: string[] | Record<string, unknown>;
  resourceQuantityValue?: number;
  resourceQuantityUnit?: string;
  effortQuantityValue?: number;
  effortQuantityUnit?: string;
  /**
   * ISO 8601 timestamp
   */
  hasPointInTime?: string;
  /**
   * ISO 8601 duration
   */
  hasDuration?: string;
  inputOf?: string;
  outputOf?: string;
  lamadEventType?: string;
  contentId?: string;
  contributorPresenceId?: string;
  pathId?: string;
  triggeredBy?: string;
  /**
   * Event lifecycle state (pending, committed, fulfilled)
   */
  state: string;
  note?: string;
  /**
   * Domain-specific metadata (parsed object)
   */
  metadata?: Record<string, unknown>;
  /**
   * DHT entry hash linking this projection to the notarized record
   */
  dhtAnchorHash?: string;
  /**
   * ISO 8601 creation timestamp
   */
  createdAt: string;
  /**
   * Place ID for spatial grounding
   */
  atLocation?: string;
}
