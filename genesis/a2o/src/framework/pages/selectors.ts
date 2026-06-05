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

// ─────────────────────────────────────────────────────────────────────────────
// Doorway App
// ─────────────────────────────────────────────────────────────────────────────

/* eslint-disable sonarjs/no-hardcoded-passwords -- data-testid selectors, not credentials */

// Doorway threshold login (doorway-app: threshold-login.component.ts)
export const THRESHOLD = {
  IDENTIFIER: 'threshold-identifier',
  PASSWORD: 'threshold-password',
  SUBMIT: 'threshold-submit',
  ERROR: 'threshold-error',
  ERROR_DISMISS: 'threshold-error-dismiss',
  FEDERATED_LOGIN: 'threshold-federated-login',
  REGISTER_LINK: 'threshold-register-link',
  DOMAIN_SUFFIX: 'threshold-domain-suffix',
  DIFFERENT_DOORWAY_HINT: 'threshold-different-doorway-hint',
  RETRY: 'threshold-retry',
} as const;

// Doorway threshold register (doorway-app: threshold-register.component.ts)
export const THRESHOLD_REGISTER = {
  ERROR_DISMISS: 'threshold-register-error-dismiss',
  DISPLAY_NAME: 'threshold-register-display-name',
  EMAIL: 'threshold-register-email',
  PASSWORD: 'threshold-register-password',
  CONFIRM_PASSWORD: 'threshold-register-confirm-password',
  SUBMIT: 'threshold-register-submit',
  FEDERATED: 'threshold-register-federated',
  RETRY: 'threshold-register-retry',
  LOGIN_LINK: 'threshold-register-login-link',
} as const;

// Doorway dashboard (doorway-app: doorway-dashboard.component.html)
export const DOORWAY_DASHBOARD = {
  REFRESH: 'dashboard-refresh',
  ERROR_RETRY: 'dashboard-error-retry',
  CAPABILITIES_BADGE: 'dashboard-capabilities-badge',
  CAPABILITY_CHIP_PREFIX: 'dashboard-capability-', // suffix with capability key
  CAPABILITY_PLACEHOLDER_PREFIX: 'capability-placeholder-', // suffix with capability key
  OVERVIEW_NO_ORCHESTRATOR: 'overview-no-orchestrator',
  NODES_NO_ORCHESTRATOR: 'nodes-no-orchestrator',
  RESOURCES_NO_ORCHESTRATOR: 'resources-no-orchestrator',
  TAB_OVERVIEW: 'dashboard-tab-overview',
  TAB_NODES: 'dashboard-tab-nodes',
  TAB_USERS: 'dashboard-tab-users',
  TAB_RESOURCES: 'dashboard-tab-resources',
  TAB_FEDERATION: 'dashboard-tab-federation',
  TAB_PIPELINE: 'dashboard-tab-pipeline',
  TAB_GRADUATION: 'dashboard-tab-graduation',
  TAB_TOPOLOGY: 'dashboard-tab-topology',
  TOPOLOGY_PANEL: 'operator-topology',
  TOPOLOGY_HOSTNAME: 'topology-hostname',
  TOPOLOGY_STEWARDS_COUNT: 'topology-stewards-count',
  TOPOLOGY_FEDERATION_COUNT: 'topology-federation-count',
  TOPOLOGY_DETAILS_TOGGLE: 'topology-details-toggle',
  TOPOLOGY_RETRY: 'topology-retry',
  NODES_STATUS_FILTER: 'nodes-status-filter',
  NODES_SORT_ID: 'nodes-sort-id',
  NODES_SORT_STATUS: 'nodes-sort-status',
  NODES_SORT_TIER: 'nodes-sort-tier',
  NODES_SORT_TRUST: 'nodes-sort-trust',
  NODES_SORT_SCORE: 'nodes-sort-score',
  USERS_SEARCH: 'users-search',
  USERS_PERMISSION_FILTER: 'users-permission-filter',
  USERS_ACTIVE_FILTER: 'users-active-filter',
  USERS_QUOTA_FILTER: 'users-quota-filter',
  USERS_SORT_IDENTIFIER: 'users-sort-identifier',
  USERS_SORT_PERMISSION: 'users-sort-permission',
  USERS_SORT_STATUS: 'users-sort-status',
  USERS_SORT_STORAGE: 'users-sort-storage',
  USERS_SORT_CREATED: 'users-sort-created',
  USERS_PAGE_PREV: 'users-page-prev',
  USERS_PAGE_NEXT: 'users-page-next',
  USER_DETAIL_OVERLAY: 'user-detail-overlay',
  USER_DETAIL_CLOSE: 'user-detail-close',
  USER_DETAIL_PERMISSION: 'user-detail-permission',
  USER_DETAIL_TOGGLE_STATUS: 'user-detail-toggle-status',
  USER_DETAIL_LOGOUT: 'user-detail-logout',
  USER_DETAIL_RESET_USAGE: 'user-detail-reset-usage',
  USER_DETAIL_DELETE: 'user-detail-delete',
} as const;

