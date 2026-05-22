/**
 * Canonical scene fixture — Hardins Life-Group, Tuesday evening.
 *
 * Grounded in storyteller canonical narrative §4.3 at
 * genesis/docs/superpowers/specs/2026-05-21-qahal-section-4-canonical-narratives.md.
 *
 * Named moments encoded:
 *   - 6 households present; group has been together 3 years
 *   - Romans 12 v1 discussion
 *   - Sarah's father in hospital
 *   - John Hardin's hosting accumulation gently caught by friction-gradient
 *   - The Lees offer to host next
 *
 * This fixture composes the existing primitives modules without authoring
 * new mock data from scratch — the LIFE_GROUP_FAMILIES, LIFE_GROUP_RUBRIC,
 * LIFE_GROUP_TUESDAY_EVENING_STREAM, and LIFE_GROUP_TOPOLOGY are
 * authoritative; this file binds them into a coherent Scene.
 */

import type { Scene } from '../../../../designed/qahal/_lib/types';
import { LIFE_GROUP_FAMILIES } from '../primitives/mock-imagodei-profiles';
import { LIFE_GROUP_RUBRIC } from '../primitives/mock-rubrics';
import { LIFE_GROUP_TUESDAY_EVENING_STREAM } from '../primitives/mock-care-economy-events';
import { LIFE_GROUP_TOPOLOGY } from '../primitives/mock-social-compute-topology';

export const hardinsLifeGroupTuesdayEvening: Scene = {
  id: 'hardins-life-group',
  qahalIcon: '🪨',
  qahalLabel: 'Tuesday Life-Group',
  qahalArchetype: 'life-group',
  otherQahals: [
    { id: 'dowell-household', icon: '🏠', label: 'Dowell Household' },
    { id: 'cofc-congregation', icon: '⛪', label: 'Local Churches of Christ' },
    { id: 'wisdom-commons', icon: '🌳', label: 'Wisdom Commons' },
  ],
  rubric: LIFE_GROUP_RUBRIC,
  members: LIFE_GROUP_FAMILIES,
  streamEvents: LIFE_GROUP_TUESDAY_EVENING_STREAM,
  computeTopology: LIFE_GROUP_TOPOLOGY,
  coStewardObservation:
    'Three years of fellowship. John has hosted twenty-three of the last twenty-four Tuesdays — gently surfacing for the group to discuss.',
  curatedEprs: [
    { id: 'study-schedule', title: 'Study Schedule', provenance: 'curated-epr' },
    { id: 'hosting-rota', title: 'Hosting Rota', provenance: 'curated-epr' },
  ],
  externalLinks: [],
  pendingAcknowledgments: LIFE_GROUP_TUESDAY_EVENING_STREAM.filter((e) =>
    /sarah.*father|hospital|host.*offer/i.test(e.content)
  )
    .slice(0, 2)
    .map((e) => e.id),
};
