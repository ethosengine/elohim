import { ElohimButton } from './elohim-button.js';
import { ElohimComputeTile } from './elohim-compute-tile.js';
import { ElohimSkeleton } from './elohim-skeleton.js';
import { ElohimMentionBase } from './elohim-mention-base.js';
import { ElohimPageChrome } from './elohim-page-chrome.js';
import { ElohimDefaultOmnibar } from './elohim-default-omnibar.js';
import { ElohimContextMenu } from './elohim-context-menu.js';
import { ElohimEprLink } from './elohim-epr-link.js';
import { ElohimEprRelationshipsPanel } from './elohim-epr-relationships-panel.js';
import { ElohimReactionBar } from './elohim-reaction-bar.js';
import { ElohimGraduatedFeedback } from './elohim-graduated-feedback.js';
import { ElohimFeedbackMechanismGateway } from './elohim-feedback-mechanism-gateway.js';
import { ElohimGateFeedbackTrigger } from './elohim-gate-feedback-trigger.js';
import { ElohimNavigator } from './elohim-navigator.js';
import { ElohimContentAnalytics } from './elohim-content-analytics.js';
import { ElohimEprPopover } from './elohim-epr-popover.js';

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

if (!customElements.get('elohim-skeleton')) {
  customElements.define('elohim-skeleton', ElohimSkeleton);
}

if (!customElements.get('elohim-mention-base')) {
  customElements.define('elohim-mention-base', ElohimMentionBase);
}

if (!customElements.get('elohim-page-chrome')) {
  customElements.define('elohim-page-chrome', ElohimPageChrome);
}

if (!customElements.get('elohim-default-omnibar')) {
  customElements.define('elohim-default-omnibar', ElohimDefaultOmnibar);
}

if (!customElements.get('elohim-context-menu')) {
  customElements.define('elohim-context-menu', ElohimContextMenu);
}

if (!customElements.get('elohim-epr-link')) {
  customElements.define('elohim-epr-link', ElohimEprLink);
}

if (!customElements.get('elohim-epr-relationships-panel')) {
  customElements.define('elohim-epr-relationships-panel', ElohimEprRelationshipsPanel);
}

if (!customElements.get('elohim-reaction-bar')) {
  customElements.define('elohim-reaction-bar', ElohimReactionBar);
}

if (!customElements.get('elohim-graduated-feedback')) {
  customElements.define('elohim-graduated-feedback', ElohimGraduatedFeedback);
}

if (!customElements.get('elohim-feedback-mechanism-gateway')) {
  customElements.define('elohim-feedback-mechanism-gateway', ElohimFeedbackMechanismGateway);
}

if (!customElements.get('elohim-gate-feedback-trigger')) {
  customElements.define('elohim-gate-feedback-trigger', ElohimGateFeedbackTrigger);
}

if (!customElements.get('elohim-navigator')) {
  customElements.define('elohim-navigator', ElohimNavigator);
}

if (!customElements.get('elohim-content-analytics')) {
  customElements.define('elohim-content-analytics', ElohimContentAnalytics);
}

if (!customElements.get('elohim-epr-popover')) {
  customElements.define('elohim-epr-popover', ElohimEprPopover);
}
