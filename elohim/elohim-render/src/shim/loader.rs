//! `NodeShimLoader` -- a module loader that resolves the Node builtin specifiers
//! the Angular server bundle imports, delegating everything else to
//! `FsModuleLoader`.
//!
//! # Why
//!
//! The bundle imports Node's `crypto` builtin with a bare specifier
//! (`import ... from "crypto"`). deno_core's `FsModuleLoader` can only resolve
//! `file://` URLs, so a bare builtin fails `resolve()` with a raw TypeError
//! ("Relative import path \"crypto\" not prefixed with / or ./ or ..") and the
//! whole render panics. This loader intercepts the builtin specifiers before the
//! filesystem loader ever sees them:
//!
//! - `crypto` / `node:crypto` -> the injected JS shim (`node_crypto.js`), which
//!   is real for `createHash` and loud, named stubs for the rest.
//! - any other `node:*` specifier, or a bare specifier in [`NODE_BUILTINS`] ->
//!   a synthetic module whose evaluation throws a clear, named error. This
//!   bounds the blast radius of an unhandled builtin: instead of a cryptic
//!   resolve-time TypeError panic, the render fails with a message naming the
//!   builtin and where to add a shim, and the request falls back safely.
//! - everything else (relative chunks, the `file://` main module) -> delegated
//!   unchanged to `FsModuleLoader`.

use std::borrow::Cow;

use deno_core::error::ModuleLoaderError;
use deno_core::{
    FsModuleLoader, ModuleLoadResponse, ModuleLoader, ModuleSource, ModuleSourceCode,
    ModuleSpecifier, ModuleType, RequestedModuleType, ResolutionKind,
};
use deno_error::JsErrorBox;

use crate::shim::node_crypto::CRYPTO_SHIM_JS;

/// URL scheme for the synthetic builtin-shim modules this loader serves.
const BUILTIN_SCHEME: &str = "elohim-builtin";

/// Node builtins we recognise as BARE specifiers (a `node:` prefix always marks
/// a builtin, list or not). `crypto` is the only one the bundle actually uses;
/// the rest are listed so a future bundle importing a bare builtin gets a loud,
/// named error module instead of the cryptic FsModuleLoader "not a file URL"
/// failure. We do NOT intercept arbitrary bare specifiers -- only known builtins
/// -- so a genuine (npm-style) bare specifier still delegates and fails honestly.
const NODE_BUILTINS: &[&str] = &[
    "assert",
    "async_hooks",
    "buffer",
    "child_process",
    "cluster",
    "console",
    "constants",
    "crypto",
    "dgram",
    "diagnostics_channel",
    "dns",
    "events",
    "fs",
    "http",
    "http2",
    "https",
    "inspector",
    "module",
    "net",
    "os",
    "path",
    "perf_hooks",
    "process",
    "punycode",
    "querystring",
    "readline",
    "repl",
    "stream",
    "string_decoder",
    "timers",
    "tls",
    "tty",
    "url",
    "util",
    "v8",
    "vm",
    "worker_threads",
    "zlib",
];

/// Strip an optional `node:` prefix and, if the result is a recognised builtin,
/// return its bare name. Returns `None` for anything that is not a Node builtin
/// (relative paths, `file://`, npm-style bare specifiers, our own
/// `elohim-builtin:` scheme).
fn builtin_name(specifier: &str) -> Option<&str> {
    if let Some(rest) = specifier.strip_prefix("node:") {
        // A `node:` prefix always denotes a builtin, listed or not.
        return Some(rest);
    }
    if NODE_BUILTINS.contains(&specifier) {
        return Some(specifier);
    }
    None
}

/// The synthetic specifier a builtin resolves to, e.g. `elohim-builtin:crypto`.
// The `Err` type is dictated by the `ModuleLoader` trait's `resolve` signature
// (`ModuleLoaderError` is a large enum owned by deno_core); this helper feeds
// that signature directly, so boxing it would just force an unbox at the call
// site. Allow the large-err lint here rather than diverge from the trait's type.
#[allow(clippy::result_large_err)]
fn builtin_specifier(name: &str) -> Result<ModuleSpecifier, ModuleLoaderError> {
    ModuleSpecifier::parse(&format!("{BUILTIN_SCHEME}:{name}")).map_err(|e| {
        JsErrorBox::generic(format!(
            "elohim-render: cannot form builtin specifier for '{name}': {e}"
        ))
        .into()
    })
}

/// Loader that serves Node-builtin shims and delegates the rest to the
/// filesystem loader.
pub struct NodeShimLoader {
    fs: FsModuleLoader,
}

impl NodeShimLoader {
    pub fn new() -> Self {
        // `FsModuleLoader` is a unit struct with no `Default` impl.
        Self { fs: FsModuleLoader }
    }
}

