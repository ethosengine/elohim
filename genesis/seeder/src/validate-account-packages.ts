/**
 * Account Package Validator
 *
 * Validates genesis/data/account-packages/*.json against account-package.schema.json.
 * Catches field drift (e.g. missing relationshipType) at dev time before the seeder
 * hits the elohim-storage HTTP endpoint.
 *
 * Usage: pnpm run validate:account-packages
 */

import { readdir, readFile } from 'node:fs/promises';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const Ajv2020 = require('ajv/dist/2020.js') as typeof import('ajv/dist/2020.js').default;
const addFormats = require('ajv-formats') as typeof import('ajv-formats').default;

const __dirname = dirname(fileURLToPath(import.meta.url));
const PKG_DIR = join(__dirname, '..', '..', 'data', 'account-packages');
const SCHEMA_PATH = join(PKG_DIR, 'account-package.schema.json');

// Files in the directory that are not account packages
const SKIP_FILES = new Set([
  'account-package.schema.json',
  'conductor-groups.json',
  'index.json',
]);

async function main() {
  const schema = JSON.parse(await readFile(SCHEMA_PATH, 'utf8'));
  const ajv = new Ajv2020({ allErrors: true, strict: false });
  addFormats(ajv);
  const validate = ajv.compile(schema);

  const files = (await readdir(PKG_DIR))
    .filter(f => f.endsWith('.json') && !SKIP_FILES.has(f));

  let pass = 0;
  let fail = 0;

  for (const file of files) {
    const raw = await readFile(join(PKG_DIR, file), 'utf8');
    let data: unknown;
    try {
      data = JSON.parse(raw);
    } catch {
      console.error(`PARSE ERROR: ${file}`);
      fail++;
      continue;
    }

    if (!validate(data)) {
      console.error(`INVALID: ${file}`);
      for (const err of validate.errors!) {
        console.error(`  ${err.instancePath} ${err.message}`);
      }
      fail++;
    } else {
      pass++;
    }
  }

  console.log(
    `\nAccount package validation: ${pass} valid, ${fail} errors, ${files.length} total`
  );
  process.exit(fail > 0 ? 1 : 0);
}

main().catch(err => {
  console.error(err);
  process.exit(1);
});