// Doorway browser (doorway-app: doorway-browser.component.html)
export const BROWSER = {
  RETRY: 'browser-retry',
  SEARCH: 'browser-search',
  REGION_FILTER: 'browser-region-filter',
  BACK_TO_LOGIN: 'browser-back-to-login',
} as const;

// Doorway federation tab (doorway-app: federation-tab.component.ts)
export const FEDERATION = {
  REFRESH: 'federation-refresh',
  PEER_URL: 'federation-peer-url',
  ADD_PEER: 'federation-add-peer',
} as const;

// Doorway pipeline tab (doorway-app: pipeline-tab.component.ts)
export const PIPELINE = {
  RETRY: 'pipeline-retry',
} as const;

// Doorway account (doorway-app: doorway-account.component.ts)
export const ACCOUNT = {
  BACK: 'account-back',
  RETRY: 'account-retry',
} as const;

// Doorway toolbar (doorway-app: toolbar.component.ts)
export const TOOLBAR = {
  PROFILE_BUBBLE: 'toolbar-profile-bubble',
  LOGOUT: 'toolbar-logout',
  BACKDROP: 'toolbar-backdrop',
} as const;

// Doorway landing (doorway-app: doorway-landing.component.ts)
export const LANDING = {
  SIGN_IN: 'landing-sign-in',
  CREATE_ACCOUNT: 'landing-create-account',
  DASHBOARD: 'landing-dashboard',
} as const;

// ─────────────────────────────────────────────────────────────────────────────
// Elohim App — Shell & Identity
// ─────────────────────────────────────────────────────────────────────────────

// App shell (elohim-app: elohim-navigator.component.html)
export const SHELL = {
  PROFILE_BUBBLE: 'profile-bubble',
  PROFILE_TRAY: 'profile-tray',
  LOGOUT: 'logout-button',
  APP_ROOT: 'app-root', // tag selector — used via locate(), not testId()
  APP_LOGO: 'nav-app-logo',
  CONTEXT_TOGGLE: 'nav-context-toggle',
  PROTOCOL_LINK: 'nav-protocol-link',
  SEARCH_INPUT: 'nav-search-input',
  SEARCH_SUBMIT: 'nav-search-submit',
  IDENTITY_LINK: 'nav-identity-link',
  LOGIN_BUTTON: 'nav-login-button',
  SIGNUP_BUTTON: 'nav-signup-button',
} as const;

// App login (elohim-app: login.component.html — Lit wrapper, E1)
//
// The Angular template is now a thin Lit wrapper. All interactive elements are
// owned by the Lit primitives inside the custom elements. E2E steps locate the
// custom elements by tag name, then interact with their internal shadow-DOM or
// delegated slots. Use tag selectors (locate()) rather than testId() here.
//
// Step mapping:
//   resolve step → <elohim-imagodei-portal-shell> contains <elohim-imagodei-federated-resolver>
//   login step   → <elohim-imagodei-portal-shell> contains <elohim-imagodei-login-card>
export const LOGIN = {
  /** Host shell — always present; [attr.step] reflects current step */
  PORTAL_SHELL: 'elohim-imagodei-portal-shell',
  /** Federated resolver element (step=resolve) */
  FEDERATED_RESOLVER: 'elohim-imagodei-federated-resolver',
  /** Login card element (step=login) */
  LOGIN_CARD: 'elohim-imagodei-login-card',
  /** Error region slot — contains the Angular-rendered error + recovery link */
  ERROR_REGION: '[slot="error-region"]',
} as const;

