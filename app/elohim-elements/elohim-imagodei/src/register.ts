// elohim-imagodei — element registration entry point.
// Each element registers its tag here so consumers can `import 'elohim-imagodei/register'`
// to make all custom elements available.

import { ElohimImagodeiIntrospectionPanel } from './elohim-imagodei-introspection-panel.js';
import { ElohimImagodeiProtectedTierMarker } from './elohim-imagodei-protected-tier-marker.js';
import { ElohimImagodeiSettingControl } from './elohim-imagodei-setting-control.js';
import { ElohimImagodeiSettingsPalette } from './elohim-imagodei-settings-palette.js';
import { ElohimImagodeiStewardConfigureBanner } from './elohim-imagodei-steward-configure-banner.js';

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
