/**
 * The `fastcdc-2020` content-defined chunker, byte profile epoch 0 — the TypeScript twin of
 * `elohim/epr/src/profile.rs`, whose module documentation is the immutable specification.
 *
 * Tables: `GEAR[256]` and `MASKS[26]` are fastcdc `=5.0.0` v2020's published, unseeded tables;
 * `GEAR_LS[i] = GEAR[i] << 1` (mod 2^64). They are identified by
 * {@link FASTCDC_2020_TABLE_DIGEST}: BLAKE3-256 of `GEAR ‖ GEAR_LS ‖ MASKS`, each entry as 8
 * little-endian bytes. {@link fastcdc2020TableDigest} recomputes it so a transcription error in
 * this file cannot pass the shared vectors.
 *
 * Arithmetic: every hash step is unsigned 64-bit, done in `BigInt` and masked to 64 bits (shifts
 * discard the bits shifted out, additions wrap mod 2^64).
 *
 * Cut: given the window of the `n` bytes not yet chunked, `n <= min` is one chunk of all `n`
 * bytes; otherwise `limit = min(n, max)`, `center = avg` if `limit >= avg` else `limit`, `hash = 0`
 * and for `i` from `min / 2` while `i < limit / 2` (small mask below `center / 2`, large mask
 * after), with `a = 2i`: `hash = (hash << 2) + GEAR_LS[w[a]]`, cut at `a` if `hash & mask_ls == 0`;
 * then `hash = hash + GEAR[w[a + 1]]`, cut at `a + 1` if `hash & mask == 0`. No cut: `limit`.
 * Empty input yields zero chunks; the final chunk may be shorter than `min`.
 */
import { blake3 } from '@noble/hashes/blake3';

export const CHUNKER_FASTCDC_2020 = 'fastcdc-2020';
export const FASTCDC_2020_TABLE_DIGEST =
  '3bd9b87949825cd9b2a592d199dc15d485812848535c9b1d2ff6979baf7a8001';

/** The chunker's parameters as a manifest records them (`Chunker::Fastcdc2020`). */
export interface Fastcdc2020 {
  id: 'fastcdc-2020';
  min: number;
  avg: number;
  max: number;
  level: number;
}

/** The byte profile epoch 0's chunker parameters. */
export const FASTCDC_2020_EPOCH_0: Fastcdc2020 = {
  id: 'fastcdc-2020',
  min: 262_144,
  avg: 1_048_576,
  max: 4_194_304,
  level: 1,
};

const M64 = (1n << 64n) - 1n;

