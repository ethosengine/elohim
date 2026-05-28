/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/entry-link-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: Holochain agent source chain in imagodei DNA (Agent-Scoped, Category B). Links are authored by the agent and live on its source chain. This view is read via the `query_my_source_chain_links` coordinator function and served by GET /api/v1/source-chain/{agentId}/links.
 */
export interface EntryLinkView {
  /**
   * Hex-encoded ActionHash of the create-link action on the source chain.
   */
  linkHash: string;
  /**
   * Hex-encoded hash of the base of the link (AnyLinkableHash).
   */
  baseHash: string;
  /**
   * Hex-encoded hash of the target of the link (AnyLinkableHash).
   */
  targetHash: string;
  /**
   * Link type discriminator as a string derived from the link_type field of the CreateLink action.
   */
  linkType: string;
  /**
   * Optional serialized tag bytes, hex-encoded. Null when the link was created without a tag.
   */
  tag?: string | null;
  /**
   * Hex-encoded AgentPubKey that authored this link.
   */
  authorAgent: string;
  /**
   * ISO-8601 timestamp when this link was created on the source chain.
   */
  timestamp: string;
  /**
   * True when a delete-link action for this link hash appears later in the source chain.
   */
  deleted: boolean;
  /**
   * ISO-8601 timestamp of the delete-link action. Null when not deleted.
   */
  deletedAt?: string | null;
}
