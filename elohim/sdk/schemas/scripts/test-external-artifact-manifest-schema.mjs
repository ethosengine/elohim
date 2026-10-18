// Test for v1/objects/external-artifact-manifest.schema.json
//
// The external-artifact manifest describes one pinned artifact made outside the protocol
// (a library version, a supplier's spec sheet, a variety record, an embedding model): its
// bytes by CID, its SPDX license and the CID of the license/NOTICE text it carries, its
// source and upstream repository, and the funding channels its makers declared, as facts
// with no amounts. Node 4 of
// genesis/docs/superpowers/specs/2026-10-15-reliance-as-cite-shared-provenance-primitives-design.md.
//
// The load-bearing probe: the existing embedding-model manifest validates UNCHANGED.

import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv2020 from 'ajv/dist/2020.js';
import schema from '../v1/objects/external-artifact-manifest.schema.json' with { type: 'json' };

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '../../../../');
const MODEL_MANIFEST = resolve(
  REPO_ROOT,
  '.epr-meta/elohim/algorithms/embedding-models/all-minilm-l6-v2.json',
);

const ajv = new Ajv2020({ strict: true, allErrors: true });
const validate = ajv.compile(schema);

const failures = [];

function check(name, value, shouldPass) {
  const ok = validate(value);
  if (shouldPass && !ok) {
    failures.push([name, validate.errors]);
  } else if (!shouldPass && ok) {
    failures.push([`${name} (should have been rejected)`, null]);
  }
}

const modelManifest = JSON.parse(readFileSync(MODEL_MANIFEST, 'utf8'));
check('embedding-model manifest, unchanged', modelManifest, true);

// A crate version a build cites: the bytes CID wraps the Cargo.lock checksum.
const crate = {
  artifact_type: 'package-manifest',
  bytes: 'bafkreigyfkfu6h3vwzbc3zsulgdn6okd2eh6ovuqiyouihuyxrgt7s4ude',
  license: 'MIT OR Apache-2.0',
  license_texts: [
    { file: 'LICENSE-MIT', text: 'bafkreif2pall7dybz7vecqka3zo24irdwabwdi4wc55jznaq75q7eaavvu' },
    { file: 'LICENSE-APACHE', text: 'bafkreigyfkfu6h3vwzbc3zsulgdn6okd2eh6ovuqiyouihuyxrgt7s4ude' },
  ],
  source: 'crates.io:example@0.1.0',
  repository: 'https://github.com/example/example',
  funding: [
    { channel: 'drips', url: 'https://www.drips.network/app/projects/github/example', declared_in: 'FUNDING.json' },
  ],
};
check('crate with dual license texts and a declared funding channel', crate, true);

// A grocer's supplier spec sheet: same shape, no code words.
check(
  'supplier spec sheet',
  {
    artifact_type: 'supplier-spec',
    bytes: 'bafkreif2pall7dybz7vecqka3zo24irdwabwdi4wc55jznaq75q7eaavvu',
    license: 'LicenseRef-supplier-terms',
    source: 'valley-farm-coop:catalogue/tomato-heirloom-2026',
  },
  true,
);

check(
  '(MIT OR Apache-2.0) AND Unicode-3.0',
  { ...crate, license: '(MIT OR Apache-2.0) AND Unicode-3.0' },
  true,
);

// Rejections.
check('no bytes field at all', { artifact_type: 'package-manifest', license: 'MIT', source: 'x' }, false);
check('missing license', (({ license, ...rest }) => rest)(crate), false);
check('license that is not an SPDX expression', { ...crate, license: 'MIT and Apache' }, false);
check(
  'funding with an amount (valuation stays out of the record)',
  { ...crate, funding: [{ ...crate.funding[0], amount: 100 }] },
  false,
);
check(
  'funding not traced to the makers own declaration',
  { ...crate, funding: [{ channel: 'drips', url: 'https://www.drips.network/x' }] },
  false,
);
check('bytes that is not a CID', { ...crate, bytes: 'sha256:d82a8b4f1f75b642' }, false);
check('an undeclared field', { ...crate, owner: 'someone' }, false);
check(
  'model-manifest missing its own required field',
  (({ dims, ...rest }) => rest)(modelManifest),
  false,
);
check('model-manifest with a non-integer dims', { ...modelManifest, dims: '384' }, false);
check(
  'model-only field on a non-model artifact',
  { ...crate, dims: 384 },
  false,
);

if (failures.length > 0) {
  console.error('external-artifact-manifest schema test FAILED:');
  for (const [name, errors] of failures) {
    console.error(`  - ${name}`);
    if (errors) console.error(JSON.stringify(errors, null, 2));
  }
  process.exit(1);
}
console.log('external-artifact-manifest schema: all cases pass (model manifest validates unchanged)');
