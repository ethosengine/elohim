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
pub mod policy;
pub use context::{
    CheckedBy, ChromeContext, ChromeMedium, ChromeTheme, ContentDelivery, ContentFrame,
    ContentIsland, DeliveryCheck, DeliveryHop, DeliveryTransport, HolderReading, HopReporter,
    Isolation, IsolationBoundary, NavLink, ReceiptLink, ThemeHint, TrustMode, Viewer, ViewerKind,
    WitnessState, CHROME_CONTEXT_VERSION,
};
pub use policy::{content_policy, ContentPolicy, REPORT_ENDPOINT_NAME};

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

/// Strict JSON-in-script escape for the v1 content shell's island: every `<`,
/// `>` and `&` becomes a `\u` escape (equivalent inside a JSON string, the only
/// place JSON can carry them), as do the JS-hostile U+2028 and U+2029. The
/// parsed value is unchanged.
fn escape_json_strict(context_json: &str) -> String {
    let mut out = String::with_capacity(context_json.len());
    for c in context_json.chars() {
        match c {
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            other => out.push(other),
        }
    }
    out
}

fn html_escape(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Render the content shell: the document every medium serves around a
/// record's bytes. It contains NO navigation bar and NO static iframe — only
/// `<main id="elohim-content-frame">` (sized to fill the viewport; the chip
/// floats over it), a `<noscript>` link to `content.frame.src`, the context
/// island and the element script. The chrome creates the iframe after it
/// mounts. Producers (doorway, storage peer, desktop) fill the
/// [`ChromeContext`]; this renderer adds nothing medium-specific, and equal
/// input yields byte-identical output.
///
/// Dormant until a producer's content-chrome switch is on: no route calls it by
/// default, and the served element ([`ELEMENT_JS`]) does not yet build the
/// frame from `data-frame-src` (that element change is deferred; see
/// `genesis/data/timeline/backlog/2026-10-29-content-chrome-element-and-app-port.md`).
#[must_use]
pub fn render_content_shell(ctx: &ChromeContext) -> String {
    let frame = ctx.content.as_ref().map(|c| &c.frame);
    let src = frame.map_or("", |f| f.src.as_str());
    let title = ctx
        .title
        .as_deref()
        .or(frame.map(|f| f.title.as_str()))
        .unwrap_or(ctx.slug.as_str());
    let frame_title = frame.map_or(title, |f| f.title.as_str());
    let noscript = if src.is_empty() {
        String::new()
    } else {
        format!(
            "<noscript><a href=\"{}\">{}</a></noscript>",
            html_escape(src),
            html_escape(frame_title)
        )
    };
    let html = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>{title}</title>\
         <style>html,body{{margin:0;height:100%}}\
         #elohim-content-frame{{display:block;margin:0;width:100%;height:100vh;height:100dvh}}</style>\
         </head><body>\
         <main id=\"elohim-content-frame\" data-frame-src=\"{src}\" data-frame-title=\"{ftitle}\">{noscript}</main>\
         </body></html>",
        title = html_escape(title),
        src = html_escape(src),
        ftitle = html_escape(frame_title),
        noscript = noscript,
    );
    // The island is pre-escaped strictly (every `<`, `>`, `&`, U+2028, U+2029
    // as a `\u` escape), so no `</script`, `<!--` or `<script` token can form
    // whatever a title holds; `inject_element`'s own escape then finds nothing
    // to rewrite. Islands of today's producers keep `escape_json_for_script`
    // unchanged until the element lands (see the backlog named above).
    inject_element(&html, &escape_json_strict(&ctx.to_json()))
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
            "receipt",
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
            theme: Some("dark".into()),
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
            receipt: Some(ReceiptLink {
                href: "/api/v1/receipt/x".to_string(),
                sentence: None,
                label: None,
            }),
            // The v1 fields (version, viewer, medium, readingsBase, content)
            // stay None: the element served today reads only the pre-v1 set,
            // and a pre-v1 producer must still serialize exactly that set.
            ..Default::default()
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

    // ── render_content_shell ────────────────────────────────────────────────

    fn sample_content(boundary: IsolationBoundary, path: Vec<DeliveryHop>) -> ContentIsland {
        ContentIsland {
            record_id: "evolution-of-trust".into(),
            content_address: "sha256-0123abcd".into(),
            format: "html5-app".into(),
            entry: "index.html".into(),
            mode: TrustMode::Trustful,
            reach: Some("commons".into()),
            witness: WitnessState::Amber,
            capabilities: vec!["forms".into(), "popups".into()],
            holders: Some(HolderReading {
                holders: 3,
                target: 3,
                headline: Some("Held by 3 of 3 households".into()),
                as_of: Some("2026-10-15T12:00:00Z".into()),
            }),
            frame: ContentFrame {
                src: "/apps/evolution-of-trust/index.html?a=1&b=<2>".into(),
                title: "The Evolution of Trust".into(),
            },
            isolation: Isolation {
                boundary,
                content_origin: None,
            },
            delivery: ContentDelivery {
                path,
                checked: None,
                receipt: None,
            },
        }
    }

    fn contexts_for_each_medium() -> Vec<(&'static str, ChromeContext)> {
        let web2 = {
            let mut c = ChromeContext::v1(
                "evolution-of-trust",
                ViewerKind::Visitor,
                ChromeMedium::Web2Doorway,
            );
            c.title = Some("The Evolution of Trust".into());
            // The object form of `theme` (schema `$defs/themeHint`).
            c.theme = Some(
                ThemeHint {
                    color_scheme: Some("dark".into()),
                    theme_color: Some("#1b1d1f".into()),
                    tokens: None,
                }
                .into(),
            );
            let mut content = sample_content(
                IsolationBoundary::OriginPair,
                vec![
                    DeliveryHop::Doorway {
                        name: "alpha.elohim.host".into(),
                        id: "alpha-A".into(),
                        did: Some("did:web:alpha.elohim.host".into()),
                        reported_by: HopReporter::Doorway,
                    },
                    DeliveryHop::Holder {
                        peer: "uhCAkpeerkey".into(),
                        transport: Some(DeliveryTransport::Iroh),
                        reported_by: HopReporter::Doorway,
                    },
                ],
            );
            content.isolation.content_origin = Some("https://apps.alpha.elohim.host".into());
            content.delivery.checked = Some(DeliveryCheck {
                by: CheckedBy::Worker,
                against: "sha256-0123abcd".into(),
            });
            content.delivery.receipt = Some("/api/v1/receipt/evolution-of-trust".into());
            c.content = Some(content);
            c
        };
        let peer = {
            let mut c = ChromeContext::v1(
                "evolution-of-trust",
                ViewerKind::DeviceAgent,
                ChromeMedium::PeerNative,
            );
            let mut content = sample_content(
                IsolationBoundary::OriginPair,
                vec![DeliveryHop::SelfPeer {
                    reported_by: HopReporter::SelfPeer,
                }],
            );
            content.holders = None;
            content.witness = WitnessState::Green;
            c.content = Some(content);
            c
        };
        let desktop = {
            let mut c = ChromeContext::v1(
                "evolution-of-trust",
                ViewerKind::HostedSession,
                ChromeMedium::Desktop,
            );
            c.readings_base = Some("http://127.0.0.1:8090".into());
            c.theme = Some("dark".into());
            c.content = Some(sample_content(
                IsolationBoundary::SchemePerRecord,
                vec![DeliveryHop::SelfPeer {
                    reported_by: HopReporter::SelfPeer,
                }],
            ));
            c
        };
        vec![
            ("web2-doorway", web2),
            ("peer-native", peer),
            ("desktop", desktop),
        ]
    }

    #[test]
    fn content_shell_renders_for_each_medium() {
        for (medium, ctx) in contexts_for_each_medium() {
            let html = render_content_shell(&ctx);
            assert!(html.starts_with("<!doctype html>"), "{medium}");
            assert!(
                html.contains("<main id=\"elohim-content-frame\""),
                "{medium}"
            );
            assert!(
                !html.contains("<iframe"),
                "{medium}: the shell holds no iframe"
            );
            assert!(!html.contains("<nav"), "{medium}: no nav bar");
            assert!(html.contains("<noscript><a href=\"/apps/evolution-of-trust/index.html?a=1&amp;b=&lt;2&gt;\">The Evolution of Trust</a></noscript>"), "{medium}: {html}");
            assert!(
                html.contains(&format!("\"medium\":\"{medium}\"")),
                "{medium}"
            );
            assert!(html.contains(&element_script_path()), "{medium}");
            assert_eq!(html.matches("</body>").count(), 1, "{medium}");
        }
    }

    #[test]
    fn content_shell_is_byte_stable_for_equal_input() {
        for (_, ctx) in contexts_for_each_medium() {
            assert_eq!(
                render_content_shell(&ctx),
                render_content_shell(&ctx.clone())
            );
        }
    }

    #[test]
    fn content_shell_escapes_hostile_titles() {
        let mut ctx = ChromeContext::v1("x", ViewerKind::Visitor, ChromeMedium::PeerNative);
        ctx.title = Some("</title><script>alert(1)</script>".into());
        let html = render_content_shell(&ctx);
        assert!(!html.contains("<script>alert(1)"));
        assert!(html.contains("&lt;/title&gt;"));
        // The island still parses, to the same title.
        let start =
            html.find("id=\"elohim-omni-context\">").unwrap() + "id=\"elohim-omni-context\">".len();
        let end = start + html[start..].find("</script>").unwrap();
        let island: serde_json::Value = serde_json::from_str(&html[start..end]).unwrap();
        assert_eq!(island["title"], "</title><script>alert(1)</script>");
    }

    // ── schema contract ─────────────────────────────────────────────────────

    /// Validate serialized contexts against chrome-context.schema.json (ajv, via
    /// node; skipped loudly when node is unavailable).
    fn validate_against_schema(
        cases: &[(String, serde_json::Value)],
    ) -> Option<Vec<(String, bool, String)>> {
        use std::io::Write;
        let input = serde_json::Value::Array(
            cases
                .iter()
                .map(|(n, v)| serde_json::json!({ "name": n, "value": v }))
                .collect(),
        );
        let script = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/validate-island.mjs");
        let mut child = match std::process::Command::new("node")
            .arg(script)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("SKIPPED: node not available ({e}); schema-contract not run");
                return None;
            }
        };
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.to_string().as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "validator failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let res: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        Some(
            res.as_array()
                .unwrap()
                .iter()
                .map(|r| {
                    (
                        r["name"].as_str().unwrap().to_string(),
                        r["valid"].as_bool().unwrap(),
                        r["errors"].to_string(),
                    )
                })
                .collect(),
        )
    }

    #[test]
    fn serialized_contexts_satisfy_the_chrome_context_schema() {
        let mut cases: Vec<(String, serde_json::Value)> = contexts_for_each_medium()
            .into_iter()
            .map(|(m, c)| (format!("valid:{m}"), serde_json::to_value(&c).unwrap()))
            .collect();
        // A v1 context with no content (landing page, EPR page).
        cases.push((
            "valid:no-content".into(),
            serde_json::to_value(ChromeContext::v1(
                "elohim-host-landing",
                ViewerKind::Visitor,
                ChromeMedium::Web2Doorway,
            ))
            .unwrap(),
        ));
        // The invalid ones the schema must reject (each differs by one fact).
        let good = serde_json::to_value(&contexts_for_each_medium()[0].1).unwrap();
        let mutate = |f: &dyn Fn(&mut serde_json::Value)| {
            let mut v = good.clone();
            f(&mut v);
            v
        };
        cases.push((
            "invalid:pre-v1".into(),
            serde_json::to_value(ChromeContext {
                slug: "a".into(),
                ..Default::default()
            })
            .unwrap(),
        ));
        cases.push((
            "invalid:unknown-capability".into(),
            mutate(&|v| v["content"]["capabilities"] = serde_json::json!(["teleport"])),
        ));
        cases.push((
            "invalid:missing-address".into(),
            mutate(&|v| {
                v["content"]
                    .as_object_mut()
                    .unwrap()
                    .remove("contentAddress");
            }),
        ));
        cases.push((
            "invalid:bad-mode".into(),
            mutate(&|v| v["content"]["mode"] = serde_json::json!("banana")),
        ));
        cases.push((
            "invalid:extra-field".into(),
            mutate(&|v| v["content"]["surprise"] = serde_json::json!(1)),
        ));
        cases.push((
            "invalid:empty-path".into(),
            mutate(&|v| v["content"]["delivery"]["path"] = serde_json::json!([])),
        ));
        cases.push((
            "invalid:null-holders-key-missing".into(),
            mutate(&|v| {
                v["content"].as_object_mut().unwrap().remove("holders");
            }),
        ));
        let Some(results) = validate_against_schema(&cases) else {
            return;
        };
        for (name, valid, errors) in results {
            let want = name.starts_with("valid:");
            assert_eq!(valid, want, "{name}: errors {errors}");
        }
    }

    #[test]
    fn unknown_holders_serialize_as_null_not_absent() {
        let ctx = &contexts_for_each_medium()[1].1;
        let v = serde_json::to_value(ctx).unwrap();
        assert!(v["content"]["holders"].is_null());
        assert!(v["content"].as_object().unwrap().contains_key("holders"));
    }

    #[test]
    fn pre_v1_contexts_stay_byte_stable() {
        let ctx = ChromeContext {
            slug: "abc".into(),
            authenticated: true,
            ..Default::default()
        };
        assert_eq!(ctx.to_json(), r#"{"slug":"abc","authenticated":true}"#);
        let v1 = ChromeContext::v1("abc", ViewerKind::Visitor, ChromeMedium::PeerNative);
        assert_eq!(
            v1.to_json(),
            r#"{"slug":"abc","authenticated":false,"version":1,"viewer":{"kind":"visitor"},"medium":"peer-native"}"#
        );
    }
}
