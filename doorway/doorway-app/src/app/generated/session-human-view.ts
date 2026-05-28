/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/session-human-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: derived projection of EconomicEvent stream × HumanProgress for a specific agent (Operational, Category C). Aggregates per-agent lamad activity counts from EconomicEvent entries filtered by lamadEventType. journeyStartedAt joined from HumanProgress. Computed by elohim-storage on Signal::EconomicEventCreated. Reconstructable from underlying EconomicEvent + HumanProgress entries in the elohim DNA content_store zome.
 */
export interface SessionHumanView {
  /**
   * AgentPubKey of the human whose session state this projects
   */
  agentId: string;
  /**
   * hApp identifier scoping this projection
   */
  hAppId: string;
  /**
   * Count of EconomicEvent entries with lamadEventType='content-view' for this agent
   */
  nodesViewed: number;
  /**
   * Count of EconomicEvent entries with lamadEventType='contributor-presence' (affinity signal) for this agent
   */
  nodesWithAffinity: number;
  /**
   * Count of EconomicEvent entries with lamadEventType='path-started' for this agent
   */
  pathsStarted: number;
  /**
   * Count of EconomicEvent entries with lamadEventType='path-completed' for this agent
   */
  pathsCompleted: number;
  /**
   * Count of EconomicEvent entries with lamadEventType='step-completed' for this agent
   */
  stepsCompleted: number;
  /**
   * ISO-8601 timestamp of the agent's first EconomicEvent entry; null if no events recorded yet
   */
  journeyStartedAt?: string | null;
  /**
   * ISO-8601 timestamp when this projection was last computed
   */
  computedAt: string;
}
