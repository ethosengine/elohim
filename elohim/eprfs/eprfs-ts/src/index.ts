/**
 * `@elohim/eprfs`: the pure TypeScript twin of eprfs-core (interfaces spec §H.1). It imports
 * neither `@elohim/storage-client` nor any DOM or Angular module (rail R-T1); the HTTP tier and
 * the placement surface live in `@elohim/storage-client`, the service worker's cache tier in the
 * app.
 */
export {
  decodeTree,
  DEFAULT_LIMITS,
  TreeError,
  type TreeEntry,
  type TreeEntryKind,
  type TreeErrorKind,
  type TreeLimits,
  type TreeNode,
} from './tree.js';
export {
  collectWalk,
  comparePaths,
  diffTrees,
  parsePath,
  pathString,
  readPath,
  View,
  walkBatched,
  type Coverage,
  type NamedAbsence,
  type TreePath,
  type WalkedView,
  type WalkItem,
  type WalkStats,
} from './walk.js';
export {
  MemoryTier,
  proximityOrder,
  standingPolicy,
  TierChain,
  type Admission,
  type BatchItem,
  DEFAULT_MAX_REQUEST_ITEMS,
  type ChainOptions,
  type ChainPolicy,
  type ChainStatsSnapshot,
  type Tier,
  type TierAnswer,
  type TierConnector,
  type TierSink,
  type WritableTier,
} from './tier.js';
export { decodeTreeView, encodeTreeView, parseTreeViewJson, treeViewCid, treeViewJson } from './view.js';
export {
  absence,
  isItemAbsence,
  isTierId,
  isTierRecord,
  isUnanswered,
  rewriteOwn,
  type ComeBack,
  type ItemAbsence,
  type Meanwhile,
  type Proximity,
  type ReachReason,
  type TierAddress,
  type TierClass,
  type TierRecord,
  type TierSource,
  type Unanswered,
} from './wire.js';
