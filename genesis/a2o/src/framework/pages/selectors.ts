/**
 * Test ID selectors — the contract between Angular apps and page objects.
 *
 * Each constant maps to a `data-testid` attribute on a specific Angular component.
 * The owning app/component is documented inline. These values are consumed by
 * page objects via `BasePage.testId()` and can be consumed by any automation tool.
 *
 * Convention: values are the `data-testid` attribute value (no brackets, no prefix).
 * Page objects wrap them with `[data-testid="..."]` via `BasePage.testId()`.
 */

// Doorway threshold login (doorway-app: threshold-login.component.ts)
export const THRESHOLD = {
  IDENTIFIER: 'threshold-identifier',
  PASSWORD: 'threshold-password',
  SUBMIT: 'threshold-submit',
  ERROR: 'threshold-error',
} as const;

// App shell (elohim-app: elohim-navigator.component.html)
export const SHELL = {
  PROFILE_BUBBLE: 'profile-bubble',
  PROFILE_TRAY: 'profile-tray',
  LOGOUT: 'logout-button',
  APP_ROOT: 'app-root', // tag selector — used via locate(), not testId()
} as const;

// App login (elohim-app: login.component.html)
export const LOGIN = {
  FEDERATED_ID: 'login-federated-id',
  IDENTIFIER: 'login-identifier',
  PASSWORD: 'login-password',
  SUBMIT: 'login-submit',
  ERROR: 'login-error',
} as const;

// Footer (elohim-app: footer.component.html)
export const FOOTER = {
  GIT_HASH: 'git-hash',
} as const;
