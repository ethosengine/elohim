// elohim-imagodei — public API surface for type imports.
// Consumers import the element classes for type annotations:
// `import type { ElohimImagodeiFoo } from 'elohim-imagodei';`

// Framework-agnostic helpers — safe to import from Angular and standalone bundles.
export {
  parseFederatedIdentifier,
  resolveGatewayToDoorwayUrl,
  type FederatedIdentifier,
  type DoorwayDescriptor,
  type ParseOutcome,
  type ResolveOutcome,
} from './federated-identifier.js';

export { ElohimImagodeiIntrospectionPanel } from './elohim-imagodei-introspection-panel.js';
export type { AffordanceTrace, SubjectSetting } from './elohim-imagodei-introspection-panel.js';

export { ElohimImagodeiProtectedTierMarker } from './elohim-imagodei-protected-tier-marker.js';
export type { ProtectedTier } from './elohim-imagodei-protected-tier-marker.js';

export { ElohimImagodeiSettingControl } from './elohim-imagodei-setting-control.js';
export type { SettingControl } from './elohim-imagodei-setting-control.js';

export { ElohimImagodeiSettingsPalette } from './elohim-imagodei-settings-palette.js';

export { ElohimImagodeiStewardConfigureBanner } from './elohim-imagodei-steward-configure-banner.js';

export { ElohimImagodeiTrustIndicator } from './elohim-imagodei-trust-indicator.js';
export type { TrustMode } from './elohim-imagodei-trust-indicator.js';

export { ElohimImagodeiAttestorRow } from './elohim-imagodei-attestor-row.js';
export type { AttestorRef } from './elohim-imagodei-attestor-row.js';

export { ElohimImagodeiPortalShell } from './elohim-imagodei-portal-shell.js';
export type { PortalStep, AuthorityResolution } from './elohim-imagodei-portal-shell.js';

export { ElohimImagodeiFederatedResolver } from './elohim-imagodei-federated-resolver.js';
export type {
  FederatedResolveOutcome,
  ResolveIdentifierFn,
} from './elohim-imagodei-federated-resolver.js';

export { ElohimImagodeiLoginCard } from './elohim-imagodei-login-card.js';
export type { OAuthProviderRef } from './elohim-imagodei-login-card.js';

export { ElohimImagodeiConsentCard } from './elohim-imagodei-consent-card.js';
export type { ClaimRef, RequestingClient } from './elohim-imagodei-consent-card.js';

export {
  ElohimImagodeiDeviceConsentCard,
  DEVICE_CONSENT_STRINGS_EN,
} from './elohim-imagodei-device-consent-card.js';
export type {
  DeviceAct,
  DeviceConsentApproveDetail,
  DeviceConsentPhase,
  DeviceConsentRequest,
  DeviceConsentSigner,
  DeviceConsentStringOverrides,
  DeviceConsentStrings,
  KnownDeviceRefusalCode,
} from './elohim-imagodei-device-consent-card.js';

export { ElohimImagodeiOauthCallback } from './elohim-imagodei-oauth-callback.js';
export type { ExchangeOutcome, ExchangeCodeFn } from './elohim-imagodei-oauth-callback.js';

export { ElohimContributorCard, presenceStateBadge } from './elohim-imagodei-contributor-card.js';
export type { ContributorPresenceState } from './elohim-imagodei-contributor-card.js';

export {
  ElohimImagodeiWitnessTrail,
  WITNESS_TRAIL_STRINGS_EN,
} from './elohim-imagodei-witness-trail.js';
export type {
  WitnessAct,
  WitnessRelation,
  WitnessSentence,
  WitnessSentenceTable,
  WitnessStep,
  WitnessStepState,
  WitnessTrailLayout,
  WitnessTrailMode,
  WitnessTrailSettledDetail,
  WitnessTrailStrings,
  WitnessWho,
} from './elohim-imagodei-witness-trail.js';
