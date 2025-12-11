/**
 * Place Model - Embodied Geographic Awareness
 *
 * Places ARE ContentNodes - they have attestations, reach, and governance.
 * This is critical: places are not metadata ON content, they ARE content.
 *
 * The names of places are Elohim-negotiated social constructs, subject to
 * all the same governance as any other content. Place names can be disputed,
 * renamed through deliberation, and have multiple co-existing names.
 *
 * Key Concepts:
 * - Two reach dimensions: ContentReach (social) and GeographicReach (spatial)
 * - Bioregional Elohim as constitutional boundary enforcers
 * - Named places as negotiated social constructs
 * - Unnamed bioregional identity (the watershed doesn't need a human name)
 *
 * Theological grounding: Humans are embodied beings, placed in particular
 * locations with particular relationships to land, water, and ecosystem.
 * "The earth is the Lord's, and everything in it" (Psalm 24:1) - the land
 * has standing in the constitutional order.
 */

import type { ContentReach, ContentMetadata, ContentFlag } from './content-node.model';
import { OpenGraphMetadata } from './open-graph.model';
import { JsonLdMetadata } from './json-ld.model';

// ============================================================================
// PLACE TYPE CLASSIFICATION
// ============================================================================

/**
 * PlaceType - Classification of places in the knowledge graph
 *
 * Places can be human-constructed (administrative) or natural (bioregional).
 * Bioregional places have constitutional authority over ecological boundaries.
 */
export type PlaceType =
  // Human-constructed administrative places
  | 'neighborhood'          // Hyper-local human settlement
  | 'municipality'          // City, town, village
  | 'county'                // County, district
  | 'province'              // State, province, region
  | 'nation-state'          // Country
  | 'supranational'         // EU, ASEAN, etc.

  // Bioregional / ecological places (boundary enforcers)
  | 'watershed'             // River basin, drainage area
  | 'forest'                // Forest ecosystem
  | 'grassland'             // Prairie, savanna
  | 'wetland'               // Marsh, swamp, bog
  | 'coastal'               // Coastal zone, estuary
  | 'mountain-range'        // Mountain ecosystem
  | 'desert'                // Arid ecosystem
  | 'urban-ecosystem'       // Urban green infrastructure

  // Cultural / spiritual places
  | 'sacred-site'           // Places of spiritual significance
  | 'cultural-landscape'    // UNESCO-style cultural landscapes
  | 'indigenous-territory'  // Traditional territories

  // Functional places
  | 'learning-hub'          // Physical learning space
  | 'gathering-place'       // Community meeting space
  | 'commons'               // Shared resource area
  | 'custom';               // User-defined place type

/**
 * PlaceTypeCategory - Grouping of place types for governance routing
 */
export type PlaceTypeCategory =
  | 'administrative'        // Human political boundaries
  | 'bioregional'           // Ecological boundaries (boundary enforcers)
  | 'cultural'              // Cultural/spiritual significance
  | 'functional';           // Purpose-defined places

/**
 * Helper to categorize place types
 */
export const PLACE_TYPE_CATEGORIES: Record<PlaceType, PlaceTypeCategory> = {
  // Administrative
  'neighborhood': 'administrative',
  'municipality': 'administrative',
  'county': 'administrative',
  'province': 'administrative',
  'nation-state': 'administrative',
  'supranational': 'administrative',

  // Bioregional (boundary enforcers)
  'watershed': 'bioregional',
  'forest': 'bioregional',
  'grassland': 'bioregional',
  'wetland': 'bioregional',
  'coastal': 'bioregional',
  'mountain-range': 'bioregional',
  'desert': 'bioregional',
  'urban-ecosystem': 'bioregional',

  // Cultural
  'sacred-site': 'cultural',
  'cultural-landscape': 'cultural',
  'indigenous-territory': 'cultural',

  // Functional
  'learning-hub': 'functional',
  'gathering-place': 'functional',
  'commons': 'functional',
  'custom': 'functional'
};

// ============================================================================
// PLACE NAMING (Negotiated Social Constructs)
// ============================================================================

/**
 * PlaceNameType - How a name relates to the place
 *
 * Multiple names can coexist with different types, languages, and reach.
 * This acknowledges that naming is political and contested.
 */
export type PlaceNameType =
  | 'official'              // Government-recognized name
  | 'traditional'           // Long-standing community name
  | 'indigenous'            // Name from indigenous peoples
  | 'colloquial'            // Common informal name
  | 'historical'            // Former name (for context)
  | 'contested';            // Name under governance dispute

