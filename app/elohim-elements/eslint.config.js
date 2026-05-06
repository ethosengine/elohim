// @ts-check
const eslint = require('@eslint/js');
const tseslint = require('typescript-eslint');
const importPlugin = require('eslint-plugin-import');
const prettierPlugin = require('eslint-plugin-prettier');
const prettierConfig = require('eslint-config-prettier');
const sonarjs = require('eslint-plugin-sonarjs');
const unicorn = require('eslint-plugin-unicorn').default;
const lit = require('eslint-plugin-lit');
const wc = require('eslint-plugin-wc');
const litA11y = require('eslint-plugin-lit-a11y');

module.exports = tseslint.config(
  {
    ignores: [
      '**/dist/**',
      '**/node_modules/**',
      '**/coverage/**',
      '**/*.scss',
    ],
  },
  {
    files: ['**/*.ts'],
    extends: [
      eslint.configs.recommended,
      ...tseslint.configs.recommended,
      ...tseslint.configs.stylistic,
      sonarjs.configs.recommended,
      lit.configs['flat/recommended'],
      wc.configs['flat/recommended'],
    ],
    plugins: {
      import: importPlugin,
      prettier: prettierPlugin,
      unicorn: unicorn,
      'lit-a11y': litA11y,
    },
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: __dirname,
      },
    },
    settings: {
      'import/resolver': {
        typescript: { project: '*/tsconfig.json' },
      },
    },
    rules: {
      // TypeScript-eslint rules — SonarQube parity (matches elohim-app)
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-unsafe-assignment': 'warn',
      '@typescript-eslint/no-unsafe-member-access': 'warn',
      '@typescript-eslint/no-unsafe-call': 'warn',
      '@typescript-eslint/no-unsafe-return': 'warn',
      '@typescript-eslint/no-unsafe-argument': 'warn',
      '@typescript-eslint/no-unused-vars': ['error', {
        argsIgnorePattern: '^_',
        varsIgnorePattern: '^_',
        caughtErrorsIgnorePattern: '^_',
      }],
      '@typescript-eslint/no-empty-function': 'warn',
      '@typescript-eslint/consistent-type-definitions': ['error', 'interface'],
      '@typescript-eslint/prefer-nullish-coalescing': 'error',
      '@typescript-eslint/prefer-optional-chain': 'error',
      '@typescript-eslint/prefer-readonly': 'error',
      '@typescript-eslint/prefer-for-of': 'error',
      '@typescript-eslint/prefer-includes': 'error',
      '@typescript-eslint/prefer-string-starts-ends-with': 'error',
      '@typescript-eslint/no-misused-promises': 'error',
      '@typescript-eslint/no-floating-promises': 'error',
      '@typescript-eslint/promise-function-async': 'warn',
      '@typescript-eslint/require-await': 'error',
      '@typescript-eslint/no-unnecessary-type-assertion': 'error',
      '@typescript-eslint/await-thenable': 'error',
      '@typescript-eslint/max-params': ['error', { max: 7 }],
      '@typescript-eslint/no-deprecated': 'warn',
      '@typescript-eslint/naming-convention': [
        'error',
        { selector: 'interface', format: ['PascalCase'] },
        { selector: 'class', format: ['PascalCase'] },
        { selector: 'typeAlias', format: ['PascalCase'] },
      ],

      // Imports
      'import/order': ['error', {
        groups: ['builtin', 'external', 'internal', 'parent', 'sibling', 'index', 'type'],
        pathGroups: [
          { pattern: 'lit', group: 'external', position: 'before' },
          { pattern: 'lit/**', group: 'external', position: 'before' },
        ],
        'newlines-between': 'always',
        alphabetize: { order: 'asc', caseInsensitive: true },
      }],
      'import/no-duplicates': 'error',
      'import/no-useless-path-segments': 'error',
      'import/no-extraneous-dependencies': ['error', {
        devDependencies: ['**/*.spec.ts', '**/*.config.{ts,mjs,js}', '**/*.config.*.{ts,mjs,js}'],
        peerDependencies: true,
      }],

      // General best practices (mirrors elohim-app)
      'no-console': ['error', { allow: ['warn', 'error'] }],
      'prefer-const': 'error',
      'no-var': 'error',
      eqeqeq: ['error', 'always'],
      'no-eval': 'error',
      'no-implied-eval': 'error',
      'no-new-func': 'error',
      'no-throw-literal': 'error',

      // SonarJS
      'sonarjs/cognitive-complexity': ['error', 15],
      'sonarjs/no-duplicate-string': ['error', { threshold: 3 }],
      'sonarjs/no-identical-functions': 'error',
      'sonarjs/no-collapsible-if': 'error',
      'sonarjs/no-redundant-jump': 'error',
      'sonarjs/prefer-immediate-return': 'error',
      'sonarjs/no-inverted-boolean-check': 'error',
      'sonarjs/no-nested-conditional': 'error',
      'sonarjs/no-gratuitous-expressions': 'error',
      'sonarjs/prefer-single-boolean-return': 'error',
      'sonarjs/no-ignored-exceptions': 'error',
      'sonarjs/no-nested-functions': 'off',
      'sonarjs/no-unused-vars': 'error',

      // Unicorn — SonarQube parity
      'unicorn/no-array-push-push': 'error',
      'unicorn/no-negated-condition': 'error',
      'unicorn/no-typeof-undefined': 'error',
      'unicorn/no-zero-fractions': 'error',
      'unicorn/prefer-array-index-of': 'error',
      'unicorn/prefer-array-some': 'error',
      'unicorn/prefer-at': 'error',
      'unicorn/prefer-blob-reading-methods': 'error',
      'unicorn/prefer-code-point': 'error',
      'unicorn/prefer-dom-node-remove': 'error',
      'unicorn/prefer-export-from': 'error',
      'unicorn/prefer-global-this': 'error',
      'unicorn/prefer-negative-index': 'error',
      'unicorn/prefer-number-properties': 'error',
      'unicorn/prefer-set-has': 'error',
      'unicorn/prefer-string-raw': 'error',
      'unicorn/prefer-structured-clone': 'error',
      'unicorn/prefer-top-level-await': 'off',

      // Lit / WC specific
      // wc/no-self-class is already enabled by wc.configs['flat/recommended']; the rules below are supplemental.
      'wc/guard-super-call': 'error',
      'wc/no-closed-shadow-root': 'error',
      'wc/require-listener-teardown': 'error',
      'lit/no-classfield-shadowing': 'error',
      'lit/no-legacy-template-syntax': 'error',
      'lit/no-template-bind': 'error',
      'lit/no-useless-template-literals': 'error',

      // Lit a11y — recommended set (plugin only ships an eslintrc-style config,
      // so we apply its rules explicitly under flat config).
      'lit-a11y/accessible-emoji': 'off',
      'lit-a11y/accessible-name': 'error',
      'lit-a11y/alt-text': 'error',
      'lit-a11y/anchor-is-valid': 'error',
      'lit-a11y/aria-activedescendant-has-tabindex': 'error',
      'lit-a11y/aria-attr-valid-value': 'error',
      'lit-a11y/aria-attrs': 'error',
      'lit-a11y/aria-role': 'error',
      'lit-a11y/aria-unsupported-elements': 'error',
      'lit-a11y/autocomplete-valid': 'error',
      'lit-a11y/click-events-have-key-events': 'error',
      'lit-a11y/definition-list': 'error',
      'lit-a11y/heading-hidden': 'error',
      'lit-a11y/iframe-title': 'error',
      'lit-a11y/img-redundant-alt': 'error',
      'lit-a11y/list': 'error',
      'lit-a11y/mouse-events-have-key-events': 'error',
      'lit-a11y/no-access-key': 'error',
      'lit-a11y/no-aria-slot': 'error',
      'lit-a11y/no-autofocus': 'error',
      'lit-a11y/no-distracting-elements': 'error',
      'lit-a11y/no-invalid-change-handler': 'off',
      'lit-a11y/no-redundant-role': 'error',
      'lit-a11y/obj-alt': 'error',
      'lit-a11y/role-has-required-aria-attrs': 'error',
      'lit-a11y/role-supports-aria-attr': 'error',
      'lit-a11y/scope': 'error',
      'lit-a11y/tabindex-no-positive': 'error',
      'lit-a11y/valid-lang': 'off',

      // Prettier
      'prettier/prettier': [process.env.CI === 'true' ? 'off' : 'error'],
      ...prettierConfig.rules,
    },
  },
  {
    // Spec files: tsconfig.json excludes src/**/*.spec.ts, and WTR uses
    // esbuild for TypeScript transpilation independently of tsconfig — no
    // separate test tsconfig is needed. Type-aware lint rules are disabled
    // here because there is no project reference for spec files. Chai BDD
    // assertions parse as bare property accesses, so no-unused-expressions
    // is also disabled.
    files: ['**/*.spec.ts'],
    languageOptions: {
      parserOptions: { projectService: false, project: null },
    },
    rules: {
      '@typescript-eslint/no-non-null-assertion': 'off',
      'sonarjs/no-duplicate-string': 'off',
      '@typescript-eslint/no-explicit-any': 'off',
      // Chai's BDD assertions read as bare property accesses (`expect(x).to.exist`).
      '@typescript-eslint/no-unused-expressions': 'off',
      // Test files often interleave fixture + register imports without strict ordering.
      'import/order': 'off',
      // Type-aware rules can't run without a project — disable the ones the
      // recommended set turns on by default.
      '@typescript-eslint/no-floating-promises': 'off',
      '@typescript-eslint/no-misused-promises': 'off',
      '@typescript-eslint/await-thenable': 'off',
      '@typescript-eslint/require-await': 'off',
      '@typescript-eslint/no-unsafe-assignment': 'off',
      '@typescript-eslint/no-unsafe-member-access': 'off',
      '@typescript-eslint/no-unsafe-call': 'off',
      '@typescript-eslint/no-unsafe-argument': 'off',
      '@typescript-eslint/no-unsafe-return': 'off',
      '@typescript-eslint/no-unnecessary-type-assertion': 'off',
      '@typescript-eslint/prefer-nullish-coalescing': 'off',
      '@typescript-eslint/prefer-optional-chain': 'off',
      '@typescript-eslint/prefer-readonly': 'off',
      '@typescript-eslint/prefer-includes': 'off',
      '@typescript-eslint/prefer-string-starts-ends-with': 'off',
      '@typescript-eslint/no-deprecated': 'off',
      '@typescript-eslint/promise-function-async': 'off',
    },
  },
  {
    files: ['**/*.config.{ts,mjs,js}', '**/*.config.*.{ts,mjs,js}'],
    languageOptions: {
      parserOptions: { projectService: false, project: null },
    },
    rules: {
      '@typescript-eslint/no-floating-promises': 'off',
      '@typescript-eslint/no-misused-promises': 'off',
      '@typescript-eslint/await-thenable': 'off',
      '@typescript-eslint/require-await': 'off',
      '@typescript-eslint/no-unsafe-assignment': 'off',
      '@typescript-eslint/no-unsafe-member-access': 'off',
      '@typescript-eslint/no-unsafe-call': 'off',
    },
  },
);
