//! `GET /chrome/*` — doorway-specific runtime chrome static assets.
//!
//! The native EPR omnibar (composed in `elohim-render` and spliced around the
//! V8-rendered Angular body by the `ComposingRenderer`) is progressively
//! enhanced by a small hand-written `omni-enhance.js`. That script is baked into
//! the `elohim-render` crate and content-addressed by its `sha256`; this route
//! serves it.
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
/// Only the content-addressed enhance script is known:
/// `GET /chrome/omni-enhance.{sha256}.js` → the script bytes (200, immutable).
/// Any other `/chrome/*` path (including a stale/mismatched hash) → 404.
#[must_use]
pub fn handle_chrome_asset(path: &str) -> Response<Full<Bytes>> {
    if path == elohim_render::enhance_script_path() {
        return Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/javascript; charset=utf-8")
            // Content-addressed ⇒ immutable. A changed script changes the hash
            // (and therefore the path), so the bytes at THIS path never change.
            .header("Cache-Control", "public, max-age=31536000, immutable")
            .body(Full::new(Bytes::from_static(
                elohim_render::enhance_js_bytes(),
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
    async fn serves_the_content_addressed_enhance_script() {
        let path = elohim_render::enhance_script_path();
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
        assert_eq!(body.as_ref(), elohim_render::enhance_js_bytes());
    }

    #[tokio::test]
    async fn unknown_chrome_path_is_404() {
        let resp = handle_chrome_asset("/chrome/omni-enhance.deadbeef.js");
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
        // spliced <script src> (Task 3) and the served route never diverge.
        let path = elohim_render::enhance_script_path();
        assert!(path.starts_with("/chrome/omni-enhance."), "{path}");
        assert!(path.ends_with(".js"), "{path}");
    }
}
