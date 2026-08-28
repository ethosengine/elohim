/**
 * ThresholdLoginPage — page object for doorway's OAuth login form.
 *
 * Encapsulates the login form at `/threshold/login` on the doorway app.
 * Elements are located via `data-testid` attributes defined in
 * doorway-app/src/app/components/login/threshold-login.component.ts.
 */

import { BasePage } from './base.page.js';
import { THRESHOLD } from './selectors.js';

export class ThresholdLoginPage extends BasePage {
  /** Wait for the identifier input to become visible. */
  async waitForReady(): Promise<void> {
    await this.testId(THRESHOLD.IDENTIFIER).waitFor({ state: 'visible', timeout: 15_000 });
  }

  /** Fill credentials and submit the login form. */
  async login(identifier: string, password: string): Promise<void> {
    await this.waitForReady();
    await this.testId(THRESHOLD.IDENTIFIER).fill(identifier);
    await this.testId(THRESHOLD.PASSWORD).fill(password);
    await this.testId(THRESHOLD.SUBMIT).click();
  }

  /**
   * Every control the human needs to sign in is actually on the page.
   *
   * `waitForReady` only proves the identifier input arrived; a portal that
   * rendered the identifier and nothing else would pass it. Used by the
   * render half of the portal e2e, whose whole point is that a painted form
   * and a working sign-in are two different claims.
   */
  async formIsRendered(): Promise<boolean> {
    for (const id of [THRESHOLD.IDENTIFIER, THRESHOLD.PASSWORD, THRESHOLD.SUBMIT]) {
      if (!(await this.testId(id).isVisible())) return false;
    }
    return true;
  }

  /** Check whether the error banner is currently visible. */
  async hasError(): Promise<boolean> {
    return this.testId(THRESHOLD.ERROR).isVisible();
  }

  /** Wait for the error banner to appear. */
  async waitForError(timeout = 10_000): Promise<void> {
    await this.testId(THRESHOLD.ERROR).waitFor({ state: 'visible', timeout });
  }
}
