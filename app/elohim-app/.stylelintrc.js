module.exports = {
  extends: ['stylelint-config-standard-scss'],
  plugins: ['@double-great/stylelint-a11y'],
  rules: {
    // ============================================================
    // ANGULAR-SPECIFIC ADJUSTMENTS
    // ============================================================
    // Allow Angular :host, ::ng-deep selectors
    'selector-pseudo-element-no-unknown': [
      true,
      { ignorePseudoElements: ['ng-deep'] }
    ],
    'selector-pseudo-class-no-unknown': [
      true,
      { ignorePseudoClasses: ['host', 'host-context'] }
    ],
    'selector-type-no-unknown': [true, { ignoreTypes: ['app-root'] }],
    'scss/at-rule-no-unknown': [
      true,
      { ignoreAtRules: ['tailwind', 'apply', 'screen'] }
    ],

    // ============================================================
    // SONARQUBE PARITY RULES
    // ============================================================

    // S4656 - Duplicate properties (strict - no exceptions for vendor prefixes)
    // SonarQube doesn't allow consecutive duplicates even with different syntax
    'declaration-block-no-duplicate-properties': [
      true,
      {
        ignore: []  // Strict: no exceptions, matches SonarQube
      }
    ],

    // S4670 - No duplicate custom properties
    'declaration-block-no-duplicate-custom-properties': true,

    // Additional code quality rules
    'no-descending-specificity': null,        // Complex component styles often need this
    'selector-class-pattern': null,           // Allow existing BEM and custom patterns

    // ============================================================
    // ACCESSIBILITY RULES - Matching SonarQube css:S7924
    // ============================================================
    'a11y/media-prefers-reduced-motion': 'warn',
    'a11y/no-outline-none': 'error',          // Don't remove focus outlines
    'a11y/selector-pseudo-class-focus': 'warn', // Focus styles should exist

    // Color contrast checking (css:S7924)
    // Note: stylelint-a11y's contrast checking has limitations compared to SonarQube's
    // SonarQube does full WCAG AA contrast calculation
    'a11y/font-size-is-readable': 'warn',

    // ============================================================
    // MODERN CSS SYNTAX - Keep strict for quality
    // ============================================================
    'color-function-notation': 'modern',       // Use rgb() not rgba()
    'alpha-value-notation': 'percentage',      // Use 50% not 0.5
    'color-hex-length': 'short',               // #fff not #ffffff

    // Media query modernization
    'media-feature-range-notation': 'context',

    // ============================================================
    // DISABLED RULES WITH JUSTIFICATION
    // ============================================================
    // These are disabled due to common patterns in the codebase
    'custom-property-empty-line-before': null,  // CSS variable organization varies
    'scss/no-global-function-names': null,      // SCSS functions like lighten(), darken()
  },
  ignoreFiles: [
    'node_modules/**',
    'dist/**',
    'coverage/**',
    '.angular/**',
    'src/assets/**/*.css'  // Vendor styles
  ]
};
