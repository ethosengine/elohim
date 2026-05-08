/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/household-devices-view.schema.json -- DO NOT EDIT */

/**
 * Domains an elohim is primed for via constitution or fine-tuning. Core specialties are protocol-known and affect dispatch routing. Extensible specialties are community-declared and accepted by storage without DNA validation.
 */
export type ElohimSpecialty =
  | 'child-safety'
  | 'family-dynamics'
  | 'content-safety'
  | 'discernment'
  | 'reach-evaluation'
  | 'medical'
  | 'legal'
  | 'crisis'
  | 'education'
  | 'code-review'
  | 'financial-advice'
  | 'creative-writing'
  | 'technical-documentation'
  | 'translation'
  | 'poetry'
  | 'philosophy'
  | 'governance'
  | 'conflict-resolution'
  | 'curriculum-design'
  | 'counseling'
  | 'theology'
  | 'science'
  | 'mathematics'
  | 'history'
  | 'language-learning'
  | 'mental-health'
  | 'disability-support'
  | 'elder-care'
  | 'research'
  | 'journalism';
/**
 * Named gate capabilities an elohim can dispatch. Core skills map directly to registered gate interfaces the protocol orchestrates (content-safety-gate, discernment-gate-v1-mechanical, reach-gate-v1). Extensible skills map to the full ElohimCapability variant set and any community-registered capabilities.
 */
export type ElohimSkill =
  | 'content-safety-review'
  | 'discernment-evaluation'
  | 'reach-negotiation'
  | 'attestation-recommendation'
  | 'spiral-detection'
  | 'care-connection'
  | 'graduated-intervention'
  | 'constitutional-verification'
  | 'accuracy-verification'
  | 'knowledge-map-synthesis'
  | 'affinity-analysis'
  | 'path-recommendation'
  | 'cross-layer-validation'
  | 'existential-boundary-enforcement'
  | 'governance-ratification'
  | 'path-analysis'
  | 'learning-objective-validation'
  | 'prerequisite-verification'
  | 'mastery-assessment-design'
  | 'family-value-alignment'
  | 'personal-agent-support'
  | 'feedback-profile-negotiation'
  | 'feedback-profile-enforcement'
  | 'feedback-profile-upgrade'
  | 'feedback-profile-downgrade'
  | 'place-attestation'
  | 'place-naming-governance'
  | 'geographic-reach-assignment'
  | 'bioregional-enforcement';
/**
 * Observed strengths accrued from attestation history. Core strengths are protocol-named reputation dimensions the imagodei layer tracks. Extensible strengths are community-named emergent patterns. Phase 10 ships a minimal core vocabulary; Phase 11+ will codify additional values from real attestation-history analysis.
 */
export type ElohimStrength =
  | 'high-confidence-judgments'
  | 'appeals-sustained'
  | 'consensus-alignment'
  | 'novel-pattern-detection'
  | 'steady-baseline'
  | 'consistent-constitutional-reasoning'
  | 'low-false-positive-rate'
  | 'fast-resolution'
  | 'cross-context-consistency'
  | 'escalation-accuracy';
/**
 * Kind of server-side renderer a doorway carries. Reserved values are valid claim values; only `angular-ssr` is implemented in elohim-render today. Source of truth: doorway runtime (Category C operational). Not a DHT entry type.
 */
export type RendererKind =
  | 'angular-ssr'
  | 'react-rsc'
  | 'vue-ssr'
  | 'svelte-ssr'
  | 'lit-ssr'
  | 'static-html';
/**
 * Kind of server-side renderer a doorway carries. Reserved values are valid claim values; only `angular-ssr` is implemented in elohim-render today. Source of truth: doorway runtime (Category C operational). Not a DHT entry type.
 */
export type RendererKind1 =
  | 'angular-ssr'
  | 'react-rsc'
  | 'vue-ssr'
  | 'svelte-ssr'
  | 'lit-ssr'
  | 'static-html';

/**
 * Devices (nodes) belonging to a household with live peer vitals. Source of truth: computed projection from stewarded_nodes (projects NodeRegistration DHT entry) LEFT JOIN peer_statuses (projects PeerStatus DHT entry). Operational Category C — no persistence, no new entry type.
 */
export interface HouseholdDevicesView {
  /**
   * Collectives DHT entry with kind:household; operational projection key.
   */
  householdId: string;
  devices: {
    shape: NodeShapeView;
    peer: null | PeerStatusView;
  }[];
}
/**
 * Projects NodeRegistration DHT entry; source of truth on DHT.
 */
