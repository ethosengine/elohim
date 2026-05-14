/**
 * Account Package Generator
 *
 * Generates per-human account packages — the initial conditions (contracts)
 * for each human's world in the network. The complex emergent state (sharding,
 * sync, gossip, relationship-driven views) emerges as pods communicate.
 *
 * An AccountPackage is a portable, self-contained representation of a human's
 * world: what content they can see, at what reach level, who they're connected
 * to, and what they steward.
 *
 * Usage:
 *   npx tsx src/account-package.ts                      # Generate all packages
 *   npx tsx src/account-package.ts --dry-run             # Preview without writing
 *   npx tsx src/account-package.ts --human matthew       # Single human
 *   npx tsx src/account-package.ts --stats               # Show assignment stats
 */

import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

import type {
  AccountIdentityView,
  AccountPackageInputView,
  CollectiveSeedView,
  ContentAssignmentView,
  OrganizationContextView,
  PackageManifestView,
  RelationshipSeedView,
  StewardshipSeedView,
} from '@elohim/storage-client';

// =============================================================================
// Directory Setup
// =============================================================================

const __filename = fileURLToPath(import.meta.url);
const SEEDER_DIR = path.dirname(path.dirname(__filename));
const GENESIS_DIR = path.resolve(SEEDER_DIR, '..');
const DATA_DIR = path.join(GENESIS_DIR, 'data', 'lamad');
const HUMANS_FILE = path.join(GENESIS_DIR, 'data', 'humans', 'humans.json');
const OUTPUT_DIR = path.join(GENESIS_DIR, 'data', 'account-packages');
const CONDUCTOR_GROUPS_FILE = path.join(OUTPUT_DIR, 'conductor-groups.json');

// =============================================================================
// Types
// =============================================================================

/** Reach levels aligned with elohim-node/src/storage/reach.rs */
export type Reach = 'private' | 'invited' | 'local' | 'neighborhood' | 'municipal' | 'commons';

/** Why this content was assigned to this human */
export type AssignmentReason = 'steward' | 'affinity' | 'relationship' | 'commons' | 'guardian';

/**
 * AccountPackage — the initial conditions for a human's world.
 *
 * This is what genesis produces (per human story) and what the platform's
 * import/export API (elohim-storage) consumes. Type alias for the API
 * contract type — ensuring the generator can only produce what the API accepts.
 */
export type AccountPackage = AccountPackageInputView;

// =============================================================================
// Types from humans.json
// =============================================================================

interface HumanEntry {
  id: string;
  displayName: string;
  bio: string;
  category: string;
  profileReach: string;
  location?: { layer: string; name: string };
  organizations?: Array<{ id: string; name: string; role: string }>;
  communities?: string[];
  affinities?: string[];
  accessibilityNeeds?: string[];
  guardianIds?: string[];
  ageCategory?: string;
  isPseudonymous?: boolean;
  acceptingConnections?: boolean;
  flags?: Array<{ type: string; reason: string; count?: number; severity?: string }>;
  notes?: string;
}

interface RelationshipEntry {
  source: string;
  target: string;
  relationshipType: string;
  intimacyLevel: 'intimate' | 'trusted' | 'connection' | 'recognition';
  context?: string;
}

interface HumansData {
  humans: HumanEntry[];
  relationships: RelationshipEntry[];
}

interface ContentItem {
  id: string;
  metadata?: { category?: string };
  tags?: string[];
}

interface StewardRatio {
  presenceId: string;
  ratio: number;
}

interface ConductorGroup {
  group: number;
  name: string;
  description: string;
  humanIds: string[];
}

// =============================================================================
// Category-to-Steward Mapping (from seed-stewardship.ts)
// =============================================================================

