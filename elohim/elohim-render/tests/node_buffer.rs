//! Behavior tests for the Node `Buffer` shim (global + `buffer` module surface).
//!
//! These drive real V8 through the same `with_shims` runtime the production path
//! uses, so they exercise the injected global `Buffer` (a real `Uint8Array`
//! subclass) and its op-backed base64/hex codecs end-to-end, plus the
//! `import { Buffer } from "buffer"` module re-export.

use elohim_render::runtime::JsRuntime;

/// The global `Buffer` is a real `Uint8Array` subclass: instances are
/// `instanceof Uint8Array` AND `instanceof Buffer`, and `Buffer.isBuffer`
/// agrees. Subclass identity is what makes `x instanceof Buffer` guards in the
/// bundle work.
#[tokio::test]
async fn buffer_is_a_real_uint8array_subclass() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(() => {
                const b = Buffer.alloc(4);
                return JSON.stringify({
                    isU8: b instanceof Uint8Array,
                    isBuf: b instanceof Buffer,
                    isBufferStatic: Buffer.isBuffer(b),
                    notBufferPlain: Buffer.isBuffer(new Uint8Array(4)),
                    length: b.length,
                    zeroFilled: Array.from(b).every((x) => x === 0),
                });
            })()"#,
        )
        .await
        .expect("subclass script runs");
    assert!(
        out.contains("\"isU8\":true"),
        "instanceof Uint8Array: {out}"
    );
    assert!(out.contains("\"isBuf\":true"), "instanceof Buffer: {out}");
    assert!(
        out.contains("\"isBufferStatic\":true"),
        "isBuffer(buffer) true: {out}"
    );
    assert!(
        out.contains("\"notBufferPlain\":false"),
        "isBuffer(plain Uint8Array) false: {out}"
    );
    assert!(out.contains("\"length\":4"), "alloc length: {out}");
    assert!(
        out.contains("\"zeroFilled\":true"),
        "alloc zero-fills: {out}"
    );
}

/// utf8 round-trips exactly, including multi-byte code points (proves the
/// TextEncoder/TextDecoder op path, not a byte-per-char shortcut).
#[tokio::test]
async fn utf8_round_trips_including_multibyte() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(() => {
                const s = "hi éè 世界 🚀";
                const b = Buffer.from(s, "utf8");
                const back = b.toString("utf8");
                return JSON.stringify({
                    roundTrips: back === s,
                    byteLen: b.length,
                    byteLenStatic: Buffer.byteLength(s, "utf8"),
                    defaultsUtf8: Buffer.from(s).toString() === s,
                });
            })()"#,
        )
        .await
        .expect("utf8 script runs");
    assert!(
        out.contains("\"roundTrips\":true"),
        "utf8 round-trips: {out}"
    );
    assert!(
        out.contains("\"defaultsUtf8\":true"),
        "from/toString default to utf8: {out}"
    );
    // "hi ee 世界 🚀": ascii(3) + 2*2 + 1 + 3+3 + 1 + 4 = 3+4+1+6+1+4 = 19 bytes.
    assert!(out.contains("\"byteLen\":19"), "utf8 byte length: {out}");
    assert!(
        out.contains("\"byteLenStatic\":19"),
        "Buffer.byteLength matches: {out}"
    );
}

/// hex encodes lowercase and round-trips exactly.
#[tokio::test]
async fn hex_round_trips_lowercase() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(() => {
                const b = Buffer.from([0x00, 0x0f, 0xa5, 0xde, 0xad, 0xbe, 0xef]);
                const hex = b.toString("hex");
                const back = Buffer.from(hex, "hex");
                return JSON.stringify({
                    hex,
                    roundTrips: back.equals(b),
                });
            })()"#,
        )
        .await
        .expect("hex script runs");
    assert!(
        out.contains("\"hex\":\"000fa5deadbeef\""),
        "hex is lowercase exact: {out}"
    );
    assert!(
        out.contains("\"roundTrips\":true"),
        "hex round-trips: {out}"
    );
}

