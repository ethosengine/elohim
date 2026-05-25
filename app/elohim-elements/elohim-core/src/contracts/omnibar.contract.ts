import type { CurrentUserView } from '../session/session.js';

/**
 * Omnibar contract — the TypeScript interface satisfied by any element
 * placed in <elohim-page-chrome slot="omnibar">.
 *
 * Pillar bundles that BYO their own toolbar (e.g. lamad's existing toolbar)
 * read this context to render appropriately. Bundles that don't BYO get
 * <elohim-default-omnibar> as the fallback (rendered by page-chrome when
 * the slot is empty).
 *
 * The contract is duck-typed at runtime: <elohim-page-chrome> sets the
 * context as a property on its slotted child. The child reads (or ignores)
 * it freely. No structural sharing or DI framework is required.
 */
export interface OmnibarContext {
  /** Current authenticated user, or null when anonymous. From Session primitive. */
  currentUser: CurrentUserView | null;
  /** Which EPR is currently mounted (for breadcrumb / location-aware nav). */
  currentEpr: EprRef | null;
  /** Caller's capabilities — drives affordance visibility. */
  capabilities: CapabilitySnapshot;
  /** Reach context — anonymous, authenticated, household, qahal. */
  reach: ReachContext;
  /** Navigate to another EPR. Wired through the elohim-epr-link primitive. */
  onNavigate: (target: EprRef) => void;
}

export interface EprRef {
  eprId: string;
  displayLabel?: string;
}

export interface CapabilitySnapshot {
  has: (capability: string) => boolean;
  list: string[];
}

export interface ReachContext {
  kind: 'anonymous' | 'authenticated' | 'household' | 'qahal';
  details: Record<string, unknown>;
}
