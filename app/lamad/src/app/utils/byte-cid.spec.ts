import { byteCidDigest, byteCidOfBytes, isByteCid } from "./byte-cid";

// Minted at authoring time by an independent implementation (multiformats + @noble/hashes blake3,
// CIDv1 raw 0x55 + BLAKE3-256 0x1e).
const ONE_TWO_THREE_FOUR =
  "bafkr4iddpaorofbfunrrf6qfrwdrfvoqke22tepmea2rz2owltnrticugi";
const ABC = "bafkr4ideg6z2yocgkez77nr3outtvdnvjdcvqrs5phnqh7jvtrwnlpm5qu";

describe("byte-cid (lamad byte-profile admission)", () => {
  it("mints the independent vectors from the real bytes", () => {
    expect(byteCidOfBytes(Uint8Array.from([1, 2, 3, 4]))).toBe(
      ONE_TWO_THREE_FOUR,
    );
    expect(
      byteCidOfBytes(new Uint8Array(new TextEncoder().encode("abc"))),
    ).toBe(ABC);
  });

  it("admits a canonical byte CID and exposes its 32-byte digest", () => {
    expect(isByteCid(ABC)).toBe(true);
    expect(ABC).toHaveLength(59);
    expect(byteCidDigest(ABC)).toHaveLength(32);
  });

  it("refuses everything that is not exactly the byte profile (full decode)", () => {
    const sha =
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    const refused = [
      "",
      null,
      undefined,
      `sha256-${sha}`,
      `sha256:${sha}`,
      `blake3-${sha}`,
      sha,
      ABC.toUpperCase(),
      ABC.slice(0, -1),
      `${ABC}a`,
      "bafkr4i", // right prefix, nothing behind it
      `${ABC.slice(0, -1)}v`, // non-zero padding bit: not the canonical text of its own bytes
      `bafkr4i${"b".repeat(52)}`, // right prefix and length, digest bits spill into the padding
      "bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku", // sha2-256 citation
      "bafyreigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi", // dag-cbor record
      "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG", // CIDv0
    ];
    for (const address of refused) expect(isByteCid(address)).toBe(false);
  });
});