/// base64 and base64url exact vectors + round-trip; base64url has no padding and
/// uses the URL-safe alphabet.
#[tokio::test]
async fn base64_and_base64url_exact_and_round_trip() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(() => {
                const b = Buffer.from("foobar", "utf8");
                const std = b.toString("base64");
                const url = Buffer.from([0xfb, 0xff, 0xbf]).toString("base64url");
                const stdBack = Buffer.from(std, "base64").toString("utf8");
                return JSON.stringify({
                    std,
                    stdRoundTrips: stdBack === "foobar",
                    urlNoPad: url.indexOf("=") === -1,
                    urlSafe: url.indexOf("+") === -1 && url.indexOf("/") === -1,
                });
            })()"#,
        )
        .await
        .expect("base64 script runs");
    assert!(
        out.contains("\"std\":\"Zm9vYmFy\""),
        "base64('foobar') exact: {out}"
    );
    assert!(
        out.contains("\"stdRoundTrips\":true"),
        "base64 round-trips: {out}"
    );
    assert!(
        out.contains("\"urlNoPad\":true"),
        "base64url has no padding: {out}"
    );
    assert!(
        out.contains("\"urlSafe\":true"),
        "base64url avoids +/: {out}"
    );
}

/// latin1/binary and ascii encodings behave: latin1 is a 1:1 byte<->char map;
/// ascii masks the high bit on decode.
#[tokio::test]
async fn latin1_and_ascii_encodings() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(() => {
                const b = Buffer.from([0x41, 0xe9, 0xff]); // 'A', latin1 e-acute, 0xff
                return JSON.stringify({
                    latin1: b.toString("latin1"),
                    binarySame: b.toString("binary") === b.toString("latin1"),
                    // ascii unsets the high bit: 0xe9&0x7f=0x69='i', 0xff&0x7f=0x7f
                    ascii: b.toString("ascii"),
                    fromLatin1: Buffer.from("Aéÿ", "latin1").length,
                });
            })()"#,
        )
        .await
        .expect("latin1/ascii script runs");
    assert!(
        out.contains("\"binarySame\":true"),
        "binary aliases latin1: {out}"
    );
    // latin1 of [0x41,0xe9,0xff] = "Aéÿ" (0x41='A', 0xe9=U+00E9, 0xff=U+00FF).
    // JSON.stringify emits printable non-ASCII literally (no \u escaping).
    assert!(
        out.contains("\"latin1\":\"Aéÿ\""),
        "latin1 is 1:1 byte->char: {out}"
    );
    assert!(
        out.contains("\"fromLatin1\":3"),
        "from(latin1) is one byte per char: {out}"
    );
}

/// concat sums the inputs and respects an explicit totalLength (truncating).
#[tokio::test]
async fn concat_joins_and_respects_total_length() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(() => {
                const a = Buffer.from([1, 2, 3]);
                const b = Buffer.from([4, 5]);
                const joined = Buffer.concat([a, b]);
                const capped = Buffer.concat([a, b], 4);
                return JSON.stringify({
                    joined: Array.from(joined),
                    isBuffer: Buffer.isBuffer(joined),
                    capped: Array.from(capped),
                    emptyLen: Buffer.concat([]).length,
                });
            })()"#,
        )
        .await
        .expect("concat script runs");
    assert!(
        out.contains("\"joined\":[1,2,3,4,5]"),
        "concat joins in order: {out}"
    );
    assert!(
        out.contains("\"isBuffer\":true"),
        "concat returns a Buffer: {out}"
    );
    assert!(
        out.contains("\"capped\":[1,2,3,4]"),
        "concat honors totalLength: {out}"
    );
    assert!(out.contains("\"emptyLen\":0"), "concat([]) is empty: {out}");
}

/// slice/subarray SHARE memory (Node semantics), NOT copy like Uint8Array.slice:
/// a write through the slice mutates the parent.
#[tokio::test]
async fn slice_and_subarray_share_memory() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(() => {
                const parent = Buffer.from([1, 2, 3, 4]);
                const sl = parent.slice(1, 3); // [2,3], shared
                sl[0] = 99;
                const sub = parent.subarray(1, 3);
                return JSON.stringify({
                    parentMutated: parent[1] === 99,
                    sliceIsBuffer: Buffer.isBuffer(sl),
                    subIsBuffer: Buffer.isBuffer(sub),
                    sliceLen: sl.length,
                });
            })()"#,
        )
        .await
        .expect("slice script runs");
    assert!(
        out.contains("\"parentMutated\":true"),
        "slice shares memory with parent: {out}"
    );
    assert!(
        out.contains("\"sliceIsBuffer\":true"),
        "slice returns a Buffer (species): {out}"
    );
    assert!(
        out.contains("\"subIsBuffer\":true"),
        "subarray returns a Buffer (species): {out}"
    );
}