/** fastcdc v2020 `GEAR`: the published, unseeded gear table. */
export const FASTCDC_2020_GEAR: readonly bigint[] = [
  0x3b5d3c7d207e37dcn, 0x784d68ba91123086n, 0xcd52880f882e7298n,
  0xeacf8e4e19fdcca7n, 0xc31f385dfbd1632bn, 0x1d5f27001e25abe6n,
  0x83130bde3c9ad991n, 0xc4b225676e9b7649n, 0xaa329b29e08eb499n,
  0xb67fcbd21e577d58n, 0x0027baaada2acf6bn, 0xe3ef2d5ac73c2226n,
  0x0890f24d6ed312b7n, 0xa809e036851d7c7en, 0xf0a6fe5e0013d81bn,
  0x1d026304452cec14n, 0x03864632648e248fn, 0xcdaacf3dcd92b9b4n,
  0xf5e012e63c187856n, 0x8862f9d3821c00b6n, 0xa82f7338750f6f8an,
  0x1e583dc6c1cb0b6fn, 0x7a3145b69743a7f1n, 0xabb20fee404807ebn,
  0xb14b3cfe07b83a5dn, 0xb9dc27898adb9a0fn, 0x3703f5e91baa62ben,
  0xcf0bb866815f7d98n, 0x3d9867c41ea9dcd3n, 0x1be1fa65442bf22cn,
  0x14300da4c55631d9n, 0xe698e9cbc6545c99n, 0x4763107ec64e92a5n,
  0xc65821fc65696a24n, 0x76196c064822f0b7n, 0x485be841f3525e01n,
  0xf652bc9c85974ff5n, 0xcad8352face9e3e9n, 0x2a6ed1dceb35e98en,
  0xc6f483badc11680fn, 0x3cfd8c17e9cf12f1n, 0x89b83c5e2ea56471n,
  0xae665cfd24e392a9n, 0xec33c4e504cb8915n, 0x3fb9b15fc9fe7451n,
  0xd7fd1fd1945f2195n, 0x31ade0853443efd8n, 0x255efc9863e1e2d2n,
  0x10eab6008d5642cfn, 0x46f04863257ac804n, 0xa52dc42a789a27d3n,
  0xdaaadf9ce77af565n, 0x6b479cd53d87febbn, 0x6309e2d3f93db72fn,
  0xc5738ffbaa1ff9d6n, 0x6bd57f3f25af7968n, 0x67605486d90d0a4an,
  0xe14d0b9663bfbdaen, 0xb7bbd8d816eb0414n, 0xdef8a4f16b35a116n,
  0xe7932d85aaaffed6n, 0x08161cbae90cfd48n, 0x855507beb294f08bn,
  0x91234ea6ffd399b2n, 0xad70cf4b2435f302n, 0xd289a97565bc2d27n,
  0x8e558437ffca99den, 0x96d2704b7115c040n, 0x0889bbcdfc660e41n,
  0x5e0d4e67dc92128dn, 0x72a9f8917063ed97n, 0x438b69d409e016e3n,
  0xdf4fed8a5d8a4397n, 0x00f41dcf41d403f7n, 0x4814eb038e52603fn,
  0x9dafbacc58e2d651n, 0xfe2f458e4be170afn, 0x4457ec414df6a940n,
  0x06e62f1451123314n, 0xbd1014d173ba92ccn, 0xdef318e25ed57760n,
  0x9fea0de9dfca8525n, 0x459de1e76c20624bn, 0xaeec189617e2d666n,
  0x126a2c06ab5a83cbn, 0xb1321532360f6132n, 0x65421503dbb40123n,
  0x2d67c287ea089ab3n, 0x6c93bff5a56bd6b6n, 0x4ffb2036cab6d98dn,
  0xce7b785b1be7ad4fn, 0xedb42ef6189fd163n, 0xdc905288703988f6n,
  0x365f9c1d2c691884n, 0xc640583680d99bfen, 0x3cd4624c07593ec6n,
  0x7f1ea8d85d7c5805n, 0x014842d480b57149n, 0x0b649bcb5a828688n,
  0xbcd5708ed79b18f0n, 0xe987c862fbd2f2f0n, 0x982731671f0cd82cn,
  0xbaf13e8b16d8c063n, 0x8ea3109cbd951bban, 0xd141045bfb385cadn,
  0x2acbc1a0af1f7d30n, 0xe6444d89df03bfdfn, 0xa18cc771b8188ff9n,
  0x9834429db01c39bbn, 0x214add07fe086a1fn, 0x8f07c19b1f6b3ff9n,
  0x56a297b1bf4ffe55n, 0x94d558e493c54fc7n, 0x40bfc24c764552cbn,
  0x931a706f8a8520cbn, 0x32229d322935bd52n, 0x2560d0f5dc4fefafn,
  0x9dbcc48355969bb6n, 0x0fd81c3985c0b56an, 0xe03817e1560f2bdan,
  0xc1bb4f81d892b2d5n, 0xb0c4864f4e28d2d7n, 0x3ecc49f9d9d6c263n,
  0x51307e99b52ba65en, 0x8af2b688da84a752n, 0xf5d72523b91b20b6n,
  0x6d95ff1ff4634806n, 0x562f21555458339an, 0xc0ce47f889336346n,
  0x487823e5089b40d8n, 0xe4727c7ebc6d9592n, 0x5a8f7277e94970ban,
  0xfca2f406b1c8bb50n, 0x5b1f8a95f1791070n, 0xd304af9fc9028605n,
  0x5440ab7fc930e748n, 0x312d25fbca2ab5a1n, 0x10f4a4b234a4d575n,
  0x90301d55047e7473n, 0x3b6372886c61591en, 0x293402b77c444e06n,
  0x451f34a4d3e97dd7n, 0x3158d814d81bc57bn, 0x034942425b9bda69n,
  0xe2032ff9e532d9bbn, 0x62ae066b8b2179e5n, 0x9545e10c2f8d71d8n,
  0x7ff7483eb2d23fc0n, 0x00945fcebdc98d86n, 0x8764bbbe99b26ca2n,
  0x1b1ec62284c0bfc3n, 0x58e0fcc4f0aa362bn, 0x5f4abefa878d458dn,
  0xfd74ac2f9607c519n, 0xa4e3fb37df8cbfa9n, 0xbf697e43cac574e5n,
  0x86f14a3f68f4cd53n, 0x24a23d076f1ce522n, 0xe725cd8048868cc8n,
  0xbf3c729eb2464362n, 0xd8f6cd57b3cc1ed8n, 0x6329e52425541577n,
  0x62aa688ad5ae1ac0n, 0x0a242566269bf845n, 0x168b1a4753aca74bn,
  0xf789afefff2e7e3cn, 0x6c3362093b6fccdbn, 0x4ce8f50bd28c09b2n,
  0x006a2db95ae8aa93n, 0x975b0d623c3d1a8cn, 0x18605d3935338c5bn,
  0x5bb6f6136cad3c71n, 0x0f53a20701f8d8a6n, 0xab8c5ad2e7e93c67n,
  0x40b5ac5127acaa29n, 0x8c7bf63c2075895fn, 0x78bd9f7e014a805cn,
  0xb2c9e9f4f9c8c032n, 0xefd6049827eb91f3n, 0x2be459f482c16fbdn,
  0xd92ce0c5745aaa8cn, 0x0aaa8fb298d965b9n, 0x2b37f92c6c803b15n,
  0x8c54a5e94e0f0e78n, 0x95f9b6e90c0a3032n, 0xe7939faa436c7874n,
  0xd16bfe8f6a8a40c9n, 0x44982b86263fd2fan, 0xe285fb39f984e583n,
  0x779a8df72d7619d3n, 0xf2d79a8de8d5dd1en, 0xd1037354d66684e2n,
  0x004c82a4e668a8e5n, 0x31d40a7668b044e6n, 0xd70578538bd02c11n,
  0xdb45431078c5f482n, 0x977121bb7f6a51adn, 0x73d5ccbd34eff8ddn,
  0xe437a07d356e17cdn, 0x47b2782043c95627n, 0x9fb251413e41d49an,
  0xccd70b60652513d3n, 0x1c95b31e8a1b49b2n, 0xcae73dfd1bcb4c1bn,
  0x34d98331b1f5b70fn, 0x784e39f22338d92fn, 0x18613d4a064df420n,
  0xf1d8dae25f0bceben, 0x33f77c15ae855efcn, 0x3c88b3b912eb109cn,
  0x956a2ec96bafeea5n, 0x1aa005b5e0ad0e87n, 0x5500d70527c4bb8en,
  0xe36c57196421cc44n, 0x13c4d286cc36ee39n, 0x5654a23d818b2a81n,
  0x77b1dc13d161abdcn, 0x734f44de5f8d5eb5n, 0x60717e174a6c89a2n,
  0xd47d9649266a211en, 0x5b13a4322bb69e90n, 0xf7669609f8b5fc3cn,
  0x21e6ac55bedcdac9n, 0x9b56b62b61166dean, 0xf48f66b939797e9cn,
  0x35f332f9c0e6ae9an, 0xcc733f6a9a878db0n, 0x3da161e41cc108c2n,
  0xb7d74ae535914d51n, 0x4d493b0b11d36469n, 0xce264d1dfba9741an,
  0xa9d1f2dc7436dc06n, 0x70738016604c2a27n, 0x231d36e96e93f3d5n,
  0x7666881197838d19n, 0x4a2a83090aaad40cn, 0xf1e761591668b35dn,
  0x7363236497f730a7n, 0x301080e37379dd4dn, 0x502dea2971827042n,
  0xc2c5eb858f32625fn, 0x786afb9edfafbdffn, 0xdaee0d868490b2a4n,
  0x617366b3268609f6n, 0xae0e35a0fe46173en, 0xd1a07de93e824f11n,
  0x079b8b115ea4cca8n, 0x93a99274558faebbn, 0xfb1e6e22e08a03b3n,
  0xea635fdba3698dd0n, 0xcf53659328503a5cn, 0xcde3b31e6fd5d780n,
  0x8e3e4221d3614413n, 0xef14d0d86bf1a22cn, 0xe1d830d3f16c5ddbn,
  0xaabd2b2a451504e1n,
];

