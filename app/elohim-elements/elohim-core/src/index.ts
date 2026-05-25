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