const CATEGORY_STEWARD_MAP: Record<string, StewardRatio[]> = {
  'value-scanner': [
    { presenceId: 'adam-firstman', ratio: 0.35 },
    { presenceId: 'jessica-spouse', ratio: 0.25 },
    { presenceId: 'matthew-dowell', ratio: 0.20 },
    { presenceId: 'frank-farmer', ratio: 0.20 },
  ],
  'public-observer': [
    { presenceId: 'eve-firstwoman', ratio: 0.50 },
    { presenceId: 'nancy-neighbor', ratio: 0.30 },
    { presenceId: 'matthew-dowell', ratio: 0.20 },
  ],
  governance: [
    { presenceId: 'nancy-neighbor', ratio: 0.40 },
    { presenceId: 'matthew-dowell', ratio: 0.35 },
    { presenceId: 'eve-firstwoman', ratio: 0.25 },
  ],
  'autonomous-entity': [
    { presenceId: 'meriadoc-moneybags', ratio: 0.50 },
    { presenceId: 'matthew-dowell', ratio: 0.30 },
    { presenceId: 'frank-farmer', ratio: 0.20 },
  ],
  'social-medium': [
    { presenceId: 'eve-firstwoman', ratio: 0.45 },
    { presenceId: 'jessica-spouse', ratio: 0.30 },
    { presenceId: 'matthew-dowell', ratio: 0.25 },
  ],
  scripture: [
    { presenceId: 'pete-pastor', ratio: 0.60 },
    { presenceId: 'matthew-dowell', ratio: 0.40 },
  ],
  fct: [
    { presenceId: 'pete-pastor', ratio: 0.50 },
    { presenceId: 'matthew-dowell', ratio: 0.50 },
  ],
  'fct-media': [
    { presenceId: 'pete-pastor', ratio: 0.50 },
    { presenceId: 'matthew-dowell', ratio: 0.50 },
  ],
  'fct-practice': [
    { presenceId: 'pete-pastor', ratio: 0.50 },
    { presenceId: 'matthew-dowell', ratio: 0.50 },
  ],
  'fct-narrative': [
    { presenceId: 'pete-pastor', ratio: 0.50 },
    { presenceId: 'matthew-dowell', ratio: 0.50 },
  ],
  'fct-activity': [
    { presenceId: 'pete-pastor', ratio: 0.50 },
    { presenceId: 'matthew-dowell', ratio: 0.50 },
  ],
  'economic-coordination': [
    { presenceId: 'meriadoc-moneybags', ratio: 0.40 },
    { presenceId: 'frank-farmer', ratio: 0.35 },
    { presenceId: 'matthew-dowell', ratio: 0.25 },
  ],
  community: [
    { presenceId: 'nancy-neighbor', ratio: 0.40 },
    { presenceId: 'adam-firstman', ratio: 0.30 },
    { presenceId: 'matthew-dowell', ratio: 0.30 },
  ],
  'local-economy': [
    { presenceId: 'frank-farmer', ratio: 0.40 },
    { presenceId: 'meriadoc-moneybags', ratio: 0.35 },
    { presenceId: 'matthew-dowell', ratio: 0.25 },
  ],
  foundation: [
    { presenceId: 'dan-developer', ratio: 0.50 },
    { presenceId: 'matthew-dowell', ratio: 0.50 },
  ],
  contributor: [
    { presenceId: 'dan-developer', ratio: 0.50 },
    { presenceId: 'matthew-dowell', ratio: 0.50 },
  ],
  general: [
    { presenceId: 'matthew-dowell', ratio: 0.60 },
    { presenceId: 'dan-developer', ratio: 0.40 },
  ],
  'landing-page-concept': [{ presenceId: 'matthew-dowell', ratio: 1.0 }],
  'algorithmic-bias': [
    { presenceId: 'eve-firstwoman', ratio: 0.50 },
    { presenceId: 'matthew-dowell', ratio: 0.50 },
  ],
};

/**
 * Map from presence IDs (used in stewardship) to human IDs (used in humans.json).
 * Presence IDs strip the "human-" prefix and use different naming.
 */
const PRESENCE_TO_HUMAN: Record<string, string> = {
  'matthew-dowell': 'human-matthew-manager',
  'adam-firstman': 'human-adam-firstman',
  'eve-firstwoman': 'human-eve-firstwoman',
  'jessica-spouse': 'human-jessica-spouse',
  'dan-developer': 'human-dan-developer',
  'nancy-neighbor': 'human-nancy-neighbor',
  'pete-pastor': 'human-pete-pastor',
  'frank-farmer': 'human-frank-farmer',
  'meriadoc-moneybags': 'human-meriadoc-moneybags',
};

/** Reverse mapping: human ID → presence ID */
const HUMAN_TO_PRESENCE: Record<string, string> = Object.fromEntries(
  Object.entries(PRESENCE_TO_HUMAN).map(([k, v]) => [v, k])
);

// =============================================================================
// Affinity Mapping
//
// Maps human affinities to content categories. A human's affinities determine
// which content categories they have natural access to at neighborhood reach.
// =============================================================================

