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
    // Scoped saga profile: cucumber-js MERGES a profile's paths with CLI
    // positionals instead of replacing them, so running the saga dir under
    // `default` executes ~800+ scenarios (content ingests included) — a heavy
    // write event that re-churns the mesh being measured (2026-08-16 Q10
    // finding). Measure saga with `--profile saga`, never bare CLI paths.
    saga: { ...base, paths: ['features/dataplane/resiliency-saga/**/*.feature'] },
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
    testnet: {
      ...base,
      // The testnet soak features live under elohim/ + deployment/ (compute-allocation,
      // persona-testnet-validation) and carry @requires:alpha-cluster-6peer / @requires:shem, so
      // scope-reconcile moves them to held/ when that capability is down. Glob only those two subtrees
      // in BOTH the live and held trees (the npm script tag-filters to @testnet): globs tolerate an
      // absent held/ tree — explicit paths broke the moment a feature was held — while keeping the
      // gherkin-parse blast radius small (a parse error in an unrelated feature won't abort this run).
      // The substrate-scope Before hook also skips these at runtime while the capability is unavailable.
      paths: [
        'features/elohim/**/*.feature',
        'features/deployment/**/*.feature',
        'held/features/elohim/**/*.feature',
        'held/features/deployment/**/*.feature',
      ],
    },
    delivery: {
      ...base,
      paths: ['features/delivery/**/*.feature'],
      worldParameters: { env: 'alpha' },
    },
    'delivery-browser': {
      ...base,
      paths: ['features/delivery/**/*.feature'],
      worldParameters: { env: 'alpha', deviceMode: 'playwright' },
    },
  };
}
