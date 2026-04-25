/**
 * AccountShellPage — page object for the account-management shell.
 *
 * Encapsulates navigation between account panes (Security & sign-in,
 * Personal info, etc.) in elohim-app/src/app/imagodei/components/account/.
 *
 * data-testid contract comes from Tasks 18–19 (M5 UI layer).
 */

import { BasePage } from '../base.page.js';

// ---------------------------------------------------------------------------
// Selector constants — mirrors data-testid attributes from account shell
// ---------------------------------------------------------------------------

export const ACCOUNT_SHELL = {
  // Nav links
  NAV_SECURITY: 'nav-security',
  NAV_PERSONAL_INFO: 'nav-personal-info',

  // Pane titles
  PANE_TITLE_SECURITY: 'pane-title-security',
  PANE_TITLE_PERSONAL_INFO: 'pane-title-personal-info',

  // Security pane — sections
  MY_KEYS_SECTION: 'my-keys-section',
  SELF_REVOKE_SECTION: 'self-revoke-section',
  VOTE_AS_EC_SECTION: 'vote-as-ec-section',
  LOST_KEY_SECTION: 'lost-key-section',

  // Key list items
  ACTIVE_KEY: 'active-key',

  // Self-revoke flow
  SELF_REVOKE_START: 'self-revoke-start',
  SELF_REVOKE_CONFIRM: 'self-revoke-confirm',
  SELF_REVOKE_CANCEL: 'self-revoke-cancel',
  SELF_REVOKE_ERROR: 'self-revoke-error',

  // Vote-as-EC
  VOTE_EMPTY: 'vote-empty',
  VOTE_ERROR: 'vote-error',

  // Lost-key entry
  LOST_KEY_RECOVERY: 'lost-key-recovery',
} as const;

// ---------------------------------------------------------------------------
// Page object
// ---------------------------------------------------------------------------

export class AccountShellPage extends BasePage {
  /** Wait for the account shell to be ready (security pane nav link visible). */
  async waitForReady(): Promise<void> {
    await this.testId(ACCOUNT_SHELL.NAV_SECURITY).waitFor({
      state: 'visible',
      timeout: 15_000,
    });
  }

  // -- Navigation ------------------------------------------------------------

  async clickNavSecurity(): Promise<void> {
    await this.testId(ACCOUNT_SHELL.NAV_SECURITY).click();
    await this.page.waitForLoadState('networkidle');
  }

  // -- Pane title assertions -------------------------------------------------

  async getSecurityPaneTitle(): Promise<string | null> {
    return this.testId(ACCOUNT_SHELL.PANE_TITLE_SECURITY).textContent();
  }

  async isSecurityPaneTitleVisible(): Promise<boolean> {
    return this.testId(ACCOUNT_SHELL.PANE_TITLE_SECURITY).isVisible();
  }

  // -- Key list --------------------------------------------------------------

  async isActiveKeyVisible(): Promise<boolean> {
    return this.testId(ACCOUNT_SHELL.ACTIVE_KEY).isVisible();
  }

  async getActiveKeyText(): Promise<string | null> {
    return this.testId(ACCOUNT_SHELL.ACTIVE_KEY).textContent();
  }

  /** Check whether a revoked key card (keyed by dhtAnchorHash) is visible. */
  async isRevokedKeyVisible(dhtAnchorHash: string): Promise<boolean> {
    return this.testId(`revoked-${dhtAnchorHash}`).isVisible();
  }

  /** Get the text content of a revoked key card. */
  async getRevokedKeyText(dhtAnchorHash: string): Promise<string | null> {
    return this.testId(`revoked-${dhtAnchorHash}`).textContent();
  }

  // -- Self-revoke flow ------------------------------------------------------

  async clickSelfRevokeStart(): Promise<void> {
    await this.testId(ACCOUNT_SHELL.SELF_REVOKE_START).click();
  }

  async clickSelfRevokeConfirm(): Promise<void> {
    await this.testId(ACCOUNT_SHELL.SELF_REVOKE_CONFIRM).click();
  }

  async clickSelfRevokeCancel(): Promise<void> {
    await this.testId(ACCOUNT_SHELL.SELF_REVOKE_CANCEL).click();
  }

  async isSelfRevokeErrorVisible(): Promise<boolean> {
    return this.testId(ACCOUNT_SHELL.SELF_REVOKE_ERROR).isVisible();
  }

  async waitForSelfRevokeError(timeout = 10_000): Promise<void> {
    await this.testId(ACCOUNT_SHELL.SELF_REVOKE_ERROR).waitFor({ state: 'visible', timeout });
  }

  /** Check that the confirm button is no longer present after cancel. */
  async isSelfRevokeConfirmAbsent(): Promise<boolean> {
    const count = await this.testId(ACCOUNT_SHELL.SELF_REVOKE_CONFIRM).count();
    return count === 0;
  }

  // -- Vote-as-EC flow -------------------------------------------------------

  /** Check whether the pending recovery card (keyed by dhtAnchorHash) is visible. */
  async isPendingRecoveryVisible(dhtAnchorHash: string): Promise<boolean> {
    return this.testId(`pending-${dhtAnchorHash}`).isVisible();
  }

  async clickApproveVote(dhtAnchorHash: string): Promise<void> {
    await this.testId(`approve-${dhtAnchorHash}`).click();
  }

  async clickRejectVote(dhtAnchorHash: string): Promise<void> {
    await this.testId(`reject-${dhtAnchorHash}`).click();
  }

  async isVoteEmptyVisible(): Promise<boolean> {
    return this.testId(ACCOUNT_SHELL.VOTE_EMPTY).isVisible();
  }

  async isVoteErrorVisible(): Promise<boolean> {
    return this.testId(ACCOUNT_SHELL.VOTE_ERROR).isVisible();
  }

  async waitForVoteError(timeout = 10_000): Promise<void> {
    await this.testId(ACCOUNT_SHELL.VOTE_ERROR).waitFor({ state: 'visible', timeout });
  }

  // -- Lost-key entry --------------------------------------------------------

  async clickLostKeyRecovery(): Promise<void> {
    await this.testId(ACCOUNT_SHELL.LOST_KEY_RECOVERY).click();
    await this.page.waitForLoadState('networkidle');
  }
}
