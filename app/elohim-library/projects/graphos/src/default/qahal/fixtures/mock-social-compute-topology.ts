/**
 * mock-social-compute-topology.ts
 *
 * Compute-stewardship relationships for each Tier-0 worked-example Qahal:
 *   - Dowell household (intimate scale, per UX spec Section 4.5 ASCII mock)
 *   - CofC congregation (congregation-scale, 230 members)
 *   - Hardins life-group (small/intimate, 8 households' shared compute trust)
 *   - Wisdom commons federation (83 congregations contributing compute share)
 *
 * Grounded in UX design spec at:
 * genesis/docs/superpowers/specs/2026-05-22-qahal-homepage-ux-design.md (Section 4.5)
 * and canonical narratives at:
 * genesis/docs/superpowers/specs/2026-05-21-qahal-section-4-canonical-narratives.md
 *
 * The social-compute panel proves the substrate's resilience claim is real — showing
 * both the care-economy (commons stream) and the compute-stewardship topology on the
 * same Qahal homepage. "In care AND compute." Both are first-class stewardship.
 *
 * This is a Category C computed view. These mocks provide the stable fixtures that
 * Library B pattern stories will render in the social-compute panel.
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/**
 * Status of a compute hub — the hub is the collection of devices contributing
 * to a household or Qahal's compute pool.
 */
export interface MockComputeHubStatus {
  /** Stable hub id — typically hub:<kind>:<qahal-id> */
  hubId: string;
  /** Current operational health of this hub. */
  status: 'healthy' | 'degraded' | 'down';
  /** Total GB allocated to this hub's compute pool. */
  allocatedGB: number;
  /** ISO datetime of last heartbeat from this hub, or 'now' for real-time. */
  lastSeen: string;
}

/**
 * A compute-steward relationship — another hub that holds a replication share
 * for this Qahal's state (or the reverse: this Qahal holding a share for another).
 * The topology makes the resilience claim visible: Gertrude's hub holds a recovery
 * share for the Dowells; the Dowells' hub holds one for hers.
 */
export interface MockComputeStewardRelationship {
  /** Stable id of the steward peer (imagodei or hub id). */
  stewardId: string;
  /** Human-readable display name (e.g., "Gertrude Dowell (grandma)"). */
  stewardName: string;
  /** Current health of this steward relationship. */
  health: 'healthy' | 'degraded' | 'down';
  /**
   * Replication health percentage. 100% = fully synced; below 90% = degraded.
   * Ethan Dowell in the narrative is at 85% (degraded, last sync 4h ago).
   */
  percent: number;
  /** Human-readable relative time of last sync (e.g., '17m ago', '3m ago', '4h ago'). */
  lastSync: string;
  /** Brief description of what this steward is replicating for the Qahal. */
  replicating: string;
}

/**
 * The full compute-stewardship topology for a Qahal — who stewards for us,
 * who we steward for, the hub's current health, and recovery readiness state.
 * Rendered in the social-compute panel (panel 5 in the deep-impl set).
 */
export interface MockComputeTopology {
  /** The Qahal this topology belongs to. */
  qahalId: string;
  /** This Qahal's own hub status. */
  selfHub: MockComputeHubStatus;
  /**
   * Peers who hold replication shares FOR this Qahal — "compute-stewards FOR us."
   * The panel checks all are active and flags degraded relationships.
   */
  stewardsForUs: MockComputeStewardRelationship[];
  /**
   * Qahals this hub stewards FOR — "we compute-steward FOR."
   * Reciprocal trust relationships: the substrate makes mutual stewardship first-class.
   */
  weStewardFor: { stewardId: string; description: string }[];
  /**
   * Recovery readiness — Shamir threshold status, last drill date, next scheduled drill.
   * The recovery drill line connects to the recovery-m4-* specs: click surfaces the
   * drill log or upcoming exercise.
   */
  recoveryReadiness: {
    ready: boolean;
    shamirThreshold: string;
    lastDrill: string;
    nextDrill: string;
  };
}

// ---------------------------------------------------------------------------
// Dowell household topology
// ---------------------------------------------------------------------------

/**
 * DOWELL_HOUSEHOLD_TOPOLOGY — the Dowell household social-compute topology as
 * rendered in UX spec Section 4.5 ASCII mock. Three compute-stewards: Gertrude
 * (healthy, 100%, 17m ago), Sheila (healthy, 100%, 3m ago), Ethan Dowell (uncle,
 * degraded, 85%, 4h ago). The Dowells steward for Gertrude's household state and
 * for the Susan household via sibling-household trust. Recovery is ready at 2/3
 * Shamir threshold; last drill was 14 days ago (passed); next is in 16 days.
 */
