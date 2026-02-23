/**
 * AppShellPage — page object for the authenticated app chrome.
 *
 * Encapsulates the navigator header, profile tray, and logout flow
 * from elohim-app/src/app/elohim/components/elohim-navigator/.
 */

import { BasePage } from './base.page.js';
import { SHELL } from './selectors.js';

export class AppShellPage extends BasePage {
  /** Wait for app-root to be visible. */
  async waitForReady(): Promise<void> {
    await this.locate(SHELL.APP_ROOT).waitFor({ state: 'visible', timeout: 10_000 });
  }

  /** Open the profile tray and click "Log Out". */
  async logout(): Promise<void> {
    const bubble = this.testId(SHELL.PROFILE_BUBBLE);
    await bubble.waitFor({ state: 'visible', timeout: 15_000 });
    await bubble.click();
    await this.page.waitForTimeout(500);

    const logoutBtn = this.testId(SHELL.LOGOUT);
    await logoutBtn.click();
    await this.page.waitForLoadState('networkidle');
  }

  /** Check whether the current URL is on an authenticated route. */
  isAuthenticated(): boolean {
    const url = this.page.url();
    return !url.includes('/identity/login') && !url.includes('/threshold');
  }
}
