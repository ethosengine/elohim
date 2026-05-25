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
