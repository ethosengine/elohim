/**
 * Data-integrity guard for the manifesto blob reference (genesis #1182 Cluster D).
 *
 * `genesis/data/lamad/content/manifesto.json` carries a hardcoded `blobHash` so
 * the a2o "Blob content loads via CID" scenario passes. That hash MUST equal the
 * sha256 of `genesis/docs/content/elohim-protocol/manifesto.md` — the file the
 * "Upload Blob-Backed Content" genesis stage actually PUTs to `/blob/<hash>`
 * (substrate-verify.sh cmd_upload, CONTENT_PATH default). If someone edits
 * manifesto.md without updating manifesto.json, the reference dangles SILENTLY
 * (the row seeds, but /blob/<old-hash> 404s). This test fails loudly instead.
 */
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

// Tests run from genesis/seeder; the two artifacts live one level up under genesis/.
const MANIFESTO_MD = resolve(process.cwd(), '../docs/content/elohim-protocol/manifesto.md');
const MANIFESTO_JSON = resolve(process.cwd(), '../data/lamad/content/manifesto.json');

describe('manifesto blob reference', () => {
  it('blobHash equals sha256 of the uploaded manifesto.md bytes (no silent dangle)', () => {
    const mdBytes = readFileSync(MANIFESTO_MD);
    const expected = 'sha256-' + createHash('sha256').update(mdBytes).digest('hex');

    const node = JSON.parse(readFileSync(MANIFESTO_JSON, 'utf8')) as { blobHash?: string };

    expect(
      node.blobHash,
      'manifesto.json.blobHash drifted from manifesto.md — re-derive it (sha256 of manifesto.md) ' +
        'or the live /blob/<hash> reference dangles 404',
    ).toBe(expected);
  });
});
