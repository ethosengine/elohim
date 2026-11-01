// Validates chrome-context islands (JSON array of {name, value} on stdin) against
// elohim/sdk/schemas/v1/views/chrome-context.schema.json. Prints
// [{name, valid, errors}] on stdout. Relative $refs between the SDK schemas are
// rewritten to the targets' $id so they resolve without a base URI.
import { readFileSync, readdirSync } from 'node:fs';
import { join, basename } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv2020 from 'ajv/dist/2020.js';

const root = fileURLToPath(new URL('../../sdk/schemas/v1/', import.meta.url));
const byFile = new Map();
for (const dir of ['enums', 'views']) {
  for (const f of readdirSync(join(root, dir))) {
    if (!f.endsWith('.schema.json')) continue;
    const s = JSON.parse(readFileSync(join(root, dir, f), 'utf8'));
    if (s.$id) byFile.set(f, s);
  }
}
function fix(node) {
  if (Array.isArray(node)) return node.forEach(fix);
  if (!node || typeof node !== 'object') return;
  if (typeof node.$ref === 'string' && !node.$ref.startsWith('#')) {
    const [file, frag] = node.$ref.split('#');
    const target = byFile.get(basename(file));
    if (target) node.$ref = target.$id + (frag ? '#' + frag : '');
  }
  for (const v of Object.values(node)) fix(v);
}
const ajv = new Ajv2020({ strict: false, allErrors: true, validateFormats: false, validateSchema: false });
for (const s of byFile.values()) { delete s.$schema; fix(s); }
for (const s of byFile.values()) ajv.addSchema(s);
const validate = ajv.getSchema('epr:schema:view:chrome-context');
if (!validate) throw new Error('chrome-context schema did not compile');

const cases = JSON.parse(readFileSync(0, 'utf8'));
const results = cases.map(({ name, value }) => {
  const valid = validate(value) === true;
  return { name, valid, errors: valid ? [] : (validate.errors || []).map((e) => `${e.instancePath} ${e.message}`) };
});
process.stdout.write(JSON.stringify(results));
