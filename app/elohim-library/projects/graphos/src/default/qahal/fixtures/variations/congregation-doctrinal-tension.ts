import type { Scene } from '../../../../designed/qahal/_lib/types';
import { cofcCongregationSundayMorning } from '../canonical/cofc-congregation-sunday-morning';

export const congregationDoctrinalTension: Scene = {
  ...cofcCongregationSundayMorning,
  id: 'congregation-doctrinal-tension',
  qahalLabel: 'Congregation (doctrinal tension)',
  coStewardObservation:
    'A teaching from a sister congregation has surfaced concern among the elders. The peer-council mediation EPR is available.',
  curatedEprs: [
    ...cofcCongregationSundayMorning.curatedEprs,
    {
      id: 'dispute-mediation',
      title: 'Dispute Mediation EPR',
      provenance: 'curated-epr',
    },
  ],
};
