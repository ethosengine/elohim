//! `GET /chrome/*` — doorway-specific runtime chrome static assets.
//!
//! The native EPR omnibar is a runtime-served, self-contained CLIENT ELEMENT
//! (`omni-element.js`, baked into the `elohim-render` crate and content-addressed
//! by its `sha256`). Any page (the doorway SSR shell, a `/deliver` CSR page, the
//! Tauri static SPA) references it via a single `<script src>`; it self-mounts
//! and renders client-side. This route serves the element bytes.
//!
//! The element SUPERSEDES the older `omni-enhance.js` (the enhance accessors in
//! `elohim-render` now delegate to the element, so `enhance_script_path()` ==
//! `element_script_path()` — one served asset). This route matches the element
//! path canonically; the enhance path resolves to the same string.
//!
//! This is doorway-specific runtime chrome — the same legitimate class as
//! `bootstrap`/`signal`/`metrics` (a surface the doorway owns and paints, NOT a
//! per-domain proxy of substrate truth). It does NOT belong in the manifest
//! registry: there is no storage endpoint behind it, and every doorway serves
//! the identical content-addressed bytes from the linked `elohim-render` crate.
//! See `doorway/CLAUDE.md` "No Per-Domain Proxy Files" — this is an explicit,
//! documented exception, an arm ABOVE the wildcard registry fallback.
//!
//! Content addressing makes the asset immutable: a new script ⇒ a new hash ⇒ a
//! new path, so the content-addressed path is served
//! `Cache-Control: public, max-age=31536000, immutable`.
//!
//! The crate ALSO publishes a STABLE, non-content-addressed alias
//! (`elohim_render::STABLE_ELEMENT_PATH` == `/chrome/omni-element.js`) for the
//! references that cannot embed a content hash — the Tauri `index.html`, and
//! any acceptance scenario naming the asset. The storage sidecar has served it
//! since the alias landed (`handle_chrome_asset` in
//! `elohim/elohim-storage/src/http.rs`); this route now serves it too, so the
//! two runtimes that link the same chrome crate honor the same published
//! contract. That alias is NOT immutable — its bytes move with the element —
//! so it is served revalidate-first with an `ETag` of the content address.

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};

/// Serve a `/chrome/*` static asset.
///
/// Two known paths — the same two the storage sidecar serves:
/// - `GET /chrome/omni-element.{sha256}.js` → the element bytes (200,
///   immutable). The legacy enhance path resolves to this same
///   content-addressed path.
/// - `GET /chrome/omni-element.js` → the STABLE alias for the CURRENT element
///   (200), served `max-age=0, must-revalidate` with an `ETag` of the content
///   address, because its bytes change whenever the element does.
///
/// Any other `/chrome/*` path (including a stale/mismatched hash) → 404.
#[must_use]
pub fn handle_chrome_asset(path: &str) -> Response<Full<Bytes>> {
    // The element is the canonical served asset; `enhance_script_path()` now
    // delegates to `element_script_path()`, so matching either is equivalent.
    if path == elohim_render::element_script_path() || path == elohim_render::enhance_script_path()
    {
        return Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/javascript; charset=utf-8")
            // Content-addressed ⇒ immutable. A changed script changes the hash
            // (and therefore the path), so the bytes at THIS path never change.
            .header("Cache-Control", "public, max-age=31536000, immutable")
            .body(Full::new(Bytes::from_static(
                elohim_render::element_js_bytes(),
            )))
            .expect("infallible chrome asset response");
    }

    // The STABLE alias: same bytes, different cache contract. Mirrors
    // `handle_chrome_asset` in elohim/elohim-storage/src/http.rs — a static
    // reference that cannot carry a content hash still gets the current
    // element, and the ETag lets a client skip the body when the content
    // address has not moved.
    if path == elohim_render::STABLE_ELEMENT_PATH {
        return Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/javascript; charset=utf-8")
            // NOT immutable: the alias always resolves to the CURRENT element,
            // so its bytes change when the element changes. Revalidate.
            .header("Cache-Control", "public, max-age=0, must-revalidate")
            .header("ETag", format!("\"{}\"", elohim_render::element_js_hash()))
            .body(Full::new(Bytes::from_static(
                elohim_render::element_js_bytes(),
            )))
            .expect("infallible chrome alias response");
    }

    not_found()
}

