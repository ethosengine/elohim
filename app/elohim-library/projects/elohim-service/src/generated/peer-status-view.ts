/* Generated from protocol schema: views/peer-status-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: Holochain infrastructure DNA DHT (Notarized, Category A). This view is a read-optimized projection from the peer_statuses SQLite table, itself populated by InfrastructureSignal::PeerStatusRecorded post-commit projections. If the projection and the DHT disagree, the DHT wins.
 */
export interface PeerStatusView {
  /**
   * Agent pubkey of the peer (base64url encoded). Matches the agent_pub_key in the infrastructure DNA
   */
  peerId: string;
  /**
   * Peer availability status as last advertised via heartbeat
   */
  status: 'online' | 'degraded' | 'leaving' | 'offline';
  /**
   * True when this peer is available to serve as a general-pool request target. Used by the doorway forwarder for candidate selection
   */
  generalPoolMember: boolean;
  /**
   * True when this peer is accepting new stewardship reserve allocations
   */
  acceptingStewardshipReserves: boolean;
  /**
   * Operator-declared node archetype. E.g. home-nuc, blade, desktop, cloud-vps. Null when the peer has not declared an archetype
   */
  archetypeClass?: string | null;
  /**
   * Holochain action timestamp in microseconds since Unix epoch (u64 as string — exceeds JS safe integer range)
   */
  timestamp: string;
  /**
   * ActionHash of the PeerStatus DHT entry this projection was built from (base64url). Allows callers to verify provenance against the DHT
   */
  dhtAnchorHash: string;
  /**
   * Unix microseconds when this projection row was last written (u64 as string)
   */
  updatedAt: string;
  /**
   * If this peer runs an elohim-agent, this block advertises its model-level capabilities. Null or absent means no elohim runs here (e.g., pure storage/relay node). Operator-configured in Phase 9; auto-detected in Phase 10+
   */
  elohimCapability?: ElohimCapabilityProfile | null;
}
/**
 * Source of truth: operator-configured at elohim-storage startup (Operational, Category C). Not auto-detected — the operator declares the model running here. Phase 10+ may derive this automatically from a live elohim-agent-service.
 */
export interface ElohimCapabilityProfile {
  /**
   * Fully-qualified model name. E.g. claude-opus-4-7, llama-3.1-70b-q4, gpt-4o
   */
  modelName: string;
  /**
   * Model family/vendor. E.g. claude, llama, gpt, mistral, gemini
   */
  modelFamily: string;
  /**
   * Maximum input context in tokens for this model instance
   */
  contextWindowTokens: number;
  /**
   * CID of the constitution document priming this elohim. Null when no specific constitution is applied (base model behavior)
   */
  constitutionCid?: string | null;
  /**
   * Quantization descriptor. E.g. q4_K_M, q8_0, bf16, f32. Null for hosted/closed-weight models where quantization is not operator-visible
   */
  quantizationSpec?: string | null;
  /**
   * Free-form host descriptor indicating where this elohim runs. E.g. tauri-desktop-mac-arm64, elohim-node-linux-x86_64, doorway-hosted-cloud
   */
  deploymentContext?: string | null;
  /**
   * Domains this elohim is primed for via constitution or fine-tuning. E.g. child-safety, medical, code-review, curriculum-design. Free-form in Phase 9; Phase 10+ may enumerate a controlled vocabulary
   */
  specialties?: string[];
  /**
   * Named capabilities this elohim can dispatch. E.g. content-safety-review, discernment-evaluation, mastery-assessment. Maps to gate names or service names that gate-client uses for dispatch
   */
  skills?: string[];
  /**
   * Observed strengths accrued from attestation history. Populated by the protocol over time as gate decisions accumulate; operator-initialized as empty
   */
  strengths?: string[];
  /**
   * ISO 8601 timestamp when this elohim instance became active at this peer
   */
  activeSince: string;
  /**
   * Reach of this specific elohim instance. Distinct from the peer node's overall reach — an elohim running at a well-connected node may still have limited reach for certain gate decisions. Null until reach attestations accrue
   */
  reachLevel?: string | null;
}
