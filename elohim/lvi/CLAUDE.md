# CLAUDE.md — lvi

Guidance for Claude Code (and any superpower agent) working in the `lvi` module. Keep this gospel
short — architecture and invariants here, dated design + scope in `docs/specs/`, task breakdown in
`docs/plans/`. Governance rules that fire at edit-time live in `.epr-meta` (this dir is a
`covers: subtree` region; it also inherits the `elohim/.epr-meta` interface-first-reuse gate).

## What lvi is

**lvi** (льви — Ukrainian for *lions*; Lviv, the city of the lion) is the Elohim Protocol's
**devspace peer-runtime**: it runs P2P-sharable development environments (openvscode-server in a
browser, preview URLs through a doorway, stewarded semi-ephemerally on a peer's storage). It is the
P2P-native successor to Eclipse Che — which Ukrainian engineers built — carried forward as a mesh
of peers that host each other's environments.

The governing thesis is **k8s *powers* re-derived over p2p, not k8s ported onto a DHT.** A devspace
is not an image you build and push; it is a **covenant you mount** — a content-addressed manifest
that lives cold as CIDs, warms onto a device on demand, and is torn down when idle while its
manifest persists forever. Design (dated): `docs/specs/2026-07-20-elohim-native-devspace-design.md`.

## Boundary & dependency direction

lvi sits *atop* its siblings and **consumes** them — it must never re-implement what they own:

`lvi → { brit (covenant + source-closures), rakia (build), eprfs (mount/materialize), doorway (ingress) }`

- **Do not re-implement content addressing / CID / dag-cbor.** Consume `eprfs-core::BlobCid` and
  `elohim/epr`. (The `elohim/.epr-meta` interface-first-reuse gate cascades here too.)
- **Do not re-implement image/layer/pull machinery.** The environment is *mounted*, not *shipped*:
  consume eprfs `ProjectionManifest` + `LocalMaterializer` (`Sparse` / `FetchMissing`). "Pull" is
  not a concept lvi owns.
- **Do not embed build-graph semantics.** Dependency-graph walk and build derivation belong to
  rakia (`rakia-executor` is the deferred artifact plane).
- **Do not embed git/covenant semantics.** Source-closures, `NodeSeed`, and provenance belong to
  brit.
- **Do not make the doorway lvi-aware beyond a generic host projection.** Ingress is a doorway
  `HostRegistry` host + generic WS/HTTP passthrough; lvi registers, the doorway routes.

lvi's own crates (`lvi-core`, `lvi-seed`, `lvi-actuator`, `lvi-ingress`, `lvi-cli`) own only the
*composition*: the `DevspaceSeed` runtime shape, the lifecycle state machine, process supervision +
sandbox containment, and the doorway-registration glue.

## Load-bearing invariants (the sharp edges; `.epr-meta` reminds you of these at edit-time)

1. **Co-resident safety is non-negotiable.** A devspace runs a trusted peer's *unvetted* code on a
   host that is also a live protocol participant. The sandbox MUST carry a hard resource quota
   (`--memory --cpus --pids-limit`, disk ceiling, `--network=none` by default) that isolates the
   devspace from the co-resident conductor/DHT. A fork-bomb / disk-filler / memory-hog inside a
   devspace must be quota-killed **without** endangering the host's participation. This is the
   safety floor the containment scenario exists to prove.
2. **Trust-graph admits; sandbox contains; TTL bounds — never conflate them.** Reach + attestation
   + a `delegates-compute` commitment decide *whose* code you host (admission, not isolation). The
   sandbox contains an already-chosen peer. The TTL bounds the window. Revocation is **lease-expiry,
   not interrupt** — do not describe or design it as SIGKILL-on-signal.
3. **Mount, don't ship.** The image dies as a unit; the closure converges onto disk exactly as far
   as execution demands (sparse mount, hydrate on touch, verify-by-hash before it lands).
4. **The devfile is an EPR Derivation, not a Dockerfile and not Nix.** Steal Nix's derivation math
   (input-CIDs → output-CIDs) and nothing else — no `/nix/store`, no stdenv, no DSL. **Input-address
   for dedup; output-diverge for trust** (convergence is a reach-earning attestation, not a
   correctness gate).
5. **Authorization is a Mishpat::Commitment `delegates-compute`**, validated by the copy-of-
   `replicates-commons` `bounds_validator` — never a bespoke token/grant. It displaces X-API-Key.
6. **Derived bytes never enter commons custody.** Source edits seal to a brit `NodeSeed`; build
   caches (`target/`, `node_modules`, sccache) are a steward-affinity lease or re-derived — never
   commons blobs. Stateful-service data is refuse-by-default unless a `persistent` set is declared
   *and verified* on teardown.

**Reach-as-promotion** (a control plane with no privileged observer; deployment is *earned reach*,
not `kubectl apply`) is the north star lvi is designed toward; its convergence mechanism is a
deferred track, not a v1 obligation.

## Discipline

- **Story/spec-first.** Find or write the scenario, decompose the spec (`docs/specs/`) into a plan
  (`docs/plans/`) before implementing. Dated filenames (`YYYY-MM-DD-*.md`), plain-header specs in
  the brit/rakia voice (`# Title` · `**Date:**` · `**Status:**` · `**Author:**`).
- **Self-contained toward submodule.** lvi is designed to graduate to its own `ethosengine/lvi`
  repository. Keep everything the module needs *inside* `elohim/lvi/` (its own docs, governance,
  CLAUDE.md); do not reach into monorepo meta for lvi-specific state.
- **Native build hygiene.** Set `CARGO_TARGET_DIR` to the pool slot; `RUSTFLAGS=""` for native
  crates. lvi is plain cargo (no WASM getrandom flag).
- **Commit-only.** In autonomous work, commit on the shift branch; the integrator is the single
  push/merge authority.

## Governance

This directory is an `.epr-meta` `covers: subtree` governance region (inheriting the
`elohim/.epr-meta` interface-first-reuse gate): the edit-time compose-gate surfaces the invariants
above when you touch the code surfaces they guard. The rules
degrade to teach-never-block advisories until a validator-EPR is registered — the reminder is the
point.
