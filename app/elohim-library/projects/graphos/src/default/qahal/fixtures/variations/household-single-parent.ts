import type { Scene } from '../../../../designed/qahal/_lib/types';
import { dowellHouseholdTuesdayMorning } from '../canonical/dowell-household-tuesday-morning';

export const householdSingleParent: Scene = {
  ...dowellHouseholdTuesdayMorning,
  id: 'household-single-parent',
  qahalLabel: 'Household (single parent)',
  members: dowellHouseholdTuesdayMorning.members.filter((m) => !m.name.includes('Jessica')),
  coStewardObservation:
    'One steward this season. The household is participating; the substrate has noted the load and is gently surfacing aid offers from the life-group.',
};