// Register (elohim-app: register.component.html)
export const REGISTER = {
  ERROR_DISMISS: 'register-error-dismiss',
  DISPLAY_NAME: 'register-display-name',
  BIO: 'register-bio',
  AFFINITIES: 'register-affinities',
  LOCATION: 'register-location',
  EMAIL: 'register-email',
  PASSWORD: 'register-password',
  PASSWORD_TOGGLE: 'register-password-toggle',
  CONFIRM_PASSWORD: 'register-confirm-password',
  SUBMIT: 'register-submit',
  GO_TO_LOGIN: 'register-go-to-login',
  BACK_HOME: 'register-back-home',
} as const;

// Hero (elohim-app: hero.component.html)
export const HERO = {
  PLAY_BUTTON: 'hero-play-button',
  VIDEO_BACK: 'hero-video-back',
  DEEP_DIVE: 'hero-deep-dive',
  DEEP_DIVE_BACK: 'hero-deep-dive-back',
  SUMMARY: 'hero-summary',
} as const;

// Footer (elohim-app: footer.component.html)
export const FOOTER = {
  GIT_HASH: 'git-hash',
} as const;

// Theme toggle (elohim-app: theme-toggle.component.html)
export const THEME = {
  TOGGLE: 'theme-toggle',
} as const;

// ─────────────────────────────────────────────────────────────────────────────
// Elohim App — Lamad (Learning)
// ─────────────────────────────────────────────────────────────────────────────

// Lamad home (elohim-app: lamad-home.component.html)
export const HOME = {
  TAB_PATHS: 'home-tab-paths',
  TAB_EXPLORE: 'home-tab-explore',
  RESUME_JOURNEY: 'home-resume-journey',
  ACTIVE_VIEW_PATH: 'home-active-view-path',
  FEATURED_BEGIN: 'home-featured-begin',
  FEATURED_DETAILS: 'home-featured-details',
  TOGGLE_ALL_PATHS: 'home-toggle-all-paths',
  PATH_GRID: 'home-path-grid',
} as const;

// Learner dashboard (elohim-app: learner-dashboard.component.html)
export const DASHBOARD = {
  EXPLORE_PATHS: 'dashboard-explore-paths',
  EMPTY_EXPLORE: 'dashboard-empty-explore',
} as const;

// Path navigator (elohim-app: path-navigator.component.html)
export const PATH_NAV = {
  ROOT: 'path-navigator', // root container — render-verifies the step navigator mounted
  SIDEBAR_TOGGLE: 'path-nav-sidebar-toggle',
  BACK_OVERVIEW: 'path-nav-back-overview',
  SIDEBAR_COLLAPSE: 'path-nav-sidebar-collapse',
  ERROR_BACK: 'path-nav-error-back',
  VIEW_RESOURCE: 'path-nav-view-resource',
  MARK_COMPLETE: 'path-nav-mark-complete',
  PREV: 'path-nav-prev',
  NEXT: 'path-nav-next',
  BREADCRUMB_HOME: 'path-nav-breadcrumb-home',
  BREADCRUMB_PATH: 'path-nav-breadcrumb-path',
} as const;

// Path overview (elohim-app: path-overview.component.html)
export const PATH_OVERVIEW = {
  ROOT: 'path-overview', // root container — render-verifies the path overview mounted
  BACK_ALL_PATHS: 'overview-back-all-paths',
  BACK_HOME: 'overview-back-home',
  BEGIN_JOURNEY: 'overview-begin-journey',
  CONTINUE_JOURNEY: 'overview-continue-journey',
  ERROR_BACK: 'overview-error-back',
  VIEW_AS_CONTENT: 'overview-view-as-content',
} as const;

