//! Synthetic, link-safe shims for the Node builtins the Angular server bundle
//! imports (everything except `crypto`, which has its own richer shim in
//! [`crate::shim::node_crypto`]).
//!
//! # The link-time trap this solves
//!
//! ESM linking is static and happens BEFORE any module body evaluates. A NAMED
//! import binds to a specific exported name:
//!
//! ```js
//! import { createRequire } from "node:module";
//! ```
//!
//! If the resolved module does not EXPORT `createRequire`, linking fails with a
//! `SyntaxError: ... does not provide an export named 'createRequire'` — and it
//! fails at LINK time, so a module body that merely `throw`s can never even run
//! to name the problem. The first-cut shim (a synthetic module whose whole body
//! was `throw new Error("builtin X not implemented")`) was therefore NOT
//! link-safe: it worked for `import x from "fs"` (default binding) but a
//! `import { createRequire } from "node:module"` died at link before the throw.
//!
//! # The contract
//!
//! Each synthetic builtin module here is **link-safe**: it carries every named
//! export the bundle statically links against (collected by scanning the real
//! deployed bundle — see `tests/node_builtin_shim.rs` for the fetch recipe).
//! Each such export is, by default, a LOUD stub: calling it throws an error that
//! names the builtin and the member, so an un-proven render path fails clearly
//! ("elohim-render net shim: createServer() is not implemented") instead of a
//! cryptic link error or "undefined is not a function".
//!
//! A small set of members are implemented for REAL because the render boot path
//! actually invokes them (proven by running the deployed bundle locally — the
//! ignored harness in `tests/node_builtin_shim.rs`):
//!
//! - `module.createRequire` — the Angular polyfill calls it at eval time
//!   (`globalThis['require'] ??= createRequire(import.meta.url)`), so it must
//!   return a real `require` function. That `require` is NOT fully loud: it
//!   resolves Node-builtin ids to synchronous CJS exports objects (see the CJS
//!   interop section below). Non-builtin ids stay loud.
//! - `util.promisify` — commonly called at module-eval time to wrap callback
//!   APIs; a correct minimal implementation avoids an eval-time throw.
//! - `events.setMaxListeners` — a listener-cap raise; a no-op is semantically
//!   safe for a single-shot SSR render.
//! - `buffer.Buffer` — re-exports the REAL global `Buffer` (a `Uint8Array`
//!   subclass) installed at init by `node_buffer_ext` (see
//!   [`crate::shim::node_buffer`]), so the module and global surfaces share one
//!   constructor.
//!
//! Everything else stays a loud stub. Extend deliberately, one member at a time,
//! as a render path proves it needs the member — never speculatively fabricate a
//! return value (that trades a loud, precise failure for a silent-wrong render).
//!
//! # CJS `require(builtin)` interop — one source of truth with the ESM surface
//!
//! The deployed bundle bundles `ws` (CommonJS). esbuild's CJS-in-ESM shim routes
//! its `require(...)` calls to `globalThis.require`, which the Angular polyfill
//! sets to `createRequire(import.meta.url)`. So `ws` reaches for a SYNCHRONOUS
//! `require("events")` / `require("stream")` / ... returning a CJS exports object
//! at module-init time. A fully-loud `require` walls the boot at the first such
//! call.
//!
//! The `require` this shim's `createRequire` returns therefore resolves any id
//! that names a builtin on [`BUILTIN_SURFACE`] (stripping an optional `node:`
//! prefix) to a CJS exports object built from the SAME table + [`real_member`]
//! that back the ESM synthetic modules — the member lists are NOT forked (a test
//! asserts CJS and ESM expose identical member names per builtin). A CJS exports
//! object is normally a plain namespace (`{ createServer, isIP, ... }`); a builtin
//! whose Node `module.exports` is a callable/constructible primary (only `events`:
//! `require("events") === EventEmitter`, so `class X extends require("events")`
//! works) is marked with [`BuiltinSurface::cjs_primary`], and its exports object
//! IS that member with the siblings hung off it. `crypto` is deliberately absent
//! from the table (it has a richer JS shim, [`crate::shim::node_crypto`]); its CJS
//! form is the SAME `cryptoShim` object, which that shim registers on
//! `globalThis.__elohimBuiltinCjs` at eval so `require("crypto")` returns it
//! without forking the crypto member list. Non-builtin ids (npm native addons like
//! `bufferutil`) stay loud — `ws` try/catches those and falls back to JS.
//!
//! # WARNING: pure ASCII
//!
//! The generated source is served as module source. Keep every string literal
//! here ASCII (the rest of `src/shim` is ASCII for the same reason).

