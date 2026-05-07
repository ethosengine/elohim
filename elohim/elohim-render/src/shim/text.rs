// Extension that injects TextEncoder/TextDecoder into the V8 isolate.
// Built on top of deno_core's op_encode / op_decode builtins (already
// registered by the runtime core). Not available as globals in a bare
// deno_core runtime without a snapshot.
deno_core::extension!(
    text_ext,
    js = [dir "src/shim", "text.js"],
);