// Content viewer (elohim-app: content-viewer.component.html)
export const VIEWER = {
  BACK_HOME: 'viewer-back-home',
  ERROR_BACK: 'viewer-error-back',
  EDIT_LINK: 'viewer-edit-link',
  EXPLORE_GRAPH: 'viewer-explore-graph',
  RETURN_TO_PATH: 'viewer-return-to-path',
  TAB_CONTENT: 'viewer-tab-content',
  TAB_GOVERNANCE: 'viewer-tab-governance',
  TAB_NETWORK: 'viewer-tab-network',
  TAB_TRUST: 'viewer-tab-trust',
  AFFINITY_INCREASE: 'viewer-affinity-increase',
  AFFINITY_DECREASE: 'viewer-affinity-decrease',
  AFFINITY_MASTERED: 'viewer-affinity-mastered',
  AFFINITY_PRACTICING: 'viewer-affinity-practicing',
  FILE_CHALLENGE: 'viewer-file-challenge',
  START_DISCUSSION: 'viewer-start-discussion',
  FEEDBACK_TRIGGER: 'viewer-feedback-trigger',
} as const;

// Gate feedback (elohim-app: gate-feedback-trigger + gate-feedback-modal)
export const FEEDBACK_GATE = {
  TRIGGER_BTN: 'feedback-trigger-btn',
  TRIGGER_MENU: 'feedback-trigger-menu',
  MENU_ITEM_FLAG: 'feedback-menu-item-flag',
  MENU_ITEM_CHALLENGE: 'feedback-menu-item-challenge',
  MENU_ITEM_FEEDBACK: 'feedback-menu-item-feedback',
  MENU_ITEM_REPORT: 'feedback-menu-item-report',
  MODAL_BACKDROP: 'feedback-modal-backdrop',
  MODAL_PANEL: 'feedback-modal-panel',
  MODAL_TITLE: 'feedback-modal-title',
  MODAL_CLOSE: 'feedback-modal-close',
  ARTIFACT_TEXTAREA: 'artifact-textarea',
  ARTIFACT_SUBMIT: 'artifact-submit',
} as const;

// Lesson view (elohim-app: lesson-view.component.ts)
export const LESSON = {
  TOGGLE_PANEL: 'lesson-toggle-panel',
  PANEL_CLOSE: 'lesson-panel-close',
  EXPLORE_GRAPH: 'lesson-explore-graph',
} as const;

// Search (elohim-app: search.component.ts)
export const SEARCH = {
  INPUT: 'search-input',
  SUBMIT: 'search-submit',
} as const;

// Focused view toggle (elohim-app: focused-view-toggle.component.ts)
export const FOCUSED_VIEW = {
  TOGGLE: 'focused-view-toggle',
} as const;

// Content download (elohim-app: content-download.component.ts)
export const CONTENT_DOWNLOAD = {
  BUTTON: 'content-download',
} as const;

// Graph explorer (elohim-app: graph-explorer.component.html)
export const GRAPH = {
  VIEW_HIERARCHY: 'graph-view-hierarchy',
  VIEW_OVERVIEW: 'graph-view-overview',
  RETURN_TO_PATH: 'graph-return-to-path',
  RESET_ZOOM: 'graph-reset-zoom',
  BACK_TO_PATHS: 'graph-back-to-paths',
  RETRY: 'graph-retry',
} as const;

// Not found (elohim-app: lamad-not-found.component.html)
export const NOT_FOUND = {
  ROOT: 'lamad-not-found', // root container — render-verifies the DESIGNED not-found page (not raw JSON)
  BROWSE_PATHS: 'not-found-browse-paths',
  SEARCH: 'not-found-search',
  EXPLORE: 'not-found-explore',
  GO_BACK: 'not-found-go-back',
} as const;

