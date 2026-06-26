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
//! new path, so we serve `Cache-Control: public, max-age=31536000, immutable`.

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};

/// Serve a `/chrome/*` static asset.
///
/// Only the content-addressed omnibar element is known:
/// `GET /chrome/omni-element.{sha256}.js` → the element bytes (200, immutable).
/// (The legacy enhance path resolves to this same content-addressed path.)
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
