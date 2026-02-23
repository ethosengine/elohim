/**
 * Cucumber-JS configuration.
 *
 * Named profiles target specific environments:
 *   default  — all features (requires tags to scope execution)
 *   alpha    — doorway-alpha environment
 *   local    — local dev stack
 *   browser  — Playwright browser tests (E2E_DEVICE_MODE=playwright)
 *   genesis  — includes aspirational BDD scenarios from genesis docs
 *
 * Run with: npx cucumber-js -p alpha --tags '@e2e'
 *
 * NOTE: For .mjs config files, cucumber-js treats the default export as the
 * default profile. To enable multiple profiles, `default` must be a function
 * that returns the profile map (see cucumber-js from_file.js).
 */

const base = {
  requireModule: ['tsx'],
  require: ['steps/**/*.ts'],
  format: [
    'progress-bar',
    ['html', 'reports/cucumber-report.html'],
    ['json', 'reports/cucumber-report.json'],
  ],
  formatOptions: { snippetInterface: 'async-await' },
};

export default function () {
  return {
    default: { ...base, paths: ['features/**/*.feature'] },
    alpha: { ...base, paths: ['features/**/*.feature'], worldParameters: { env: 'alpha' } },
    local: { ...base, paths: ['features/**/*.feature'], worldParameters: { env: 'local' } },
    browser: {
      ...base,
      paths: ['features/browser/**/*.feature'],
      worldParameters: { env: 'alpha', deviceMode: 'playwright' },
    },
    genesis: {
      ...base,
      paths: ['features/**/*.feature', '../docs/content/elohim-protocol/**/*.feature'],
    },
  };
}