const AFFINITY_TO_CATEGORIES: Record<string, string[]> = {
  'governance': ['governance'],
  'ai-ethics': ['governance', 'algorithmic-bias'],
  'holochain': ['foundation', 'contributor'],
  'family-systems': ['social-medium', 'value-scanner'],
  'christian-theology': ['scripture', 'fct', 'fct-media', 'fct-practice', 'fct-narrative', 'fct-activity'],
  'gardening': ['value-scanner', 'community'],
  'stewardship': ['value-scanner', 'economic-coordination'],
  'ecology': ['value-scanner'],
  'knowledge-seeking': ['public-observer', 'algorithmic-bias'],
  'courage': ['public-observer'],
  'truth-seeking': ['public-observer', 'algorithmic-bias'],
  'education': ['value-scanner', 'community'],
  'community-building': ['community', 'social-medium'],
  'health-wellness': ['value-scanner'],
  'community-organizing': ['community', 'governance'],
  'local-government': ['governance', 'community'],
  'media-literacy': ['public-observer', 'algorithmic-bias'],
  'pastoral-care': ['scripture', 'fct', 'fct-media', 'fct-practice', 'fct-narrative', 'fct-activity'],
  'discipleship': ['scripture', 'fct'],
  'faith-and-technology': ['scripture', 'fct', 'foundation'],
  'learning-science': ['value-scanner', 'community'],
  'mastery-learning': ['value-scanner'],
  'rust': ['foundation', 'contributor'],
  'distributed-systems': ['foundation', 'contributor'],
  'open-source': ['foundation', 'contributor'],
  'privacy': ['foundation', 'public-observer'],
  'impact-investing': ['economic-coordination', 'autonomous-entity', 'local-economy'],
  'community-economics': ['economic-coordination', 'local-economy'],
  'regenerative-finance': ['economic-coordination'],
  'social-enterprise': ['autonomous-entity'],
  'rea-economics': ['economic-coordination'],
  'regenerative-agriculture': ['value-scanner', 'local-economy'],
  'local-food': ['local-economy', 'community'],
  'small-business': ['local-economy', 'autonomous-entity'],
  'supply-chain': ['local-economy', 'autonomous-entity'],
  'manufacturing': ['local-economy', 'autonomous-entity'],
  'worker-ownership': ['autonomous-entity', 'economic-coordination'],
  'skilled-trades': ['local-economy'],
  'construction': ['local-economy'],
  'apprenticeship': ['local-economy', 'community'],
  'gig-economy': ['value-scanner', 'economic-coordination'],
  'worker-rights': ['value-scanner', 'autonomous-entity'],
  'labor-rights': ['value-scanner', 'autonomous-entity'],
  'infrastructure': ['local-economy'],
  'language-learning': ['community'],
  'engineering': ['foundation'],
  'faith': ['scripture', 'fct'],
};

// =============================================================================
// Conductor Groups
// =============================================================================

const CONDUCTOR_GROUPS: ConductorGroup[] = [
  {
    group: 0,
    name: 'Dowell Household',
    description: 'Core family + workplace — the founders and builders',
    humanIds: [
      'human-matthew-manager',
      'human-jessica-spouse',
      'human-james-son',
      'human-gertrude-grandma',
      'human-dan-developer',
    ],
  },
  {
    group: 1,
    name: 'Eden Household',
    description: 'The originals + community connectors',
    humanIds: [
      'human-adam-firstman',
      'human-eve-firstwoman',
      'human-nancy-neighbor',
      'human-pam-polarized',
      'human-janice-janitor',
    ],
  },
  {
    group: 2,
    name: 'Valley Economy',
    description: 'Local economy workers, builders, and faith community',
    humanIds: [
      'human-frank-farmer',
      'human-georgina-grocer',
      'human-manny-manufacturer',
      'human-bub-builder',
      'human-pete-pastor',
      'human-terrance-tutor',
      'human-tommy-teacher',
    ],
  },
  {
    group: 3,
    name: 'Extended Network',
    description: 'Investors, gig workers, newcomers, visitors, and adversarial actors',
    humanIds: [
      'human-meriadoc-moneybags',
      'human-ginny-gigworker',
      'human-mary-miner',
      'human-lenny-lineworker',
      'human-mary-migrant',
      'human-ronald-refugee',
      'human-traveler',
      'human-charlie-challenged',
      'human-sam-shipper',
      'human-renold-regent',
      'human-dolittle-doctor',
      'human-tiffany-teen',
    ],
  },
];

