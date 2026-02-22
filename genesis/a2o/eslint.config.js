import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import eslint from '@eslint/js';
import prettierConfig from 'eslint-config-prettier';
import importPlugin from 'eslint-plugin-import';
import prettierPlugin from 'eslint-plugin-prettier';
import sonarjs from 'eslint-plugin-sonarjs';
import unicorn from 'eslint-plugin-unicorn';
import tseslint from 'typescript-eslint';

const __dirname = dirname(fileURLToPath(import.meta.url));

export default tseslint.config(
  {
    ignores: ['dist/**', 'node_modules/**', 'reports/**'],
  },
  {
    files: ['**/*.ts'],
    extends: [
      eslint.configs.recommended,
      ...tseslint.configs.recommended,
      ...tseslint.configs.stylistic,
      sonarjs.configs.recommended,
    ],
    plugins: {
      import: importPlugin,
      prettier: prettierPlugin,
      unicorn,
    },
    languageOptions: {
      parserOptions: {
        project: ['./tsconfig.json'],
        tsconfigRootDir: __dirname,
      },
    },
    settings: {
      'import/resolver': {
        typescript: {
          project: './tsconfig.json',
        },
      },
    },
    rules: {
      // ============================================================
      // TYPESCRIPT-ESLINT
      // ============================================================

      // Type safety
      '@typescript-eslint/no-explicit-any': 'error',
      '@typescript-eslint/no-unsafe-assignment': 'warn',
      '@typescript-eslint/no-unsafe-member-access': 'warn',
      '@typescript-eslint/no-unsafe-call': 'warn',
      '@typescript-eslint/no-unsafe-return': 'warn',
      '@typescript-eslint/no-unsafe-argument': 'warn',

      // Unused code
      '@typescript-eslint/no-unused-vars': [
        'error',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
          caughtErrorsIgnorePattern: '^_',
        },
      ],
      '@typescript-eslint/no-empty-function': 'warn',

      // Code style
      '@typescript-eslint/consistent-type-definitions': ['error', 'interface'],
      '@typescript-eslint/prefer-nullish-coalescing': 'error',
      '@typescript-eslint/prefer-optional-chain': 'error',
      '@typescript-eslint/prefer-readonly': 'error',
      '@typescript-eslint/no-array-constructor': 'error',
      '@typescript-eslint/prefer-for-of': 'error',
      '@typescript-eslint/prefer-includes': 'error',
      '@typescript-eslint/prefer-string-starts-ends-with': 'error',

      // Promise handling
      '@typescript-eslint/no-misused-promises': 'error',
      '@typescript-eslint/no-floating-promises': 'error',
      '@typescript-eslint/promise-function-async': 'warn',
      '@typescript-eslint/require-await': 'error',

      // Type assertion hygiene
      '@typescript-eslint/no-unnecessary-type-assertion': 'error',
      '@typescript-eslint/await-thenable': 'error',
      '@typescript-eslint/max-params': ['error', { max: 7 }],

      // Deprecated APIs
      '@typescript-eslint/no-deprecated': 'warn',

      // Naming conventions
      '@typescript-eslint/naming-convention': [
        'error',
        {
          selector: 'interface',
          format: ['PascalCase'],
          custom: { regex: '^[A-Z]', match: true },
        },
        { selector: 'class', format: ['PascalCase'] },
        { selector: 'typeAlias', format: ['PascalCase'] },
      ],

      // ============================================================
      // IMPORT RULES
      // ============================================================
      'import/order': [
        'error',
        {
          groups: ['builtin', 'external', 'internal', 'parent', 'sibling', 'index', 'type'],
          pathGroups: [
            { pattern: '@cucumber/**', group: 'external', position: 'before' },
            { pattern: '@framework/**', group: 'internal', position: 'before' },
          ],
          pathGroupsExcludedImportTypes: ['type'],
          'newlines-between': 'always',
          alphabetize: { order: 'asc', caseInsensitive: true },
        },
      ],
      'import/no-duplicates': 'error',
      'import/no-useless-path-segments': 'error',

      // ============================================================
      // GENERAL BEST PRACTICES
      // ============================================================
      'no-console': ['error', { allow: ['warn', 'error'] }],
      'prefer-const': 'error',
      'no-var': 'error',
      eqeqeq: ['error', 'always'],
      'no-eval': 'error',
      'no-implied-eval': 'error',
      'no-new-func': 'error',
      'no-throw-literal': 'error',

      // ============================================================
      // SONARJS
      // ============================================================
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
      'sonarjs/no-unused-vars': 'error',
      // Arrow functions in callbacks are idiomatic
      'sonarjs/no-nested-functions': 'off',

      // ============================================================
      // UNICORN (Node.js applicable subset)
      // ============================================================
      'unicorn/prefer-set-has': 'error',
      'unicorn/no-zero-fractions': 'error',
      'unicorn/prefer-number-properties': 'error',
      'unicorn/prefer-code-point': 'error',
      'unicorn/prefer-array-index-of': 'error',
      'unicorn/no-typeof-undefined': 'error',
      'unicorn/prefer-export-from': 'error',
      'unicorn/prefer-global-this': 'error',
      'unicorn/no-negated-condition': 'error',
      'unicorn/no-array-push-push': 'error',
      'unicorn/prefer-string-raw': 'error',
      'unicorn/prefer-array-some': 'error',
      'unicorn/prefer-negative-index': 'error',
      'unicorn/prefer-at': 'error',
      'unicorn/prefer-structured-clone': 'error',
      // Scripts use main().catch() pattern
      'unicorn/prefer-top-level-await': 'off',
      // DOM-only rules: disabled for Node.js
      'unicorn/prefer-blob-reading-methods': 'off',
      'unicorn/prefer-dom-node-remove': 'off',

      // ============================================================
      // PRETTIER
      // ============================================================
      'prettier/prettier': [process.env.CI === 'true' ? 'off' : 'error'],
      ...prettierConfig.rules,
    },
  },
  {
    // Scripts: allow console.log (CLI tools with intentional output)
    files: ['scripts/**/*.ts'],
    rules: {
      'no-console': 'off',
    },
  },
  {
    // Steps: relaxed duplicate-string threshold (Given/When/Then repetition)
    files: ['steps/**/*.ts'],
    rules: {
      'sonarjs/no-duplicate-string': ['error', { threshold: 5 }],
    },
  }
);
