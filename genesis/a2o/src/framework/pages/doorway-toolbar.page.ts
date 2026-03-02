/**
 * DoorwayToolbarPage — page object for the doorway-app toolbar.
 *
 * Encapsulates the authenticated toolbar with profile bubble, logout,
 * and backdrop elements from doorway-app/src/app/components/toolbar/.
 */

import { BasePage } from './base.page.js';
import { TOOLBAR } from './selectors.js';

export class DoorwayToolbarPage extends BasePage {
  /** Wait for the toolbar profile bubble to be visible. */
  async waitForReady(): Promise<void> {
    await this.testId(TOOLBAR.PROFILE_BUBBLE).waitFor({ state: 'visible', timeout: 15_000 });
  }

  /** Open profile menu and click logout. */
  async logout(): Promise<void> {
    await this.testId(TOOLBAR.PROFILE_BUBBLE).click();
    await this.testId(TOOLBAR.LOGOUT).waitFor({ state: 'visible', timeout: 5_000 });
    // force:true needed because the backdrop overlay covers the logout button
    await this.testId(TOOLBAR.LOGOUT).click({ force: true });
    await this.page.waitForTimeout(2_000);
  }

  /** Check whether the profile bubble is visible (indicates authenticated state). */
  async isAuthenticated(): Promise<boolean> {
    return this.testId(TOOLBAR.PROFILE_BUBBLE).isVisible();
  }
}
