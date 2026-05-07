//! deno_core extension shims (console, URL, TextEncoder/TextDecoder, fetch).
//!
//! WARNING: All `.js` files in this directory must be pure ASCII. The
//! deno_core `js = [dir "...", "..."]` macro uses `ascii_str_include!`
//! internally, which panics at const-eval time on any non-ASCII byte
//! (em-dashes, curly quotes, etc.) with a cryptic `assertion failed:
//! buffer.is_ascii()` error. If you need a non-ASCII character in a
//! comment, escape it: `&#x2014;` for em-dash, `&#x2018;`/`&#x2019;` for
//! curly quotes.

pub(crate) mod console;
pub(crate) mod fetch;
pub(crate) mod text;
pub(crate) mod url;
