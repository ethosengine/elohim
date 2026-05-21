// elohim-qahal — element registration entry point.
// Each element registers its tag here so consumers can `import 'elohim-qahal/register'`
// to make all custom elements available.

import { ElohimQahalCapabilityTierChip } from './elohim-qahal-capability-tier-chip.js';
import { ElohimQahalCareEconomyMarker } from './elohim-qahal-care-economy-marker.js';
import { ElohimQahalCollectiveSwitcher } from './elohim-qahal-collective-switcher.js';
import { ElohimQahalContextColumn } from './elohim-qahal-context-column.js';
import { ElohimQahalImagodeiBadge } from './elohim-qahal-imagodei-badge.js';
import { ElohimQahalMainViewer } from './elohim-qahal-main-viewer.js';
import { ElohimQahalProvenanceMarker } from './elohim-qahal-provenance-marker.js';
import { ElohimQahalSidebar } from './elohim-qahal-sidebar.js';
import { ElohimQahalStandingRing } from './elohim-qahal-standing-ring.js';

if (!customElements.get('elohim-qahal-capability-tier-chip')) {
  customElements.define('elohim-qahal-capability-tier-chip', ElohimQahalCapabilityTierChip);
}

if (!customElements.get('elohim-qahal-care-economy-marker')) {
  customElements.define('elohim-qahal-care-economy-marker', ElohimQahalCareEconomyMarker);
}

if (!customElements.get('elohim-qahal-collective-switcher')) {
  customElements.define('elohim-qahal-collective-switcher', ElohimQahalCollectiveSwitcher);
}

if (!customElements.get('elohim-qahal-context-column')) {
  customElements.define('elohim-qahal-context-column', ElohimQahalContextColumn);
}

if (!customElements.get('elohim-qahal-imagodei-badge')) {
  customElements.define('elohim-qahal-imagodei-badge', ElohimQahalImagodeiBadge);
}

if (!customElements.get('elohim-qahal-main-viewer')) {
  customElements.define('elohim-qahal-main-viewer', ElohimQahalMainViewer);
}

if (!customElements.get('elohim-qahal-provenance-marker')) {
  customElements.define('elohim-qahal-provenance-marker', ElohimQahalProvenanceMarker);
}

if (!customElements.get('elohim-qahal-sidebar')) {
  customElements.define('elohim-qahal-sidebar', ElohimQahalSidebar);
}

if (!customElements.get('elohim-qahal-standing-ring')) {
  customElements.define('elohim-qahal-standing-ring', ElohimQahalStandingRing);
}
