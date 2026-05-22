import type { Scene } from '../../../../designed/qahal/_lib/types';
import { cofcCongregationSundayMorning } from '../canonical/cofc-congregation-sunday-morning';

export const congregationNewlyFormed: Scene = {
  ...cofcCongregationSundayMorning,
  id: 'congregation-newly-formed',
  qahalLabel: 'Congregation (newly formed)',
  streamEvents: cofcCongregationSundayMorning.streamEvents.slice(0, 3),
  coStewardObservation:
    'The congregation is in its first season. Cohesion thresholds are not yet reached; the substrate watches and waits.',
};
