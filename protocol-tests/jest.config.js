module.exports = {
  testEnvironment: 'node',
  testMatch: ['**/step-definitions/**/*.test.js'],
  coverageDirectory: 'reports/coverage',
  coverageReporters: ['text', 'html', 'json-summary'],
  collectCoverageFrom: ['step-definitions/**/*.js'],
  verbose: true,
  testTimeout: 10000,
  reporters: [
    'default',
    [
      'jest-html-reporter',
      {
        pageTitle: 'Elohim Protocol Specification Test Report',
        outputPath: 'reports/html/index.html',
        includeFailureMsg: true,
        includeConsoleLog: true,
        theme: 'darkTheme',
        executionMode: 'reporter',
        dateFormat: 'yyyy-mm-dd HH:MM:ss',
      },
    ],
  ],
};
