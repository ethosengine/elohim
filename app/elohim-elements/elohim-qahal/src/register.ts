// elohim-qahal — element registration entry point.
// Each element registers its tag here so consumers can `import 'elohim-qahal/register'`
// to make all custom elements available.

import { ElohimQahalImagodeiBadge } from './elohim-qahal-imagodei-badge.js';

if (!customElements.get('elohim-qahal-imagodei-badge')) {
  customElements.define('elohim-qahal-imagodei-badge', ElohimQahalImagodeiBadge);
}