impl Default for NodeShimLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleLoader for NodeShimLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        kind: ResolutionKind,
    ) -> Result<ModuleSpecifier, ModuleLoaderError> {
        match builtin_name(specifier) {
            Some(name) => builtin_specifier(name),
            None => self.fs.resolve(specifier, referrer, kind),
        }
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        maybe_referrer: Option<&ModuleSpecifier>,
        is_dyn_import: bool,
        requested_module_type: RequestedModuleType,
    ) -> ModuleLoadResponse {
        if module_specifier.scheme() == BUILTIN_SCHEME {
            // `elohim-builtin:crypto` -> the crypto shim; any other builtin -> a
            // module that throws a clear, named error at evaluation. Path() is the
            // opaque body after the scheme (e.g. "crypto").
            let name = module_specifier.path().to_string();
            let source = match name.as_str() {
                "crypto" => Cow::Borrowed(CRYPTO_SHIM_JS),
                other => {
                    // Log once per distinct builtin (load is called once per unique
                    // module) so an unhandled builtin is visible in server logs, not
                    // just the render trace.
                    tracing::warn!(
                        target: "elohim_render::shim",
                        builtin = %other,
                        "SSR bundle imported an unshimmed Node builtin; \
                         serving a loud error module (add a shim in src/shim/ if \
                         the render path needs it)"
                    );
                    Cow::Owned(unimplemented_builtin_module(other))
                }
            };
            let module = ModuleSource::new(
                ModuleType::JavaScript,
                ModuleSourceCode::String(source.into_owned().into()),
                module_specifier,
                None,
            );
            return ModuleLoadResponse::Sync(Ok(module));
        }
        self.fs.load(
            module_specifier,
            maybe_referrer,
            is_dyn_import,
            requested_module_type,
        )
    }
}

/// Source for a synthetic module that throws a clear, named error when the
/// bundle evaluates it. Loud (names the builtin) and bounded (only this render
/// fails, then falls back) -- never a raw TypeError panic.
fn unimplemented_builtin_module(name: &str) -> String {
    // The name comes from an import specifier the bundler emitted; escape the
    // quote/backslash chars so it is safe to embed in a JS string literal.
    let safe = name.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        "throw new Error(\"elohim-render: node builtin '{safe}' is not implemented in \
         the SSR runtime shim (elohim/elohim-render/src/shim/). It was imported by the \
         server bundle; add a shim if the render path needs it.\");\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loader() -> NodeShimLoader {
        NodeShimLoader::new()
    }

    #[test]
    fn resolves_bare_crypto_to_builtin() {
        let spec = loader()
            .resolve(
                "crypto",
                "file:///bundle/main.server.mjs",
                ResolutionKind::Import,
            )
            .expect("crypto resolves");
        assert_eq!(spec.as_str(), "elohim-builtin:crypto");
    }

    #[test]
    fn resolves_node_crypto_to_same_builtin() {
        let spec = loader()
            .resolve(
                "node:crypto",
                "file:///bundle/chunk.mjs",
                ResolutionKind::Import,
            )
            .expect("node:crypto resolves");
        assert_eq!(
            spec.as_str(),
            "elohim-builtin:crypto",
            "crypto and node:crypto resolve to the same shim"
        );
    }

    #[test]
    fn resolves_other_node_builtin_to_builtin_scheme() {
        let spec = loader()
            .resolve(
                "node:zlib",
                "file:///bundle/chunk.mjs",
                ResolutionKind::Import,
            )
            .expect("node:zlib resolves");
        assert_eq!(spec.as_str(), "elohim-builtin:zlib");
        assert_eq!(spec.scheme(), BUILTIN_SCHEME);
    }

    #[test]
    fn delegates_relative_specifier_to_fs_loader() {
        // A relative chunk import must resolve against its referrer unchanged,
        // exactly as FsModuleLoader would -- the loader must not hijack it.
        let spec = loader()
            .resolve(
                "./chunk-GZTFS4N4.mjs",
                "file:///bundle/main.server.mjs",
                ResolutionKind::Import,
            )
            .expect("relative resolves");
        assert_eq!(spec.as_str(), "file:///bundle/chunk-GZTFS4N4.mjs");
    }

    #[test]
    fn does_not_hijack_unknown_bare_specifier() {
        // A non-builtin bare specifier is NOT a Node builtin, so it must delegate
        // (and fail as FsModuleLoader would) rather than be served a shim.
        let name = builtin_name("some-npm-package");
        assert!(
            name.is_none(),
            "non-builtin bare specifier is not intercepted"
        );
    }

    #[test]
    fn builtin_name_strips_node_prefix() {
        assert_eq!(builtin_name("node:crypto"), Some("crypto"));
        assert_eq!(builtin_name("crypto"), Some("crypto"));
        // node: prefix always denotes a builtin even if not in the bare list.
        assert_eq!(
            builtin_name("node:some_future_builtin"),
            Some("some_future_builtin")
        );
    }

    #[test]
    fn unimplemented_module_names_the_builtin_and_throws() {
        let src = unimplemented_builtin_module("zlib");
        assert!(src.contains("zlib"), "names the builtin: {src}");
        assert!(
            src.starts_with("throw new Error("),
            "throws at evaluation: {src}"
        );
    }
}
