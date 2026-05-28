/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/upgrade-prompt-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: derived projection of SessionHumanView × Manifest{kind:'onboarding'} for a specific agent (Operational, Category C). Reads the onboarding Manifest payload to discover which prompts exist and their trigger conditions, then joins with the agent's SessionHumanView to determine which prompts are currently active. Reconstructable from EconomicEvent stream + onboarding Manifest at any time.
 */
export interface UpgradePromptView {
  /**
   * AgentPubKey of the human whose upgrade prompts this projects
   */
  agentId: string;
  /**
   * hApp identifier scoping this projection
   */
  hAppId: string;
  /**
   * Ordered list of currently-active (non-dismissed) upgrade prompts derived from the onboarding Manifest
   */
  activePrompts: {
    /**
     * Stable identifier for this prompt as declared in the onboarding Manifest
     */
    promptId: string;
    /**
     * The lamadEventType or session condition that activated this prompt (e.g. 'path-started', 'return-visit', 'progress-at-risk')
     */
    trigger: string;
    /**
     * Short display title for the prompt
     */
    title: string;
    /**
     * Human-readable message explaining why the prompt appeared
     */
    message: string;
    /**
     * Bullet-point benefits the agent gains by upgrading (from the Manifest payload)
     */
    benefits: string[];
  }[];
  /**
   * ISO-8601 timestamp when this projection was last computed
   */
  computedAt: string;
}