/**
 * PlaceNameDisputeStatus - Governance state of a name
 */
export type PlaceNameDisputeStatus =
  | 'undisputed'            // No active challenges
  | 'contested'             // Under active dispute
  | 'pending-governance'    // Awaiting deliberation outcome
  | 'resolved-coexist'      // Multiple names officially coexist
  | 'deprecated';           // Name formally deprecated (historical only)

/**
 * PlaceName - A name for a place
 *
 * Names are Elohim-negotiated social constructs. The same place can have
 * multiple names (colonial vs. indigenous, official vs. colloquial) with
 * different reach levels and dispute statuses.
 */
export interface PlaceName {
  /** The name string */
  name: string;

  /** How this name relates to the place */
  nameType: PlaceNameType;

  /** Language of the name (ISO 639-1 code) */
  language?: string;

  /** Cultural origin of this name */
  culturalOrigin?: string;

  /** Who recognizes/attests to this name */
  attestedBy?: string[];

  /** Different names can have different reach */
  reach: ContentReach;

  /** Governance state of this name */
  disputeStatus: PlaceNameDisputeStatus;

  /** If disputed, reference to governance deliberation */
  deliberationId?: string;

  /** When this name was added */
  addedAt: string;

  /** Who proposed this name */
  proposedBy?: string;
}

// ============================================================================
// PLACE GEOGRAPHY (Optional Physical Anchoring)
// ============================================================================

/**
 * BoundaryType - How precise/certain the boundaries are
 *
 * Bioregional places often have fluid boundaries (ecotones, shifting watersheds).
 * Contested places may have multiple claimed boundaries.
 */
export type BoundaryType =
  | 'precise'               // Clear, well-defined boundary
  | 'approximate'           // General area, not precisely defined
  | 'fluid'                 // Boundaries shift (seasonal, ecological)
  | 'contested'             // Multiple competing boundary claims
  | 'fractal';              // Boundaries exist at multiple scales

/**
 * GeoJSONGeometry - Standard GeoJSON geometry types
 * Using a simplified type for now; full GeoJSON spec is extensive
 */
export interface GeoJSONGeometry {
  type: 'Point' | 'Polygon' | 'MultiPolygon' | 'LineString';
  coordinates: number[] | number[][] | number[][][] | number[][][][];
}

/**
 * PlaceGeography - Physical location and boundaries
 *
 * Geography is OPTIONAL. A place can exist in the knowledge graph
 * without precise coordinates (e.g., conceptual places, places with
 * contested locations, or places defined by relationship rather than position).
 */
export interface PlaceGeography {
  /** Soft location - reference point (optional) */
  approximateCenter?: {
    latitude: number;
    longitude: number;
  };

  /** How certain are the boundaries? */
  boundaryType?: BoundaryType;

  /** Boundary polygon (optional - GeoJSON format) */
  boundary?: GeoJSONGeometry;

  /** Area for rough comparison (km²) */
  approximateAreaKm2?: number;

  /** Elevation context (important for watersheds, mountains) */
  elevationRange?: {
    minMeters: number;
    maxMeters: number;
  };

  /** Climate zone (for ecological context) */
  climateZone?: string;

  /** Timezone (for human coordination) */
  timezone?: string;

  /** Data source for geographic information */
  dataSource?: {
    source: 'openstreetmap' | 'government' | 'community' | 'scientific' | 'indigenous-mapping' | 'custom';
    sourceId?: string;
    lastUpdated: string;
    confidence: 'high' | 'medium' | 'low';
  };
}

// ============================================================================
// ECOLOGICAL RELATIONSHIPS (Bioregional Context)
// ============================================================================

/**
 * EcologicalRelationshipType - How places relate ecologically
 */
export type EcologicalRelationshipType =
  | 'drains-to'             // Water flows to (watershed hierarchy)
  | 'feeds-from'            // Water source
  | 'habitat-corridor'      // Wildlife movement pathway
  | 'ecotone'               // Transition zone between ecosystems
  | 'depends-on'            // Ecological dependency
  | 'supports'              // Provides ecological services to
  | 'climate-influenced-by' // Climate relationship
  | 'fire-regime-shared'    // Shared fire ecology
  | 'migration-route';      // Animal migration pathway

/**
 * EcologicalRelationship - How this place relates to other places ecologically
 */
export interface EcologicalRelationship {
  /** Related place ID */
  relatedPlaceId: string;

  /** Type of ecological relationship */
  relationshipType: EcologicalRelationshipType;

