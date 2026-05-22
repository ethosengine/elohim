// elohim-qahal — element registration entry point.
// Each element registers its tag here so consumers can `import 'elohim-qahal/register'`
// to make all custom elements available.

import { ElohimQahalAttestationsPanel } from './elohim-qahal-attestations-panel.js';
import { ElohimQahalCapabilityTierChip } from './elohim-qahal-capability-tier-chip.js';
import { ElohimQahalCareEconomyMarker } from './elohim-qahal-care-economy-marker.js';
import { ElohimQahalCoStewardPanel } from './elohim-qahal-co-steward-panel.js';
import { ElohimQahalCollectiveSwitcher } from './elohim-qahal-collective-switcher.js';
import { ElohimQahalContextColumn } from './elohim-qahal-context-column.js';
import { ElohimQahalGraphDiscoveryPanel } from './elohim-qahal-graph-discovery-panel.js';
import { ElohimQahalImagodeiBadge } from './elohim-qahal-imagodei-badge.js';
import { ElohimQahalMainViewer } from './elohim-qahal-main-viewer.js';
import { ElohimQahalMemberRingPanel } from './elohim-qahal-member-ring-panel.js';
import { ElohimQahalProvenanceMarker } from './elohim-qahal-provenance-marker.js';
import { ElohimQahalRulesPanel } from './elohim-qahal-rules-panel.js';
import { ElohimQahalShefaResourcesPanel } from './elohim-qahal-shefa-resources-panel.js';
import { ElohimQahalSidebar } from './elohim-qahal-sidebar.js';
import { ElohimQahalSocialComputePanel } from './elohim-qahal-social-compute-panel.js';
import { ElohimQahalStandingInspectorPanel } from './elohim-qahal-standing-inspector-panel.js';
import { ElohimQahalStandingRing } from './elohim-qahal-standing-ring.js';
import { ElohimQahalStreamPanel } from './elohim-qahal-stream-panel.js';

if (!customElements.get('elohim-qahal-attestations-panel')) {
  customElements.define('elohim-qahal-attestations-panel', ElohimQahalAttestationsPanel);
}

if (!customElements.get('elohim-qahal-capability-tier-chip')) {
  customElements.define('elohim-qahal-capability-tier-chip', ElohimQahalCapabilityTierChip);
}

if (!customElements.get('elohim-qahal-care-economy-marker')) {
  customElements.define('elohim-qahal-care-economy-marker', ElohimQahalCareEconomyMarker);
}

if (!customElements.get('elohim-qahal-co-steward-panel')) {
  customElements.define('elohim-qahal-co-steward-panel', ElohimQahalCoStewardPanel);
}

if (!customElements.get('elohim-qahal-collective-switcher')) {
  customElements.define('elohim-qahal-collective-switcher', ElohimQahalCollectiveSwitcher);
}

if (!customElements.get('elohim-qahal-context-column')) {
  customElements.define('elohim-qahal-context-column', ElohimQahalContextColumn);
}

if (!customElements.get('elohim-qahal-graph-discovery-panel')) {
  customElements.define('elohim-qahal-graph-discovery-panel', ElohimQahalGraphDiscoveryPanel);
}

if (!customElements.get('elohim-qahal-imagodei-badge')) {
  customElements.define('elohim-qahal-imagodei-badge', ElohimQahalImagodeiBadge);
}

if (!customElements.get('elohim-qahal-main-viewer')) {
  customElements.define('elohim-qahal-main-viewer', ElohimQahalMainViewer);
}

if (!customElements.get('elohim-qahal-member-ring-panel')) {
  customElements.define('elohim-qahal-member-ring-panel', ElohimQahalMemberRingPanel);
}

if (!customElements.get('elohim-qahal-provenance-marker')) {
  customElements.define('elohim-qahal-provenance-marker', ElohimQahalProvenanceMarker);
}

if (!customElements.get('elohim-qahal-rules-panel')) {
  customElements.define('elohim-qahal-rules-panel', ElohimQahalRulesPanel);
}

if (!customElements.get('elohim-qahal-shefa-resources-panel')) {
  customElements.define('elohim-qahal-shefa-resources-panel', ElohimQahalShefaResourcesPanel);
}

if (!customElements.get('elohim-qahal-sidebar')) {
  customElements.define('elohim-qahal-sidebar', ElohimQahalSidebar);
}

if (!customElements.get('elohim-qahal-social-compute-panel')) {
  customElements.define('elohim-qahal-social-compute-panel', ElohimQahalSocialComputePanel);
}

if (!customElements.get('elohim-qahal-standing-inspector-panel')) {
  customElements.define('elohim-qahal-standing-inspector-panel', ElohimQahalStandingInspectorPanel);
}

if (!customElements.get('elohim-qahal-standing-ring')) {
  customElements.define('elohim-qahal-standing-ring', ElohimQahalStandingRing);
}

if (!customElements.get('elohim-qahal-stream-panel')) {
  customElements.define('elohim-qahal-stream-panel', ElohimQahalStreamPanel);
}
