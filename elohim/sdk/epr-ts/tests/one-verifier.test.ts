import { readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

/**
 * Rail R-T3 (interfaces spec §1.3): one TypeScript verifier, `verifyAddress`. A sha2-256 digest
 * taken anywhere else is a second verifier in the making, so the set of source files that take
 * one is fixed here, across every TypeScript tree that reads content addresses: this package,
 * `@elohim/eprfs`, `@elohim/storage-client`, and the app (its service worker included). A new
 * digest site fails this test until it is moved behind `verifyAddress` or named below with its
 * reason. All admission checks delegate to verifyAddress; no independent verifier is allowed.
 *
 * The scan is recursive and resolved from this file, not from the working directory, so it reads
 * the same trees however the test is run. Tests and specs are not scanned: they mint fixtures.
 */
const DIGEST = /\bsha256\.digest\(|\bsha256\(|subtle\.digest\(/;

const REPO = fileURLToPath(new URL('../../../../', import.meta.url));
const ROOTS = [
  'elohim/sdk/epr-ts/src',
  'elohim/eprfs/eprfs-ts/src',
  'elohim/sdk/storage-client-ts/src',
  'app/elohim-app/src',
];

const ALLOWED: Record<string, string> = {
  'elohim/sdk/epr-ts/src/address.ts': 'the verifier itself',
  'elohim/sdk/epr-ts/src/cid.ts':
    'computeCid names test and envelope bytes; verifyCid delegates to verifyAddress',
  'elohim/sdk/epr-ts/src/blob-hash.ts':
    'the reader: three answers about a row-declared blob hash (match | mismatch | unverifiable), ' +
    'over forms verifyAddress refuses; a reporting question, not admission',
  'elohim/eprfs/eprfs-ts/src/tree.ts': 'cidOf mints a node CID for the walk; it checks nothing',
  'app/elohim-app/src/app/elohim/utils/epr-codec.ts': 'encodeEprHead mints a head CID; it checks nothing',
  'app/elohim-app/src/app/shefa/services/duplicate-detection.service.ts':
    'mentions crypto.subtle.digest in a comment only',
  'app/elohim-app/src/app/elohim/services/context-assembly.service.ts':
    'mentions crypto.subtle.digest in a comment only',
  'app/elohim-app/src/app/generated/create-relationship-input.ts':
    'generated doc comment naming how storage mints an edge id',
};

function sources(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const path = join(dir, e.name);
    if (e.isDirectory()) return e.name === 'node_modules' ? [] : sources(path);
    return e.name.endsWith('.ts') && !/\.(spec|test)\.ts$/.test(e.name) ? [path] : [];
  });
}

describe('one TypeScript verifier (R-T3)', () => {
  it('takes a sha2-256 digest only in the files named here', () => {
    const hashing = ROOTS.flatMap((root) => sources(join(REPO, root)))
      .filter((f) => DIGEST.test(readFileSync(f, 'utf8')))
      .map((f) => relative(REPO, f).split('\\').join('/'))
      .sort();
    expect(hashing).toEqual(Object.keys(ALLOWED).sort());
  });
});
