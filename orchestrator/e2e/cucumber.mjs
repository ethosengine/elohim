/**
 * Cucumber-JS configuration.
 *
 * Two feature source paths:
 *   1. orchestrator/e2e/features/  — hand-written executable tests
 *   2. genesis/docs/content/       — aspirational BDD scenarios (most will be "pending")
 *
 * Genesis features are included so `--dry-run` reports undefined steps and the
 * gap scanner can track readiness. Running without `--tags` will attempt all
 * features, so always filter with tags for focused execution.
 */
export default {
  paths: ['features/**/*.feature', '../../genesis/docs/content/elohim-protocol/**/*.feature'],
  requireModule: ['tsx'],
  require: ['steps/**/*.ts'],
  format: [
    'progress-bar',
    ['html', 'reports/cucumber-report.html'],
    ['json', 'reports/cucumber-report.json'],
  ],
  formatOptions: { snippetInterface: 'async-await' },
};
