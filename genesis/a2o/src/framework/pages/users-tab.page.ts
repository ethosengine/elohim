/**
 * UsersTabPage — page object for the Users tab on the doorway dashboard.
 *
 * Encapsulates user search, detail overlay, and user management actions
 * from doorway-app/src/app/components/dashboard/.
 */

import { BasePage } from './base.page.js';
import { DOORWAY_DASHBOARD } from './selectors.js';

export class UsersTabPage extends BasePage {
  /** Wait for the users search input to become visible. */
  async waitForReady(): Promise<void> {
    await this.testId(DOORWAY_DASHBOARD.USERS_SEARCH).waitFor({
      state: 'visible',
      timeout: 15_000,
    });
  }

  /** Type a search query into the users search input. */
  async searchUser(query: string): Promise<void> {
    await this.testId(DOORWAY_DASHBOARD.USERS_SEARCH).fill(query);
  }

  /** Open the user detail overlay by clicking on a user row identifier. */
  async openUserDetail(identifier: string): Promise<void> {
    // Click the table row containing the identifier text
    await this.page.locator(`tr`).getByText(identifier).first().click();
    await this.testId(DOORWAY_DASHBOARD.USER_DETAIL_OVERLAY).waitFor({
      state: 'visible',
      timeout: 10_000,
    });
  }

  /** Toggle user active/suspended status in the detail overlay. */
  async toggleUserStatus(): Promise<void> {
    await this.testId(DOORWAY_DASHBOARD.USER_DETAIL_TOGGLE_STATUS).click();
  }

  /** Delete user via the detail overlay. */
  async deleteUser(): Promise<void> {
    await this.testId(DOORWAY_DASHBOARD.USER_DETAIL_DELETE).click();
  }

  /** Close the user detail overlay. */
  async closeDetail(): Promise<void> {
    await this.testId(DOORWAY_DASHBOARD.USER_DETAIL_CLOSE).click();
  }

  /** Force logout a user via the detail overlay. */
  async forceLogout(): Promise<void> {
    await this.testId(DOORWAY_DASHBOARD.USER_DETAIL_LOGOUT).click();
  }
}
