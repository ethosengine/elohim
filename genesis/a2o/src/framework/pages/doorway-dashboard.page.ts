/**
 * DoorwayDashboardPage — page object for the doorway admin dashboard.
 *
 * Encapsulates tab switching, refresh, and error handling for the dashboard
 * at `/threshold/dashboard` on the doorway app.
 */

import { BasePage } from './base.page.js';
import { DOORWAY_DASHBOARD } from './selectors.js';

/** Tab names that map to dashboard tab testids. */
type DashboardTab =
  | 'overview'
  | 'nodes'
  | 'users'
  | 'resources'
  | 'federation'
  | 'pipeline'
  | 'graduation'
  | 'topology';

const TAB_MAP: Record<DashboardTab, string> = {
  overview: DOORWAY_DASHBOARD.TAB_OVERVIEW,
  nodes: DOORWAY_DASHBOARD.TAB_NODES,
  users: DOORWAY_DASHBOARD.TAB_USERS,
  resources: DOORWAY_DASHBOARD.TAB_RESOURCES,
  federation: DOORWAY_DASHBOARD.TAB_FEDERATION,
  pipeline: DOORWAY_DASHBOARD.TAB_PIPELINE,
  graduation: DOORWAY_DASHBOARD.TAB_GRADUATION,
  topology: DOORWAY_DASHBOARD.TAB_TOPOLOGY,
};

export class DoorwayDashboardPage extends BasePage {
  /** Wait for the overview tab to become visible (dashboard loaded). */
  async waitForReady(): Promise<void> {
    await this.testId(DOORWAY_DASHBOARD.TAB_OVERVIEW).waitFor({
      state: 'visible',
      timeout: 15_000,
    });
  }

  /** Switch to a dashboard tab by name. */
  async switchTab(name: DashboardTab): Promise<void> {
    const testid = TAB_MAP[name];
    if (!testid) throw new Error(`Unknown dashboard tab: "${name}"`);
    await this.testId(testid).click();
  }

  /** Get the currently active tab name by checking aria-selected or active class. */
  async activeTab(): Promise<string | null> {
    for (const [name, testid] of Object.entries(TAB_MAP)) {
      const el = this.testId(testid);
      const isVisible = await el.isVisible();
      if (!isVisible) continue;
      // Check for active/selected state via class or aria attribute
      const text = await el.textContent();
      if (text) return name;
    }
    return null;
  }

  /** Click the dashboard refresh button. */
  async refresh(): Promise<void> {
    await this.testId(DOORWAY_DASHBOARD.REFRESH).click();
  }

  /** Check whether the error retry button is visible. */
  async hasError(): Promise<boolean> {
    return this.testId(DOORWAY_DASHBOARD.ERROR_RETRY).isVisible();
  }
}