  /** Description of the relationship */
  description?: string;

  /** Strength/importance of relationship (0.0 - 1.0) */
  strength?: number;

  /** Evidence for this relationship */
  evidenceIds?: string[];
}

// ============================================================================
// CULTURAL CONTEXT
// ============================================================================

/**
 * CulturalContext - Cultural significance and context of a place
 */
export interface CulturalContext {
  /** Cultures with significant connection to this place */
  associatedCultures: string[];

  /** Languages historically/currently spoken */
  languages?: string[];

  /** Religious/spiritual significance */
  spiritualSignificance?: {
    traditions: string[];
    description?: string;
    restrictions?: string[];   // Access/behavior restrictions
  };

  /** Historical significance */
  historicalContext?: {
    summary: string;
    periodStart?: string;
    periodEnd?: string;
    eventIds?: string[];       // Related historical content
  };

  /** Learning traditions associated with this place */
  learningTraditions?: string[];

  /** Governance traditions */
  governanceTraditions?: string[];
}

// ============================================================================
// GEOGRAPHIC REACH (Parallel to ContentReach)
// ============================================================================

/**
 * GeographicReach - WHERE content can physically spread
 *
 * This is SEPARATE from ContentReach (social reach).
 * Elohim apply wisdom to align both dimensions.
 *
 * Example: A neighborhood newsletter might have:
 * - ContentReach: 'community' (members of the community)
 * - GeographicReach: 'neighborhood' (only relevant locally)
 *
 * Bioregional content might have:
 * - ContentReach: 'commons' (publicly available)
 * - GeographicReach: 'watershed' (only meaningful in that watershed)
 */
export type GeographicReach =
  | 'hyperlocal'            // Single building, block, or POI
  | 'neighborhood'          // Immediate geographic community
  | 'municipal'             // City/town level
  | 'regional'              // Province, state, bioregion
  | 'national'              // Country-level
  | 'continental'           // Continental relevance
  | 'global'                // Universally relevant
  | 'place-specific';       // Tied to specific named place(s)

/**
 * GeographicDeterminationMethod - How geographic context was assigned
 */
export type GeographicDeterminationMethod =
  | 'author-declared'       // Content author specified
  | 'elohim-inferred'       // Elohim analyzed and assigned
  | 'community-assigned'    // Community governance decided
  | 'bioregional-enforcement'// Bioregional Elohim required this
  | 'inherited';            // Inherited from parent content/path

/**
 * GeographicDetermination - How geographic context was determined
 */
export interface GeographicDetermination {
  /** How was this determined? */
  method: GeographicDeterminationMethod;

  /** Who/what made the determination */
  determinedBy: string;

  /** Reasoning for this determination */
  reasoning?: string;

  /** Can this be challenged through governance? */
  challengeable: boolean;

  /** When was this determined */
  determinedAt: string;
}

/**
 * GeographicContext - Geographic dimension of content
 *
 * Attached to ContentNodes to indicate geographic relevance.
 * This is parallel to (not replacing) ContentReach.
 */
export interface GeographicContext {
  /** Primary geographic reach level */
  reach: GeographicReach;

  /** If place-specific, which places? */
  specificPlaceIds?: string[];

  /** Bioregional context (content relevant to ecological boundaries) */
  bioregionalRelevance?: string[];

  /** Cultural geographic context */
  culturalRelevance?: string[];

  /** How was this determined? */
  determination: GeographicDetermination;

  /** Is this content geo-restricted? (can only be viewed from certain locations) */
  geoRestricted?: boolean;

  /** If geo-restricted, what places can view? */
  viewableFromPlaces?: string[];
}

// ============================================================================
// BIOREGIONAL AUTHORITY (Constitutional Boundary Enforcement)
// ============================================================================

/**
 * EcologicalLimitType - Categories of ecological limits
 *
 * These are CONSTITUTIONAL LIMITS - human governance cannot override them.
 * The watershed doesn't care about municipal boundaries.
 */
export type EcologicalLimitType =
  | 'carrying-capacity'     // Population/resource limit
  | 'water-availability'    // Watershed capacity
  | 'carbon-budget'         // Emissions limit for this place
  | 'biodiversity-threshold'// Minimum species/habitat requirements
  | 'pollution-threshold'   // Maximum contamination levels
  | 'land-use-boundary'     // Development/use restrictions
  | 'seasonal-restriction'  // Time-based restrictions (breeding season, etc.)
  | 'sacred-boundary'       // Cultural/spiritual protection (also constitutional)
  | 'fire-regime'           // Fire management requirements
  | 'flood-plain'           // Flood risk management
  | 'erosion-control'       // Soil protection requirements
  | 'aquifer-protection'    // Groundwater limits
  | 'custom';               // Other ecological limits