// =============================================================================
// Data Loading
// =============================================================================

function loadHumans(): HumansData {
  const raw = fs.readFileSync(HUMANS_FILE, 'utf-8');
  return JSON.parse(raw);
}

function loadContentItems(): ContentItem[] {
  const contentDir = path.join(DATA_DIR, 'content');
  if (!fs.existsSync(contentDir)) {
    console.error(`Content directory not found: ${contentDir}`);
    return [];
  }

  const items: ContentItem[] = [];

  // Load flat files
  for (const file of fs.readdirSync(contentDir)) {
    const filePath = path.join(contentDir, file);
    if (file.endsWith('.json') && !fs.statSync(filePath).isDirectory()) {
      try {
        items.push(JSON.parse(fs.readFileSync(filePath, 'utf-8')));
      } catch {
        // Skip malformed
      }
    }
  }

  // Load subdirectory files (e.g., humans/)
  for (const dir of fs.readdirSync(contentDir)) {
    const dirPath = path.join(contentDir, dir);
    if (fs.statSync(dirPath).isDirectory()) {
      for (const file of fs.readdirSync(dirPath)) {
        if (file.endsWith('.json') && file !== 'index.json') {
          try {
            const item = JSON.parse(fs.readFileSync(path.join(dirPath, file), 'utf-8'));
            if (item.id) {
              items.push(item);
            }
          } catch {
            // Skip malformed
          }
        }
      }
    }
  }

  return items;
}

function getConductorGroup(humanId: string): number {
  for (const group of CONDUCTOR_GROUPS) {
    if (group.humanIds.includes(humanId)) {
      return group.group;
    }
  }
  return 3; // Default to extended network
}

// =============================================================================
// Content Assignment Algorithm
//
// Determines what content each human can see and at what reach level.
// Priority (highest reach wins):
//   1. Steward → local (they have full access to content they steward)
//   2. Affinity match → neighborhood (content matches their interests)
//   3. Relationship graph → municipal (connected via relationships)
//   4. Commons → always accessible to all
// =============================================================================

function buildContentAssignments(
  human: HumanEntry,
  allContent: ContentItem[],
  relationships: RelationshipEntry[],
  allHumans: HumanEntry[]
): ContentAssignmentView[] {
  const assignments = new Map<string, ContentAssignmentView>();
  const humanAffinities = new Set(human.affinities || []);
  const presenceId = HUMAN_TO_PRESENCE[human.id];

  // Build this human's relationship graph (direct connections)
  const connectedHumanIds = new Set<string>();
  for (const rel of relationships) {
    if (rel.source === human.id) connectedHumanIds.add(rel.target);
    if (rel.target === human.id) connectedHumanIds.add(rel.source);
  }

  // Build the set of categories this human has affinity for
  const affinityCategories = new Set<string>();
  for (const affinity of humanAffinities) {
    const cats = AFFINITY_TO_CATEGORIES[affinity];
    if (cats) {
      for (const cat of cats) affinityCategories.add(cat);
    }
  }

  // Build the set of categories connected humans steward
  const relationshipCategories = new Set<string>();
  for (const connectedId of connectedHumanIds) {
    const connectedPresence = HUMAN_TO_PRESENCE[connectedId];
    if (!connectedPresence) continue;
    for (const [category, stewards] of Object.entries(CATEGORY_STEWARD_MAP)) {
      if (stewards.some(s => s.presenceId === connectedPresence)) {
        relationshipCategories.add(category);
      }
    }
  }

  for (const content of allContent) {
    const category = content.metadata?.category;
    const contentId = content.id;

    // 1. Is this human a steward of this content's category?
    if (presenceId && category && CATEGORY_STEWARD_MAP[category]) {
      const stewardEntry = CATEGORY_STEWARD_MAP[category].find(
        s => s.presenceId === presenceId
      );
      if (stewardEntry) {
        assignments.set(contentId, {
          contentId,
          reach: 'local',
          reason: 'steward',
          stewardRatio: stewardEntry.ratio,
        });
        continue;
      }
    }

    // 2. Does this content's category match the human's affinities?
    if (category && affinityCategories.has(category)) {
      assignments.set(contentId, {
        contentId,
        reach: 'familiar',
        reason: 'affinity',
        stewardRatio: null,
      });
      continue;
    }

    // 3. Is this content in a category stewarded by a connected human?
    if (category && relationshipCategories.has(category)) {
      assignments.set(contentId, {
        contentId,
        reach: 'community',
        reason: 'relationship',
        stewardRatio: null,
      });
      continue;
    }

    // 4. Content with no category or "general" category → commons
    if (!category || category === 'general' || category === 'landing-page-concept' || category === 'NONE') {
      assignments.set(contentId, {
        contentId,
        reach: 'commons',
        reason: 'commons',
        stewardRatio: null,
      });
      continue;
    }

    // 5. Everything else — not assigned to this human
    // (they can still discover it via P2P if it's at commons reach elsewhere)
  }

  // Guardian access: if this human has guardians, they see what guardians see
  if (human.guardianIds) {
    // Guardians don't expand content assignment — guardians manage access.
    // The child sees content appropriate to their affinities, not the guardian's.
  }

  return Array.from(assignments.values());
}

