---
epr-meta-version: 1
id: elohim-error-governance
covers: subtree
purpose: >
  The storage error vocabulary — one `StorageError` enum and the four `From` conversions that
  feed it, carved out of `elohim-storage/src/error.rs` as the bottom crate of the storage stack.
  It refuses to know what a service, a connection, a route, a transport or a runtime is: the only
  foreign types it names are the four whose failures it converts (diesel, std::io, serde_json,
  sled). That refusal is held by `tests/boundary.rs` over this crate's own lockfile, not by a
  rule here — cargo is the layering rule, so this manifest does not restate it.
rules:
  - id: rs-loc-ceiling
    policy: source-file-loc-ceiling@1
cites:
  - "storage-crate-decomposition-design | Why this crate exists — §4 row 1 is elohim-error, §5 its proof | sha256:90ddbd59cbfea650 | path: genesis/docs/superpowers/specs/2026-09-19-storage-crate-decomposition-design.md"
---
# elohim-error — governance package

This crate is the first extraction of the storage decomposition, and its whole discipline is
subtractive: it exists to be depended on, so anything added here is paid for by every layer
above. The boundary that matters is structural — the dependency graph, asserted transitively by
the lockfile boundary test — which is why the one rule bound here is the LoC ceiling rather than
a layering rule cargo already enforces.

The drift this tree can actually produce is growth: a `String` variant is cheap and correct,
while a new `From<ForeignError>` pulls its crate down the stack and must land here because of
Rust's orphan rule. The ceiling is the observation that catches the first shape; the boundary
test catches the second. Read the design spec cited above for why this crate sits where it does
and what follows it.
