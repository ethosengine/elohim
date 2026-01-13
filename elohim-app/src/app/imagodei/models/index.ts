/**
 * Imago Dei Models - Identity Types
 *
 * Human identity, session management, attestations, doorway federation,
 * and social recovery.
 */

export * from './session-human.model';
export * from './profile.model';
export * from './attestations.model';
export * from './sovereignty.model';
// Re-export identity.model.ts selectively to avoid HumanProfile name collision
// The HumanProfile from profile.model.ts is the rich view model
// The HumanProfile from identity.model.ts is the simple Holochain entry (import directly if needed)
export {
  type IdentityMode,
  isStewardMode,
  normalizeIdentityMode,
  isNetworkMode,
  type KeyLocation,
  type KeyBackupStatus,
  ProfileReachLevels,
  type ProfileReach,
  type RegisterHumanRequest,
  type UpdateProfileRequest,
  type IdentityState,
  type HostingCostSummary,
  type NodeOperatorHostingIncome,
  INITIAL_IDENTITY_STATE,
  type MigrationStatus,
  type MigrationDirection,
  type MigrationState,
  INITIAL_MIGRATION_STATE,
  type MigrationResult,
  type TransitionRequirement,
  type SovereigntyTransition,
  getInitials,
  getReachLabel,
  getReachDescription,
} from './identity.model';
export * from './presence.model';
export * from './doorway.model';
// Selective export to avoid DoorwayInfo/DoorwaySelection name collision with doorway.model
export {
  type DoorwayTrustTier,
  type DoorwayCapability,
  type RegisteredDoorway,
  type DoorwayRegistry,
  type AddDoorwayInput,
  type AddDoorwayResult,
  type DoorwayHealthCheck,
  TRUST_TIER_THRESHOLDS,
  TRUST_TIER_DISPLAY,
  createEmptyRegistry,
  sortDoorwaysByTrust,
  getRecoveryDoorways,
  getPrimaryRecoveryDoorway,
} from './doorway-registry.model';
export * from './recovery.model';
export * from './consent-relationship.model';
export * from './human-relationship.model';
// Selective export to avoid AttestationType name collision with attestations.model
export {
  ATTESTATION_TYPES as IDENTITY_ATTESTATION_TYPES,
  type AttestationType as IdentityAttestationType,
  ANOMALY_TYPES,
  type AnomalyType,
  ANOMALY_SEVERITIES,
  type AnomalySeverity,
  type HumanityAttestation,
  type BehavioralAnomaly,
  type BindingReport,
  type KeyShard,
  type ShardDistribution,
  type IdentityFreeze,
  type KeyRevocation,
  type IdentitySecurityStatus,
  calculateTrustScore,
  shouldAutoFreeze,
  hasRecentAttestation,
  INITIAL_SECURITY_STATUS,
} from './identity-attestation.model';
export * from './stewardship.model';
// Note: Node status types are in shefa/models/shefa-dashboard.model.ts