function buildRelationshipSeeds(
  human: HumanEntry,
  relationships: RelationshipEntry[]
): RelationshipSeedView[] {
  const seeds: RelationshipSeedView[] = [];

  for (const rel of relationships) {
    if (rel.source === human.id) {
      seeds.push({
        targetId: rel.target,
        relationshipType: rel.relationshipType,
        intimacyLevel: rel.intimacyLevel,
        isBidirectional: true,
        reach: null,
      });
    } else if (rel.target === human.id) {
      // Bidirectional: also include reverse relationships
      seeds.push({
        targetId: rel.source,
        relationshipType: rel.relationshipType,
        intimacyLevel: rel.intimacyLevel,
        isBidirectional: true,
        reach: null,
      });
    }
  }

  return seeds;
}

function buildStewardshipSeeds(human: HumanEntry): StewardshipSeedView[] {
  const presenceId = HUMAN_TO_PRESENCE[human.id];
  if (!presenceId) return [];

  const seeds: StewardshipSeedView[] = [];
  for (const [category, stewards] of Object.entries(CATEGORY_STEWARD_MAP)) {
    const entry = stewards.find(s => s.presenceId === presenceId);
    if (entry) {
      seeds.push({
        contentCategory: category,
        allocationRatio: entry.ratio,
        contributionType: 'steward',
      });
    }
  }

  return seeds;
}

// =============================================================================
// Collective Seeds
//
// Three tiers of collective participation:
//   1. Household (from conductor group) — intimate/trusted
//   2. Communities (from human.communities) — connection
//   3. Organizations (from human.organizations) — connection with role context
// =============================================================================

/** Conductor group index → household collective ID */
const CONDUCTOR_GROUP_COLLECTIVE: Record<number, string> = {
  0: 'household-dowell',
  1: 'household-eden',
  2: 'household-valley-economy',
  3: 'household-extended',
};

function buildCollectiveSeeds(human: HumanEntry): CollectiveSeedView[] {
  const seeds: CollectiveSeedView[] = [];

  // Tier 1: Household from conductor group
  const groupIndex = getConductorGroup(human.id);
  const householdId = CONDUCTOR_GROUP_COLLECTIVE[groupIndex];
  if (householdId) {
    // Groups 0-1 are family households → intimate; groups 2-3 are broader → trusted
    const intimacy = groupIndex <= 1 ? 'intimate' : 'trusted';
    seeds.push({
      collectiveId: householdId,
      roleContext: null,
      intimacyLevel: intimacy,
    });
  }

  // Tier 2: Communities from human.communities
  if (human.communities) {
    for (const communityId of human.communities) {
      seeds.push({
        collectiveId: communityId,
        roleContext: null,
        intimacyLevel: 'connection',
      });
    }
  }

  // Tier 3: Organizations from human.organizations
  if (human.organizations) {
    for (const org of human.organizations) {
      seeds.push({
        collectiveId: org.id,
        roleContext: org.role,
        intimacyLevel: 'connection',
      });
    }
  }

  return seeds;
}

// =============================================================================
// Package Generation
// =============================================================================

function computeSimpleHash(data: string): string {
  // Simple hash for manifest tracking (not cryptographic)
  let hash = 0;
  for (let i = 0; i < data.length; i++) {
    const char = data.charCodeAt(i);
    hash = ((hash << 5) - hash) + char;
    hash |= 0;
  }
  return Math.abs(hash).toString(16).padStart(8, '0');
}

