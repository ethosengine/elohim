/**
 * W3C Verifiable Credentials - Structure Alignment
 *
 * This defines the W3C VC structure WITHOUT implementing full VC functionality.
 * Goal: Prevent tech debt by aligning attestation structure with W3C spec.
 *
 * Key Insight: Our existing Attestation model already matches W3C VC structure!
 * This type documents the standard so we can serialize to VC format later.
 *
 * Reference: https://www.w3.org/TR/vc-data-model/
 */

/**
 * W3C Verifiable Credential structure (minimal)
 *
 * Verifiable Credentials are tamper-evident credentials that can be
 * cryptographically verified. They're designed for decentralized systems
 * where credentials need to be portable and trustworthy.
 *
 * Our existing Attestation model already has most of this data,
 * we're just documenting the W3C-aligned shape.
 *
 * Example mapping from Lamad Attestation to VC:
 * ```typescript
 * const attestation: Attestation = {
 *   id: 'abc123',
 *   earnedAt: '2024-01-15T10:00:00Z',
 *   expiresAt: '2025-01-15T10:00:00Z',
 *   issuedBy: 'did:web:elohim-protocol.org:system',
 *   proof: 'placeholder-signature'
 * };
 *
 * // Can be serialized as:
 * const vc: VerifiableCredential = {
 *   '@context': ['https://www.w3.org/2018/credentials/v1'],
 *   type: ['VerifiableCredential', 'LamadAttestation'],
 *   id: attestation.id,
 *   issuer: attestation.issuedBy,
 *   issuanceDate: attestation.earnedAt,
 *   expirationDate: attestation.expiresAt,
 *   credentialSubject: {
 *     id: 'did:web:elohim-protocol.org:agents:session123',
 *     attestationName: attestation.name,
 *     // ... other attestation fields
 *   },
 *   proof: {
 *     type: 'HolochainSignature2024',
 *     created: attestation.earnedAt,
 *     verificationMethod: `${attestation.issuedBy}#keys-1`,
 *     proofPurpose: 'assertionMethod',
 *     proofValue: attestation.proof
 *   }
 * };
 * ```
 */
export interface VerifiableCredential {
  /**
   * JSON-LD context (required by W3C VC spec)
   *
   * Typically:
   * - 'https://www.w3.org/2018/credentials/v1' (required)
   * - Additional custom contexts for domain-specific claims
   */
  '@context': string | string[];

  /**
   * Credential type (required)
   *
   * Must include 'VerifiableCredential' plus optional specific types.
   * Examples:
   * - ['VerifiableCredential', 'LamadAttestation']
   * - ['VerifiableCredential', 'EducationalCredential']
   */
  type: string[];

  /**
   * Unique credential identifier (optional but recommended)
   *
   * Examples:
   * - 'urn:uuid:abc123'
   * - 'https://elohim-protocol.org/credentials/abc123'
   */
  id?: string;

  /**
   * DID of the issuer
   *
   * In Lamad:
   * - System-issued: 'did:web:elohim-protocol.org:system'
   * - Steward-issued: 'did:web:elohim-protocol.org:stewards:{id}'
   * - Future Holochain: 'did:holochain:{AgentPubKey}'
   */
  issuer: string;

  /**
   * When the credential was issued (ISO 8601)
   *
   * Maps to Attestation.earnedAt
   */
  issuanceDate: string;

  /**
   * When the credential expires (optional, ISO 8601)
   *
   * Maps to Attestation.expiresAt
   * Some attestations never expire (relational, time-based)
   */
  expirationDate?: string;

  /**
   * The credential subject (who/what the claims are about)
   *
   * In Lamad, this is the agent earning the attestation.
   * The `id` field should be a DID.
   */
  credentialSubject: {
    /** DID of the subject (the agent earning the attestation) */
    id: string;

    /**
     * Claims about the subject
     *
     * For Lamad attestations, this includes:
     * - attestationName: string
     * - attestationDescription: string
     * - attestationType: AttestationType
     * - journey: AttestationJourney (proof of learning)
     */
    [key: string]: any;
  };

  /**
   * Cryptographic proof (optional in spec, but critical for trust)
   *
   * MVP: Placeholder strings
   * Holochain: Will use Holochain signatures
   * Future: May use JWS (JSON Web Signature) or LD-Signatures
   */
  proof?: {
    /** Proof type (e.g., 'Ed25519Signature2020', 'HolochainSignature2024') */
    type: string;

    /** When the proof was created (ISO 8601) */
    created: string;

    /**
     * Verification method (DID URL or public key reference)
     *
     * Example: 'did:web:elohim-protocol.org:system#keys-1'
     */
    verificationMethod: string;

    /**
     * Proof purpose
     *
     * Common values:
     * - 'assertionMethod': For issuing credentials
     * - 'authentication': For proving identity
     */
    proofPurpose: string;

    /** The actual signature (hex, base64, or JWS format) */
    proofValue?: string;
  };
}
