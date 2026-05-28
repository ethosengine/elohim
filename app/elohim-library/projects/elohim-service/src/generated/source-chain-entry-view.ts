/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/source-chain-entry-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: Holochain agent source chain in imagodei DNA (Agent-Scoped, Category B). Entries are private to the authoring agent's source chain; they are never gossiped to the DHT. This view is read via the `query_my_source_chain` coordinator function and served by GET /api/v1/source-chain/{agentId}/entries.
 */
export interface SourceChainEntryView {
  /**
   * Hex-encoded ActionHash from the Holochain source chain record header.
   */
  actionHash: string;
  /**
   * Hex-encoded EntryHash (hash of the serialized entry content).
   */
  entryHash: string;
  /**
   * Hex-encoded AgentPubKey that authored this entry.
   */
  authorAgent: string;
  /**
   * Entry type discriminator as a string (e.g. 'Human', 'ContributorPresence', 'ContentMastery'). Derived from the record's action entry_type.
   */
  entryType: string;
  /**
   * JSON-serialized entry content. Null for App entries that cannot be decoded to a known type, or for system entries (Dna, AgentValidationPkg, InitZomesComplete).
   */
  contentJson?: string | null;
  /**
   * Hex-encoded ActionHash of the previous action in the chain. Null for the genesis record.
   */
  prevActionHash?: string | null;
  /**
   * Zero-based sequence number of this record in the agent's source chain.
   */
  sequence: number;
  /**
   * ISO-8601 timestamp when this entry was committed to the source chain.
   */
  timestamp: string;
}
