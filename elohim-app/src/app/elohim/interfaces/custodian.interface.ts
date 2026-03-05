/**
 * ICustodianSelector — Abstract interface for custodian selection.
 *
 * Decouples content delivery from the concrete CustodianSelectionService
 * which scores custodians by health, latency, bandwidth, specialization,
 * and commitment. Consumers inject the CUSTODIAN_SELECTOR token; the
 * default factory resolves to CustodianSelectionService.
 *
 * Tests provide a mock ICustodianSelector via the same token — no concrete
 * service import needed.
 *
 * @example
 * ```typescript
 * @Injectable({ providedIn: 'root' })
 * export class ContentDeliveryService {
 *   private readonly custodians = inject(CUSTODIAN_SELECTOR);
 *
 *   async getBestEndpoint(contentId: string): Promise<string | null> {
 *     const result = await this.custodians.selectBestCustodian(contentId);
 *     return result?.custodian.endpoint ?? null;
 *   }
 * }
 * ```
 */

import { InjectionToken, inject } from '@angular/core';

import {
  CustodianSelectionService,
  type CustodianScore,
} from '../services/custodian-selection.service';

/**
 * Abstract custodian selector — picks the best custodian to serve content.
 *
 * Implementations handle scoring custodians by health, latency, bandwidth,
 * specialization, and commitment level. The contract is: content ID in,
 * best custodian score out (or null if none available).
 */
export interface ICustodianSelector {
  /**
   * Select the best custodian to serve the given content.
   *
   * @param contentId - Hash/ID of the content to serve
   * @returns Best custodian with score breakdown, or null if none available
   */
  selectBestCustodian(contentId: string): Promise<CustodianScore | null>;

  /**
   * Score all known custodians, sorted by score descending.
   *
   * Useful for analytics dashboards and monitoring.
   */
  scoreAllCustodians(): Promise<CustodianScore[]>;

  /**
   * Get top N custodians by score.
   *
   * @param limit - Maximum number of results (default: 10)
   */
  getTopCustodians(limit?: number): Promise<CustodianScore[]>;
}

/**
 * Injection token for the custodian selector.
 *
 * Default factory resolves to CustodianSelectionService which scores
 * custodians by health (40%), latency (30%), bandwidth (15%),
 * specialization (10%), and commitment tier (5%).
 *
 * Override in tests:
 * ```typescript
 * { provide: CUSTODIAN_SELECTOR, useValue: mockCustodianSelector }
 * ```
 */
export const CUSTODIAN_SELECTOR = new InjectionToken<ICustodianSelector>('CustodianSelector', {
  providedIn: 'root',
  factory: () => inject(CustodianSelectionService),
});
