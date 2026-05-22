import type { Scene } from '../../../../designed/qahal/_lib/types';
import { dowellHouseholdTuesdayMorning } from '../canonical/dowell-household-tuesday-morning';

export const householdWithToddlers: Scene = {
  ...dowellHouseholdTuesdayMorning,
  id: 'household-with-toddlers',
  qahalLabel: 'Household (toddlers)',
  members: dowellHouseholdTuesdayMorning.members.map((m) =>
    m.name.includes('James') ? { ...m, name: 'Toddler', bloomTier: 'remember' as const } : m
  ),
  coStewardObservation:
    'Care is mostly under-three care this week. The household is steady; the parents are tired.',
};