// Profile page (elohim-app: profile-page.component.html)
export const PROFILE = {
  IDENTITY_LINK: 'profile-identity-link',
  EDIT: 'profile-edit',
  JOIN_NETWORK: 'profile-join-network',
  TAB_OVERVIEW: 'profile-tab-overview',
  TAB_PATHS: 'profile-tab-paths',
  TAB_TIMELINE: 'profile-tab-timeline',
  RESUME_CONTINUE: 'profile-resume-continue',
  RESUME_BROWSE: 'profile-resume-browse',
  VIEW_TIMELINE: 'profile-view-timeline',
  DASHBOARD_LINK: 'profile-dashboard-link',
  BROWSE_PATHS: 'profile-browse-paths',
  START_EXPLORING: 'profile-start-exploring',
  FILTER_ALL: 'profile-filter-all',
  FILTER_VIEW: 'profile-filter-view',
  FILTER_AFFINITY: 'profile-filter-affinity',
  FILTER_PATH_START: 'profile-filter-path-start',
  FILTER_STEP_COMPLETE: 'profile-filter-step-complete',
} as const;

// ─────────────────────────────────────────────────────────────────────────────
// Elohim App — Love Map (Intimate Paths)
// ─────────────────────────────────────────────────────────────────────────────

// Love map negotiation & path UI (elohim-app: future love-map components)
export const LOVE_MAP = {
  NEGOTIATION_PROPOSE: 'love-map-propose',
  NEGOTIATION_ACCEPT: 'love-map-accept',
  STRATEGY_SELECT: 'love-map-strategy',
  ATTESTATION_BADGE: 'love-map-attestation-badge',
  CHAPTER_TITLE: 'love-map-chapter-title',
  CONSENT_GATE: 'love-map-consent-gate',
  PRIVACY_INDICATOR: 'love-map-privacy-indicator',
} as const;

// ─────────────────────────────────────────────────────────────────────────────
// Elohim App — Quiz Engine & Renderers
// ─────────────────────────────────────────────────────────────────────────────

// Discovery quiz (elohim-app: discovery-quiz.component.ts)
export const QUIZ = {
  CONTINUE: 'quiz-continue',
  EXPLORE: 'quiz-explore',
} as const;

// Pre-assessment (elohim-app: pre-assessment.component.ts)
export const ASSESSMENT = {
  TAKE: 'assessment-take',
  SKIP: 'assessment-skip',
  APPLY: 'assessment-apply',
  NO_SKIP: 'assessment-no-skip',
  START: 'assessment-start',
} as const;

// Mastery gate (elohim-app: mastery-gate.component.ts)
export const GATE = {
  START: 'gate-start',
  CONTINUE: 'gate-continue',
} as const;

// Inline quiz (elohim-app: inline-quiz.component.ts)
export const INLINE_QUIZ = {
  HEADER_TOGGLE: 'quiz-header-toggle',
  CHECK: 'quiz-check',
  NEXT: 'quiz-next',
} as const;

// Quiz renderer (elohim-app: quiz-renderer.component.html)
export const QUIZ_RENDERER = {
  SUBMIT: 'quiz-submit',
  RETAKE: 'quiz-retake',
} as const;

// Sophia renderer (elohim-app: sophia-renderer.component.ts)
export const SOPHIA = {
  CONTINUE: 'sophia-continue',
  DASHBOARD: 'sophia-dashboard',
} as const;

// Perseus renderer (elohim-app: perseus-renderer.component.ts)
export const PERSEUS = {
  HINT: 'perseus-hint',
  SUBMIT: 'perseus-submit',
  NEXT: 'perseus-next',
} as const;

// EPR links and popover (elohim-app: epr-link.component.ts, epr-popover.component.ts)
export const EPR = {
  LINK_INLINE: 'epr-link-inline',
  LINK_CHIP: 'epr-link-chip',
  LINK_CARD: 'epr-link-card',
  POPOVER: 'epr-popover',
  POPOVER_TITLE: 'epr-popover-title',
  POPOVER_TYPE: 'epr-popover-type',
  POPOVER_DESC: 'epr-popover-desc',
  POPOVER_QAHAL: 'epr-popover-qahal',
  POPOVER_LINK: 'epr-popover-link',
} as const;