export interface NodeShapeView {
  nodeId: string;
  hostname: string;
  deviceArchetypeId: string;
  householdId: string;
  role: 'edge' | 'archival' | 'inference' | 'doorway';
  capabilityLevel: number;
  committed: {
    cpuCores: number;
    memoryGb: number;
    storageTb: number;
    bandwidthMbps?: number;
    maxCustodyGb?: number;
    canSteward?: boolean;
    canInfer?: boolean;
    canDoorway?: boolean;
  };
  stewardTier?: 'caretaker' | 'guardian' | 'steward' | 'pioneer';
  custodianOptIn?: boolean;
  region?: string | null;
  signature: string;
  signedAt: string;
  dhtAnchorHash?: string | null;
}
/**
 * Projects PeerStatus DHT entry; source of truth on DHT.
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
  /**
   * If this peer runs a doorway that can server-render, advertises its render-capability profile. Null or absent = no SSR (storage-only peer, or operator opted out). Layered post-construction by build_peer_status_view (Category C operational, NOT a DHT entry).
   */
  renderCapability?: RenderCapabilityProfile | null;
  /**
   * Tier-2 extension capabilities. Apps register kebab-case capability names; each entry has a schemaRef + structured profile. Layered post-construction by build_peer_status_view (Category C operational, NOT a DHT entry).
   */
  extensions?: CapabilityExtensions | null;
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
   * Domains this elohim is primed for via constitution or fine-tuning. Values constrained to elohim-specialty vocabulary (Task 10.3). Core values affect gate dispatch routing; extensible values inform peer-diversity selection
   */
  specialties?: ElohimSpecialty[];
  /**
   * Named gate capabilities this elohim can dispatch. Values constrained to elohim-skill vocabulary (Task 10.3). Core values map to active gate interfaces (content-safety-gate, discernment-gate-v1-mechanical, reach-gate-v1); extensible values map to ElohimCapability variants
   */
  skills?: ElohimSkill[];
  /**
   * Observed strengths accrued from attestation history. Values constrained to elohim-strength vocabulary (Task 10.3). Populated by the protocol over time as gate decisions accumulate; operator-initialized as empty
   */
  strengths?: ElohimStrength[];
  /**
   * ISO 8601 timestamp when this elohim instance became active at this peer
   */
  activeSince: string;
  /**
   * Reach of this specific elohim instance. Distinct from the peer node's overall reach — an elohim running at a well-connected node may still have limited reach for certain gate decisions. Null until reach attestations accrue
   */
  reachLevel?: string | null;
}
/**
 * Source of truth: auto-derived at doorway startup from on-disk bundles intersected with elohim-storage's manifest of SSR-eligible routes (Operational, Category C). doorway-config.toml may reduce the claim but never inflate it. Layered into PeerStatusView via build_peer_status_view, mirroring the elohimCapability pattern. NOT a DHT entry.
 */
export interface RenderCapabilityProfile {
  /**
   * @minItems 1
   */
  bundles: [
    {
      /**
       * Bundle name (e.g. lamad-app, qahal-app)
       */
      name: string;
      /**
       * Semver bundle version
       */
      version: string;
      renderer: RendererKind;
      /**
       * Optional sha256 hash of the bundle file
       */
      digest?: string | null;
    },
    ...{
      /**
       * Bundle name (e.g. lamad-app, qahal-app)
       */
      name: string;
      /**
       * Semver bundle version
       */
      version: string;
      renderer: RendererKind;
      /**
       * Optional sha256 hash of the bundle file
       */
      digest?: string | null;
    }[],
  ];
  /**
   * Distinct renderer kinds (deduplicated bundles[].renderer). Cheap-to-query summary.
   *
   * @minItems 1
   */
  renderers: [RendererKind1, ...RendererKind1[]];
  /**
   * Auth modes this doorway honors for SSR. 'anonymous' must always be present.
   *
   * @minItems 1
   */
  authModes: [
    'anonymous' | 'doorway-hosted' | 'steward-presence',
    ...('anonymous' | 'doorway-hosted' | 'steward-presence')[],
  ];
  /**
   * Operator-declared concurrency budget. CSR fallback fires when reached.
   */
  maxConcurrentRenders: number;
  /**
   * Operator-declared memory ceiling for the renderer (informational; null = cgroup-managed)
   */
  memoryBudgetMib?: number | null;
}
/**
 * Tier-2 capability claims map. Source of truth: each capability's owner declares it via runtime registration (Category C operational). Each key is a kebab-case capability name registered in the protocol's capability registry. Each value carries a schemaRef pointer (so consumers can resolve the profile schema) and an opaque structured profile. The validator checks structural well-formedness only — claim CONTENTS are interpreted by consumers who recognize the capability name. When a Tier-2 capability proves load-bearing, it graduates to Tier 1 with a typed sibling field on peer-status. NOT a DHT entry.
 */
export interface CapabilityExtensions {
  /**
   * This interface was referenced by `CapabilityExtensions`'s JSON-Schema definition
   * via the `patternProperty` "^[a-z][a-z0-9-]{2,30}$".
   */
  [k: string]: {
    /**
     * Schema URI for this capability's profile (e.g., 'epr:schema:view:transcode-capability-profile')
     */
    schemaRef: string;
    /**
     * Capability-specific claim. Shape defined by schemaRef. Validator checks 'is an object'; consumers do deep validation.
     */
    profile: Record<string, unknown>;
  };
}
