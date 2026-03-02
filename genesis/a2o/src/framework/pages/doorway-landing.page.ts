/**
 * DoorwayLandingPage — page object for the doorway landing/welcome page.
 *
 * Encapsulates the landing page at `/threshold/` on the doorway app.
 * Elements are located via `data-testid` attributes defined in
 * doorway-app/src/app/components/landing/doorway-landing.component.ts.
 */

import { BasePage } from './base.page.js';
import { LANDING } from './selectors.js';

export class DoorwayLandingPage extends BasePage {
  /** Wait for the sign-in button to become visible. */
  async waitForReady(): Promise<void> {
    await this.testId(LANDING.SIGN_IN).waitFor({ state: 'visible', timeout: 15_000 });
  }

  /** Click the "Sign In" button to navigate to the login form. */
  async clickSignIn(): Promise<void> {
    await this.testId(LANDING.SIGN_IN).click();
  }

  /** Click the "Create Account" button to navigate to the registration form. */
  async clickCreateAccount(): Promise<void> {
    await this.testId(LANDING.CREATE_ACCOUNT).click();
  }

  /** Click the "Dashboard" button (visible when authenticated). */
  async clickDashboard(): Promise<void> {
    await this.testId(LANDING.DASHBOARD).click();
  }
}