/** fastcdc v2020 `GEAR_LS`: each gear entry shifted left one bit, mod 2^64. */
export const FASTCDC_2020_GEAR_LS: readonly bigint[] = FASTCDC_2020_GEAR.map((g) => (g << 1n) & M64);

/** fastcdc v2020 `MASKS`: indexed by `round(log2(avg)) ± level`. */
export const FASTCDC_2020_MASKS: readonly bigint[] = [
  0x0000000000000000n, 0x0000000000000000n, 0x0000000000000000n,
  0x0000000000000000n, 0x0000000000000000n, 0x0000000001804110n,
  0x0000000001803110n, 0x0000000018035100n, 0x0000001800035300n,
  0x0000019000353000n, 0x0000590003530000n, 0x0000d90003530000n,
  0x0000d90103530000n, 0x0000d90303530000n, 0x0000d90313530000n,
  0x0000d90f03530000n, 0x0000d90303537000n, 0x0000d90703537000n,
  0x0000d90707537000n, 0x0000d91707537000n, 0x0000d91747537000n,
  0x0000d91767537000n, 0x0000d93767537000n, 0x0000d93777537000n,
  0x0000d93777577000n, 0x0000db3777577000n,
];

/** BLAKE3-256 hex of `GEAR ‖ GEAR_LS ‖ MASKS`, each entry as 8 little-endian bytes. */
export function fastcdc2020TableDigest(): string {
  const tables = [...FASTCDC_2020_GEAR, ...FASTCDC_2020_GEAR_LS, ...FASTCDC_2020_MASKS];
  const bytes = new Uint8Array(tables.length * 8);
  const view = new DataView(bytes.buffer);
  tables.forEach((value, i) => view.setBigUint64(i * 8, value, true));
  return Array.from(blake3(bytes), (b) => b.toString(16).padStart(2, '0')).join('');
}