export const DOWELL_HOUSEHOLD_TOPOLOGY: MockComputeTopology = {
  qahalId: 'dowell-household',
  selfHub: {
    hubId: 'hub:dwelling:dowell-household',
    status: 'healthy',
    allocatedGB: 4,
    lastSeen: 'now',
  },
  stewardsForUs: [
    {
      stewardId: 'gertrude-dowell',
      stewardName: 'Gertrude Dowell (grandma)',
      health: 'healthy',
      percent: 100,
      lastSync: '17m ago',
      replicating: 'household state + care-economy ledger',
    },
    {
      stewardId: 'sheila-hardin',
      stewardName: 'Sheila Hardin (sister)',
      health: 'healthy',
      percent: 100,
      lastSync: '3m ago',
      replicating: 'household state',
    },
    {
      stewardId: 'ethan-dowell',
      stewardName: 'Ethan Dowell (uncle)',
      health: 'degraded',
      percent: 85,
      lastSync: '4h ago',
      replicating: 'household state · degraded',
    },
  ],
  weStewardFor: [
    {
      stewardId: 'gertrude-dowell',
      description: 'her household state',
    },
    {
      stewardId: 'susan-household',
      description: 'sibling-household trust',
    },
  ],
  recoveryReadiness: {
    ready: true,
    shamirThreshold: '2/3',
    lastDrill: '14 days ago (passed)',
    nextDrill: '16 days',
  },
};

// ---------------------------------------------------------------------------
// CofC congregation topology
// ---------------------------------------------------------------------------

/**
 * COFC_CONGREGATION_TOPOLOGY — congregation-scale compute topology. A larger hub
 * allocation reflecting 230 member-households contributing compute. More stewards
 * across member-households; reciprocal trust relationships with neighboring
 * congregations in the fellowship. Recovery readiness at 3/5 Shamir threshold
 * (higher threshold for a larger, more consequential Qahal).
 */
export const COFC_CONGREGATION_TOPOLOGY: MockComputeTopology = {
  qahalId: 'cofc-congregation',
  selfHub: {
    hubId: 'hub:congregation:cofc-congregation',
    status: 'healthy',
    allocatedGB: 48,
    lastSeen: 'now',
  },
  stewardsForUs: [
    {
      stewardId: 'elder-thompson-household',
      stewardName: 'Elder Thompson (household hub)',
      health: 'healthy',
      percent: 100,
      lastSync: '5m ago',
      replicating: 'congregation state + member standing ledger',
    },
    {
      stewardId: 'elder-davis-household',
      stewardName: 'Elder Davis (household hub)',
      health: 'healthy',
      percent: 100,
      lastSync: '8m ago',
      replicating: 'congregation state',
    },
    {
      stewardId: 'elder-rhodes-household',
      stewardName: 'Elder Rhodes (household hub)',
      health: 'healthy',
      percent: 98,
      lastSync: '12m ago',
      replicating: 'congregation state',
    },
    {
      stewardId: 'deacon-williams-household',
      stewardName: 'Deacon Williams (household hub)',
      health: 'healthy',
      percent: 100,
      lastSync: '2m ago',
      replicating: 'congregation state + youth program records',
    },
    {
      stewardId: 'sister-congregation-nearby',
      stewardName: 'Sister congregation (neighboring)',
      health: 'healthy',
      percent: 95,
      lastSync: '1h ago',
      replicating: 'congregation state — mutual resilience pact',
    },
  ],
  weStewardFor: [
    {
      stewardId: 'sister-congregation-nearby',
      description: 'their congregation state under mutual resilience pact',
    },
    {
      stewardId: 'hardins-life-group',
      description: 'life-group records nested within congregation',
    },
  ],
  recoveryReadiness: {
    ready: true,
    shamirThreshold: '3/5',
    lastDrill: '21 days ago (passed)',
    nextDrill: '9 days',
  },
};

// ---------------------------------------------------------------------------
// Hardins life-group topology
// ---------------------------------------------------------------------------

/**
 * LIFE_GROUP_TOPOLOGY — the small/intimate topology for the Tuesday-evening life-group.
 * Eight households' shared compute trust. The life-group's state is nested within the
 * congregation's topology; recovery shares are held by the congregation hub and by
 * two life-group member households. A 2/3 Shamir threshold is sufficient at this intimate scale.
 */