// Markdown renderer (elohim-app: markdown-renderer.component.ts)
export const MARKDOWN = {
  TOC_TOGGLE: 'markdown-toc-toggle',
  TOC_CLOSE: 'markdown-toc-close',
  BACK_TO_TOP: 'markdown-back-to-top',
} as const;

// Code editor (elohim-app: default-code-editor.component.html)
export const EDITOR = {
  SAVE: 'editor-save',
  CANCEL: 'editor-cancel',
  TITLE: 'editor-title',
  DESCRIPTION: 'editor-description',
  TAGS: 'editor-tags',
  CONTENT: 'editor-content',
} as const;

// ─────────────────────────────────────────────────────────────────────────────
// Elohim App — Shared Components
// ─────────────────────────────────────────────────────────────────────────────

// Alert banner (elohim-app: alert-banner.component.html)
export const ALERT = {
  BANNER_TOGGLE: 'alert-banner-toggle',
  BANNER_DISMISS: 'alert-banner-dismiss',
  BANNER_EXPAND: 'alert-banner-expand',
  SHOW_MORE: 'alert-show-more',
  SHOW_MORE_CARDS: 'alert-show-more-cards',
  INLINE_DISMISS: 'alert-inline-dismiss',
} as const;

// ─────────────────────────────────────────────────────────────────────────────
// Elohim App — Discovery Assessments
// ─────────────────────────────────────────────────────────────────────────────

// Likert scale widget (sophia: likert-scale.tsx)
export const LIKERT = {
  TRACK: 'likert-scale-track',
  THUMB: 'likert-scale-thumb',
  TICK: 'likert-scale-tick',
  VALUE: 'likert-scale-value',
  MIN_LABEL: 'likert-scale-min-label',
  MAX_LABEL: 'likert-scale-max-label',
} as const;

// Discovery assessment results (elohim-app: discovery components)
export const DISCOVERY = {
  PROFILE: 'discovery-profile',
  SUBSCALE_SCORE: 'discovery-subscale-score',
  PRIMARY_TYPE: 'discovery-primary-type',
  ATTESTATION_BADGE: 'discovery-attestation-badge',
} as const;

// Assessment completion summary (elohim-app: assessment-completion-summary.component.ts)
export const COMPLETION = {
  SUMMARY: 'completion-summary',
  RESULT_CARD: 'completion-result-card',
  ICON: 'completion-icon',
  HEADLINE: 'completion-headline',
  DESCRIPTION: 'completion-description',
  HEX_BADGE: 'completion-hex-badge',
  BADGE_REVEAL: 'completion-badge-reveal',
  SUBSCALES: 'completion-subscales',
  SCORE: 'completion-score',
  MASTERY_PREVIEW: 'completion-mastery-preview',
  PROFILE_LINK: 'completion-profile-link',
  CONTINUE: 'completion-continue',
  NAV: 'completion-nav',
} as const;

// ─────────────────────────────────────────────────────────────────────────────
// Elohim App — Elohim Presence (elohim insight & transparency)
// ─────────────────────────────────────────────────────────────────────────────

// Elohim insight panel (elohim-app: elohim-insight components)
export const ELOHIM_PRESENCE = {
  INSIGHT_SECTION: 'elohim-insight-section',
  RECOMMENDATION_MESSAGE: 'elohim-recommendation-message',
  REASONING_EXPANDABLE: 'elohim-reasoning-expandable',
  REASONING_PRINCIPLE: 'elohim-reasoning-principle',
  REASONING_INTERPRETATION: 'elohim-reasoning-interpretation',
  COST_TOKENS: 'elohim-cost-tokens',
  COST_TIME: 'elohim-cost-time',
  TEST_CONNECTION_BTN: 'elohim-test-connection-btn',
  TEST_RESULT: 'elohim-test-result',
} as const;

// Banner notification (elohim-app: banner-notification.component.ts)
export const BANNER = {
  NOTIFICATION: 'banner-notification',
} as const;

// ─────────────────────────────────────────────────────────────────────────────
// Elohim App — Shefa Topology Surfaces (light-up-the-topology sprint)
// ─────────────────────────────────────────────────────────────────────────────

