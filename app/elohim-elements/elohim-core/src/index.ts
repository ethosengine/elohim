// Side-effect-free entry. Re-exports component classes and types.
// Consumers that want auto-registration import from './register' instead.

export {
  VIEWPORT_ARCHETYPES,
  type ViewportArchetype,
  type ViewportArchetypeName,
} from './viewport-archetypes.js';

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

export {
  ThemeStore,
  getThemeStore,
  THEME_STORAGE_KEY,
  THEME_CHANGE_EVENT,
} from './theme/theme-store.js';
export type { ElohimTheme } from './theme/theme-store.js';

export {
  installEprLinkInterceptor,
  recordCrossBundleHandoff,
  baseHrefOwnsPath,
} from './navigation/epr-link-interceptor.js';
export type { EprLinkInterceptorOptions } from './navigation/epr-link-interceptor.js';

export { ElohimThemeToggle } from './elohim-theme-toggle.js';

export { ElohimLangPicker } from './elohim-lang-picker.js';

export { ElohimHypercardPanel } from './elohim-hypercard-panel.js';

export {
  LocaleStore,
  getLocaleStore,
  detectLocale,
  SUPPORTED_LOCALES,
  LOCALE_LABELS,
  LOCALE_STORAGE_KEY,
  LOCALE_CHANGE_EVENT,
} from './localize/locale-store.js';
export type { ElohimLocale } from './localize/locale-store.js';
