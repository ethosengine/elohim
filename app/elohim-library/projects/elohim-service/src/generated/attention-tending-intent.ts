/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: intents/attention-tending-intent.schema.json -- DO NOT EDIT */

/**
 * Source of truth: wire shape for POST /api/v1/attention/tending request body. Aligns with CreateAttentionTendingInput Rust struct in elohim/holochain/dna/elohim/zomes/content_store/src/attention_tending.rs. The substrate derives signer_pubkey from agent_info() — the client never supplies it. Entity classification: Category B (Agent-Scoped, private source chain) — AttentionTending entries are Visibility::Private and never gossiped to DHT. The create_attention_tending coordinator already exists; this is the HTTP intent wire shape only.
 */
export interface AttentionTendingIntent {
  /**
   * JSON-encoded FilterSubject. Must be syntactically valid JSON; the coordinator gates on parse failure. Identifies the content node, path, or agent whose reach the tending applies to.
   */
  filterSubjectJson: string;
  /**
   * Attention classification — why the agent's attention tended toward or away from this subject. Must match one of the values validated by the integrity zome.
   */
  classification: 'values-forward' | 'fatigue' | 'scope-mismatch' | 'safety';
  /**
   * Optional human-readable reason. Surfaced to the agent's own elohim for reflection; never shared cross-agent.
   */
  reason?: string;
  /**
   * Time-to-live in seconds. Minimum 3600 (1 hour) enforced by the integrity zome floor. The tending-policy Manifest may declare a higher minimum via min_ttl_seconds.
   */
  ttlSeconds: number;
  /**
   * JSON-encoded ContextScope. Must be syntactically valid JSON. Describes the context in which attention occurred (e.g., which pillar surface, session phase, collective context).
   */
  contextJson: string;
  /**
   * Elapsed time in milliseconds that the agent spent on the subject before this tending was recorded. The proxy route compares this against the tending-policy Manifest's dwell_qualification_ms before calling the coordinator. Replaces the client-side DWELL_THRESHOLD_MS = 3000 constant (F-POLICY-4 retirement).
   */
  elapsedMs: number;
}
