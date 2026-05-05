import { create } from 'storybook/theming';

export default create({
  base: 'dark',

  brandTitle: 'graphos',
  brandUrl: '/',

  colorPrimary: '#c9a961',
  colorSecondary: '#c9a961',

  // App chrome
  appBg: '#1c1917',
  appContentBg: '#292524',
  appPreviewBg: '#1c1917',
  appBorderColor: '#44403c',
  appBorderRadius: 8,

  // Typography
  fontBase: '"Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
  fontCode: '"JetBrains Mono", "Fira Code", monospace',

  // Text
  textColor: '#f5f4f0',
  textInverseColor: '#1c1917',
  textMutedColor: '#a8a29e',

  // Toolbar
  barBg: '#292524',
  barTextColor: '#a8a29e',
  barSelectedColor: '#c9a961',
  barHoverColor: '#f5f4f0',

  // Form elements
  inputBg: '#44403c',
  inputBorder: '#57534e',
  inputTextColor: '#f5f4f0',
  inputBorderRadius: 4,
});
