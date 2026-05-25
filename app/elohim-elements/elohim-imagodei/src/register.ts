// elohim-imagodei — element registration entry point.
// Each element registers its tag here so consumers can `import 'elohim-imagodei/register'`
// to make all custom elements available.

import { ElohimImagodeiAttestorRow } from './elohim-imagodei-attestor-row.js';
import { ElohimImagodeiIntrospectionPanel } from './elohim-imagodei-introspection-panel.js';
import { ElohimImagodeiProtectedTierMarker } from './elohim-imagodei-protected-tier-marker.js';
import { ElohimImagodeiSettingControl } from './elohim-imagodei-setting-control.js';
import { ElohimImagodeiSettingsPalette } from './elohim-imagodei-settings-palette.js';
import { ElohimImagodeiStewardConfigureBanner } from './elohim-imagodei-steward-configure-banner.js';
import { ElohimImagodeiTrustIndicator } from './elohim-imagodei-trust-indicator.js';

if (!customElements.get('elohim-imagodei-attestor-row')) {
  customElements.define('elohim-imagodei-attestor-row', ElohimImagodeiAttestorRow);
}

if (!customElements.get('elohim-imagodei-introspection-panel')) {
  customElements.define('elohim-imagodei-introspection-panel', ElohimImagodeiIntrospectionPanel);
}

if (!customElements.get('elohim-imagodei-protected-tier-marker')) {
  customElements.define('elohim-imagodei-protected-tier-marker', ElohimImagodeiProtectedTierMarker);
}

if (!customElements.get('elohim-imagodei-setting-control')) {
  customElements.define('elohim-imagodei-setting-control', ElohimImagodeiSettingControl);
}

if (!customElements.get('elohim-imagodei-settings-palette')) {
  customElements.define('elohim-imagodei-settings-palette', ElohimImagodeiSettingsPalette);
}

if (!customElements.get('elohim-imagodei-steward-configure-banner')) {
  customElements.define(
    'elohim-imagodei-steward-configure-banner',
    ElohimImagodeiStewardConfigureBanner
  );
}

if (!customElements.get('elohim-imagodei-trust-indicator')) {
  customElements.define('elohim-imagodei-trust-indicator', ElohimImagodeiTrustIndicator);
}

import { ElohimImagodeiPortalShell } from './elohim-imagodei-portal-shell.js';

if (!customElements.get('elohim-imagodei-portal-shell')) {
  customElements.define('elohim-imagodei-portal-shell', ElohimImagodeiPortalShell);
}

import { ElohimImagodeiFederatedResolver } from './elohim-imagodei-federated-resolver.js';

if (!customElements.get('elohim-imagodei-federated-resolver')) {
  customElements.define('elohim-imagodei-federated-resolver', ElohimImagodeiFederatedResolver);
}

import { ElohimImagodeiLoginCard } from './elohim-imagodei-login-card.js';

if (!customElements.get('elohim-imagodei-login-card')) {
  customElements.define('elohim-imagodei-login-card', ElohimImagodeiLoginCard);
}

import { ElohimImagodeiConsentCard } from './elohim-imagodei-consent-card.js';

if (!customElements.get('elohim-imagodei-consent-card')) {
  customElements.define('elohim-imagodei-consent-card', ElohimImagodeiConsentCard);
}