/// A Node builtin the bundle imports, and the named exports it links against.
///
/// `crypto` is intentionally absent — it has a dedicated, richer shim.
struct BuiltinSurface {
    /// Bare builtin name (no `node:` prefix), e.g. `"net"`.
    name: &'static str,
    /// Named exports the bundle statically imports from this builtin. Every one
    /// must exist as a named export for `import { X } from "..."` to link.
    named: &'static [&'static str],
    /// For CJS `require(builtin)` only: the member (if any) that IS the Node
    /// `module.exports` — a callable/constructible value with the other members
    /// hung off it as properties, rather than a plain namespace object. This
    /// mirrors Node's own shape for builtins like `events`
    /// (`require("events") === EventEmitter`, so `class X extends
    /// require("events")` and `require("events").EventEmitter` both work). `None`
    /// (the common case) means the CJS exports is a plain `{ ...members }` object.
    /// Irrelevant to the ESM surface, which always exports named + a default
    /// Proxy.
    cjs_primary: Option<&'static str>,
}

/// The static import surface of the deployed Angular server bundle, minus
/// `crypto`. Collected by scanning every chunk for `import ... from "<builtin>"`
/// and `import ... from "node:<builtin>"` (see the ignored test's doc comment for
/// the exact recipe). A NEW named import that appears in a future bundle and is
/// not listed here will fail at link with a clear deno error naming the missing
/// export — the honest signal to add it.
const BUILTIN_SURFACE: &[BuiltinSurface] = &[
    BuiltinSurface {
        // `Buffer` is NOT a loud stub: it re-exports the REAL global `Buffer` (a
        // Uint8Array subclass) installed at init by `node_buffer_ext` (see
        // [`crate::shim::node_buffer`] and `real_member`), so `import { Buffer }
        // from "buffer"` and the global `Buffer` are the same constructor.
        name: "buffer",
        named: &["Buffer"],
        cjs_primary: None,
    },
    BuiltinSurface {
        name: "cluster",
        named: &[],
        cjs_primary: None,
    },
    BuiltinSurface {
        name: "dgram",
        named: &["createSocket"],
        cjs_primary: None,
    },
    BuiltinSurface {
        name: "events",
        named: &["EventEmitter", "on", "setMaxListeners"],
        // Node: `require("events") === EventEmitter`. `ws` does
        // `class WebSocket extends require("events")`, so the CJS exports must be
        // the constructible EventEmitter, not a plain namespace.
        cjs_primary: Some("EventEmitter"),
    },
    BuiltinSurface {
        name: "fs",
        named: &[],
        cjs_primary: None,
    },
    BuiltinSurface {
        name: "http",
        named: &["Agent"],
        cjs_primary: None,
    },
    BuiltinSurface {
        name: "https",
        named: &[],
        cjs_primary: None,
    },
    BuiltinSurface {
        name: "module",
        named: &["createRequire"],
        cjs_primary: None,
    },
    BuiltinSurface {
        name: "net",
        named: &["createServer", "isIP", "isIPv4", "isIPv6"],
        cjs_primary: None,
    },
    BuiltinSurface {
        name: "os",
        named: &["networkInterfaces"],
        cjs_primary: None,
    },
    BuiltinSurface {
        name: "path",
        named: &[],
        cjs_primary: None,
    },
    BuiltinSurface {
        name: "process",
        named: &[],
        cjs_primary: None,
    },
    BuiltinSurface {
        // The Node stream base classes the bundle EXTENDS at module-eval time
        // (`class Receiver extends require("stream").Writable`, etc. in `ws`,
        // multiformats, libp2p). Like `events.EventEmitter`, a loud-stub function
        // is a legal `extends` target -- the class DEFINITION evaluates fine and
        // only `new`-ing it (connection time, never during a single-shot SSR
        // render) throws. Verified: the bundle has NO eval-time `new Writable()` /
        // `new Readable()` / etc., so loud stubs are safe here. Stream helper
        // functions (`pipeline`/`finished`/...) are deliberately NOT listed: they
        // are require-destructured (CJS, no link requirement) and read as
        // `undefined` unless called -- honest absence over a speculative stub.
        name: "stream",
        named: &["Duplex", "Readable", "Writable", "Transform", "PassThrough"],
        cjs_primary: None,
    },
    BuiltinSurface {
        name: "tls",
        named: &["TLSSocket", "connect"],
        cjs_primary: None,
    },
    BuiltinSurface {
        name: "tty",
        named: &[],
        cjs_primary: None,
    },
    BuiltinSurface {
        name: "url",
        named: &[],
        cjs_primary: None,
    },
    BuiltinSurface {
        name: "util",
        named: &["promisify"],
        cjs_primary: None,
    },
    BuiltinSurface {
        name: "vm",
        named: &[],
        cjs_primary: None,
    },
    BuiltinSurface {
        name: "worker_threads",
        named: &[],
        cjs_primary: None,
    },
    BuiltinSurface {
        name: "zlib",
        // CJS-only: `ws`'s permessage-deflate does `require("zlib")` (never an ESM
        // import) and calls `createDeflateRaw` / `createInflateRaw` at
        // connection-compression time (not boot). Loud stubs so a connection-time
        // call fails with a named error; the `Z_*` constants ws reads are absent
        // (undefined) rather than misrepresented as loud-stub functions, and are
        // not on the boot path.
        named: &["createDeflateRaw", "createInflateRaw"],
        cjs_primary: None,
    },
];

