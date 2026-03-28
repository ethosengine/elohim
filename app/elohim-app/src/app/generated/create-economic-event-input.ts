/* Generated from protocol schema: inputs/create-economic-event-input.schema.json -- DO NOT EDIT */

/**
 * Input for creating REA economic events. Must match Rust CreateEconomicEventInputView in views.rs. Economic events are Category A (DHT-notarized) — they are the protocol's value accounting layer.
 */
export interface CreateEconomicEventInput {
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
   * ResourceSpecification URI the affected resource conforms to
   */
  resourceConformsTo?: string;
  /**
   * EconomicResource being acted upon
   */
  resourceInventoriedAs?: string;
  /**
   * Classification URIs for the resource
   */
  resourceClassifiedAs?: string[];
  /**
   * Quantity of resource affected
   */
  resourceQuantityValue?: number;
  /**
   * Unit of measure for resource quantity
   */
  resourceQuantityUnit?: string;
  /**
   * Effort expended (e.g. time spent)
   */
  effortQuantityValue?: number;
  /**
   * Unit of measure for effort
   */
  effortQuantityUnit?: string;
  /**
   * ISO 8601 timestamp when event occurred
   */
  hasPointInTime?: string;
  /**
   * ISO 8601 duration of the event
   */
  hasDuration?: string;
  /**
   * Process this event is an input to
   */
  inputOf?: string;
  /**
   * Process this event is an output of
   */
  outputOf?: string;
  /**
   * App-specific event classification (e.g. content-view, assessment-complete)
   */
  lamadEventType?: string;
  /**
   * Content this event relates to
   */
  contentId?: string;
  /**
   * Contributor presence this event relates to
   */
  contributorPresenceId?: string;
  /**
   * Learning path this event relates to
   */
  pathId?: string;
  /**
   * ID of the signal or event that triggered this
   */
  triggeredBy?: string;
  /**
   * Human-readable note about the event
   */
  note?: string;
  /**
   * Domain-specific metadata (parsed object, not stringified)
   */
  metadata?: Record<string, unknown>;
  /**
   * Place ID for spatial grounding of this event
   */
  atLocation?: string;
}
