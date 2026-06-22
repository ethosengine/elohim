/**
 * ContributorsPage — page object for the "Contributors / Inspired by" section
 * of the content viewer (lamad: content-viewer.component.html).
 *
 * The section is conditionally rendered (`*ngIf="contributorPresences.length > 0"`)
 * so helper methods clearly distinguish "present" from "absent". Per-card testids
 * follow the pattern `viewer-contributor-card-{presence.id}` stamped via Angular's
 * `[attr.data-testid]` binding on each `<elohim-contributor-card>` host element.
 *
 * NOTE: `<elohim-contributor-card>` is a Lit web component — the card's rendered
 * content (display name, image) lives in its shadow DOM. Assertions on card content
 * use Playwright's accessible-name engine (`getByRole`) which pierces open shadow
 * roots, rather than `textContent()` on the host (which returns empty for shadow DOM).
 */

import { BasePage } from './base.page.js';
import { CONTRIBUTORS } from './selectors.js';

export class ContributorsPage extends BasePage {
  /** Wait until the contributors section is present and visible. */
  async waitForReady(): Promise<void> {
    await this.testId(CONTRIBUTORS.SECTION).waitFor({ state: 'visible', timeout: 15_000 });
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Section presence
  // ─────────────────────────────────────────────────────────────────────────

  /** Whether the contributors section is visible. */
  async isSectionVisible(timeoutMs = 10_000): Promise<boolean> {
    return this.testId(CONTRIBUTORS.SECTION)
      .waitFor({ state: 'visible', timeout: timeoutMs })
      .then(() => true)
      .catch(() => false);
  }

  /** Whether the contributors section is absent (content has no contributors). */
  async isSectionAbsent(timeoutMs = 5_000): Promise<boolean> {
    return this.testId(CONTRIBUTORS.SECTION)
      .waitFor({ state: 'hidden', timeout: timeoutMs })
      .then(() => true)
      .catch(() => false);
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Card counting and identity
  // ─────────────────────────────────────────────────────────────────────────

  /** Count of contributor cards rendered inside the contributors list. */
  async getCardCount(): Promise<number> {
    return this.testId(CONTRIBUTORS.LIST)
      .locator(`[data-testid^="${CONTRIBUTORS.CARD_PREFIX}"]`)
      .count();
  }

  /** Whether a specific card (by presence id) is present in the list. */
  async hasCardForPresence(presenceId: string): Promise<boolean> {
    return this.testId(`${CONTRIBUTORS.CARD_PREFIX}${presenceId}`).isVisible();
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Display name assertions (shadow-DOM piercing via accessible-name)
  //
  // <elohim-contributor-card> renders a shadow <article> with:
  //   aria-label="Contributor: {displayName}"
  // Playwright's getByRole() pierces open shadow roots — we assert on the
  // accessible name rather than textContent() (which returns empty on hosts).
  // ─────────────────────────────────────────────────────────────────────────

  /**
   * Whether a contributor card with the given display name is visible.
   * Matches the `aria-label="Contributor: {name}"` on the shadow <article>.
   */
  async hasContributorNamed(displayName: string): Promise<boolean> {
    const label = `Contributor: ${displayName}`;
    return this.page
      .getByRole('article', { name: label })
      .waitFor({ state: 'visible', timeout: 10_000 })
      .then(() => true)
      .catch(() => false);
  }

  /**
   * Return the accessible names of all contributor cards currently rendered,
   * stripped of the "Contributor: " prefix.
   */
  async getContributorNames(): Promise<string[]> {
    const cards = this.testId(CONTRIBUTORS.LIST).locator(
      `[data-testid^="${CONTRIBUTORS.CARD_PREFIX}"]`
    );
    const count = await cards.count();
    const names: string[] = [];
    for (let i = 0; i < count; i++) {
      const label = await cards.nth(i).getAttribute('aria-label');
      if (label) {
        // Cards carry aria-label via the shadow article; if the host has it too, strip prefix.
        names.push(label.replace(/^Contributor:\s*/u, '').trim());
      }
    }
    return names;
  }
}