/**
 * EnforcementLevel - What happens when a limit is exceeded
 */
export type EnforcementLevel =
  | 'warning'               // Alert but no action
  | 'restrict-reach'        // Limit content/activity reach
  | 'require-governance'    // Require deliberation before proceeding
  | 'hard-block';           // Constitutional prohibition, cannot proceed

/**
 * EcologicalLimit - A specific ecological boundary
 */
export interface EcologicalLimit {
  /** Unique identifier */
  id: string;

  /** Type of limit */
  limitType: EcologicalLimitType;

  /** Human-readable description */
  description: string;

  /** Quantitative limit if measurable */
  quantitativeLimit?: {
    metric: string;
    maxValue: number;
    unit: string;
    currentValue?: number;
    lastMeasured?: string;
    measurementSource?: string;
  };

  /** What happens when limit is exceeded? */
  enforcement: EnforcementLevel;

  /** Evidence/science behind this limit */
  evidenceIds?: string[];

  /** Who established this limit? */
  establishedBy: string;

  /** When was this established? */
  establishedAt: string;

  /** Is this limit under review? */
  underReview?: boolean;

  /** If under review, deliberation ID */
  reviewDeliberationId?: string;
}

/**
 * BioregionalAuthority - Constitutional limits from ecological boundaries
 *
 * Bioregional Elohim are BOUNDARY ENFORCERS - they represent ecological limits
 * that human governance cannot override. The watershed doesn't care about
 * municipal boundaries.
 *
 * "The earth is the Lord's" - the land has standing in our constitutional order.
 */
export interface BioregionalAuthority {
  /** The bioregional place this authority derives from */
  placeId: string;

  /** What ecological limits does this place enforce? */
  ecologicalLimits: EcologicalLimit[];

  /** Constitutional basis for enforcement (text or content ID) */
  constitutionalBasis: string;

  /** Can human governance appeal? (high bar, but possible) */
  appealable: boolean;

  /** If appealable, what's the process? */
  appealProcess?: string;

  /** Required evidence standard for appeals */
  appealEvidenceStandard?: 'preponderance' | 'clear-and-convincing' | 'beyond-reasonable-doubt';

  /** Who granted this authority? */
  authorityGrantedBy: string;

  /** When was authority established? */
  authorityEstablishedAt: string;

  /** Is this authority active? */
  isActive: boolean;

  /** If inactive, why? */
  inactiveReason?: string;
}

// ============================================================================
// PLACE-AWARE ELOHIM
// ============================================================================

/**
 * PlaceCapability - Elohim capabilities for place-awareness
 */
export type PlaceCapability =
  | 'place-attestation'           // Attest to place existence/boundaries
  | 'place-naming-governance'     // Participate in naming deliberation
  | 'geographic-reach-assignment' // Assign geographic reach to content
  | 'bioregional-enforcement'     // Enforce ecological limits
  | 'cultural-context-mediation'  // Mediate cultural place disputes
  | 'place-relationship-mapping'  // Map place relationships
  | 'ecological-limit-assessment' // Assess ecological limits
  | 'place-stewardship';          // General place stewardship

/**
 * PlaceAwareElohim - Extensions to ElohimAgent for place-awareness
 */
export interface PlaceAwareElohim {
  /** What places does this Elohim serve? */
  servicePlaces: string[];

  /** Is this a bioregional Elohim (boundary enforcer)? */
  isBioregionalEnforcer: boolean;

  /** If bioregional, what ecological authority? */
  bioregionalAuthority?: BioregionalAuthority;

  /** Geographic scope even without specific places */
  geographicScope?: GeographicReach;

  /** Place-specific capabilities */
  placeCapabilities: PlaceCapability[];

  /** Languages this Elohim serves in this place */
  serviceLanguages?: string[];

  /** Cultural contexts this Elohim understands */
  culturalCompetencies?: string[];
}

// ============================================================================
// PLACE INTERFACE (Main Entity)
// ============================================================================

/**
 * Place - A location in the knowledge graph
 *
 * Places ARE ContentNodes - they have attestations, reach, and governance.
 * This is the core interface for representing places in Lamad.
 */
export interface Place {
  /** Content node identity */
  id: string;

  /** Content type is always 'place' */
  contentType: 'place';