// /shefa/cluster — federated multi-device view (my-cluster.component.html)
export const SHEFA_CLUSTER = {
  PAGE: 'my-cluster-page',
  LOADING: 'my-cluster-loading',
  SUMMARY: 'my-cluster-summary',
  TOTALS: 'my-cluster-totals',
  STATUS: 'my-cluster-status',
  SHOW_DETAILS_TOGGLE: 'my-cluster-show-details-toggle',
  DETAIL_JSON: 'my-cluster-detail-json',
  ERROR: 'my-cluster-error',
  DEVICE_TILE: 'device-tile',
} as const;

// /shefa/peers — per-household reciprocation topology (peer-topology.component.ts)
export const SHEFA_PEERS = {
  PAGE: 'peer-topology-page',
  SUMMARY: 'peer-topology-summary',
  CLIFF: 'peer-topology-cliff',
  SHOW_DETAILS_TOGGLE: 'peer-topology-show-details-toggle',
  DETAIL_JSON: 'peer-topology-detail-json',
  HOUSEHOLD_CARD: 'peer-household-card',
} as const;

// /shefa/reciprocity — inflow/outflow ledger (reciprocity-ledger.component.ts)
export const SHEFA_RECIPROCITY = {
  PAGE: 'reciprocity-page',
  INFLOW_ROW: 'reciprocity-inflow-row',
  INFLOW_EMPTY: 'reciprocity-inflow-empty',
  OUTFLOW_ROW: 'reciprocity-outflow-row',
  OUTFLOW_EMPTY: 'reciprocity-outflow-empty',
  // Cell-level testids on each row, scoped via INFLOW_ROW / OUTFLOW_ROW parents.
  COUNTERPARTY: 'reciprocity-counterparty',
  COMMITTED: 'reciprocity-committed',
  DELIVERED: 'reciprocity-delivered',
  NET: 'reciprocity-net',
  CAPACITY: 'reciprocity-capacity',
} as const;

// ─────────────────────────────────────────────────────────────────────────────
// Elohim App — Shefa Topology drill-down + capacity affordances (Epic A:
// clickable capacity bar + expandable detail panels). Append-only block; the
// SHEFA_* blocks above are unchanged.
// ─────────────────────────────────────────────────────────────────────────────

// my-cluster.component.html — clickable stacked storage capacity bar + the
// totals panel it reveals when expanded.
export const SHEFA_CLUSTER_CAPACITY = {
  CAPACITY_BAR: 'my-cluster-capacity-bar',
} as const;

// device-tile.component.ts — per-device expandable detail panel (freshness,
// hosting/projecting counts, peerId). The tile element itself (SHEFA_CLUSTER.
// DEVICE_TILE = 'device-tile') is the role=button toggle, so EXPAND reuses it.
export const DEVICE_TILE_DETAIL = {
  EXPAND: 'device-tile',
  DETAIL_PANEL: 'device-tile-detail-panel',
  DETAIL_FRESHNESS: 'device-tile-detail-freshness',
  DETAIL_HOSTING: 'device-tile-detail-hosting',
  DETAIL_PROJECTING: 'device-tile-detail-projecting',
  DETAIL_PEER_ID: 'device-tile-detail-peer-id',
} as const;

// peer-topology.component.ts — per-household expandable detail panel
// (lastSyncSec, hosted-cid counts, critical-for-me/them flags).
export const SHEFA_PEERS_DETAIL = {
  EXPAND: 'peer-household-expand',
  DETAIL_PANEL: 'peer-household-detail-panel',
  DETAIL_LAST_SYNC: 'peer-detail-last-sync',
  DETAIL_HOSTED_BY_THEM: 'peer-detail-hosted-by-them',
  DETAIL_HOSTED_BY_ME: 'peer-detail-hosted-by-me',
  DETAIL_CRITICAL_FOR_ME: 'peer-detail-critical-for-me',
  DETAIL_CRITICAL_FOR_THEM: 'peer-detail-critical-for-them',
} as const;

