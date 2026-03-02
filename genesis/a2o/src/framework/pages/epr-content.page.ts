/**
 * EprContentPage — page object for EPR (Elohim Protocol Reference) link interactions.
 *
 * Locates EPR links rendered as `[data-epr]` anchors in markdown content
 * and the `<app-epr-popover>` popover that appears on hover.
 *
 * Works across both elohim-app and doorway-app — selectors target
 * data-testid attributes that are app-agnostic.
 */

import { BasePage } from './base.page.js';
import { EPR } from './selectors.js';

export class EprContentPage extends BasePage {
  /** Wait until at least one EPR link is rendered on the page. */
  async waitForReady(): Promise<void> {
    // EPR links in markdown use data-epr attribute (not data-testid)
    await this.locate('[data-epr]').first().waitFor({ state: 'visible', timeout: 15_000 });
  }

  /** Get count of EPR links visible on the page. */
  async getEprLinkCount(): Promise<number> {
    return this.locate('[data-epr]').count();
  }

  /**
   * Click an EPR link that references a specific content ID.
   * Matches against the `data-epr` attribute value (e.g., "epr:manifesto").
   */
  async clickEprLink(contentId: string): Promise<void> {
    const selector = `[data-epr*="${contentId}"]`;
    await this.locate(selector).first().click();
  }

  /**
   * Hover over an EPR link to trigger the popover.
   * Matches against the `data-epr` attribute value.
   */
  async hoverEprLink(contentId: string): Promise<void> {
    const selector = `[data-epr*="${contentId}"]`;
    await this.locate(selector).first().hover();
  }

  /** Check if the EPR popover is currently visible. */
  async isPopoverVisible(): Promise<boolean> {
    return this.testId(EPR.POPOVER).isVisible();
  }

  /** Wait for the popover to appear. */
  async waitForPopover(): Promise<void> {
    await this.testId(EPR.POPOVER).waitFor({ state: 'visible', timeout: 5_000 });
  }

  /** Read the popover title text. */
  async getPopoverTitle(): Promise<string | null> {
    return this.testId(EPR.POPOVER_TITLE).textContent();
  }

  /** Read the popover content type badge text. */
  async getPopoverTypeBadge(): Promise<string | null> {
    return this.testId(EPR.POPOVER_TYPE).textContent();
  }

  /** Read the popover description text. */
  async getPopoverDescription(): Promise<string | null> {
    return this.testId(EPR.POPOVER_DESC).textContent();
  }

  /** Read the popover qahal reach level. */
  async getPopoverReach(): Promise<string | null> {
    const qahal = this.testId(EPR.POPOVER_QAHAL);
    const isVisible = await qahal.isVisible();
    if (!isVisible) return null;
    return qahal.locator('.epr-popover-reach').textContent();
  }

  /** Check if the "Open resource" link is present in the popover. */
  async hasPopoverLink(): Promise<boolean> {
    return this.testId(EPR.POPOVER_LINK).isVisible();
  }

  /**
   * Click an EPR link card (used on landing pages or content listings).
   * These are `<app-epr-link display="card">` components.
   */
  async clickEprCard(contentId: string): Promise<void> {
    const selector = `[data-testid="${EPR.LINK_CARD}"][data-epr*="${contentId}"]`;
    await this.locate(selector).first().click();
  }
}
