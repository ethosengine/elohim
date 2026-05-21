// elohim-qahal — element registration entry point.
// Each element registers its tag here so consumers can `import 'elohim-qahal/register'`
// to make all custom elements available.

import { ElohimQahalCapabilityTierChip } from './elohim-qahal-capability-tier-chip.js';
import { ElohimQahalCareEconomyMarker } from './elohim-qahal-care-economy-marker.js';
import { ElohimQahalImagodeiBadge } from './elohim-qahal-imagodei-badge.js';
import { ElohimQahalProvenanceMarker } from './elohim-qahal-provenance-marker.js';
import { ElohimQahalStandingRing } from './elohim-qahal-standing-ring.js';

if (!customElements.get('elohim-qahal-capability-tier-chip')) {
  customElements.define('elohim-qahal-capability-tier-chip', ElohimQahalCapabilityTierChip);
}

if (!customElements.get('elohim-qahal-care-economy-marker')) {
  customElements.define('elohim-qahal-care-economy-marker', ElohimQahalCareEconomyMarker);
}

if (!customElements.get('elohim-qahal-imagodei-badge')) {
  customElements.define('elohim-qahal-imagodei-badge', ElohimQahalImagodeiBadge);
}

if (!customElements.get('elohim-qahal-provenance-marker')) {
  customElements.define('elohim-qahal-provenance-marker', ElohimQahalProvenanceMarker);
}

if (!customElements.get('elohim-qahal-standing-ring')) {
  customElements.define('elohim-qahal-standing-ring', ElohimQahalStandingRing);
}
