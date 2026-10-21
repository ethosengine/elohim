import * as dagCbor from '@ipld/dag-cbor';

/** Encode any JSON-compatible value to canonical dag-cbor bytes. */
export function encodeCanonical(value: unknown): Uint8Array {
  return dagCbor.encode(value);
}

/** Decode canonical dag-cbor bytes to a JS value. */
export function decodeCanonical<T = unknown>(bytes: Uint8Array): T {
  return dagCbor.decode(bytes) as T;
}

/**
 * The byte length of the one complete CBOR data item that starts at `offset`, or `null` when the
 * bytes end before it does. A reader of a CBOR sequence (RFC 8742, e.g. the eprfs batch route's
 * `application/cbor-seq` answer) calls this to find each item's boundary and then decodes the
 * item with `decodeCanonical`, so a streamed answer is decoded item by item as it arrives.
 *
 * Throws on what DAG-CBOR forbids or cannot frame: indefinite lengths, reserved additional-info
 * values, and a byte length or element count too large to address. It does not validate the
 * item; `decodeCanonical` does that.
 *
 * Only for majors 2-5 is the head's argument a length or a count. For majors 0 and 1 it is the
 * integer's value, for major 6 the tag number, and for major 7 the simple value or the float's
 * bits; the framer skips those bytes and never reads them as a number, so a float64 (the only
 * float width DAG-CBOR writes) or an integer at or above 2^53 frames like any other item.
 */
export function cborItemLength(bytes: Uint8Array, offset = 0): number | null {
  return new CborItemFramer().frame(offset === 0 ? bytes : bytes.subarray(offset));
}

/**
 * `cborItemLength`, resumable across a growing buffer. A streaming reader holds one framer per
 * item and calls `frame` with the item's bytes so far, each call's bytes extending the previous
 * call's (the same prefix, possibly more after it). The framer keeps its place after the last
 * complete head, so it visits each head once however the item arrives: an item of `n` heads
 * delivered in `k` chunks costs `n + k` head visits, not `n * k`. Call `reset` before the next
 * item. A framer that has thrown stays thrown: reset it before reuse.
 */
export class CborItemFramer {
  /** Offset just past the last complete head (and string body), from the item's start. */
  private at = 0;
  /** Items still owed: the item itself, then the elements and tag contents its heads opened. */
  private pending = 1;
  /** Heads read so far, counting a head re-read after it arrived only in part. For tests. */
  visits = 0;

  reset(): void {
    this.at = 0;
    this.pending = 1;
    this.visits = 0;
  }

  /** The item's length once it is complete in `bytes`, else `null` (call again with more). */
  frame(bytes: Uint8Array): number | null {
    let at = this.at;
    let pending = this.pending;
    while (pending > 0) {
      if (at >= bytes.length) break;
      this.visits += 1;
      const initial = bytes[at];
      const major = initial >> 5;
      const info = initial & 0x1f;
      let width: number;
      if (info < 24) {
        width = 0;
      } else if (info <= 27) {
        width = 1 << (info - 24);
        if (at + 1 + width > bytes.length) break;
      } else {
        throw new Error(`cbor: additional info ${info} is not admitted in DAG-CBOR`);
      }
      if (major < 2 || major > 5) {
        // 0, 1: integers; 6: a tag, followed by its one item; 7: simple values and floats. The
        // argument is a value, not a length: skip its bytes.
        at += 1 + width;
        pending += major === 6 ? 0 : -1;
        continue;
      }
      let argument = info < 24 ? info : 0;
      for (let i = 0; i < width; i++) argument = argument * 256 + bytes[at + 1 + i];
      if (!Number.isSafeInteger(argument)) throw new Error('cbor: length too large');
      switch (major) {
        case 2:
        case 3:
          // A string's head and body are one step: stop before the head until the body is in.
          if (at + 1 + width + argument > bytes.length) {
            this.at = at;
            this.pending = pending;
            return null;
          }
          at += 1 + width + argument;
          pending -= 1;
          break;
        case 4:
          at += 1 + width;
          pending += argument - 1;
          break;
        default: // 5
          at += 1 + width;
          pending += 2 * argument - 1;
          break;
      }
    }
    this.at = at;
    this.pending = pending;
    return pending === 0 ? at : null;
  }
}

/** Decode a whole CBOR sequence (RFC 8742) of canonical DAG-CBOR items. Throws on a truncated tail. */
export function decodeCanonicalSequence<T = unknown>(bytes: Uint8Array): T[] {
  const out: T[] = [];
  let at = 0;
  while (at < bytes.length) {
    const length = cborItemLength(bytes, at);
    if (length === null) throw new Error('cbor: sequence ends inside an item');
    out.push(decodeCanonical<T>(bytes.subarray(at, at + length)));
    at += length;
  }
  return out;
}
