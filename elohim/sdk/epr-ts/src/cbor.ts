import * as dagCbor from '@ipld/dag-cbor';

/** Encode any JSON-compatible value to canonical dag-cbor bytes. */
export function encodeCanonical(value: unknown): Uint8Array {
  return dagCbor.encode(value);
}

/** Decode canonical dag-cbor bytes to a JS value. */
export function decodeCanonical<T = unknown>(bytes: Uint8Array): T {
  return dagCbor.decode(bytes) as T;
}