export const LIFE_GROUP_TOPOLOGY: MockComputeTopology = {
  qahalId: 'hardins-life-group',
  selfHub: {
    hubId: 'hub:life-group:hardins-life-group',
    status: 'healthy',
    allocatedGB: 6,
    lastSeen: 'now',
  },
  stewardsForUs: [
    {
      stewardId: 'john-hardin',
      stewardName: 'John Hardin (host household)',
      health: 'healthy',
      percent: 100,
      lastSync: '4m ago',
      replicating: 'life-group state + prayer records',
    },
    {
      stewardId: 'jin-lee',
      stewardName: 'Jin Lee (Lee household)',
      health: 'healthy',
      percent: 100,
      lastSync: '6m ago',
      replicating: 'life-group state',
    },
    {
      stewardId: 'cofc-congregation',
      stewardName: 'Congregation hub (parent Qahal)',
      health: 'healthy',
      percent: 100,
      lastSync: '2m ago',
      replicating: 'life-group state — nested resilience',
    },
  ],
  weStewardFor: [
    {
      stewardId: 'cofc-congregation',
      description: 'contributes to congregation compute pool via member households',
    },
  ],
  recoveryReadiness: {
    ready: true,
    shamirThreshold: '2/3',
    lastDrill: '30 days ago (passed)',
    nextDrill: '30 days',
  },
};

// ---------------------------------------------------------------------------
// Wisdom commons federation topology
// ---------------------------------------------------------------------------

/**
 * WISDOM_COMMONS_TOPOLOGY — federation-scale compute topology. 83 congregations
 * contributing compute share to the commons. The federation has a large distributed
 * hub pool; no single congregation's hub is dominant (friction-gradient enforces this).
 * Recovery readiness at 4/7 Shamir threshold for federation-scale durability.
 * Compute contributions are voluntary and tracked as REA economic events.
 */
export const WISDOM_COMMONS_TOPOLOGY: MockComputeTopology = {
  qahalId: 'wisdom-commons',
  selfHub: {
    hubId: 'hub:federation:wisdom-commons',
    status: 'healthy',
    allocatedGB: 830,
    lastSeen: 'now',
  },
  stewardsForUs: [
    {
      stewardId: 'cofc-congregation',
      stewardName: 'Matthew\'s congregation (Tennessee region)',
      health: 'healthy',
      percent: 100,
      lastSync: '15m ago',
      replicating: 'federation commons + wisdom-exchange ledger',
    },
    {
      stewardId: 'tennessee-congregation',
      stewardName: 'Tennessee congregation (sermon-series contributor)',
      health: 'healthy',
      percent: 98,
      lastSync: '22m ago',
      replicating: 'federation commons',
    },
    {
      stewardId: 'oklahoma-congregation',
      stewardName: 'Oklahoma congregation (neighborhood-meals contributor)',
      health: 'healthy',
      percent: 100,
      lastSync: '10m ago',
      replicating: 'federation commons',
    },
    {
      stewardId: 'texas-congregation',
      stewardName: 'Texas congregation (theological-essay contributor)',
      health: 'healthy',
      percent: 96,
      lastSync: '45m ago',
      replicating: 'federation commons',
    },
    {
      stewardId: 'arkansas-sister-congregation',
      stewardName: 'Arkansas sister congregation',
      health: 'healthy',
      percent: 100,
      lastSync: '30m ago',
      replicating: 'federation commons — peer fellowship maintained',
    },
    {
      stewardId: 'ohio-congregation',
      stewardName: 'Ohio congregation (minister search)',
      health: 'degraded',
      percent: 72,
      lastSync: '3h ago',
      replicating: 'federation commons · capacity reduced during transition',
    },
    {
      stewardId: 'west-virginia-congregation',
      stewardName: 'West Virginia congregation (grieving)',
      health: 'degraded',
      percent: 80,
      lastSync: '2h ago',
      replicating: 'federation commons · reduced capacity during bereavement period',
    },
  ],
  weStewardFor: [
    {
      stewardId: 'all-83-participating-congregations',
      description:
        'each congregation contributes compute to the commons pool; the commons holds resilience shares for each participant — no single congregation is the compute-dominant node',
    },
  ],
  recoveryReadiness: {
    ready: true,
    shamirThreshold: '4/7',
    lastDrill: '45 days ago (passed)',
    nextDrill: '15 days',
  },
};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/** All topologies keyed by Qahal id for O(1) lookup. */
const ALL_TOPOLOGIES: Record<string, MockComputeTopology> = {
  'dowell-household': DOWELL_HOUSEHOLD_TOPOLOGY,
  'cofc-congregation': COFC_CONGREGATION_TOPOLOGY,
  'hardins-life-group': LIFE_GROUP_TOPOLOGY,
  'wisdom-commons': WISDOM_COMMONS_TOPOLOGY,
};

/**
 * Look up a compute topology by its Qahal's stable id.
 *
 * @param qahalId - The Qahal's stable kebab-case id
 * @returns The matching topology, or undefined if not found.
 *
 * @example
 * const topology = topologyForQahal('dowell-household');
 */
export function topologyForQahal(qahalId: string): MockComputeTopology | undefined {
  return ALL_TOPOLOGIES[qahalId];
}
