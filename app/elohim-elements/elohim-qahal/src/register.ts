// elohim-qahal — element registration entry point.
// Each element registers its tag here so consumers can `import 'elohim-qahal/register'`
// to make all custom elements available.

import { ElohimQahalImagodeiBadge } from './elohim-qahal-imagodei-badge.js';
import { ElohimQahalStandingRing } from './elohim-qahal-standing-ring.js';

if (!customElements.get('elohim-qahal-imagodei-badge')) {
  customElements.define('elohim-qahal-imagodei-badge', ElohimQahalImagodeiBadge);
}

if (!customElements.get('elohim-qahal-standing-ring')) {
  customElements.define('elohim-qahal-standing-ring', ElohimQahalStandingRing);
}
