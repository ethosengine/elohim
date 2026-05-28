/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/content-engagement-stats-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: derived projection of EconomicEvent stream filtered by (content_id, lamadEventType IN ['content-view', 'content-complete']) (Operational, Category C). Computed by elohim-storage on Signal::EconomicEventCreated. Not persisted as DHT; reconstructable from the underlying EconomicEvent entries in the elohim DNA content_store zome.
 */
export interface ContentEngagementStatsView {
  /**
   * The identifier of the content this engagement projection covers
   */
  contentId: string;
  /**
   * Total count of EconomicEvent entries with lamadEventType='content-view' for this contentId
   */
  views: number;
  /**
   * Total count of EconomicEvent entries with lamadEventType='content-complete' for this contentId
   */
  completions: number;
  /**
   * Count of distinct provider (agent) values across content-view EconomicEvents for this contentId
   */
  uniqueViewers: number;
  /**
   * Ratio of completions to views in [0, 1]. Zero when views == 0.
   */
  completionRate: number;
  /**
   * ISO-8601 timestamp when this projection was last computed
   */
  computedAt: string;
}