/// Look up the surface entry for a bare builtin name.
fn surface(name: &str) -> Option<&'static BuiltinSurface> {
    BUILTIN_SURFACE.iter().find(|b| b.name == name)
}

/// The REAL implementation body for `(builtin, member)`, or `None` for a loud
/// stub. Each returned string is a JS expression that evaluates to the member's
/// value (a function or class). Kept tiny and semantically-safe — never a
/// fabricated data return. `("module", "createRequire")` is NOT here — its body
/// embeds the CJS registry and is generated by [`create_require_body`] (see
/// [`member_body`]).
fn real_member(builtin: &str, member: &str) -> Option<&'static str> {
    match (builtin, member) {
        // A correct minimal promisify: wrap a Node error-first callback API in a
        // Promise. Commonly invoked at module-eval time, so a loud stub would
        // throw before the render even starts.
        ("util", "promisify") => Some(
            "function promisify(fn) {\n\
            \x20 return function promisified(...args) {\n\
            \x20   const self = this;\n\
            \x20   return new Promise(function (resolve, reject) {\n\
            \x20     try {\n\
            \x20       fn.call(self, ...args, function (err, ...vals) {\n\
            \x20         if (err) reject(err);\n\
            \x20         else resolve(vals.length > 1 ? vals : vals[0]);\n\
            \x20       });\n\
            \x20     } catch (e) {\n\
            \x20       reject(e);\n\
            \x20     }\n\
            \x20   });\n\
            \x20 };\n\
            }",
        ),
        // Raising the listener cap is a no-op for a single-shot SSR render.
        // Real (not loud) because it is frequently called at eval time.
        ("events", "setMaxListeners") => Some("function setMaxListeners() { return undefined; }"),
        // `import { Buffer } from "buffer"` (and `require("buffer").Buffer`)
        // resolve to the REAL global `Buffer` (a Uint8Array subclass) installed at
        // init by `node_buffer_ext`. One source of truth with the global, so
        // `x instanceof Buffer` is coherent across the module and global surfaces.
        // The global exists before any builtin module body evaluates (extension
        // `js` runs at isolate init), so this read is always defined.
        ("buffer", "Buffer") => Some("globalThis.Buffer"),
        _ => None,
    }
}

