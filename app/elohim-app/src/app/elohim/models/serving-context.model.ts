/**
 * ServingContext — a dimension orthogonal to reach: the system state an EPR
 * is being projected through. Rendered by the protocol-omni serving-context
 * segment (trust surface). Read-only view-model over existing build/config
 * values — nothing persisted, no entity (spec §2, §5.1).
 *
 * `variant` is RESERVED: EPR-natively a variant is WHICH project-epr
 * commitment / bundle CID served you (blue/green, A/B) — it fills from
 * substrate provenance when spec §9.7 lands, never from k8s vocabulary.
 */
export interface ServingContext {
  readonly tier?: 'development' | 'alpha' | 'staging' | 'production';
  readonly logLevel?: string;
  /** Short gitHash today → doorway-attested bundle CID when spec §9.7 lands. */
  readonly buildId?: string;
  readonly variant?: string;
}
