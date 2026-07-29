#!/usr/bin/env node
/**
 * Parse every checked-in A2O feature before the E2E suite is allowed to run.
 *
 * Cucumber loads feature files as a batch: one malformed file prevents the
 * formatter from producing the report for every scenario.  Reuse Cucumber's
 * parser rather than approximating Gherkin with regular expressions, so route
 * paths such as `/health` remain valid while bare continuation lines and
 * malformed tables fail with the parser's file and line diagnostics.
 */

import { readdir, readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const cucumberRequire = createRequire(require.resolve('@cucumber/cucumber/package.json'));
const { AstBuilder, GherkinClassicTokenMatcher, Parser } = cucumberRequire('@cucumber/gherkin');
const { IdGenerator } = cucumberRequire('@cucumber/messages');

const scriptDir = dirname(fileURLToPath(import.meta.url));
const a2oRoot = resolve(scriptDir, '..');
const featureRoots = [join(a2oRoot, 'features'), join(a2oRoot, 'held', 'features')];

async function collectFeatureFiles(directory) {
  let entries;
  try {
    entries = await readdir(directory, { withFileTypes: true });
  } catch (error) {
    if (error.code === 'ENOENT') return [];
    throw error;
  }

  const files = await Promise.all(
    entries.map(async entry => {
      const path = join(directory, entry.name);
      if (entry.isDirectory()) return collectFeatureFiles(path);
      return entry.isFile() && entry.name.endsWith('.feature') ? [path] : [];
    })
  );
  return files.flat();
}

export function parseFeature(source) {
  const parser = new Parser(new AstBuilder(IdGenerator.uuid()), new GherkinClassicTokenMatcher());
  return parser.parse(source);
}

export async function lintFeatureFiles(files) {
  const errors = [];
  for (const file of files.sort()) {
    try {
      parseFeature(await readFile(file, 'utf8'));
    } catch (error) {
      errors.push(`${relative(a2oRoot, file)}\n${error.message}`);
    }
  }
  return errors;
}

async function main() {
  const files = (await Promise.all(featureRoots.map(collectFeatureFiles))).flat();
  const errors = await lintFeatureFiles(files);

  if (errors.length > 0) {
    console.error(`gherkin-prepush-lint: ${errors.length} malformed feature file(s):`);
    for (const error of errors) console.error(`\n${error}`);
    process.exitCode = 1;
    return;
  }

  console.log(`gherkin-prepush-lint: parsed ${files.length} feature files`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main();
}