export function generatePackage(
  human: HumanEntry,
  allContent: ContentItem[],
  humansData: HumansData
): AccountPackage {
  const contentAssignments = buildContentAssignments(
    human,
    allContent,
    humansData.relationships,
    humansData.humans
  );

  const relationships = buildRelationshipSeeds(human, humansData.relationships);
  const stewardship = buildStewardshipSeeds(human);
  const collectives = buildCollectiveSeeds(human);
  const conductorGroup = getConductorGroup(human.id);

  // Flatten location object to string for API compatibility
  const locationStr = human.location
    ? `${human.location.name} (${human.location.layer})`
    : null;

  // Pass-through organizations for identity display
  const organizations: OrganizationContextView[] = (human.organizations || []).map(org => ({
    id: org.id,
    name: org.name,
    role: org.role,
  }));

  return {
    schemaVersion: 1,
    identity: {
      humanId: human.id,
      displayName: human.displayName,
      category: human.category,
      profileReach: human.profileReach,
      bio: human.bio,
      location: locationStr,
      affinities: human.affinities || [],
      organizations,
    },
    content: contentAssignments,
    relationships,
    stewardship,
    collectives,
    conductorGroup,
    manifest: {
      version: '1.0.0',
      generatedAt: new Date().toISOString(),
      sourceStory: 'genesis',
      contentHash: computeSimpleHash(JSON.stringify(humansData)),
    },
  };
}

export function generateAllPackages(
  humansData: HumansData,
  allContent: ContentItem[]
): Map<string, AccountPackage> {
  const packages = new Map<string, AccountPackage>();

  for (const human of humansData.humans) {
    packages.set(human.id, generatePackage(human, allContent, humansData));
  }

  return packages;
}

// =============================================================================
// Stats Display
// =============================================================================

function computeReachDistribution(content: ContentAssignmentView[]): Record<Reach, number> {
  const dist: Record<Reach, number> = {
    private: 0, invited: 0, local: 0, neighborhood: 0, municipal: 0, commons: 0,
  };
  for (const a of content) dist[a.reach as Reach]++;
  return dist;
}

function displayStats(packages: Map<string, AccountPackage>, totalContent: number): void {
  console.log('\n' + '='.repeat(70));
  console.log('ACCOUNT PACKAGE GENERATION STATS');
  console.log('='.repeat(70));
  console.log(`Total content items: ${totalContent}`);
  console.log(`Total humans: ${packages.size}`);
  console.log();

  // Per-human summary
  console.log('Human'.padEnd(25) + 'Content'.padStart(8) + 'Rels'.padStart(6) +
    'Stew'.padStart(6) + 'Coll'.padStart(6) + 'Cond'.padStart(6) + '  Reach Distribution');
  console.log('-'.repeat(76));

  const globalReach: Record<string, number> = {};

  for (const [, pkg] of [...packages.entries()].sort((a, b) =>
    b[1].content.length - a[1].content.length
  )) {
    const rd = computeReachDistribution(pkg.content);
    const reachStr = Object.entries(rd)
      .filter(([, v]) => v > 0)
      .map(([k, v]) => `${k[0].toUpperCase()}:${v}`)
      .join(' ');

    console.log(
      pkg.identity.displayName.padEnd(25) +
      String(pkg.content.length).padStart(8) +
      String(pkg.relationships.length).padStart(6) +
      String(pkg.stewardship.length).padStart(6) +
      String(pkg.collectives.length).padStart(6) +
      String(pkg.conductorGroup ?? 0).padStart(6) +
      '  ' + reachStr
    );

    for (const [reach, count] of Object.entries(rd)) {
      globalReach[reach] = (globalReach[reach] || 0) + count;
    }
  }

  console.log();
  console.log('Global reach distribution (sum across all humans):');
  for (const [reach, count] of Object.entries(globalReach).sort((a, b) => b[1] - a[1])) {
    const bar = '#'.repeat(Math.min(50, Math.round(count / 50)));
    console.log(`  ${reach.padEnd(15)} ${String(count).padStart(6)} ${bar}`);
  }

  // Conductor group summary
  console.log();
  console.log('Conductor groups:');
  for (const group of CONDUCTOR_GROUPS) {
    const groupPkgs = [...packages.values()].filter(p => p.conductorGroup === group.group);
    const totalContent = groupPkgs.reduce((sum, p) => sum + p.content.length, 0);
    console.log(`  ${group.group}: ${group.name} — ${groupPkgs.length} humans, ${totalContent} content assignments`);
  }

  // Content coverage
  const allAssignedIds = new Set<string>();
  for (const pkg of packages.values()) {
    for (const a of pkg.content) {
      allAssignedIds.add(a.contentId);
    }
  }
  const unassigned = totalContent - allAssignedIds.size;
  console.log();
  console.log(`Content coverage: ${allAssignedIds.size}/${totalContent} items assigned to at least one human`);
  if (unassigned > 0) {
    console.log(`  ${unassigned} items not assigned to any human (will need commons reach or explicit assignment)`);
  }

  console.log('='.repeat(70));
}