fn not_found() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(Full::new(Bytes::from_static(b"chrome asset not found")))
        .expect("infallible chrome 404 response")
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn serves_the_content_addressed_element_script() {
        let path = elohim_render::element_script_path();
        let resp = handle_chrome_asset(&path);

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("Content-Type").unwrap(),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(
            resp.headers().get("Cache-Control").unwrap(),
            "public, max-age=31536000, immutable"
        );

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body.as_ref(), elohim_render::element_js_bytes());
    }

    #[tokio::test]
    async fn enhance_path_resolves_to_the_same_element_bytes() {
        // Supersession: the legacy enhance path now resolves to the element.
        let path = elohim_render::enhance_script_path();
        let resp = handle_chrome_asset(&path);
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body.as_ref(), elohim_render::element_js_bytes());
    }

    #[tokio::test]
    async fn serves_the_stable_alias_with_a_revalidating_cache_contract() {
        // The alias `elohim-chrome-asset` publishes for references that cannot
        // embed a content hash. The storage sidecar has always served it; the
        // doorway must serve the same bytes under the same contract, or the
        // two runtimes linking one chrome crate disagree about its surface.
        let resp = handle_chrome_asset(elohim_render::STABLE_ELEMENT_PATH);

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get("Content-Type").unwrap(),
            "text/javascript; charset=utf-8"
        );
        // Revalidate, never immutable — the alias' bytes move with the element.
        assert_eq!(
            resp.headers().get("Cache-Control").unwrap(),
            "public, max-age=0, must-revalidate"
        );
        assert_eq!(
            resp.headers().get("ETag").unwrap(),
            &format!("\"{}\"", elohim_render::element_js_hash())
        );

        let body = resp.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(body.as_ref(), elohim_render::element_js_bytes());
    }

    #[test]
    fn the_stable_alias_is_the_path_the_chrome_crate_publishes() {
        // Pinned literally so a rename in the crate cannot silently un-serve
        // the address the Tauri index.html and the acceptance scenarios name.
        assert_eq!(
            elohim_render::STABLE_ELEMENT_PATH,
            "/chrome/omni-element.js"
        );
        assert_ne!(
            elohim_render::STABLE_ELEMENT_PATH,
            elohim_render::element_script_path(),
            "the stable alias must be a DIFFERENT path from the content-addressed one"
        );
    }

    #[tokio::test]
    async fn the_content_addressed_path_is_still_immutable() {
        // The alias arm must not have relaxed the immutable contract of the
        // content-addressed path — they are two different cache promises.
        let resp = handle_chrome_asset(&elohim_render::element_script_path());
        assert_eq!(
            resp.headers().get("Cache-Control").unwrap(),
            "public, max-age=31536000, immutable"
        );
        assert!(resp.headers().get("ETag").is_none());
    }

    #[tokio::test]
    async fn unknown_chrome_path_is_404() {
        let resp = handle_chrome_asset("/chrome/omni-element.deadbeef.js");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn unrelated_chrome_path_is_404() {
        let resp = handle_chrome_asset("/chrome/not-a-thing.js");
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn the_path_carries_the_hash_render_computed() {
        // The route's known path is exactly what elohim-render exposes, so the
        // referenced <script src> and the served route never diverge.
        let path = elohim_render::element_script_path();
        assert!(path.starts_with("/chrome/omni-element."), "{path}");
        assert!(path.ends_with(".js"), "{path}");
    }
}
