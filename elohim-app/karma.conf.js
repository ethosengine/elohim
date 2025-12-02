// Karma configuration file, see link for more information
// https://karma-runner.github.io/1.0/config/configuration-file.html

module.exports = function (config) {
  // Detect if we're in a headless environment (CI/CD, Eclipse Che, etc.)
  const isHeadless = process.env.CI || process.env.DEVWORKSPACE_ID || process.env.CHE_WORKSPACE_ID;

  config.set({
    basePath: '',
    frameworks: ['jasmine', '@angular-devkit/build-angular'],
    plugins: [
      require('karma-jasmine'),
      require('karma-chrome-launcher'),
      require('karma-jasmine-html-reporter'),
      require('karma-coverage'),
      require('@angular-devkit/build-angular/plugins/karma')
    ],
    client: {
      jasmine: {
        // you can add configuration options for Jasmine here
        // the possible options are listed at https://jasmine.github.io/api/edge/Configuration.html
        // for example, you can disable the random execution order
        // random: false
        failFast: false // set to true to stop on first failure
      },
      clearContext: false // leave Jasmine Spec Runner output visible in browser
    },
    // Use 'dots' for minimal output, 'spec' for verbose, 'progress' for default
    // 'dots' shows just dots for passes and F for failures - most concise
    reporters: isHeadless ? ['dots', 'coverage'] : ['progress', 'kjhtml', 'coverage'],
    jasmineHtmlReporter: {
      suppressAll: true // removes the duplicated traces
    },
    coverageReporter: {
      dir: require('path').join(__dirname, './coverage/elohim-app'),
      subdir: '.',
      reporters: [
        { type: 'html' },
        { type: 'text-summary' },
        { type: 'lcovonly', file: 'lcov.info' }
      ],
      check: {
        // Only enforce coverage on services (business logic)
        '**/services/*.ts': {
          statements: 50,
          branches: 15,
          functions: 50,
          lines: 50
        }
        // No global threshold - components/models are advisory only
      }
    },
    // Automatically use ChromeHeadless in CI/Eclipse Che, Chrome locally
    browsers: [isHeadless ? 'ChromeHeadlessCI' : 'Chrome'],
    customLaunchers: {
      ChromeHeadlessCI: {
        base: 'ChromeHeadless',
        flags: [
          '--headless=new',
          '--no-sandbox',
          '--disable-gpu',
          '--disable-dev-shm-usage',
          '--disable-setuid-sandbox',
          '--disable-software-rasterizer'
        ]
      }
    },
    restartOnFileChange: true
  });
};