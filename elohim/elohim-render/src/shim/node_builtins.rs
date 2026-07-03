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
//!   return a real `require` function (whose body is itself loud-if-called: the
//!   render path must not synchronously require a module).
//! - `util.promisify` — commonly called at module-eval time to wrap callback
//!   APIs; a correct minimal implementation avoids an eval-time throw.
//! - `events.setMaxListeners` — a listener-cap raise; a no-op is semantically
//!   safe for a single-shot SSR render.
//!
//! Everything else stays a loud stub. Extend deliberately, one member at a time,
//! as a render path proves it needs the member — never speculatively fabricate a
//! return value (that trades a loud, precise failure for a silent-wrong render).
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
}

/// The static import surface of the deployed Angular server bundle, minus
/// `crypto`. Collected by scanning every chunk for `import ... from "<builtin>"`
/// and `import ... from "node:<builtin>"` (see the ignored test's doc comment for
/// the exact recipe). A NEW named import that appears in a future bundle and is
/// not listed here will fail at link with a clear deno error naming the missing
/// export — the honest signal to add it.
const BUILTIN_SURFACE: &[BuiltinSurface] = &[
    BuiltinSurface {
        name: "buffer",
        named: &["Buffer"],
    },
    BuiltinSurface {
        name: "cluster",
        named: &[],
    },
    BuiltinSurface {
        name: "dgram",
        named: &["createSocket"],
    },
    BuiltinSurface {
        name: "events",
        named: &["EventEmitter", "on", "setMaxListeners"],
    },
    BuiltinSurface {
        name: "fs",
        named: &[],
    },
    BuiltinSurface {
        name: "http",
        named: &["Agent"],
    },
    BuiltinSurface {
        name: "https",
        named: &[],
    },
    BuiltinSurface {
        name: "module",
        named: &["createRequire"],
    },
    BuiltinSurface {
        name: "net",
        named: &["createServer", "isIP", "isIPv4", "isIPv6"],
    },
    BuiltinSurface {
        name: "os",
        named: &["networkInterfaces"],
    },
    BuiltinSurface {
        name: "path",
        named: &[],
    },
    BuiltinSurface {
        name: "process",
        named: &[],
    },
    BuiltinSurface {
        name: "stream",
        named: &["Duplex"],
    },
    BuiltinSurface {
        name: "tls",
        named: &["TLSSocket", "connect"],
    },
    BuiltinSurface {
        name: "tty",
        named: &[],
    },
    BuiltinSurface {
        name: "url",
        named: &[],
    },
    BuiltinSurface {
        name: "util",
        named: &["promisify"],
    },
    BuiltinSurface {
        name: "vm",
        named: &[],
    },
    BuiltinSurface {
        name: "worker_threads",
        named: &[],
    },
];

/// Look up the surface entry for a bare builtin name.
fn surface(name: &str) -> Option<&'static BuiltinSurface> {
    BUILTIN_SURFACE.iter().find(|b| b.name == name)
}

/// The REAL implementation body for `(builtin, member)`, or `None` for a loud
/// stub. Each returned string is a JS expression that evaluates to the member's
/// value (a function or class). Kept tiny and semantically-safe — never a
/// fabricated data return.
fn real_member(builtin: &str, member: &str) -> Option<&'static str> {
    match (builtin, member) {
        // The Angular polyfill calls createRequire at eval time and assigns the
        // result to globalThis.require. It must return a real require function;
        // that require is itself loud-if-called (an SSR render must not
        // synchronously require a module). require.resolve / .cache / .main
        // exist because esbuild banners sometimes probe them at eval time.
        ("module", "createRequire") => Some(
            "function createRequire(_from) {\n\
            \x20 function require(id) {\n\
            \x20   throw new Error(\n\
            \x20     \"elohim-render module shim: require(\" + JSON.stringify(id) + \
            \") is not supported in the SSR runtime. createRequire returns a stub \
            require; the render path must not synchronously require a module.\"\n\
            \x20   );\n\
            \x20 }\n\
            \x20 require.resolve = function (id) {\n\
            \x20   throw new Error(\n\
            \x20     \"elohim-render module shim: require.resolve(\" + \
            JSON.stringify(id) + \") is not supported in the SSR runtime.\"\n\
            \x20   );\n\
            \x20 };\n\
            \x20 require.cache = Object.create(null);\n\
            \x20 require.main = undefined;\n\
            \x20 return require;\n\
            }",
        ),
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
        _ => None,
    }
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

    // Named exports: real impl where we have one, else a loud stub.
    for member in entry.named {
        match real_member(name, member) {
            Some(body) => {
                out.push_str(&format!("const {member} = ({body});\n"));
            }
            None => {
                out.push_str(&format!("const {member} = __loud(\"{member}\");\n"));
            }
        }
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
        // The returned require is itself loud-if-called.
        assert!(
            src.contains("must not synchronously require a module"),
            "the returned require must be loud-if-called: {src}"
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
        // EventEmitter stays a loud stub (not proven called on the render path).
        assert!(
            events.contains("const EventEmitter = __loud(\"EventEmitter\")"),
            "EventEmitter stays a loud stub until proven needed: {events}"
        );
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