  /** Primary name (the most widely recognized) */
  primaryName: string;

  /** All names for this place (including primary) */
  alternateNames?: PlaceName[];

  /** Place classification */
  placeType: PlaceType;

  /** Is this a bioregional/ecological place? */
  isBioregional: boolean;

  /** Description of the place */
  description: string;

  /** Optional geographic anchoring */
  geography?: PlaceGeography;

  // ---- Relationships ----

  /** Parent places (e.g., neighborhood in city in county) */
  containedBy?: string[];

  /** Child places */
  contains?: string[];

  /** Places that share territory (overlapping boundaries) */
  overlaps?: string[];

  /** Neighboring places */
  adjacentTo?: string[];

  /** Ecological relationships (for bioregional places) */
  ecologicalRelationships?: EcologicalRelationship[];

  // ---- Context ----

  /** Cultural context */
  culturalContext?: CulturalContext;

  /** Human communities associated with this place */
  associatedCommunityIds?: string[];

  /** Elohim steward for this place */
  placeElohimId?: string;

  /** If bioregional, the authority this place exercises */
  bioregionalAuthority?: BioregionalAuthority;

  // ---- Standard Content Fields ----

  /** Social reach (who in the network can see this place definition) */
  reach: ContentReach;

  /** Trust score computed from attestations */
  trustScore: number;

  /** Active attestations on this place */
  activeAttestationIds: string[];

  /** Active flags/disputes */
  flags?: ContentFlag[];

  /** Standard metadata */
  metadata: ContentMetadata;

  /** Timestamps */
  createdAt: string;
  updatedAt: string;

  /** Author/contributor who created this place entry */
  authorId?: string;

  // =========================================================================
  // Social Graph Metadata (for sharing places)
  // =========================================================================

  /**
   * Open Graph metadata for social sharing.
   * When a place is shared, this provides rich preview cards with maps/images.
   */
  socialMetadata?: OpenGraphMetadata;

  /**
   * Optional JSON-LD metadata for semantic web interoperability.
   *
   * Future: Schema.org Place or AdministrativeArea types.
   * Prevents tech debt when we need semantic web export.
   */
  linkedData?: JsonLdMetadata;
}

// ============================================================================
// PLACE HIERARCHY AND RELATIONSHIPS
// ============================================================================

/**
 * PlaceHierarchy - The containment hierarchy of a place
 */
export interface PlaceHierarchy {
  /** The place at the center of this hierarchy */
  placeId: string;

  /** Ancestors (places this is contained by), ordered from immediate parent to root */
  ancestors: PlaceHierarchyNode[];

  /** Descendants (places this contains), as a tree */
  descendants: PlaceHierarchyNode[];

  /** Siblings (places at the same level in the hierarchy) */
  siblings?: string[];
}

/**
 * PlaceHierarchyNode - A node in the place hierarchy
 */
export interface PlaceHierarchyNode {
  placeId: string;
  primaryName: string;
  placeType: PlaceType;
  children?: PlaceHierarchyNode[];
}

// ============================================================================
// SERVICE INTERFACE (Future Implementation)
// ============================================================================

/**
 * PlaceService - Service interface for place operations
 *
 * This is the contract for the PlaceService that will be implemented.
 * Models only - no implementation here.
 */
export interface PlaceServiceInterface {
  /** Get a place by ID */
  getPlace(placeId: string): Promise<Place | null>;

  /** Find places containing a geographic point */
  findPlacesContaining(lat: number, lng: number): Promise<Place[]>;

  /** Get places of a specific type */
  getPlacesByType(type: PlaceType): Promise<Place[]>;

  /** Get bioregional authorities governing a location */
  getBioregionalAuthorities(lat: number, lng: number): Promise<BioregionalAuthority[]>;

  /** Get place hierarchy (containment tree) */
  getPlaceHierarchy(placeId: string): Promise<PlaceHierarchy>;

  /** Propose a new name for a place (triggers governance) */
  proposePlaceName(placeId: string, name: PlaceName): Promise<string>; // Returns deliberation ID

  /** Get content geographically relevant to a place */
  getContentForPlace(placeId: string, options?: {
    geographicReach?: GeographicReach;
    includeBioregional?: boolean;
  }): Promise<string[]>; // Returns content IDs

  /** Check if an ecological limit would be exceeded */
  checkEcologicalLimits(placeId: string, action: string): Promise<{
    wouldExceed: boolean;
    limits: EcologicalLimit[];
    enforcement: EnforcementLevel;
  }>;
}