/// `Buffer.from(arrayBuffer)` shares the underlying ArrayBuffer (zero-copy),
/// while `Buffer.from(uint8array)` copies the bytes.
#[tokio::test]
async fn from_arraybuffer_shares_from_typedarray_copies() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(() => {
                const ab = new ArrayBuffer(4);
                const shared = Buffer.from(ab);
                new Uint8Array(ab)[0] = 7; // mutate through the ArrayBuffer
                const u8 = Uint8Array.of(1, 2, 3);
                const copied = Buffer.from(u8);
                u8[0] = 42; // must NOT affect the copy
                return JSON.stringify({
                    sharesAb: shared[0] === 7,
                    copiedIndependent: copied[0] === 1,
                });
            })()"#,
        )
        .await
        .expect("from script runs");
    assert!(
        out.contains("\"sharesAb\":true"),
        "from(ArrayBuffer) shares memory: {out}"
    );
    assert!(
        out.contains("\"copiedIndependent\":true"),
        "from(Uint8Array) copies bytes: {out}"
    );
}

/// copy and fill behave: copy returns the number of bytes written; fill supports
/// numeric and string fills.
#[tokio::test]
async fn copy_and_fill_behave() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(() => {
                const src = Buffer.from([1, 2, 3, 4]);
                const dst = Buffer.alloc(4);
                const n = src.copy(dst, 1, 0, 2); // copy [1,2] to offset 1
                const numFill = Buffer.alloc(3).fill(7);
                const strFill = Buffer.alloc(4).fill("ab"); // repeats pattern
                return JSON.stringify({
                    n,
                    dst: Array.from(dst),
                    numFill: Array.from(numFill),
                    strFill: strFill.toString("latin1"),
                });
            })()"#,
        )
        .await
        .expect("copy/fill script runs");
    assert!(out.contains("\"n\":2"), "copy returns byte count: {out}");
    assert!(
        out.contains("\"dst\":[0,1,2,0]"),
        "copy writes at target offset: {out}"
    );
    assert!(out.contains("\"numFill\":[7,7,7]"), "numeric fill: {out}");
    assert!(
        out.contains("\"strFill\":\"abab\""),
        "string fill repeats the pattern: {out}"
    );
}

/// `import { Buffer } from "buffer"` resolves to the SAME constructor as the
/// global `Buffer` -- the module surface and the global share one class, so
/// cross-surface `instanceof` is coherent.
#[tokio::test]
async fn buffer_module_import_is_the_same_class_as_global() {
    let mut rt = JsRuntime::with_shims();
    let html = rt
        .render_via_module(&fixture("buffer-consumer.mjs"), "/x")
        .await
        .expect("buffer-consumer renders");
    assert!(
        html.contains("same-class:true"),
        "module Buffer === global Buffer: {html}"
    );
    assert!(
        html.contains("cross-instanceof:true"),
        "a global-made Buffer is instanceof the module Buffer: {html}"
    );
    assert!(
        html.contains("hello-b64:aGVsbG8="),
        "module Buffer round-trips base64: {html}"
    );
}

/// An off-boot-path binary read/write member (`writeUInt16BE`) is a LOUD, NAMED
/// stub on `Buffer.prototype`: calling it throws an error that names the member
/// and this shim file -- never a silent-wrong byte write.
#[tokio::test]
async fn unimplemented_integer_member_throws_named_error() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(() => {
                const b = Buffer.alloc(2);
                // Feature-detection sees it as present (Node buffers have it)...
                const present = typeof b.writeUInt16BE === "function";
                let threw = "";
                try {
                    b.writeUInt16BE(1, 0);
                } catch (e) {
                    threw = e.message;
                }
                return JSON.stringify({ present, threw });
            })()"#,
        )
        .await
        .expect("loud member script runs");
    assert!(
        out.contains("\"present\":true"),
        "the member is present on the prototype: {out}"
    );
    assert!(
        out.contains("writeUInt16BE"),
        "the error names the member: {out}"
    );
    assert!(
        out.contains("not implemented"),
        "the error is a clear not-implemented message: {out}"
    );
    assert!(
        out.contains("node_buffer.rs"),
        "the error points at the shim file: {out}"
    );
}

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}
