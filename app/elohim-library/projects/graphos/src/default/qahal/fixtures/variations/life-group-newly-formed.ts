import type { Scene } from '../../../../designed/qahal/_lib/types';
import { hardinsLifeGroupTuesdayEvening } from '../canonical/hardins-life-group-tuesday-evening';

export const lifeGroupNewlyFormed: Scene = {
  ...hardinsLifeGroupTuesdayEvening,
  id: 'life-group-newly-formed',
  qahalLabel: 'Life-Group (newly formed)',
  streamEvents: hardinsLifeGroupTuesdayEvening.streamEvents.slice(0, 2),
  coStewardObservation:
    'Three meetings in. Vulnerability has not yet been offered. The substrate honors patience.',
};
