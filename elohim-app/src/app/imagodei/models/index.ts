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
export * from './doorway-registry.model';
export * from './recovery.model';
export * from './consent-relationship.model';
export * from './human-relationship.model';
export * from './identity-attestation.model';
export * from './stewardship.model';
// Note: Node status types are in shefa/models/shefa-dashboard.model.ts
