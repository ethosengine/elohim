import type { ReachTier } from '@app/elohim/services/gate-interaction.service';

export type JournalRoutingState = 'writing' | 'confirming' | 'routing' | 'routed';

export type DestinationType = 'content' | 'exchange-request' | 'governance-proposal';

export type SuggestionKind = 'filing' | 'derivative';

export type SuggestionStatus = 'suggested' | 'posting' | 'posted' | 'dismissed';

export interface IntentAnalysis {
  summary: string;
  detectedTypes: DestinationType[];
  suggestedPath: string;
}

export interface RoutingSuggestion {
  id: string;
  kind: SuggestionKind;
  destinationType: 'journal-filing' | DestinationType;
  title: string;
  summary: string;
  suggestedPath: string;
  reach: ReachTier;
  contextMetadata: Record<string, unknown>;
  status: SuggestionStatus;
}
