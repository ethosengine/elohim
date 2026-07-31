//! Content-addressed runtime-served EPR omnibar ELEMENT — the **light** home.
//!
//! The native EPR omnibar is a runtime-served, self-contained CLIENT ELEMENT
//! (`omni-element.js`, baked into this crate via `include_str!` and
//! content-addressed by its `sha256`). Any page — the doorway SSR shell, a
//! `/deliver` CSR page, the Tauri static `index.html` — references it via a
//! single `<script src>`; it self-mounts, acquires the wrapped EPR's context
//! (inline-injected OR fetched), renders the rich `protocol-omni` markup,
//! applies the EPR theme, and wires the behavior — all client-side, identically
//! everywhere.
//!
//! ## Why a separate crate (the V8 boundary)
//!
//! Serving this static JS file must NOT require compiling V8. The element
//! markup/style **renderers** (the Rust SSR-splice source-of-truth) live in
//! `elohim-render::chrome`, which hard-depends on `deno_core` (V8). This crate
//! carries ONLY the served bytes + hash + path helpers + the SSR-HTML inject
//! helper, plus the [`ChromeContext`] producer struct + `serde`/`serde_json`
//! (deliberately kept light — no V8, no heavy deps). Both the doorway (which
//! already pulls `elohim-render`, re-exporting these) AND the **default**
//! (non-`ssr`) `elohim-storage` build depend on this crate cheaply, with no
//! V8 in the tree.
//!
//! `elohim-render::chrome::element` re-exports everything here so the doorway's
//! existing `elohim_render::element_*` usage is unchanged.
//!
//! Hashing convention mirrors `enhance.rs` / `bootstrap.rs`
//! (`format!("{:x}", Sha256::digest)`). The filename hash is bare lowercase hex
//! (a clean content address); [`element_js_hash`] exposes that same hex.
//!
//! [`context`] carries [`ChromeContext`] — the single typed producer struct
//! shared between the doorway (producer) and this crate's structural contract
//! test (consumer guard), closing the inline-context-island edge of the
//! producer/consumer contract that the sibling resilience-mapper edge closed
//! for `/api/v1/resilience/{slug}` (see `resilience_mapper_speaks_the_snapshot_contract`).

use sha2::{Digest, Sha256};
use std::sync::OnceLock;

mod context;
pub use context::{ChromeContext, NavLink};

/// The hand-written, self-contained vanilla element script, baked at compile
/// time. Self-mounts, acquires EPR context, renders + themes + wires behavior.
pub const ELEMENT_JS: &str = include_str!("omni-element.js");

/// The id of the inline JSON context island the element reads (the doorway
/// inject path). Kept here so [`inject_element`] and the element JS agree on the
/// exact id — drift between them would silently break inline-context delivery.
pub const CONTEXT_SCRIPT_ID: &str = "elohim-omni-context";

/// The script's bare lowercase-hex `sha256`, computed once on first use.
fn element_js_hash_cell() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| format!("{:x}", Sha256::digest(ELEMENT_JS.as_bytes())))
}

/// The element bytes the `/chrome/` route serves.
#[must_use]
pub fn element_js_bytes() -> &'static [u8] {
    ELEMENT_JS.as_bytes()
}

/// The element's content address — bare lowercase-hex `sha256` of [`ELEMENT_JS`].
#[must_use]
pub fn element_js_hash() -> &'static str {
    element_js_hash_cell()
}

/// The content-addressed URL path the runtime serves the element at, e.g.
/// `/chrome/omni-element.<sha256hex>.js`. Any page splices a `<script src>`
/// pointing here; the `/chrome/` route (doorway AND storage sidecar) serves the
/// bytes at exactly this path.
#[must_use]
pub fn element_script_path() -> String {
    format!("/chrome/omni-element.{}.js", element_js_hash())
}

/// The STABLE (non-content-addressed) alias the `/chrome/` route also serves the
/// CURRENT element at. Static references that cannot embed a content hash (the
/// Tauri `index.html`) point at this; it always serves the current bytes.
///
/// Trade-off: this path is NOT immutable — its bytes change when the element
/// changes — so the route serves it with a revalidation-friendly cache policy,
/// unlike the content-addressed [`element_script_path`] (immutable).
pub const STABLE_ELEMENT_PATH: &str = "/chrome/omni-element.js";

