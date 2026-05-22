/**
 * Canonical scene fixture — CofC Congregation, Sunday morning.
 *
 * Grounded in storyteller canonical narrative §4.2 at
 * genesis/docs/superpowers/specs/2026-05-21-qahal-section-4-canonical-narratives.md.
 *
 * Named moments encoded:
 *   - 230 members; 4 elders (Brother Cal, Thompson, Davis, Rhodes)
 *   - Romans 12 sermon series, fourth week
 *   - Youth retreat needing 2 drivers
 *   - 3 prayer requests
 *   - Co-steward observation: reach into the neighborhood is rising; 3 life-groups at cohesion threshold
 *
 * This fixture composes the existing primitives modules without authoring
 * new mock data from scratch — the COFC_ELDERS, COFC_CONGREGATION_RUBRIC,
 * COFC_SUNDAY_MORNING_STREAM, and COFC_CONGREGATION_TOPOLOGY are
 * authoritative; this file binds them into a coherent Scene.
 */

import type { Scene } from '../../../../designed/qahal/_lib/types';
import { COFC_ELDERS } from '../primitives/mock-imagodei-profiles';
import { COFC_CONGREGATION_RUBRIC } from '../primitives/mock-rubrics';
import { COFC_SUNDAY_MORNING_STREAM } from '../primitives/mock-care-economy-events';
import { COFC_CONGREGATION_TOPOLOGY } from '../primitives/mock-social-compute-topology';

export const cofcCongregationSundayMorning: Scene = {
  id: 'cofc-congregation',
  qahalIcon: '⛪',
  qahalLabel: 'Local Churches of Christ',
  qahalArchetype: 'congregation',
  otherQahals: [
    { id: 'dowell-household', icon: '🏠', label: 'Dowell Household' },
    { id: 'hardins-life-group', icon: '🪨', label: 'Tuesday Life-Group' },
    { id: 'wisdom-commons', icon: '🌳', label: 'Wisdom Commons' },
  ],
  rubric: COFC_CONGREGATION_RUBRIC,
  members: COFC_ELDERS,
  streamEvents: COFC_SUNDAY_MORNING_STREAM,
  computeTopology: COFC_CONGREGATION_TOPOLOGY,
  coStewardObservation:
    "The congregation's reach into the neighborhood is rising slightly. Three life-groups are nearing the cohesion threshold.",
  curatedEprs: [
    { id: 'romans-12-series', title: 'Romans 12 Sermon Series', provenance: 'curated-epr' },
    { id: 'communion-rota', title: 'Communion Rota', provenance: 'curated-epr' },
    { id: 'youth-retreat-plans', title: 'Youth Retreat Plans', provenance: 'curated-epr' },
  ],
  externalLinks: [
    {
      id: 'congregation-website',
      title: 'Congregation Website',
      url: 'https://example.church',
      visibilityRequirement: ['engaged', 'contributor', 'steward'],
    },
  ],
  pendingAcknowledgments: COFC_SUNDAY_MORNING_STREAM.filter((e) =>
    /retreat.*driver|prayer request/i.test(e.content)
  )
    .slice(0, 3)
    .map((e) => e.id),
};
