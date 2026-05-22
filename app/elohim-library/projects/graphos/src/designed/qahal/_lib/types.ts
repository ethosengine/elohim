/**
 * Type contracts for the Qahal homepage Library B composer.
 *
 * - `Scene` is the fixture shape every canonical + variation file conforms to.
 *   It bundles a coherent slice of the Qahal substrate at one moment (the
 *   rubric, the relevant members, the stream, the social-compute topology,
 *   the co-steward observation, the curated EPRs + external links).
 *
 * - `RenderOpts` is the rendering-context payload every story passes to
 *   `renderQahalHomepage`. It drives capability gating, power-user gating,
 *   lens forwarding, and panel routing.
 *
 * - `QahalArchetype` discriminates the four worked-example archetypes; the
 *   composer reads it to vary panel set + sidebar shape per UX spec §3 + §4.
 */

import type {
  MockImagodeiProfile,
  CapabilityTier,
} from '../../../default/qahal/fixtures/primitives/mock-imagodei-profiles';
import type { MockCareEconomyEvent } from '../../../default/qahal/fixtures/primitives/mock-care-economy-events';
import type { MockRubric } from '../../../default/qahal/fixtures/primitives/mock-rubrics';
import type { MockComputeTopology } from '../../../default/qahal/fixtures/primitives/mock-social-compute-topology';

export type QahalArchetype = 'household' | 'congregation' | 'life-group' | 'wisdom-commons';

export type Lens = 'minimal' | 'simple' | 'standard' | 'detail' | 'debug' | 'trace';

export type ActivePanel =
  | 'stream'
  | 'member-ring'
  | 'rules'
  | 'co-steward'
  | 'social-compute'
  | 'standing-inspector'
  | 'shefa-resources'
  | 'attestations'
  | 'graph-discovery';

/** Reference to another Qahal the viewer participates in (collective-switcher icon row). */
export interface QahalReference {
  id: string;
  icon: string;
  label: string;
}

/** A curated EPR shown under the ◆ sidebar section. */
export interface CuratedEpr {
  id: string;
  title: string;
  provenance: 'curated-epr';
}

/** An external hyperlink shown under the ⤤ sidebar section (capability-gated by rubric). */
export interface ExternalLink {
  id: string;
  title: string;
  url: string;
  /** Capability tiers permitted to see this link per the household rubric. */
  visibilityRequirement: CapabilityTier[];
}

/**
 * A Scene — coherent slice of one Qahal at one moment.
 *
 * Each canonical scene is grounded in a specific narrative from
 * genesis/docs/superpowers/specs/2026-05-21-qahal-section-4-canonical-narratives.md.
 * Variations are synthetic but follow the same shape.
 */
export interface Scene {
  id: string;
  qahalIcon: string;
  qahalLabel: string;
  qahalArchetype: QahalArchetype;

  /** Other Qahals the viewer participates in — rendered as additional switcher icons. */
  otherQahals: QahalReference[];

  rubric: MockRubric;
  members: MockImagodeiProfile[];
  streamEvents: MockCareEconomyEvent[];
  computeTopology: MockComputeTopology;

  /** The co-steward's reflective observation (e.g., "the household is steady"). */
  coStewardObservation: string;

  curatedEprs: CuratedEpr[];
  externalLinks: ExternalLink[];

  /** Stream-event IDs flagged as awaiting acknowledgment (per UX §4.1). */
  pendingAcknowledgments: string[];
}

/**
 * Rendering context — what the story passes to `renderQahalHomepage`.
 */
export interface RenderOpts {
  /** Capability tier of the viewer. Drives external-link gating + protected-tier markers. */
  viewerTier: CapabilityTier;

  /** Imagodei-setting 'Power-user view' — true mounts power-user-expandable section. */
  powerUserVisible: boolean;

  /** Capability profile lens forwarded to every element. */
  lens: Lens;

  /** Active panel in the main viewer. Defaults to 'stream' inside the composer. */
  activePanel?: ActivePanel;

  /** Active Qahal id in the switcher. Defaults to scene.id. */
  activeQahalId?: string;

  /** Locale forwarded to every element. Defaults to 'en'. */
  locale?: string;

  /** Theme override. Defaults to 'auto' (decorator-level light/dark wins). */
  theme?: 'auto' | 'light' | 'dark';
}
