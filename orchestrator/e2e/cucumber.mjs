export default {
  paths: ['features/**/*.feature'],
  requireModule: ['tsx'],
  require: ['steps/**/*.ts'],
  format: [
    'progress-bar',
    ['html', 'reports/cucumber-report.html'],
    ['json', 'reports/cucumber-report.json'],
  ],
  formatOptions: { snippetInterface: 'async-await' },
};
