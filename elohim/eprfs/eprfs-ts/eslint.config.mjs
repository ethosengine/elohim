// Rail R-T1 (interfaces spec §1.3): @elohim/eprfs stays pure. It imports neither
// @elohim/storage-client nor any DOM or Angular module. `test/purity.test.ts` enforces the same
// rule in the package's own test run, and tsconfig's `lib` carries no DOM.
import tsParser from '@typescript-eslint/parser';

export default [
  {
    files: ['src/**/*.ts'],
    languageOptions: { parser: tsParser, sourceType: 'module' },
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: [
            { group: ['@elohim/storage-client', '@elohim/storage-client/*'], message: 'R-T1: eprfs-ts is pure; the HTTP tier lives in storage-client.' },
            { group: ['@angular/*', 'rxjs', 'rxjs/*'], message: 'R-T1: eprfs-ts is pure; no framework.' },
          ],
        },
      ],
      'no-restricted-globals': ['error', 'window', 'document', 'navigator', 'caches', 'localStorage', 'fetch'],
    },
  },
];
