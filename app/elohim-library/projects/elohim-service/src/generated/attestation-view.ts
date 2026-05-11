/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/attestation-view.schema.json -- DO NOT EDIT */

/**
 * Source of truth: Holochain DHT (elohim DNA, Content entry, content_type LIKE 'attestation:%', Category A per p2p-design-gate). This record is a read-optimised projection populated by the AttestationProjector post-commit signal. If this projection and the DHT disagree, the DHT wins.
 */
export interface AttestationView {
  /**
   * CID of this attestation (content-derived identity)
   */
  id: string;
  /**
   * ActionHash (hex) of the DHT entry — provenance anchor
   */
  dhtAnchorHash: string;
  /**
   * Discriminator matching attestation:<subtype>
   */
  attestationKind: string;
  /**
   * CID of the entity being attested
   */
  subjectCid: string;
  /**
   * Kind of the subject: agent | content | device | hub | computation | governance-action
   */
  subjectKind: string;
  /**
   * CID of the issuing agent
   */
  issuerCid: string;
  /**
   * CID of the parent governance-action, if this attestation is a vote
   */
  parentGovernanceActionCid?: string | null;
  /**
   * Vote value (null for non-votes)
   */
  voteValue?: 'approve' | 'reject' | 'abstain' | null;
  /**
   * Optional vote weight as a decimal string (null when unweighted)
   */
  voteWeight?: string | null;
  /**
   * Proof class: witness | self-attest | audit-signature | computational
   */
  proofClass: string;
  /**
   * Serialised proof evidence JSON (opaque string)
   */
  proofEvidenceJson: string;
  /**
   * Serialised full evidence JSON (opaque string)
   */
  evidenceJson: string;
  /**
   * ISO 8601 expiry timestamp, if any
   */
  expiresAt?: string | null;
  /**
   * CID of the attestation this one supersedes, if any
   */
  supersedesCid?: string | null;
  /**
   * Reason this attestation was revoked, if revoked
   */
  revocationReason?: string | null;
  /**
   * ISO 8601 revocation timestamp, if revoked
   */
  revokedAt?: string | null;
  /**
   * ISO 8601 creation timestamp
   */
  createdAt: string;
  /**
   * Manifest reference (e.g. mishpat, lamad)
   */
  manifestRef: string;
  /**
   * Human-readable title from the Content entry
   */
  title: string;
  /**
   * Optional description from the Content entry
   */
  description?: string | null;
}
