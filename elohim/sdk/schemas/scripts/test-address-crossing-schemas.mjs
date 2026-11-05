#!/usr/bin/env node
/**
 * Address-crossing schemas (re-genesis 1, byte profile epoch 0): put-blob-response, signature,
 * epr-envelope-view proof, compute task/receipt descriptors, and the byte-CID patterns on the
 * four view/object schemas. Every legacy spelling must be refused; unknown signature schemes
 * must be admitted structurally so a reader can HELD them.
 */
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv';
import Ajv2020 from 'ajv/dist/2020.js';

const here = dirname(fileURLToPath(import.meta.url));
const load = (p) => JSON.parse(readFileSync(resolve(here, '../v1', p), 'utf8'));
let failures = 0;
let passes = 0;
function check(ok, message) {
  if (ok) {
    passes++;
    console.log(`PASS: ${message}`);
  } else {
    failures++;
    console.error(`FAIL: ${message}`);
  }
}
const compile = (schema) =>
  (schema.$schema.includes('2020-12')
    ? new Ajv2020({ allErrors: true, strict: false })
    : new Ajv({ allErrors: true, strict: false })
  ).compile(schema);

// Kernel vectors (blake3 of "hello world", "abc", ""): byte CIDs; and the sha2 citation CID.
const CID_A = 'bafkr4igxjga67jykbseaxdmmdgc5a5o3zp3htom2l6mrjznk7fvyggu6eq';
const CID_B = 'bafkr4ideg6z2yocgkez77nr3outtvdnvjdcvqrs5phnqh7jvtrwnlpm5qu';
const CID_C = 'bafkr4ifpcne3t5pzugtkaqcn5i3nzskjtpfslsnnyejlpte2spfoihzsmi';
const CITE_CID = 'bafkreifzjut3te2nhyekklss27nh3k72ysco7y32koao5eei66wof36n5e';

// ── put-blob-response ──────────────────────────────────────────────────────
{
  const v = compile(load('views/put-blob-response.schema.json'));
  const base = () => ({
    blobCid: CID_A,
    byteLen: 11,
    mimeType: 'application/octet-stream',
    profile: {
      epoch: 0,
      chunker: { id: 'fastcdc-2020', min: 262144, avg: 1048576, max: 4194304, level: 1 },
      erasure: { id: 'none', data: 0, parity: 0 },
    },
    chunks: [{ cid: CID_A, len: 11 }],
    stripes: [],
    reach: 'commons',
    authorId: null,
    createdAt: '2026-11-05T00:00:00Z',
    verifiedAt: null,
  });
  check(v(base()), 'put-blob-response accepts a single-chunk blob with erasure none');
  const rs = base();
  rs.profile.erasure = { id: 'rs-gf8-0x11d-v1', data: 4, parity: 3 };
  rs.stripes = [
    {
      index: 0,
      byteLen: 11,
      shards: [CID_A, CID_B, CID_C, CID_A, CID_B, CID_C, CID_A].map((cid, index) => ({ cid, len: 3, index })),
    },
  ];
  check(v(rs), 'put-blob-response accepts an RS stripe with 4+3 shards');
  const empty = base();
  empty.byteLen = 0;
  empty.chunks = [];
  empty.blobCid = CID_C;
  check(v(empty), 'put-blob-response accepts the empty blob (zero chunks)');
  for (const field of ['blobHash', 'blake3Hash', 'encoding', 'dataShards', 'totalShards', 'shardSize', 'shardHashes', 'totalSize']) {
    check(!v({ ...base(), [field]: 'x' }), `put-blob-response refuses the retired field ${field}`);
  }
  for (const [name, bad] of [
    ['sha256- spelling', `sha256-${'aa'.repeat(32)}`],
    ['blake3- spelling', `blake3-${'aa'.repeat(32)}`],
    ['bafkrei citation CID', CITE_CID],
    ['wrong length', CID_A.slice(0, -1)],
  ]) {
    check(!v({ ...base(), blobCid: bad }), `put-blob-response refuses blobCid: ${name}`);
    check(!v({ ...base(), chunks: [{ cid: bad, len: 1 }] }), `put-blob-response refuses chunk cid: ${name}`);
  }
  const noProfile = base();
  delete noProfile.profile;
  check(!v(noProfile), 'put-blob-response requires the declared profile');
  const noStripes = base();
  delete noStripes.stripes;
  check(!v(noStripes), 'put-blob-response requires stripes (empty when erasure is none)');
}