/// The JS expression that evaluates to the REAL value for `(builtin, member)`,
/// or `None` for a loud stub. The single resolver both the ESM synthetic modules
/// and the CJS `require` registry consult, so the two surfaces never fork: a
/// member is real in both or loud in both. `createRequire` is the one member
/// whose body is generated (it embeds the registry), so it lives here rather than
/// in the `&'static str` [`real_member`] table.
fn member_body(builtin: &str, member: &str) -> Option<String> {
    if (builtin, member) == ("module", "createRequire") {
        return Some(create_require_body());
    }
    real_member(builtin, member).map(str::to_string)
}

/// The JS expression for a member's value in a generated object: the real impl
/// wrapped in parens, else `loud_call` (a call to whichever loud-stub factory is
/// in scope — `__loud("member")` for an ESM module, `__loud("builtin","member")`
/// for the CJS registry).
fn member_value_expr(builtin: &str, member: &str, loud_call: &str) -> String {
    match member_body(builtin, member) {
        Some(body) => format!("({body})"),
        None => loud_call.to_string(),
    }
}

/// A JS string literal for `s`, double-quoted with the JS-significant chars
/// escaped. Used for member names / builtin names embedded in generated source.
/// (Our names are ASCII identifiers, but escaping keeps the generator correct if
/// the table ever carries an odd char, and satisfies the ASCII-source rule.)
fn js_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The `createRequire` implementation, including the embedded CJS builtin
/// registry its returned `require` resolves against. Generated (not a static
/// literal) so the registry is built from the SAME [`BUILTIN_SURFACE`] +
/// [`member_body`] as the ESM synthetic modules.
///
/// The Angular polyfill calls this once at eval time
/// (`globalThis.require ??= createRequire(import.meta.url)`), so the registry is
/// built once. `require(id)` strips an optional `node:` prefix and returns the
/// matching builtin's CJS exports; `crypto` is served from the global the crypto
/// shim registers; anything else is loud. `require.resolve` / `.cache` / `.main`
/// exist because esbuild banners probe them at eval time.
fn create_require_body() -> String {
    let mut out = String::new();
    out.push_str("function createRequire(_from) {\n");
    out.push_str("  const __registry = ");
    out.push_str(&cjs_registry_expr());
    out.push_str(";\n");
    out.push_str("  function require(id) {\n");
    out.push_str(
        "    const key = (typeof id === \"string\" && id.slice(0, 5) === \"node:\") ? id.slice(5) : id;\n",
    );
    // crypto is served from the global the richer crypto shim registers on eval,
    // so the CJS form is literally the same object as the ESM default export.
    out.push_str(
        "    if ((key === \"crypto\") && typeof globalThis !== \"undefined\" && globalThis.__elohimBuiltinCjs && globalThis.__elohimBuiltinCjs.crypto) {\n\
        \x20     return globalThis.__elohimBuiltinCjs.crypto;\n\
        \x20   }\n",
    );
    out.push_str(
        "    if (typeof key === \"string\" && Object.prototype.hasOwnProperty.call(__registry, key)) {\n\
        \x20     return __registry[key];\n\
        \x20   }\n",
    );
    out.push_str(
        "    throw new Error(\n\
        \x20     \"elohim-render module shim: require(\" + JSON.stringify(id) + \
        \") is not supported in the SSR runtime. Only Node builtins on the shim surface \
        (elohim/elohim-render/src/shim/node_builtins.rs) resolve synchronously via require; \
        this id is a non-builtin (e.g. an npm native addon) or an unshimmed builtin.\"\n\
        \x20   );\n",
    );
    out.push_str("  }\n");
    out.push_str(
        "  require.resolve = function (id) {\n\
        \x20   throw new Error(\n\
        \x20     \"elohim-render module shim: require.resolve(\" + JSON.stringify(id) + \
        \") is not supported in the SSR runtime.\"\n\
        \x20   );\n\
        \x20 };\n",
    );
    out.push_str("  require.cache = Object.create(null);\n");
    out.push_str("  require.main = undefined;\n");
    out.push_str("  return require;\n");
    out.push('}');
    out
}