export class ChunkerError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ChunkerError';
  }
}

/** The specification's admitted parameter ranges; a wire-read profile is refused outside them. */
export function validateFastcdc2020(params: Fastcdc2020): void {
  const { min, avg, max, level } = params;
  const even = [min, avg, max].every((v) => Number.isInteger(v) && v % 2 === 0);
  if (
    params.id !== CHUNKER_FASTCDC_2020 ||
    !even ||
    min < 64 ||
    min > 1_048_576 ||
    avg < 256 ||
    avg > 4_194_304 ||
    max < 1024 ||
    max > 16_777_216 ||
    !(min <= avg && avg <= max) ||
    !Number.isInteger(level) ||
    level < 0 ||
    level > 3
  ) {
    throw new ChunkerError(`chunker parameters out of range: ${JSON.stringify(params)}`);
  }
}

/** Chunk `bytes` under `params`: `[offset, length]` pairs in order, covering every byte once. */
export function chunkBytes(bytes: Uint8Array, params: Fastcdc2020 = FASTCDC_2020_EPOCH_0): Array<[number, number]> {
  validateFastcdc2020(params);
  const { min, avg, max, level } = params;
  // round half away from zero, as Rust's f64::round
  const bits = Math.round(Math.log2(avg));
  const maskS = FASTCDC_2020_MASKS[bits + level];
  const maskL = FASTCDC_2020_MASKS[bits - level];
  const maskSLs = (maskS << 1n) & M64;
  const maskLLs = (maskL << 1n) & M64;
  const gear = FASTCDC_2020_GEAR;
  const gearLs = FASTCDC_2020_GEAR_LS;
  const out: Array<[number, number]> = [];
  let start = 0;
  while (start < bytes.length) {
    const n = bytes.length - start;
    let len: number;
    if (n <= min) {
      len = n;
    } else {
      const limit = Math.min(n, max);
      const center = limit >= avg ? avg : limit;
      let hash = 0n;
      let cut = -1;
      for (let i = min >> 1; i < limit >> 1; i++) {
        const small = i < center >> 1;
        const a = 2 * i;
        hash = ((hash << 2n) + gearLs[bytes[start + a]]) & M64;
        if ((hash & (small ? maskSLs : maskLLs)) === 0n) {
          cut = a;
          break;
        }
        hash = (hash + gear[bytes[start + a + 1]]) & M64;
        if ((hash & (small ? maskS : maskL)) === 0n) {
          cut = a + 1;
          break;
        }
      }
      len = cut < 0 ? limit : cut;
    }
    out.push([start, len]);
    start += len;
  }
  return out;
}