/// Escape a JSON string so it can be embedded safely inside an HTML
/// `<script type="application/json">…</script>` island without the JSON being
/// able to break out of the script element.
///
/// The HTML tokenizer ends a script element at the first `</script` (ASCII
/// case-insensitive) regardless of JS/JSON string context, so a slug or title
/// containing the literal `</script>` would otherwise terminate the island
/// early and inject markup. We neutralize it by escaping the `</` sequence to
/// `<\/` — `\/` is a valid JSON escape for `/`, so the parsed value is
/// unchanged, but the `</script` token can no longer form. We also defang `<!--`
/// (HTML comment open) for the same tokenizer reason.
#[must_use]
pub fn escape_json_for_script(context_json: &str) -> String {
    context_json.replace("</", "<\\/").replace("<!--", "<\\!--")
}

/// Inject the runtime omnibar element into a finalized SSR HTML document.
///
/// Inserts, immediately before the closing `</body>` (appends if absent —
/// defensive, never corrupt the document):
///   1. the inline JSON context island
///      `<script type="application/json" id="elohim-omni-context">{json}</script>`
///      (the doorway path supplies the authoritative slug + the flags the
///      element cannot infer client-side), and
///   2. the element loader `<script src="{element_script_path()}" defer></script>`.
///
/// The `context_json` is escaped with [`escape_json_for_script`] so it cannot
/// break out of the island. The element is idempotent (it no-ops if already
/// mounted), so a double-inject is harmless.
#[must_use]
pub fn inject_element(html: &str, context_json: &str) -> String {
    let escaped = escape_json_for_script(context_json);
    let snippet = format!(
        "<script type=\"application/json\" id=\"{id}\">{json}</script>\
         <script src=\"{src}\" defer></script>",
        id = CONTEXT_SCRIPT_ID,
        json = escaped,
        src = element_script_path(),
    );

    // Insert before the LAST `</body>` (case-insensitively) so a `</body>`
    // appearing earlier inside a string/comment doesn't misplace the splice.
    if let Some(idx) = find_last_body_close(html) {
        let mut out = String::with_capacity(html.len() + snippet.len());
        out.push_str(&html[..idx]);
        out.push_str(&snippet);
        out.push_str(&html[idx..]);
        out
    } else {
        // No </body>: append defensively. The element still self-mounts.
        let mut out = String::with_capacity(html.len() + snippet.len());
        out.push_str(html);
        out.push_str(&snippet);
        out
    }
}

