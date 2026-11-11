# Host compatibility contract

`elohim-host-contract` (Rust library `host_contract`) is the pure comparison layer
(at `crates/elohim-host-contract`; every path in this README is relative to the
repository root) for its consumers: ark (`elohim/ark`, the envelope that launches a device's
programs), placement (`elohim/placement`, which decides which peer runs a task) and
the artifact store (`elohim/eprfs`, which shares build outputs between devices). It has no host discovery, I/O,
launch, fetch, authorization or capacity accounting. Its one first-party
dependency is `elohim-epr` (`elohim/epr`, the protocol's core record crate), for
the `Verdict` type (`elohim/epr/src/verdict.rs`) and the CID helpers.
The design spec is `genesis/docs/superpowers/specs/2026-11-10-host-compatibility-foundation-design.md`.

Axes are open, namespaced strings; a new axis needs no code change here. The spec's
initial axes (§3.1) are `arch`, `os`, `isa.x86`, `isa.arm`, `isa.riscv`,
`kernel.linux`, `abi.libc`, `abi.loader`, `lib`, `accel.<api>` and `engine.wasm`;
only the x86-64 Linux rows are proven so far.

Build `Requires` with `Requires::try_from(Vec<Clause>)`. Clauses name an open
`AxisId`, a `Predicate` (`Equal`, `Subset`, `AtLeast`, `Present`) and a
`ClauseSource` saying where the requirement was learned: `Declared` (named by the
build's inputs), `Extracted` (read from the artifact's bytes) or `Proven` (the
exact bytes ran on a host of that class). Merging unions feature/name sets and takes the higher numeric
version within one named line. Conflicting scalar values, kinds or version lines
return `MergeError`; handle that error before admitting an artifact. Original
evidence and declared/extracted disagreements survive merging and serialization.

Construct `Provides::default()`, then call `observe(axis, value)` for actual
observations or `mark_unobserved(axis)` for recognized axes that could not be
measured. A failed observation removes a stale value. `Provides` implements
neither Serialize nor Deserialize; synthetic test hosts use a separate fixture
shape. This crate never observes a host: reading CPU flags, the loaded libc and
loader paths is done by the caller on the device that will run the bytes (for
ark, `elohim/ark/supervisor/src/driver.rs`). Map Linux `/proc/cpuinfo` spellings
onto the psABI (processor-specific ABI: the x86-64 supplement to the System V ABI, which
defines the microarchitecture levels v2, v3 and v4) names used here. For the shipped x86-64 bundles exactly two
differ:

| `/proc/cpuinfo` flag | name used here |
|---|---|
| `pni` | `sse3` |
| `abm` | `lzcnt` |

Every other bundle feature has the same name in both.

`admit(&requires, &provides)` returns `elohim_epr::verdict::Verdict` from the
protocol's core record crate (`elohim/epr`, package `elohim-epr`), with one check per merged
axis. An unknown or unobserved axis refuses; an empty requirement permits.
Each check's `observed` object contains `required`, original `evidence`, `host`
and an optional typed `refusal`. Keep this host-bearing verdict local: disclose
only its effect, such as a selected variant. Compatibility does not grant
execution permission or prove fitness for wider reach.

`X86_64_V2.to_bundle()` and `X86_64_V3.to_bundle()` create the shipped psABI
feature documents. `Bundle::cid()` addresses the name, axis and sorted features
using `elohim-epr`'s canonical DAG-CBOR (IPLD's deterministic CBOR) encoder (`elohim_epr::cbor`) and its
record CID helper (`elohim_epr::cid`), so every peer derives the same CID.

From the repository root:

```sh
just gate host-contract
```

The gate runs `cargo fmt --check`, clippy with `-D warnings` and `cargo test` in the
cargo target slot its `build-manifest.json` names, on the repository's pinned
Rust toolchain (1.96.1, edition 2021). Success means
passing contract tests, vector tests and the `compile_fail` doctests, which prove that
serializing `Provides` does not compile.

To add a synthetic conformance case, copy a JSON file in `tests/vectors/`. Every
`*.json` there is loaded automatically. Required fields are `requires` (an array
of original clauses), `provides` (`observed` axis map and `unobserved` axis list),
and `expected` (`{"type":"permit"}` or `{"type":"refuse"}`). Refusing cases
include `expected_refusals` in axis order. Optional `merge`, `expected_merged`
and `expected_disagreements` exercise conjunction. Keep real device inventories
out of fixtures; a sibling device may contribute extracted artifact requirements
against a synthetic host. See `tests/vectors/declared-extracted-floor.json` for
the merge format. In the design spec, §3 is the contract, §4 says why `Provides`
stays on the device and §10 lists the consumers that call `admit`.
