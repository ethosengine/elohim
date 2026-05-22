import type { Scene } from '../../../../designed/qahal/_lib/types';
import { dowellHouseholdTuesdayMorning } from '../canonical/dowell-household-tuesday-morning';

export const householdMultiGeneration: Scene = {
  ...dowellHouseholdTuesdayMorning,
  id: 'household-multi-generation',
  qahalLabel: 'Household (multi-generation)',
  coStewardObservation:
    'Three generations under one roof this week. Gertrude is here; James is here; care flows both directions.',
};
