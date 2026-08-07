# Vendored `holochain_serialized_bytes` 0.0.56

Third-party crate, vendored **verbatim** except for a single dependency-pin line.
`src/lib.rs` and `src/prelude.rs` are byte-identical to the crates.io release.

## Why this exists

Landed 2026-08-07 to unblock the alpha iroh admin-seam partition.

The forked iroh conductor (`elohim/holochain-conductor`, tag
`fixt-0.6.3-6-g6d0814266`) emits `relay_url` in the `AppManifestV0` wire shape
carried inside every `AppInfo`. Upstream renamed `signal_url` -> `relay_url` in
the 0.7 line at `holochain_types 0.7.0-dev.23`. `AppManifestV0` is
`#[serde(deny_unknown_fields)]`, so any client older than that rejects **every**
`list_apps` / `install_app` / `enable_app` response:

```
Deserialize("unknown field relay_url, expected one of name, description, roles,
 allow_deferred_memproofs, bootstrap_url, signal_url")
```

That takes down the whole storage<->conductor admin seam fleet-wide.

The fix is to move `holochain_client` to `>= 0.9.0-dev.24` (which requires
`holochain_types >= 0.7.0-dev.23`). That bump was blocked by a pair of mutually
exclusive **exact** `serde` pins:

| chain | resolves to |
|---|---|
| sdk domain-types (`lamad-types`, `imagodei-types`, …) -> `holo_hash =0.6.0` -> `holochain_serialized_bytes =0.0.56` | `serde =1.0.219` |
| `holochain_client >=0.9.0-dev.24` -> `holo_hash 0.7.0-dev.9` -> `holochain_serialized_bytes =0.0.57` | `serde =1.0.228` |

Only one `serde` 1.x can exist in a graph, so the two are unresolvable. This is
the "Lane C hold" recorded in `doorway/doorway-service/Cargo.toml`.

The `holo_hash = "=0.6.0"` pins in `elohim/sdk/domains/*/types` are **not**
incidental — those crates are also consumed by the DNA zome workspaces
(`elohim/holochain/dna/*/zomes/*`), where `holo_hash` is compiled into integrity
WASM. Relaxing them is the one thing that could silently move a DNA hash and
re-key the fleet. So the pin stays; the vendored crate moves instead.

## The change

One line. `serde = "=1.0.219"` becomes `serde = "=1.0.228"` — 0.0.57's pin.
Both `hsb` versions then agree on `serde`, coexist, and the client bump resolves.
`rmp-serde` (the other exact pin, and the one that actually governs MessagePack
wire bytes) is `=1.3.0` in both 0.0.56 and 0.0.57 and is untouched.

This does not loosen the pin discipline: `serde` remains exactly pinned, to the
version the rest of the holochain family already ships against.

## Scope

Applied via `[patch.crates-io]` in the `elohim-storage` and `doorway-service`
workspace roots **only**. The DNA zome workspaces have their own Cargo.lock files
and are unaffected — `holo_hash` stays at `0.6.0` there, so no DNA hash moves.

## Retiring this

Delete the vendor directory and both `[patch.crates-io]` stanzas once the sdk
domain-types crates move off `holo_hash =0.6.0` (the convergence campaign's
Wave-3 family move to the 0.7 finals). At that point only `hsb 0.0.57` remains in
the graph and the conflict is gone.
