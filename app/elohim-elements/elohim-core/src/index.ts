// Side-effect-free entry. Re-exports component classes and types.
// Consumers that want auto-registration import from './register' instead.

export { ElohimButton } from './elohim-button.js';
export type { ElohimButtonVariant } from './elohim-button.js';

export { ElohimComputeTile } from './elohim-compute-tile.js';
export type {
  ComputeTileValue,
  ComputeTileHubValue,
  ComputeTileDeviceValue,
  ComputeTileArchetype,
  ComputeTileState,
} from './elohim-compute-tile.js';

export { ElohimSkeleton } from './elohim-skeleton.js';
export { ElohimMentionBase } from './elohim-mention-base.js';
export { ElohimPageChrome } from './elohim-page-chrome.js';
export { ElohimDefaultOmnibar } from './elohim-default-omnibar.js';
export { ElohimContextMenu } from './elohim-context-menu.js';
export type { ContextMenuItem } from './elohim-context-menu.js';

export { ElohimEprLink } from './elohim-epr-link.js';
export type { EprLinkDisplay, EprLinkLoadLevel, EprLinkResolution } from './elohim-epr-link.js';

export { ElohimEprRelationshipsPanel } from './elohim-epr-relationships-panel.js';
export type { EprRelationship } from './elohim-epr-relationships-panel.js';

export { ElohimReactionBar } from './elohim-reaction-bar.js';
export type {
  EmotionalReactionType,
  ReactionConstraints,
  MediatedReaction,
  ReactionCount,
  ReactionSubmitEvent,
  MediationProceedEvent,
} from './elohim-reaction-bar.js';

export { ElohimGraduatedFeedback } from './elohim-graduated-feedback.js';
export type {
  FeedbackContext,
  ScalePosition,
  ScaleDefinition,
  GraduatedFeedbackInput,
  FeedbackDistribution,
} from './elohim-graduated-feedback.js';

export { ElohimFeedbackMechanismGateway } from './elohim-feedback-mechanism-gateway.js';
export type {
  FeedbackMechanismLevel,
  FeedbackRenderTarget,
  MechanismSelection,
  AccumulationStatus,
  GatewayLoadResult,
} from './elohim-feedback-mechanism-gateway.js';

export { ElohimGateFeedbackTrigger } from './elohim-gate-feedback-trigger.js';
export type {
  GateFeedbackType,
  GateFeedbackMenuItem,
  GateFeedbackPostedEvent,
} from './elohim-gate-feedback-trigger.js';

export { ElohimNavigator } from './elohim-navigator.js';
export type {
  ElohimContextApp,
  ElohimContextAppConfig,
  ElohimNavigatorSession,
  ElohimNavigatorBannerItem,
} from './elohim-navigator.js';

export { ElohimContentAnalytics } from './elohim-content-analytics.js';
export type {
  ContentAnalyticsMetrics,
  ContentAnalyticsLoader,
} from './elohim-content-analytics.js';

export { ElohimEprPopover } from './elohim-epr-popover.js';
export type {
  EprHead,
  EprLamadContext,
  EprShefaContext,
  EprQahalContext,
} from './elohim-epr-popover.js';

export * from './capability/index.js';

export { Loader } from './loader/loader.js';
export type { LoaderTransport, LoaderResolution, LoaderOptions } from './loader/loader.js';

export { Session } from './session/session.js';
export type { CurrentUserView } from './session/session.js';

export type {
  OmnibarContext,
  EprRef,
  CapabilitySnapshot,
  ReachContext,
} from './contracts/omnibar.contract.js';
