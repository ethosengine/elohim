---
epr-meta-version: 1
id: elohim-settings-governance
covers: subtree
purpose: >
  The storage node's settings layer — the boot `Config` and the watched runtime-config
  registry, which are mutually recursive and were carved out of
  `elohim-storage/src/config.rs` + `src/runtime_config.rs` as one unit (step 3 of the storage
  decomposition). It refuses to know what a service, a transport, a database, a route or an
  ASYNC RUNTIME is: the watcher's decisions live here and are synchronous `std`, while its
  clock (`tokio::spawn` + `tokio::time::interval`) lives one layer up in
  `elohim_storage::runtime_config_watch`. That refusal is held by `tests/boundary.rs` over
  this crate's own lockfile, not by a rule here — cargo is the layering rule, so this manifest
  does not restate it.
rules:
  - id: rs-loc-ceiling
    policy: source-file-loc-ceiling@1
cites:
  - "storage-crate-decomposition-design | Why this crate exists — §4 row 3 is elohim-settings, §8a its cut list and order, §5 its proof | sha256:bfd72116ad75dd41 | path: genesis/docs/superpowers/specs/2026-09-19-storage-crate-decomposition-design.md"
---
# elohim-settings — governance package

This crate is the third extraction of the storage decomposition, and the first whose boundary
is about a *runtime* rather than a dependency weight. Everything above it — services, both
transport stacks, the reconcile controller, the HTTP boundary — reads its config; if a settings
layer took tokio for the sake of one poll loop, it would colour all of them with that choice.
So the poll loop stays above and this crate stays synchronous, held by the lockfile boundary
test.

The drift this tree can actually produce is a lever that reports "applied" and changes nothing.
Three shapes of it have already shipped or nearly shipped: a publisher with no call site
(2026-09-19, twice), a boot value published to the wrong key, and a knob registered as hot whose
read site captured it once at spawn. The first two are caught by the boot assertion's two halves
— `BOOT_PUBLISHERS` × `main.rs` statically, `Setting::published` ×
`assert_boot_published()` at boot — and the third by the honesty of the `BOOT_ONLY` list. The
one rule bound here is the LoC ceiling, because `config.rs` is where per-knob documentation
accretes and god-file growth is the other drift this tree produces.

Read the design spec cited above for why this crate sits where it does, what moved with it, and
what follows it.