// =============================================================================
// File Output
// =============================================================================

function writePackages(packages: Map<string, AccountPackage>): void {
  if (!fs.existsSync(OUTPUT_DIR)) {
    fs.mkdirSync(OUTPUT_DIR, { recursive: true });
  }

  for (const [humanId, pkg] of packages) {
    const filename = `${humanId.replace('human-', '')}.json`;
    const filePath = path.join(OUTPUT_DIR, filename);
    fs.writeFileSync(filePath, JSON.stringify(pkg, null, 2));
  }

  // Write conductor groups file
  fs.writeFileSync(
    CONDUCTOR_GROUPS_FILE,
    JSON.stringify(
      {
        version: '1.0.0',
        generatedAt: new Date().toISOString(),
        groups: CONDUCTOR_GROUPS,
      },
      null,
      2
    )
  );

  // Write index
  const index = {
    version: '1.0.0',
    generatedAt: new Date().toISOString(),
    totalPackages: packages.size,
    packages: [...packages.entries()].map(([id, pkg]) => ({
      humanId: id,
      displayName: pkg.identity.displayName,
      category: pkg.identity.category,
      conductorGroup: pkg.conductorGroup,
      contentCount: pkg.content.length,
      relationshipCount: pkg.relationships.length,
      stewardshipCount: pkg.stewardship.length,
      collectiveCount: pkg.collectives.length,
    })),
  };

  fs.writeFileSync(path.join(OUTPUT_DIR, 'index.json'), JSON.stringify(index, null, 2));
}

// =============================================================================
// CLI
// =============================================================================

function main(): void {
  const args = process.argv.slice(2);
  const DRY_RUN = args.includes('--dry-run');
  const STATS_ONLY = args.includes('--stats');
  const humanFilter = args.find(a => a.startsWith('--human='))?.split('=')[1];

  console.log('Account Package Generator');
  console.log('========================');
  console.log(`Humans file: ${HUMANS_FILE}`);
  console.log(`Content dir: ${DATA_DIR}/content`);
  console.log(`Output dir:  ${OUTPUT_DIR}`);
  if (DRY_RUN) console.log('Mode: DRY RUN');
  if (humanFilter) console.log(`Filter: ${humanFilter}`);
  console.log();

  // Load data
  console.log('Loading humans...');
  const humansData = loadHumans();
  console.log(`  ${humansData.humans.length} humans, ${humansData.relationships.length} relationships`);

  console.log('Loading content...');
  const allContent = loadContentItems();
  console.log(`  ${allContent.length} content items`);

  // Generate
  console.log('Generating packages...');
  let packages: Map<string, AccountPackage>;

  if (humanFilter) {
    const human = humansData.humans.find(
      h => h.id.includes(humanFilter) || h.displayName.toLowerCase().includes(humanFilter.toLowerCase())
    );
    if (!human) {
      console.error(`Human not found: ${humanFilter}`);
      process.exit(1);
    }
    packages = new Map([[human.id, generatePackage(human, allContent, humansData)]]);
  } else {
    packages = generateAllPackages(humansData, allContent);
  }

  // Display stats
  displayStats(packages, allContent.length);

  if (STATS_ONLY || DRY_RUN) {
    if (DRY_RUN) console.log('\nDry run complete. No files written.');
    return;
  }

  // Write
  console.log('\nWriting packages...');
  writePackages(packages);
  console.log(`  Written ${packages.size} packages to ${OUTPUT_DIR}`);
  console.log(`  Written conductor-groups.json`);
  console.log(`  Written index.json`);
  console.log('\nDone!');
}

// Run if executed directly
main();
