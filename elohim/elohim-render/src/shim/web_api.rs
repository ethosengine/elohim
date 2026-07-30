//! Web-platform API shim: `DOMException`, `AbortController`/`AbortSignal`,
//! `Headers`, `Request`, `Response`, `ReadableStream`, `Blob`.
//!
//! Pure JS, no ops — everything this surface needs (bytes, text codecs, timers)
//! is already provided by [`crate::shim::text`] and [`crate::shim::node_globals`].
//!
//! # Why the runtime needs it
//!
//! `deno_core` ships none of the web-platform globals; they live in `deno_web` /
//! `deno_fetch`, which this crate deliberately does not depend on. Angular's
//! `FetchBackend` (the SSR HTTP backend the server config pins via
//! `{provide: HttpBackend, useClass: FetchBackend}`) reaches for them on BOTH
//! sides of a request:
//!
//! - request path: `new AbortController()` runs synchronously inside
//!   `handle()`'s `Observable` subscribe function, and the request-timeout leg
//!   constructs `new DOMException('signal timed out', 'TimeoutError')`.
//! - response path: `new HttpHeaders(response.headers)` takes its fast path only
//!   for a real `Headers` instance, `response.headers.get('content-length')` /
//!   `get('content-type')` gate the size check and body parse, and the body is
//!   drained through `response.body.getReader()` / `reader.read()` /
//!   `reader.cancel()`.
//!
//! Before Angular 22 the backend only touched `globalThis.fetch`, so a bare
//! isolate got away with [`crate::shim::fetch`] alone. From 22 on, a missing
//! `AbortController` throws inside subscribe, the HttpClient `PendingTask` never
//! settles, and `ApplicationRef.whenStable()` never resolves — the render either
//! rides to the wall-time limit (an app with live polling timers) or dies with
//! "Promise resolution is still pending but the event loop has already resolved"
//! (an app with no live timers). A missing `ReadableStream` is quieter and worse:
//! every *successful* fetch yields a null HTTP body, so no render can carry
//! content.
//!
//! Ordering matters: this extension must be registered BEFORE
//! [`crate::shim::fetch`] in the runtime's extension list, because `fetch.js`
//! constructs a `Response` and reads `Headers` at call time and reaches for the
//! `streamOfBytes` helper this file installs.
//!
//! Scope discipline and the deliberate spec divergences are documented at the
//! head of `web_api.js` (no global `EventTarget`; combined header values; extra
//! `Response` init keys; queue-only `ReadableStream`; byte-container `Blob`).
//!
//! # WARNING: pure ASCII
//!
//! `web_api.js` is served through the `js = [dir ...]` macro, which panics at
//! const-eval on any non-ASCII byte. Keep every literal ASCII.

deno_core::extension!(
    web_api_ext,
    js = [dir "src/shim", "web_api.js"],
);
