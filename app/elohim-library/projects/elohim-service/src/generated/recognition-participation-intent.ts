/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: intents/recognition-participation-intent.schema.json -- DO NOT EDIT */

/**
 * Source of truth: request body for mishpat-coordinator-mediated governance participation EconomicEvent creation. The coordinator reads the standing-policy Manifest to look up recognition_weight_by_level[mechanismLevel], composes the EconomicEvent shape (action: raise, provider: humanId, receiver: entityId), and calls the existing create_economic_event coordinator. No new DHT entry type — uses existing EconomicEvent (Category A, elohim DNA content_store_integrity). This intent wire shape replaces client-side GovernanceRecognitionService.recordParticipation() + WEIGHT_BY_LEVEL constant (F-REA-3 + F-POLICY-3). Route: POST /api/v1/mishpat/recognition/participation.
 */
export interface RecognitionParticipationIntent {
  /**
   * What is being governed (e.g. 'content', 'proposal')
   */
  entityType: string;
  /**
   * ID of the governed entity
   */
  entityId: string;
  /**
   * Agent public key or human ID of the participant who performed the governance act
   */
  humanId: string;
  /**
   * Governance mechanism level (0–7, maps to recognition_weight_by_level in the standing-policy Manifest). Level 0: context-menu only. Level 1: reaction. Level 2: graduated feedback. Level 3: approval. Level 4: ranked-choice. Level 5: score. Level 6: consent. Level 7: deliberation.
   */
  mechanismLevel: number;
  /**
   * Semantic type of the participation act — used to compose the lamadEventType on the resulting EconomicEvent
   */
  participationType: 'reaction' | 'graduated-feedback' | 'vote' | 'block';
}
