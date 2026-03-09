/**
 * Mutual Aid Models - Governance Primitive for Resource Solidarity
 *
 * A MutualAidContext represents a declared period of resource solidarity —
 * emergency compute sharing, bootstrap sponsorship, or scheduled capacity
 * sharing between regions or communities.
 *
 * Key design principle: Emergency priority is a GOVERNANCE ACT, not a routing
 * flag. A MutualAidContext is declared by an elohim agent, carries a
 * constitutional basis (EPR ref), and is scoped by the coordination envelope's
 * CoordinationScope. This prevents abuse of emergency powers because:
 *
 * 1. Declaration is auditable (who declared, on what constitutional authority)
 * 2. Scope is bounded (not a global override)
 * 3. Expiry is mandatory (no permanent emergency)
 * 4. Closure triggers mandatory after-action review (lessons-captured lifecycle)
 *
 * The RoutingContext in coordination-envelope.model deliberately has NO priority
 * field — priority comes from the ambient MutualAidContext, which is a governance
 * decision subject to review, not a technical flag any node can set.
 *
 * Lifecycle: active -> pending-review -> reviewed -> lessons-captured
 * When a MutualAidContext is closed, reviewStatus transitions through these
 * stages, ensuring the community reflects on how emergency powers were used.
 */

import type { CoordinationScope, AgentRef } from '@app/elohim/models/coordination-envelope.model';
import type { ResourceClassification } from '@app/elohim/models/rea-bridge.model';

// ============================================================================
// MUTUAL AID MODE
// ============================================================================

/**
 * MutualAidMode - The reason for activating resource solidarity.
 *
 * - bootstrap: New community or region needs initial capacity sponsorship
 * - emergency: Disaster, outage, or sudden capacity loss requiring immediate aid
 * - scheduled: Planned capacity sharing (e.g., seasonal load balancing)
 */
export type MutualAidMode = 'bootstrap' | 'emergency' | 'scheduled';

// ============================================================================
// REVIEW STATUS
// ============================================================================

/**
 * MutualAidReviewStatus - After-action review lifecycle.
 *
 * Every closed MutualAidContext must progress through review stages.
 * This is non-negotiable: emergency powers without accountability
 * invite abuse.
 *
 * - active: Context is currently in effect
 * - pending-review: Context closed, awaiting after-action deliberation
 * - reviewed: Deliberation complete, findings recorded
 * - lessons-captured: Findings integrated into governance precedent
 */
export type MutualAidReviewStatus = 'active' | 'pending-review' | 'reviewed' | 'lessons-captured';

// ============================================================================
// MUTUAL AID CONTEXT
// ============================================================================

/**
 * MutualAidContext - A declared period of resource solidarity.
 *
 * This is the governance primitive that makes emergency coordination
 * auditable. Rather than embedding priority flags in routing messages
 * (which any node could set), priority emerges from an explicit
 * governance declaration that references constitutional authority.
 *
 * The RoutingContext in InvocationEnvelope consults the ambient
 * MutualAidContext to determine priority — the routing layer reads
 * governance state, it does not create it.
 */
export interface MutualAidContext {
  /** Unique identifier for this mutual aid context */
  id: string;

  /** Why this solidarity period was activated */
  mode: MutualAidMode;

  /** Who this context covers (geographic, affinity, layer scope) */
  scope: CoordinationScope;

  /** The elohim agent who declared this context */
  declaredBy: AgentRef;

  /** EPR ref to the constitutional clause authorizing this declaration */
  constitutionalBasis: string;

  /** When this context was activated (ISO 8601) */
  activatedAt: string;

  /** When this context expires (ISO 8601). Mandatory for emergency mode. */
  expiresAt?: string;

  /** What kind of resource is being shared */
  resourceType: ResourceClassification;

  /** EPR refs to REA commitments made under this context */
  commitmentRefs: string[];

  /** When this context was closed (ISO 8601) */
  closedAt?: string;

  /** The agent who closed this context */
  closedBy?: AgentRef;

  /** After-action review lifecycle stage */
  reviewStatus: MutualAidReviewStatus;

  /** EPR ref to the after-action review deliberation */
  afterActionRef?: string;
}
