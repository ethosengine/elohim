// Jest config for `test-storybook` (picked up by @storybook/test-runner by this
// file name). It extends the runner's defaults with one change.
//
// The runner roots jest at the git root (`getProjectRoot()`), so jest's
// snapshot bookkeeping scans every `.snap` file in the monorepo — including
// Rust `insta` snapshots inside git submodules (e.g. elohim/brit's
// `snapshots/*.snap`). No story claims them, so jest reports them as obsolete
// snapshot files, and under CI that fails the whole Storybook gate even when
// every story passes. Submodules are separate repositories with their own
// tests, so keep jest out of all of them, read from .gitmodules so a new
// submodule is covered without editing this file.
const { readFileSync } = require('node:fs');
const { join } = require('node:path');
const { getJestConfig } = require('@storybook/test-runner');

const base = getJestConfig();

function submoduleIgnorePatterns(rootDir) {
  let gitmodules = '';
  try {
    gitmodules = readFileSync(join(rootDir, '.gitmodules'), 'utf8');
  } catch {
    return [];
  }
  return [...gitmodules.matchAll(/^\s*path\s*=\s*(.+?)\s*$/gm)].map(
    match => `<rootDir>/${match[1]}/`
  );
}

const submodules = submoduleIgnorePatterns(base.rootDir);

module.exports = {
  ...base,
  modulePathIgnorePatterns: [...(base.modulePathIgnorePatterns ?? []), ...submodules],
  testPathIgnorePatterns: [...(base.testPathIgnorePatterns ?? []), ...submodules],
};