// ── signature (object) ─────────────────────────────────────────────────────
{
  const v = compile(load('objects/signature.schema.json'));
  const signer = [1, 85, 18, 32, ...new Array(32).fill(7)];
  const ed = {
    signer,
    envelope: { scheme: 0, key: [0xed, 0x01, ...new Array(32).fill(9)], sig: new Array(64).fill(3) },
  };
  check(v(ed), 'signature accepts an Ed25519 envelope (scheme 0)');
  check(
    v({ signer, envelope: { scheme: 513, key: [0x80, 0x24, 1, 2, 3], sig: new Array(2420).fill(1) } }),
    'signature admits an unknown scheme with unknown key/sig lengths (so a reader can HELD it)',
  );
  check(!v({ signer, algorithm: 'ed25519', signature: new Array(64).fill(3) }), 'signature refuses the retired {algorithm, signature} shape');
  check(!v({ ...ed, algorithm: 'ed25519' }), 'signature refuses the retired algorithm field beside an envelope');
  check(!v({ signer, envelope: { ...ed.envelope, scheme: 65536 } }), 'signature refuses a scheme above u16');
  check(!v({ signer, envelope: { ...ed.envelope, scheme: -1 } }), 'signature refuses a negative scheme');
  const noKey = { signer, envelope: { scheme: 0, sig: ed.envelope.sig } };
  check(!v(noKey), 'signature requires the envelope key');
  check(!v({ signer, envelope: { ...ed.envelope, key: 'ed01' } }), 'signature key is bytes (an integer array), not a string');
}

// ── epr-envelope-view proof ────────────────────────────────────────────────
{
  const schema = load('views/epr-envelope-view.schema.json');
  const proof = compile({ ...schema.properties.proof, $schema: 'http://json-schema.org/draft-07/schema#' });
  const good = { signer: CID_A, envelope: { scheme: 0, key: `ed01${'bb'.repeat(32)}`, sig: 'aa'.repeat(64) } };
  check(proof(good), 'epr-envelope-view proof accepts {signer, envelope:{scheme,key,sig}} with hex bytes');
  check(proof({ ...good, envelope: { ...good.envelope, scheme: 513, key: 'ff', sig: 'ff' } }), 'epr-envelope-view proof admits an unknown scheme');
  check(!proof({ signer: CID_A, algorithm: 'ed25519', signature: 'aa'.repeat(64) }), 'epr-envelope-view proof refuses the retired shape');
  check(!proof({ ...good, envelope: { ...good.envelope, key: 'ED01' } }), 'epr-envelope-view proof refuses uppercase hex');
}

// ── compute task / receipt: {cid, bytes} only ──────────────────────────────
{
  for (const name of ['compute-task', 'compute-receipt']) {
    const text = readFileSync(resolve(here, '../v1', `${name}.schema.json`), 'utf8');
    const schema = JSON.parse(text);
    const hasSha256Prop = (o) =>
      o !== null &&
      typeof o === 'object' &&
      (('properties' in o && o.properties && 'sha256' in o.properties) ||
        (Array.isArray(o.required) && o.required.includes('sha256')) ||
        Object.values(o).some(hasSha256Prop));
    check(!hasSha256Prop(schema), `${name} descriptors carry no sha256 property or requirement`);
    check(text.includes('bafkr4i'), `${name} artifact cid pattern is the byte-CID pattern`);
  }
  const task = load('compute-task.schema.json');
  check(task.properties.runtimeImage.pattern.startsWith('^sha256:'), 'compute-task runtimeImage stays an OCI sha256: digest (external evidence)');
}

// ── bafkr4i patterns on the view/object schemas ────────────────────────────
for (const [file, pointer] of [
  ['views/bin-release.schema.json', null],
  ['views/source-state.schema.json', null],
  ['views/placement-explanation-view.schema.json', null],
  ['objects/placement-parameter-set.schema.json', null],
]) {
  const text = readFileSync(resolve(here, '../v1', file), 'utf8');
  check(text.includes('^bafkr4i[a-z2-7]'), `${file} pattern is bafkr4i`);
  check(!text.includes('bafkrei[a-z2-7]'), `${file} keeps no bafkrei pattern`);
}

console.log(`\n${passes} passed, ${failures} failed`);
process.exit(failures === 0 ? 0 : 1);