/// The JS expression for the CJS builtin registry: an object mapping each bare
/// builtin name to its CJS exports object, built from [`BUILTIN_SURFACE`] +
/// [`member_body`] (the SAME source the ESM synthetic modules use). `module` is
/// excluded — `require("module")` is never needed, and including it would recurse
/// (its `createRequire` member embeds this very registry). `crypto` is excluded —
/// it is served from `globalThis.__elohimBuiltinCjs` (see [`create_require_body`]).
fn cjs_registry_expr() -> String {
    let mut out = String::new();
    out.push_str("(function () {\n");
    out.push_str(
        "  function __loud(builtin, member) {\n\
        \x20   return function loudStub() {\n\
        \x20     throw new Error(\n\
        \x20       \"elohim-render \" + builtin + \" shim (CJS via require): \" + member + \
        \"() is not implemented (SSR runtime shim, \
        elohim/elohim-render/src/shim/node_builtins.rs). The server bundle require()'d it \
        during render; implement it deliberately if the render path needs it.\"\n\
        \x20     );\n\
        \x20   };\n\
        \x20 }\n",
    );
    out.push_str("  const reg = Object.create(null);\n");
    for entry in BUILTIN_SURFACE {
        // Skip `module` (recursion: its createRequire embeds this registry).
        if entry.name == "module" {
            continue;
        }
        out.push_str("  {\n");
        for member in entry.named {
            let loud_call = format!("__loud({}, {})", js_string(entry.name), js_string(member));
            let val = member_value_expr(entry.name, member, &loud_call);
            out.push_str(&format!("    const m_{member} = {val};\n"));
        }
        match entry.cjs_primary {
            // module.exports IS the primary member; siblings hang off it.
            Some(primary) => {
                out.push_str(&format!("    const __exports = m_{primary};\n"));
                for member in entry.named {
                    out.push_str(&format!("    __exports.{member} = m_{member};\n"));
                }
            }
            // Plain namespace object.
            None => {
                out.push_str("    const __exports = {};\n");
                for member in entry.named {
                    out.push_str(&format!("    __exports.{member} = m_{member};\n"));
                }
            }
        }
        out.push_str(&format!(
            "    reg[{}] = __exports;\n",
            js_string(entry.name)
        ));
        out.push_str("  }\n");
    }
    out.push_str("  return reg;\n");
    out.push_str("})()");
    out
}

/// Build the link-safe synthetic module source for a builtin, or `None` if the
/// builtin has no synthetic shim (caller falls back to the loud whole-module
/// throw for a genuinely-unknown builtin).
pub(crate) fn synthetic_module_source(name: &str) -> Option<String> {
    let entry = surface(name)?;
    Some(render_module(entry))
}

