/**
 * Canonical scene fixture — Dowell Household, Tuesday morning.
 *
 * Grounded in storyteller canonical narrative §4.1 at
 * genesis/docs/superpowers/specs/2026-05-21-qahal-section-4-canonical-narratives.md.
 *
 * Named moments encoded:
 *   - James (11) is sick (sore throat, hard week at school)
 *   - Sheila (Matthew's sister) sent chicken-and-rice soup the night before
 *   - Gertrude (Matthew's mother, half a continent away) checked in
 *   - 3 pending acknowledgments (Sheila's soup, Gertrude's check-in, neighbor's offer to bring dinner)
 *   - Co-steward observation: "the household is steady"
 *
 * This fixture composes the existing primitives modules without authoring
 * new mock data from scratch — the DOWELL_FAMILY, DOWELL_HOUSEHOLD_RUBRIC,
 * DOWELL_TUESDAY_MORNING_STREAM, and DOWELL_HOUSEHOLD_TOPOLOGY are
 * authoritative; this file binds them into a coherent Scene.
 */

import type { Scene } from '../../../../designed/qahal/_lib/types';
import { DOWELL_FAMILY } from '../primitives/mock-imagodei-profiles';
import { DOWELL_HOUSEHOLD_RUBRIC } from '../primitives/mock-rubrics';
import { DOWELL_TUESDAY_MORNING_STREAM } from '../primitives/mock-care-economy-events';
import { DOWELL_HOUSEHOLD_TOPOLOGY } from '../primitives/mock-social-compute-topology';

// Pending acknowledgments — IDs from DOWELL_TUESDAY_MORNING_STREAM.
// Resolve at fixture-load time by matching content against the narrative's
// three pending items: Sheila's soup, Gertrude's check-in, the neighbor's dinner offer.
const pendingAcknowledgmentIds = DOWELL_TUESDAY_MORNING_STREAM.filter((event) => {
  const c = event.content.toLowerCase();
  return (
    (c.includes('sheila') && c.includes('soup')) ||
    (c.includes('gertrude') && c.includes('check')) ||
    (c.includes('neighbor') && (c.includes('dinner') || c.includes('bring')))
  );
}).map((event) => event.id);

export const dowellHouseholdTuesdayMorning: Scene = {
  id: 'dowell-household',
  qahalIcon: '🏠',
  qahalLabel: 'Dowell Household',
  qahalArchetype: 'household',
  otherQahals: [
    { id: 'cofc-congregation', icon: '⛪', label: 'Local Churches of Christ' },
    { id: 'hardins-life-group', icon: '🪨', label: 'Tuesday Life-Group' },
    { id: 'wisdom-commons', icon: '🌳', label: 'Wisdom Commons' },
  ],
  rubric: DOWELL_HOUSEHOLD_RUBRIC,
  members: DOWELL_FAMILY,
  streamEvents: DOWELL_TUESDAY_MORNING_STREAM,
  computeTopology: DOWELL_HOUSEHOLD_TOPOLOGY,
  coStewardObservation: 'The household is steady.',
  curatedEprs: [
    { id: 'family-recipes', title: 'Family Recipes', provenance: 'curated-epr' },
    { id: 'birthday-calendar', title: 'Birthday Calendar', provenance: 'curated-epr' },
    { id: 'sick-day-playlist', title: 'Sick-Day Playlist', provenance: 'curated-epr' },
  ],
  externalLinks: [
    {
      id: 'family-google-doc',
      title: 'Family Google Doc',
      url: 'https://docs.google.com/example',
      visibilityRequirement: ['engaged', 'contributor', 'steward'],
    },
  ],
  pendingAcknowledgments: pendingAcknowledgmentIds,
};
