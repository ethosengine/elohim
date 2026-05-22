/**
 * Canonical scene fixture — Wisdom Commons, Thursday afternoon.
 *
 * Grounded in storyteller canonical narrative §4.4.
 *
 * Named moments encoded:
 *   - 83 participating congregations
 *   - Brother Cal's concern surface, submitted to the Arkansas sister congregation
 *   - Peer council convening; sits with the witness for two months
 *   - REA reconciliation event recorded when the congregation responds
 */

import type { Scene } from '../../../../designed/qahal/_lib/types';
import {
  COFC_ELDERS,
  ARKANSAS_SISTER_CONGREGATION_ELDER,
} from '../primitives/mock-imagodei-profiles';
import { WISDOM_COMMONS_RUBRIC } from '../primitives/mock-rubrics';
import { WISDOM_COMMONS_THURSDAY_STREAM } from '../primitives/mock-care-economy-events';
import { WISDOM_COMMONS_TOPOLOGY } from '../primitives/mock-social-compute-topology';

export const wisdomCommonsThursdayAfternoon: Scene = {
  id: 'wisdom-commons',
  qahalIcon: '🌳',
  qahalLabel: 'Wisdom Commons',
  qahalArchetype: 'wisdom-commons',
  otherQahals: [
    { id: 'dowell-household', icon: '🏠', label: 'Dowell Household' },
    { id: 'cofc-congregation', icon: '⛪', label: 'Local Churches of Christ' },
    { id: 'hardins-life-group', icon: '🪨', label: 'Tuesday Life-Group' },
  ],
  rubric: WISDOM_COMMONS_RUBRIC,
  members: [...COFC_ELDERS, ARKANSAS_SISTER_CONGREGATION_ELDER],
  streamEvents: WISDOM_COMMONS_THURSDAY_STREAM,
  computeTopology: WISDOM_COMMONS_TOPOLOGY,
  coStewardObservation:
    'The federation has 83 participating congregations. A concern surface from a sister congregation has been recorded; the peer council is convening.',
  curatedEprs: [
    { id: 'federation-rubric', title: 'Federation Rubric Template', provenance: 'curated-epr' },
    {
      id: 'peer-council-protocol',
      title: 'Peer Council Convening Protocol',
      provenance: 'curated-epr',
    },
  ],
  externalLinks: [],
  pendingAcknowledgments: WISDOM_COMMONS_THURSDAY_STREAM.filter((e) =>
    /arkansas|witness|council/i.test(e.content)
  )
    .slice(0, 2)
    .map((e) => e.id),
};
