/**
 * SecuritySigninPane — page object for the Security & sign-in pane content.
 *
 * Focused on the detailed sub-elements within the security pane:
 * section visibility, key display fields, and informative error banners.
 *
 * Extends AccountShellPage concerns — use AccountShellPage for navigation
 * and this page for assertion-level detail within the security pane.
 *
 * data-testid contract comes from Tasks 18–19 (M5 UI layer).
 */

import { BasePage } from '../base.page.js';
import { ACCOUNT_SHELL } from './account-shell.page.js';

export class SecuritySigninPane extends BasePage {
  /** Wait until the Security & sign-in pane title is visible. */
  async waitForReady(): Promise<void> {
    await this.testId(ACCOUNT_SHELL.PANE_TITLE_SECURITY).waitFor({
      state: 'visible',
      timeout: 15_000,
    });
  }

  // -- Section visibility ----------------------------------------------------

  async isMyKeysSectionVisible(): Promise<boolean> {
    return this.testId(ACCOUNT_SHELL.MY_KEYS_SECTION).isVisible();
  }

  async isSelfRevokeSectionVisible(): Promise<boolean> {
    return this.testId(ACCOUNT_SHELL.SELF_REVOKE_SECTION).isVisible();
  }

  async isVoteAsEcSectionVisible(): Promise<boolean> {
    return this.testId(ACCOUNT_SHELL.VOTE_AS_EC_SECTION).isVisible();
  }

  async isLostKeySectionVisible(): Promise<boolean> {
    return this.testId(ACCOUNT_SHELL.LOST_KEY_SECTION).isVisible();
  }

  // -- Active key detail -----------------------------------------------------

  /**
   * Returns the text content of the active key card.
   * The card is expected to include the newAgentPubkey from the KeyRotation
   * DTO and the rotatedAt date.
   */
  async activeKeyText(): Promise<string | null> {
    return this.testId(ACCOUNT_SHELL.ACTIVE_KEY).textContent();
  }

  /**
   * Verify the active key card mentions the given pubkey substring.
   * M5 contract: displayed field is newAgentPubkey, never newPubKey.
   */
  async activeKeyMentions(pubkeySubstring: string): Promise<boolean> {
    const text = await this.activeKeyText();
    return text?.includes(pubkeySubstring) ?? false;
  }

  /**
   * Verify the active key card does NOT mention a forbidden field name.
   * Used to enforce that no legacy field like "newPubKey" surfaces in the UI.
   */
  async activeKeyDoesNotMention(forbiddenText: string): Promise<boolean> {
    const text = await this.activeKeyText();
    return !(text?.includes(forbiddenText) ?? false);
  }

  // -- Revocation history detail ---------------------------------------------

  /**
   * Returns the full text of a revoked key card identified by its dhtAnchorHash.
   */
  async revokedKeyText(dhtAnchorHash: string): Promise<string | null> {
    return this.testId(`revoked-${dhtAnchorHash}`).textContent();
  }

  /**
   * Check whether the revoked key card mentions the given triggerType label.
   */
  async revokedKeyMentionsTriggerType(dhtAnchorHash: string, triggerType: string): Promise<boolean> {
    const text = await this.revokedKeyText(dhtAnchorHash);
    return text?.includes(triggerType) ?? false;
  }

  // -- Pending recovery cards ------------------------------------------------

  /** Get text of a pending recovery card. */
  async pendingRecoveryText(dhtAnchorHash: string): Promise<string | null> {
    return this.testId(`pending-${dhtAnchorHash}`).textContent();
  }

  // -- Portal host redirect (doorway-app) ------------------------------------

  /** Check whether the "Manage from your steward" redirect button is visible. */
  async isPortalHostRedirectVisible(): Promise<boolean> {
    return this.testId('portal-host-redirect').isVisible();
  }

  async portalHostRedirectCount(): Promise<number> {
    return this.testId('portal-host-redirect').count();
  }
}
