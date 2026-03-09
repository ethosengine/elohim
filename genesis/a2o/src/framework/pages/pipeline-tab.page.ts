/**
 * PipelineTabPage — page object for the Pipeline tab on the doorway dashboard.
 *
 * Encapsulates the agency pipeline funnel view with retry capability
 * from doorway-app/src/app/components/dashboard/.
 */

import { BasePage } from './base.page.js';
import { PIPELINE } from './selectors.js';

export class PipelineTabPage extends BasePage {
  /** Wait for the pipeline content to load (retry button may or may not appear). */
  async waitForReady(): Promise<void> {
    // The pipeline tab loads data; wait a reasonable time for content
    await this.page.waitForLoadState('networkidle');
  }

  /** Check whether the retry button is visible (indicates a load error). */
  async isRetryVisible(): Promise<boolean> {
    return this.testId(PIPELINE.RETRY).isVisible();
  }

  /** Click the retry button to reload pipeline data. */
  async retry(): Promise<void> {
    await this.testId(PIPELINE.RETRY).click();
  }
}
