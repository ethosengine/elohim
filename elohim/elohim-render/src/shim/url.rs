// Extension that injects the WHATWG URL shim into the V8 isolate.
// deno_core's bare runtime (no snapshot) does not include URL as a global;
// url.js provides a minimal pure-JS implementation.
deno_core::extension!(
    url_ext,
    js = [dir "src/shim", "url.js"],
);