/// Byte index of the last `</body>` (ASCII case-insensitive) in `html`, or
/// `None` if absent.
fn find_last_body_close(html: &str) -> Option<usize> {
    const NEEDLE: &[u8] = b"</body>";
    let bytes = html.as_bytes();
    if bytes.len() < NEEDLE.len() {
        return None;
    }
    // Scan from the end for a case-insensitive match.
    let mut i = bytes.len() - NEEDLE.len();
    loop {
        if bytes[i..i + NEEDLE.len()].eq_ignore_ascii_case(NEEDLE) {
            return Some(i);
        }
        if i == 0 {
            return None;
        }
        i -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_matches_script_bytes() {
        let expected = format!("{:x}", Sha256::digest(ELEMENT_JS.as_bytes()));
        assert_eq!(element_js_hash(), expected);
    }

    #[test]
    fn script_path_carries_the_hash() {
        let path = element_script_path();
        assert!(path.starts_with("/chrome/omni-element."), "{path}");
        assert!(path.ends_with(".js"), "{path}");
        assert!(path.contains(element_js_hash()), "{path}");
    }

    #[test]
    fn hash_is_lowercase_hex_64_chars() {
        let h = element_js_hash();
        assert_eq!(h.len(), 64, "sha256 hex is 64 chars: {h}");
        assert!(
            h.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "hash must be lowercase hex: {h}"
        );
    }

    #[test]
    fn bytes_match_const() {
        assert_eq!(element_js_bytes(), ELEMENT_JS.as_bytes());
    }

    #[test]
    fn stable_alias_is_distinct_from_the_hashed_path() {
        // The stable alias is the un-hashed path; the content-addressed path
        // carries the hash. They must never be the same string.
        assert_ne!(STABLE_ELEMENT_PATH, element_script_path());
        assert_eq!(STABLE_ELEMENT_PATH, "/chrome/omni-element.js");
    }

    #[test]
    fn script_is_self_contained_and_bounded() {
        // The element is render + theme + behavior in one file: larger than the
        // enhance-only script, but still a single small asset. The bounds catch
        // accidental bloat, not a hard limit.
        let len = ELEMENT_JS.len();
        assert!(len > 3000, "element unexpectedly small: {len} bytes");
        assert!(len < 60_000, "element unexpectedly large: {len} bytes");
    }

    #[test]
    fn element_carries_the_self_mount_and_context_contract() {
        // The element MUST self-mount (#elohim-omni), read an inline context
        // island OR fetch the content node, and resolve the landing slug. These
        // string anchors guard the contract sibling tasks depend on.
        assert!(
            ELEMENT_JS.contains("elohim-omni"),
            "missing omni container id"
        );
        assert!(
            ELEMENT_JS.contains(CONTEXT_SCRIPT_ID),
            "missing inline-context island id"
        );
        assert!(
            ELEMENT_JS.contains("/db/content/"),
            "missing content-node fetch fallback"
        );
        assert!(
            ELEMENT_JS.contains("elohim-host-landing"),
            "missing landing-slug resolution"
        );
        // Behavior contract (absorbed from omni-enhance.js).
        assert!(
            ELEMENT_JS.contains("elohim-theme-changed"),
            "missing theme-changed event"
        );
        assert!(
            ELEMENT_JS.contains("/api/v1/resilience/"),
            "missing lazy resilience fetch"
        );
    }

    #[test]
    fn resilience_mapper_speaks_the_snapshot_contract() {
        // Structural contract gate for the resilience mapper. The element
        // once shipped reading phantom fields (data.glyph/standing/reach —
        // never in any ResilienceSnapshotView) so the fetch succeeded forever
        // while the mapper matched nothing and the card stayed neutral. This
        // list mirrors `resilience-snapshot-view.schema.json` `properties`
        // (source of truth; storage's schema_contract.rs guards struct↔schema
        // — keep this list in sync when the mapper's reads change; the
        // durable fix is compiling the element against the generated TS view,
        // tracked in the omni contract backlog item).
        for field in [
            "protectionStatus",
            "feltStatus",
            "headline",
            "reassurance",
            "coverageShortfall",
            "stewardingCollectives",
            "commitmentBackedCollectives",
            "diversityScore",
            "distributionState",
            "floor",
            "hasHouseholds",
        ] {
            assert!(
                ELEMENT_JS.contains(field),
                "element no longer reads wire field `{field}` — update this list with the mapper"
            );
        }
        // The phantom accessors must never come back.
        for phantom in ["data.glyph", "data.standing", "data.reach"] {
            assert!(
                !ELEMENT_JS.contains(phantom),
                "phantom wire accessor `{phantom}` reintroduced — not in any schema"
            );
        }
        // The household felt-status feed (spec §11) is the primary fetch.
        assert!(
            ELEMENT_JS.contains("/household"),
            "resilience fetch no longer targets the /household felt-status variant"
        );
    }

    #[test]
    fn claims_mapper_speaks_the_epr_projection_contracts() {
        // Structural contract gate for the backing-claims section — the client
        // leg of the two Category-C EPR projections. Sibling of
        // `resilience_mapper_speaks_the_snapshot_contract`: the element reads a
        // wire shape it does not compile against, so the field names are pinned
        // here against their schemas.

        // Both projections are fetched (parallel legs, independent failure).
        assert!(
            ELEMENT_JS.contains("/nav-context"),
            "missing lazy EPR nav-context fetch"
        );
        assert!(
            ELEMENT_JS.contains("/api/v1/epr/"),
            "claims fetch must target the EPR projection routes"
        );
        assert!(
            ELEMENT_JS.contains("Promise.all"),
            "the two projections must be fetched in parallel, not chained"
        );

        // epr-nav-context-view.schema.json properties the mapper reads.
        for field in ["prev", "next", "partOf", "related", "resilienceTier"] {
            assert!(
                ELEMENT_JS.contains(field),
                "element no longer reads EprNavContextView field `{field}`"
            );
        }
        // epr-raw-view.schema.json properties the mapper reads (the three
        // coupling legs — story/value/governance — plus reverse part-of).
        for field in [
            "coupling",
            "knowledge",
            "value",
            "governance",
            "reversePartOf",
        ] {
            assert!(
                ELEMENT_JS.contains(field),
                "element no longer reads EprRawView field `{field}`"
            );
        }
        // Neither projection carries these — a phantom read here is the exact
        // shape of the resilience-mapper regression.
        for phantom in ["nav.claims", "raw.legs", "raw.partOf", "nav.coupling"] {
            assert!(
                !ELEMENT_JS.contains(phantom),
                "phantom EPR projection accessor `{phantom}` — not in any schema"
            );
        }

        // Links target the universal EPR address (doorway §12.1), routed
        // through the href allowlist like every other element-built href.
        assert!(
            ELEMENT_JS.contains("'/epr/' + encodeURIComponent(cid)"),
            "claim links must target the universal /epr/{{cid}} address"
        );
    }

    #[test]
    fn claims_section_is_tri_state_and_quiet_when_absent() {
        // The DOM must testify the section's state the same way the resilience
        // section does — "loading" → "applied" | "unmatched" — so a probe can
        // tell "no claims recorded" (the common case while the atom seeder is
        // manual-only) from "the fetch never ran".
        assert!(
            ELEMENT_JS.contains("data-omni-claims-loaded"),
            "missing the tri-state claims marker attribute"
        );
        for state in ["'loading'", "'applied'", "'unmatched'"] {
            assert!(
                ELEMENT_JS.contains(state),
                "claims marker must be able to settle to {state}"
            );
        }
        // Quiet-when-absent: the toggle ships `hidden` and is only revealed
        // once at least one claim link rendered.
        assert!(
            ELEMENT_JS.contains("data-omni-action=\"claims-toggle\""),
            "missing the claims toggle action hook"
        );
        assert!(
            ELEMENT_JS.contains("removeAttribute('hidden')"),
            "the claims group must be revealed only after links render"
        );
        assert!(
            ELEMENT_JS.contains("data-omni-claims-count"),
            "missing the rendered-claims count attestation"
        );
        // The card is never offered empty (mirrors toggleResilienceCard).
        assert!(
            ELEMENT_JS.contains("no empty flyout"),
            "the no-empty-flyout discipline must stay documented at the toggle"
        );
    }

    #[test]
    fn nav_slots_are_client_resolved_not_deferred() {
        // The mutual-deferral bug: the doorway left nav "to the element's own
        // client-side resolution" while the element hard-nulled navBack /
        // navForward because "the doorway supplies those" — so nav never
        // rendered from either side. The element now owns the resolution.
        assert!(
            ELEMENT_JS.contains("data-omni-nav-slot"),
            "missing the nav insertion slots the fetched nav-context fills"
        );
        assert!(
            ELEMENT_JS.contains("fillNavSlot"),
            "missing the client-side nav resolver"
        );
        assert!(
            !ELEMENT_JS.contains("the doorway supplies those"),
            "the stale mutual-deferral comment is back — nav is client-resolved"
        );
        // An island-supplied nav target still wins (the slot is only filled
        // when empty) — the element resolves what was LEFT to it, it does not
        // override a producer that spoke.
        assert!(
            ELEMENT_JS.contains("if (!slot || slot.firstChild) return false;"),
            "an island-authored nav slot must never be overwritten"
        );
    }

    #[test]
    fn chrome_context_speaks_the_producer_contract() {
        // Structural contract gate for the inline context island — the third
        // (and last) unchecked producer/consumer edge of the omni element,
        // sibling to `resilience_mapper_speaks_the_snapshot_contract` above.
        // `ChromeContext` (context.rs) is the producer; `omni-element.js`'s
        // `ctx.<field>` reads across `mount`/`buildMarkup`/
        // `contextFromContentNode` are the consumer. A field dropped from one
        // side without the other is exactly the drift class that shipped the
        // resilience-mapper bug (a fetch that always "succeeds" while nothing
        // matches).
        let known_fields: std::collections::HashSet<&str> = [
            "slug",
            "title",
            "theme",
            "buildMarker",
            "envTier",
            "showEnv",
            "authenticated",
            "accountHref",
            "showThemeToggle",
            "navBack",
            "navForward",
        ]
        .into_iter()
        .collect();

        // Direction 1: every known (ChromeContext) field is actually read by
        // the element — a producer field the consumer never looks at is dead
        // weight (and a sign the list drifted from the JS).
        for field in &known_fields {
            let needle = format!("ctx.{field}");
            assert!(
                ELEMENT_JS.contains(needle.as_str()),
                "ChromeContext field `{field}` is never read as `{needle}` by the element — \
                 drop it from context.rs or fix the accessor name"
            );
        }

        // Direction 2: scan the element for every `ctx.<ident>` accessor and
        // assert it is a known field — this is the direction that catches a
        // *phantom* accessor (a JS read with no producer field), the exact
        // shape of the resilience-mapper regression.
        for (pos, _) in ELEMENT_JS.match_indices("ctx.") {
            // Guard against matching mid-identifier (e.g. a hypothetical
            // `somectx.foo`) — the char immediately before `ctx.` must not be
            // an identifier char.
            let preceded_ok = pos == 0 || {
                let prev = ELEMENT_JS.as_bytes()[pos - 1];
                !(prev.is_ascii_alphanumeric() || prev == b'_')
            };
            if !preceded_ok {
                continue;
            }
            let rest = &ELEMENT_JS[pos + "ctx.".len()..];
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(rest.len());
            let ident = &rest[..end];
            if ident.is_empty() {
                continue;
            }
            assert!(
                known_fields.contains(ident),
                "element reads phantom ctx field `{ident}` — not a ChromeContext field; \
                 add it to context.rs or fix the accessor"
            );
        }

        // Cross-check against the actual serialized field names: a fully
        // populated ChromeContext must expose exactly this field set (catches
        // context.rs itself drifting from the mirror list above).
        let full = ChromeContext {
            slug: "s".to_string(),
            authenticated: true,
            title: Some("t".to_string()),
            theme: Some("dark".to_string()),
            build_marker: Some("m".to_string()),
            env_tier: Some("staging".to_string()),
            show_env: Some(true),
            account_href: Some("/account".to_string()),
            show_theme_toggle: Some(true),
            nav_back: Some(NavLink {
                href: "/b".to_string(),
                label: None,
            }),
            nav_forward: Some(NavLink {
                href: "/f".to_string(),
                label: None,
            }),
        };
        let v = serde_json::to_value(&full).expect("ChromeContext serializes");
        let obj = v.as_object().expect("ChromeContext serializes to a map");
        assert_eq!(
            obj.len(),
            known_fields.len(),
            "serialized ChromeContext field count drifted from the mirror list: {obj:?}"
        );
        for key in obj.keys() {
            assert!(
                known_fields.contains(key.as_str()),
                "ChromeContext serializes unexpected field `{key}` — not read by the element"
            );
        }
    }

    #[test]
    fn element_carries_the_css_injection_and_href_guards() {
        // The token-value CSS-injection guard and the href scheme allowlist are
        // load-bearing XSS guards baked into the served JS. These string anchors
        // ensure a refactor cannot silently drop them (the JS has no in-tree
        // engine test; the behavioral proof lives in elohim-render's Rust twin).
        assert!(
            ELEMENT_JS.contains("isSafeTokenValue"),
            "missing CSS-value allowlist guard"
        );
        assert!(
            ELEMENT_JS.contains("safeHref"),
            "missing href scheme allowlist guard"
        );
        // The token guard must reject the structural break-out / active-content
        // chars and fall back to the base palette.
        assert!(
            ELEMENT_JS.contains("[;{}<@]"),
            "token guard must deny the structural break-out chars"
        );
        assert!(
            ELEMENT_JS.contains("expression"),
            "token guard must deny expression("
        );
    }

    // ── inject_element ──────────────────────────────────────────────────────

    #[test]
    fn inject_places_snippet_before_body_close() {
        let html = "<!doctype html><html><body><app-root></app-root></body></html>";
        let out = inject_element(html, r#"{"slug":"x"}"#);

        // The island + loader land before </body>, in order.
        let island_at = out
            .find("id=\"elohim-omni-context\"")
            .expect("island present");
        let loader_at = out.find("omni-element.").expect("loader present");
        let body_close = out.rfind("</body>").expect("body close present");
        assert!(island_at < body_close, "island must precede </body>");
        assert!(loader_at < body_close, "loader must precede </body>");
        assert!(island_at < loader_at, "island must precede loader");

        // The loader references the content-addressed element path with `defer`.
        assert!(
            out.contains(&format!("src=\"{}\" defer", element_script_path())),
            "loader src/defer: {out}"
        );
        // The original document content survives intact.
        assert!(out.contains("<app-root></app-root>"));
        assert!(out.ends_with("</body></html>"));
        // Exactly one </body> (we inserted before it, didn't duplicate).
        assert_eq!(out.matches("</body>").count(), 1);
    }

    #[test]
    fn inject_appends_when_no_body_close() {
        let html = "<div>fragment with no body tag</div>";
        let out = inject_element(html, r#"{"slug":"y"}"#);
        assert!(out.starts_with(html), "original preserved at head");
        assert!(
            out.contains("id=\"elohim-omni-context\""),
            "island appended"
        );
        assert!(out.contains("omni-element."), "loader appended");
    }

    #[test]
    fn inject_targets_the_last_body_close() {
        // A </body> inside earlier content must not be the splice point.
        let html = "<body>a</body><!-- stray --><body>b</body>";
        let out = inject_element(html, "{}");
        // Inserted before the LAST </body>: the snippet sits after "b".
        let snippet_at = out.find("id=\"elohim-omni-context\"").unwrap();
        let last_close = out.rfind("</body>").unwrap();
        assert!(snippet_at < last_close);
        assert!(
            out.contains("b</script>") || out.contains("b<script"),
            "near last body: {out}"
        );
    }

    #[test]
    fn escape_neutralizes_script_break_out() {
        // A slug carrying a literal </script> must not be able to close the
        // island. After escaping, the dangerous </script token is gone but the
        // JSON value is unchanged once parsed (\/ is a valid JSON escape for /).
        let malicious = r#"{"slug":"</script><img src=x onerror=alert(1)>"}"#;
        let escaped = escape_json_for_script(malicious);
        assert!(
            !escaped.to_ascii_lowercase().contains("</script"),
            "</script token must be neutralized: {escaped}"
        );
        assert!(
            escaped.contains("<\\/script"),
            "expected <\\/ escape: {escaped}"
        );

        // Embedded in the full inject, the island cannot be broken out of.
        let html = "<html><body></body></html>";
        let out = inject_element(html, malicious);
        // The only literal </script in the document are the legitimate closers
        // (the island's own and the loader's own) — not one from the payload.
        // There are exactly two script elements ⇒ two real </script> closers.
        assert_eq!(
            out.to_ascii_lowercase().matches("</script>").count(),
            2,
            "exactly the two injected script closers: {out}"
        );
    }

    #[test]
    fn escape_neutralizes_html_comment_open() {
        let payload = r#"{"note":"<!-- comment open"}"#;
        let escaped = escape_json_for_script(payload);
        assert!(
            !escaped.contains("<!--"),
            "comment-open must be defanged: {escaped}"
        );
    }

    #[test]
    fn inject_is_idempotent_safe_double() {
        // Double-injection is harmless (the element self-no-ops if mounted);
        // here we just assert it does not corrupt structure — two islands, two
        // loaders, all before the single </body>.
        let html = "<html><body>x</body></html>";
        let once = inject_element(html, "{}");
        let twice = inject_element(&once, "{}");
        assert_eq!(twice.matches("</body>").count(), 1);
        assert_eq!(twice.matches("id=\"elohim-omni-context\"").count(), 2);
    }
}
