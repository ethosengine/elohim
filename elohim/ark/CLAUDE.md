# ark — the compute envelope

Tevah in prose, `ark` in code. No crate, type, module, or path here may contain
`tevah`, `pod`, or `seed`. Spec:
`genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md`
(§3 manifest/berth, §5.1 verdicts, §6 witness path, §8 boundaries, §11 S0).

## Three crates, one boundary

| dir | package (lib/bin) | may depend on |
|---|---|---|
| `core/` | `elohim-ark-core` (`ark_core`) | pure data + decisions only |
| `supervisor/` | `elohim-ark-supervisor` (`ark_supervisor`) | core + `nix`/`libc`; I/O, **no network** |
| `cli/` | `elohim-ark` (bin `ark`) | core + supervisor + `clap` |

`ark-core`'s dependency graph **is** the purity boundary: `boundary::no_runtime_or_io_deps`
(in `core/src/lib.rs`) reads the crate's own `Cargo.toml` and refuses `tokio`, `nix`, `libc`,
`diesel`, `rusqlite`, `libp2p`, `iroh`, `reqwest`, `hyper`, `axum`, `hdk`, `hdi` — a pure decision
must never be able to do I/O; a second guard, `boundary::no_fs_in_pure_core`, greps the crate's own
sources for `std::fs`/`File::`/`Sidecar*Store` — ark-core takes `elohim-epr-rea` with
`default-features = false` (the model, never the sidecar store; only the supervisor writes a disk).
Ontology is INHERITED, never re-minted (`rea.rs`): VF intent/event/process projections, a `Bound`
on the self-contract, algedonic breach evidence, `Stock`/`Window`. Identity math is never re-derived: CIDs come from
`elohim_epr::cid::compute_cid` over `serde_ipld_dagcbor::to_vec`, never local sha-to-cid code.

## Declared S0 simplifications

- **CIDs inside records are strings** (base32 `bafy…`); the manifest becomes an `Epr` payload in S1.
- **Threads, not tokio**; tokio arrives with the admin socket in S2. `tokio::process` is refused —
  children are spawned with `std::process::Command` and reaped by our own reaper.
- **`Native` driver only**; `effective_tier` is `None` until enforcement lands.
- **`amber-local` spool only** — a content-addressed LOCAL projection under `<data_root>/ark/` whose
  CIDs the DHT anchors in S1; the sidecar `flows.jsonl` is the same valueflow mechanism `epr flow` reads.

## Running it

```
cd /projects/elohim/elohim && CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim/dev RUSTFLAGS="" cargo test -p elohim-ark-core -p elohim-ark-supervisor -p elohim-ark
just gate elohim-ark
MESH_CONDUCTOR_LAUNCH=ark ARK_BIN=<path-to-ark> just mesh start
```
