//! deno_core extension shims (console, URL, TextEncoder/TextDecoder, fetch).
//!
//! WARNING: All `.js` files in this directory must be pure ASCII. The
//! deno_core `js = [dir "...", "..."]` macro uses `ascii_str_include!`
//! internally, which panics at const-eval time on any non-ASCII byte
//! (em-dashes, curly quotes, etc.) with a cryptic `assertion failed:
//! buffer.is_ascii()` error. If you need a non-ASCII character in a
//! comment, escape it: `&#x2014;` for em-dash, `&#x2018;`/`&#x2019;` for
//! curly quotes.
//!
//! NOTE on web_api: `web_api.js` hand-rolls the web-platform globals
//! `deno_core` does not ship (`DOMException`, `AbortController`/`AbortSignal`,
//! `Headers`, `Request`, `Response`, `ReadableStream`, `Blob`) because Angular's
//! `FetchBackend` touches them on both the request and the response path. It must
//! be registered BEFORE `fetch_ext`, which builds a `Response` at call time. See
//! [`web_api`] for the mechanism and `web_api.js`'s header for the deliberate
//! spec divergences.
//!
//! NOTE on URL: the hand-rolled `url.js` covers the WHATWG surface that
//! Angular HttpClient and Universal SSR exercise (constructor, accessors,
//! URLSearchParams, IPv6, default-port elision). If Angular pushes
//! beyond this -- username/password, opaque origins, IDN -- consider
//! replacing with the `whatwg-url` npm polyfill (~50KB) for full
//! spec compliance.

pub(crate) mod console;
pub(crate) mod fetch;
pub(crate) mod loader;
pub(crate) mod node_buffer;
pub(crate) mod node_builtins;
pub(crate) mod node_crypto;
pub(crate) mod node_globals;
pub(crate) mod text;
pub(crate) mod url;
pub(crate) mod web_api;
