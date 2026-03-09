/**
 * BasePage — abstract base class for all page objects.
 *
 * Provides locator helpers that page objects inherit. The `testId()` method
 * is the primary way to find elements — it targets `[data-testid="..."]`
 * attributes that Angular components expose as part of the test contract.
 */

import type { PWPage, PWLocator } from '../devices/playwright-device.js';

export abstract class BasePage {
  constructor(protected readonly page: PWPage) {}

  /** Locate an element by raw CSS selector. */
  protected locate(selector: string): PWLocator {
    return this.page.locator(selector);
  }

  /** Locate an element by its `data-testid` attribute value. */
  protected testId(id: string): PWLocator {
    return this.page.locator(`[data-testid="${id}"]`);
  }

  /** Wait until the page is ready for interaction. */
  abstract waitForReady(): Promise<void>;
}
