/**
 * FederationTabPage — page object for the Federation tab on the doorway dashboard.
 *
 * Encapsulates peer management: adding peers, refreshing the peer list,
 * and reading peer counts from doorway-app/src/app/components/dashboard/.
 */

import { BasePage } from './base.page.js';
import { FEDERATION } from './selectors.js';

export class FederationTabPage extends BasePage {
  /** Wait for the federation refresh button to become visible. */
  async waitForReady(): Promise<void> {
    await this.testId(FEDERATION.REFRESH).waitFor({ state: 'visible', timeout: 15_000 });
  }

  /** Add a federation peer by URL. */
  async addPeer(url: string): Promise<void> {
    await this.testId(FEDERATION.PEER_URL).fill(url);
    await this.testId(FEDERATION.ADD_PEER).click();
  }

  /** Click the refresh button to reload peer statuses. */
  async refresh(): Promise<void> {
    await this.testId(FEDERATION.REFRESH).click();
  }

  /** Get the count of visible peer rows in the table. */
  async getPeerCount(): Promise<number> {
    // Count table rows in the federation peer list (excluding header)
    return this.page.locator('[data-testid^="federation-peer-"]').count();
  }
}