// ─────────────────────────────────────────────────────────────────────────────
// Lane B — EPR relationship card + shefa network posture (append-only)
// ─────────────────────────────────────────────────────────────────────────────

// epr-relationship-card.component.ts — a typed, navigable relationship card
// beneath a concept. Reach + resilience trust signals; the peers count is a
// subtle distinct-peers badge rendered only when distinctPeers > 0.
export const RELATIONSHIP_CARD = {
  CARD: 'epr-relationship-card',
  TYPE: 'epr-rel-card-type',
  TITLE: 'epr-rel-card-title',
  REACH: 'epr-rel-card-reach',
  RESILIENCE: 'epr-rel-card-resilience',
  PEERS: 'epr-rel-card-peers',
} as const;

// epr-relationships-panel.component.ts — groups relationship cards by type
// (Prerequisites / This teaches / References) into per-type sections.
export const RELATIONSHIP_PANEL = {
  PANEL: 'epr-relationships-panel',
  GROUP: 'epr-rel-group',
} as const;

// network-health-tab.component.html — P2P mesh posture surface mounted in the
// shefa dashboard (/shefa/dashboard). The endpoint GET /api/v1/network/posture
// is live; testids mirror the rendered posture cards.
export const SHEFA_POSTURE = {
  CARD: 'network-posture-card',
  ACTIVE_PEERS: 'posture-active-peers',
  HOUSEHOLDS_RECIPROCATING: 'posture-households-reciprocating',
  ALWAYS_ON: 'posture-always-on',
  COMPUTE: 'posture-compute',
  STORAGE_PRESSURE: 'posture-storage-pressure',
  HOUSEHOLD_BREAKDOWN: 'household-breakdown',
} as const;

// <elohim-epr-link> + nested <elohim-context-menu> (elohim-core Lit primitives).
// These are blank-slate web components rendered in shadow DOM — they expose ARIA
// roles, not data-testids (adding testids would brand the primitive). The EPR-link
// hypercard steps pierce the shadow roots and assert by tag + role + label.
export const EPR_LINK = {
  // The host custom-element tag (light DOM).
  HOST: 'elohim-epr-link',
  // The interactive chip/button inside the host's shadow root (L2/L3 resolved).
  ANCHOR: 'button.anchor',
  // The L4/unreachable fallback: the host renders <elohim-mention-base> (the
  // `fallback` part) instead of the live anchor when the EPR can't be resolved.
  FALLBACK: 'elohim-mention-base',
  // The visible marker dot inside the fallback chip (role=presentation).
  FALLBACK_MARK: '[part="fallback-mark"]',
  // The nested context-menu custom element (opened on right-click / Shift+F10).
  CONTEXT_MENU: 'elohim-context-menu',
} as const;

// <elohim-context-menu> shadow-DOM role selectors (no testids — role-based by design).
export const CONTEXT_MENU = {
  MENU_ROLE: '[role="menu"]',
  ITEM_ROLE: '[role="menuitem"]',
  // The three MVP labels asserted present by the right-click scenario.
  MVP_LABELS: ['Open', 'About this EPR', 'Copy EPR link'],
} as const;

// protocol-omni.component.ts — DOM-tier protocol chrome (the trust surface).
// Renders on protocolContent routes ('' and /resource/:resourceId) in
// elohim-app's app.component. The RESILIENCE segment wraps the library's
// <elohim-resilience-snapshot density="icon"> once the snapshot loads; while
// no data is available it holds a neutral glyph making no status claim
// (trust surface never cries wolf). Inner icon/tooltip testids come from
// resilience-snapshot.component.html (resilience-icon / resilience-tooltip).
export const PROTOCOL_OMNI = {
  CHIP: 'protocol-omni-chip',
  TOOLBAR: 'protocol-omni-toolbar',
  EPR: 'protocol-omni-epr',
  ENV: 'protocol-omni-env',
  RESILIENCE: 'protocol-omni-resilience',
  BACK: 'protocol-omni-back',
  FORWARD: 'protocol-omni-forward',
  ACCOUNT: 'protocol-omni-account',
  COLLAPSE: 'protocol-omni-collapse',
} as const;