/// Render the JS source for a surface entry.
fn render_module(entry: &BuiltinSurface) -> String {
    let name = entry.name;
    let mut out = String::new();

    out.push_str(&format!(
        "// elohim-render synthetic shim for the Node builtin '{name}'.\n\
        // Link-safe: exports every name the bundle links against. Unimplemented\n\
        // members are LOUD stubs (throw a named error on call). Extend\n\
        // deliberately in src/shim/node_builtins.rs -- never auto-stub a return.\n\n"
    ));

    // Loud-stub factory, parameterised by member name.
    out.push_str(&format!(
        "function __loud(member) {{\n\
        \x20 return function loudStub() {{\n\
        \x20   throw new Error(\n\
        \x20     \"elohim-render {name} shim: \" + member + \"() is not implemented \
        (SSR runtime shim, elohim/elohim-render/src/shim/node_builtins.rs). The \
        server bundle invoked it during render; implement it deliberately if the \
        render path needs it.\"\n\
        \x20   );\n\
        \x20 }};\n\
        }}\n\n"
    ));

    // Named exports: real impl where we have one, else a loud stub. `member_body`
    // (not `real_member`) so the ESM `module` shim's `createRequire` gets the
    // same registry-backed body the CJS path uses — one source, both surfaces.
    for member in entry.named {
        let val = member_value_expr(name, member, &format!("__loud(\"{member}\")"));
        out.push_str(&format!("const {member} = {val};\n"));
    }

    // The named-export statement (empty braces are legal if there are no names).
    out.push_str("export {");
    for (i, member) in entry.named.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push(' ');
        out.push_str(member);
    }
    out.push_str(" };\n\n");

    // The default export: an object carrying the named members, wrapped in a
    // Proxy so member access on `import x from "builtin"` is LOUD for unknown
    // members too -- while staying safe for the JS protocol / interop /
    // feature-detection names that must read as absent (undefined), not as a
    // present-but-throwing function. Returning a stub for `then` would make the
    // object spuriously thenable; returning one for `__esModule` would break
    // esbuild interop; so those pass through as undefined.
    out.push_str("const __named = {");
    for (i, member) in entry.named.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push(' ');
        out.push_str(member);
    }
    out.push_str(" };\n");
    out.push_str(
        "const __protocolSafe = new Set([\n\
        \x20 \"then\", \"catch\", \"finally\", \"default\", \"__esModule\",\n\
        \x20 \"constructor\", \"prototype\", \"toJSON\", \"toString\", \"valueOf\",\n\
        \x20 \"inspect\", \"$$typeof\", \"nodeType\", \"length\", \"name\"\n\
        ]);\n",
    );
    out.push_str(&format!(
        "export default new Proxy(__named, {{\n\
        \x20 get(target, prop, recv) {{\n\
        \x20   if (typeof prop !== \"string\") return Reflect.get(target, prop, recv);\n\
        \x20   if (prop in target) return target[prop];\n\
        \x20   if (__protocolSafe.has(prop)) return undefined;\n\
        \x20   return function loudStub() {{\n\
        \x20     throw new Error(\n\
        \x20       \"elohim-render {name} shim: \" + prop + \" is not implemented \
        (accessed off the default import; SSR runtime shim, \
        elohim/elohim-render/src/shim/node_builtins.rs).\"\n\
        \x20     );\n\
        \x20   }};\n\
        \x20 }}\n\
        }});\n"
    ));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bundle_builtin_has_a_surface_entry() {
        // Guard: the builtins the bundle links against are all present. crypto is
        // handled by its own shim and must NOT be here.
        assert!(surface("crypto").is_none(), "crypto has its own shim");
        assert!(surface("module").is_some());
        assert!(surface("net").is_some());
        assert!(surface("util").is_some());
    }

    #[test]
    fn module_source_carries_named_exports() {
        let src = synthetic_module_source("net").expect("net has a shim");
        // Every named export the bundle links against must be exported.
        for member in ["createServer", "isIP", "isIPv4", "isIPv6"] {
            assert!(
                src.contains(&format!("const {member} = ")),
                "net shim must define {member}: {src}"
            );
        }
        assert!(src.contains("export {"), "net shim exports names");
        assert!(src.contains("export default"), "net shim has a default");
    }

    #[test]
    fn createrequire_is_real_not_loud() {
        let src = synthetic_module_source("module").expect("module has a shim");
        assert!(
            src.contains("function createRequire(_from)"),
            "createRequire must be the real impl, not a loud stub: {src}"
        );
        // The returned require resolves builtins from an embedded CJS registry...
        assert!(
            src.contains("const __registry = "),
            "createRequire must embed the CJS builtin registry: {src}"
        );
        // ...and is loud for NON-builtin ids (npm addons, unshimmed builtins).
        assert!(
            src.contains("is not supported in the SSR runtime"),
            "the returned require must be loud for non-builtin ids: {src}"
        );
    }

    #[test]
    fn promisify_and_setmaxlisteners_are_real() {
        let util = synthetic_module_source("util").expect("util shim");
        assert!(
            util.contains("function promisify(fn)"),
            "promisify must be real: {util}"
        );
        let events = synthetic_module_source("events").expect("events shim");
        assert!(
            events.contains("function setMaxListeners()"),
            "setMaxListeners must be a real no-op: {events}"
        );
        // EventEmitter stays a loud stub in BOTH surfaces (not proven called on
        // the render path; a WebSocket instantiation would upgrade it to real).
        assert!(
            events.contains("const EventEmitter = __loud(\"EventEmitter\")"),
            "EventEmitter stays a loud stub until proven needed: {events}"
        );
    }

    #[test]
    fn createrequire_registry_resolves_builtins_and_stays_loud_for_others() {
        let src = synthetic_module_source("module").expect("module has a shim");
        // The registry carries the builtins ws require()s, keyed by bare name.
        for name in ["events", "net", "stream", "http", "https", "tls", "util"] {
            assert!(
                src.contains(&format!("reg[\"{name}\"]")),
                "CJS registry must carry '{name}': {src}"
            );
        }
        // A `node:` prefix is stripped before lookup.
        assert!(
            src.contains("id.slice(5)"),
            "require must strip a node: prefix before lookup: {src}"
        );
        // `module` itself is excluded (would recurse) and crypto is served from
        // the global the crypto shim registers.
        assert!(
            !src.contains("reg[\"module\"]"),
            "the module builtin must be excluded from the registry: {src}"
        );
        assert!(
            src.contains("globalThis.__elohimBuiltinCjs.crypto"),
            "require('crypto') is served from the crypto shim's global: {src}"
        );
    }

    #[test]
    fn events_cjs_exports_is_the_constructible_primary() {
        // `require("events")` must be EventEmitter itself (constructible), with
        // the siblings hung off it -- so `class X extends require("events")` and
        // `require("events").EventEmitter` both work, per Node's shape.
        let src = synthetic_module_source("module").expect("module has a shim");
        assert!(
            src.contains("const __exports = m_EventEmitter;"),
            "events CJS exports must be the constructible primary: {src}"
        );
        assert!(
            src.contains("__exports.EventEmitter = m_EventEmitter;"),
            "events primary must self-reference .EventEmitter: {src}"
        );
    }

    #[test]
    fn cjs_and_esm_member_lists_never_fork() {
        // The keystone single-source guard: for every builtin (except the
        // excluded module/crypto), the members the CJS registry hangs off its
        // exports are EXACTLY the ESM synthetic module's named exports. Both come
        // from BUILTIN_SURFACE, so a future edit that adds a member to one surface
        // but not the other trips this test.
        let module_src = synthetic_module_source("module").expect("module shim");
        for entry in BUILTIN_SURFACE {
            if entry.name == "module" {
                // module is not in the CJS registry (would recurse); its ESM
                // surface is still checked by module_source_carries_named_exports.
                continue;
            }
            let esm = synthetic_module_source(entry.name)
                .unwrap_or_else(|| panic!("{} has an ESM shim", entry.name));
            for member in entry.named {
                // ESM: exported as a named binding.
                assert!(
                    esm.contains(&format!("const {member} = ")),
                    "ESM {} must define {member}",
                    entry.name
                );
                // CJS: assigned onto the registry exports for this builtin.
                assert!(
                    module_src.contains(&format!("__exports.{member} = m_{member};")),
                    "CJS registry for {} must carry {member} (surfaces forked)",
                    entry.name
                );
            }
        }
    }

    #[test]
    fn unknown_builtin_has_no_synthetic_shim() {
        // A builtin not in the bundle's surface returns None so the loader falls
        // back to its loud whole-module throw.
        assert!(synthetic_module_source("readline").is_none());
        assert!(surface("readline").is_none());
    }

    #[test]
    fn empty_named_builtin_still_has_a_default() {
        // fs/path/os/etc. have only default imports; the module must still parse
        // and export a default (a loud Proxy).
        let src = synthetic_module_source("fs").expect("fs shim");
        assert!(src.contains("export {  };") || src.contains("export { };"));
        assert!(src.contains("export default new Proxy"));
    }
}
