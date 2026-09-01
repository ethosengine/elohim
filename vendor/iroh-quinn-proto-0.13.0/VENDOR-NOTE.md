# Vendored `iroh-quinn-proto` 0.13.0

Third-party crate vendored from the crates.io release. It is byte-identical to
the published crate except for the one-line GSO/tail-loss-probe correction
described below and this note.

## Why this exists

On 2026-08-31, a view-federation burst followed by an abruptly disappearing
peer triggered this assertion in a fleet storage process:

```text
assertion failed: untracked_bytes <= segment_size as u64
```

The panic poisoned iroh-quinn's connection mutex; the destructor then panicked
and aborted the process. The affected dependency is `iroh-quinn-proto 0.13.0`,
resolved by the storage crate's deliberately frozen iroh 0.92 family.

Quinn issue #2127 reports the same assertion and abort shape. The reporter and
maintainer confirmed that Quinn PR #2167 fixes it. That fix is commit
`434c35861e68aac1da568bcd0b1523603f73f255`:

- <https://github.com/quinn-rs/quinn/issues/2127>
- <https://github.com/quinn-rs/quinn/pull/2167>
- <https://github.com/quinn-rs/quinn/commit/434c35861e68aac1da568bcd0b1523603f73f255>

iroh 0.93 and 0.94 still resolve the same immutable
`iroh-quinn-proto 0.13.0` release, so a superficial minor bump does not carry
the fix. iroh 1.x replaces this dependency family, but that is a broader API
and Holochain-family migration assigned to Wave 3. Disabling GSO would also
avoid the path, but would discard its bulk-transfer efficiency fleet-wide.

## The change

One line, matching upstream commit `434c3586` exactly. When a tail-loss probe
is appended to a GSO batch, its buffer allowance is clamped to the smaller of
the current segment size and `INITIAL_MTU`:

```rust
std::cmp::min(segment_size, usize::from(INITIAL_MTU))
```

This prevents the partial packet from exceeding the GSO segment size and
invalidating the `untracked_bytes <= segment_size` invariant. No public API,
ALPN, framing, payload cap, or wire type changes.

## Verification

Dependency work is verified with real tests, never `cargo check` alone:

- the feature-enabled `just test-iroh` suite;
- `just gate elohim-storage`;
- the existing view-federation deployed-reader-floor/MAX_PAYLOAD guards.

`cargo tree -i iroh-quinn-proto --features p2p-iroh` must resolve this path
dependency, not the crates.io checksum entry.

## Retiring this

Delete this directory and the storage `[patch.crates-io]` stanza when the
Wave-3 iroh-family migration removes `iroh-quinn-proto 0.13.0`, or when a
compatible published iroh release demonstrably carries the correction.
