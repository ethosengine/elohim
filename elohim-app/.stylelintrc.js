module.exports = {
  extends: ['stylelint-config-standard-scss'],
  rules: {
    // Allow Angular :host, ::ng-deep selectors
    'selector-pseudo-element-no-unknown': [
      true,
      { ignorePseudoElements: ['ng-deep'] }
    ],
    'selector-pseudo-class-no-unknown': [
      true,
      { ignorePseudoClasses: ['host', 'host-context'] }
    ],

    // Angular-specific adjustments only
    'no-descending-specificity': null,        // Complex component styles often need this
    'selector-class-pattern': null,           // Allow existing BEM and custom patterns
    'selector-type-no-unknown': [true, { ignoreTypes: ['app-root'] }],
    'scss/at-rule-no-unknown': [
      true,
      { ignoreAtRules: ['tailwind', 'apply', 'screen'] }
    ],

    // All other rules from stylelint-config-standard-scss remain strict
  },
  ignoreFiles: [
    'node_modules/**',
    'dist/**',
    'coverage/**',
    '.angular/**',
    'src/assets/**/*.css'  // Vendor styles
  ]
};
