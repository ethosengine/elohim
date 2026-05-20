import { ElohimButton } from './elohim-button.js';
import { ElohimComputeTile } from './elohim-compute-tile.js';

// Registration lives here, not on the ElohimButton class via @customElement,
// to keep ./index.ts truly side-effect-free. Importing from 'elohim-core'
// (the bare specifier) does NOT register the element; consumers must
// explicitly import 'elohim-core/register'. This avoids a tree-shaking trap
// where a bundler could drop an unused class import and silently lose the
// registration.
//
// Known CEM quirk: the @custom-elements-manifest/analyzer 0.10.x emits an
// absolute-looking module path ("/src/elohim-button.js") for the
// custom-element-definition export instead of a relative form, because the
// imperative customElements.define call is path-resolved differently than
// the decorator form. This is cosmetic — IDE extensions that resolve tag
// names to implementations may not navigate correctly. Acceptable
// tradeoff for the side-effect-free contract.

if (!customElements.get('elohim-button')) {
  customElements.define('elohim-button', ElohimButton);
}

if (!customElements.get('elohim-compute-tile')) {
  customElements.define('elohim-compute-tile', ElohimComputeTile);
}
