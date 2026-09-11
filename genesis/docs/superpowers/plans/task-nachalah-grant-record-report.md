---
id: nachalah-grant-record-report
title: Exact notarized grant record authentication
status: DONE_WITH_CONCERNS
class: protocol-canonical
gap: nachalah-supervised-activation#1b-a
actor: agent:implementer@gpt-6
habits: [runtime-upgrade-propagation]
commits: []
cites:
  - "nachalah-supervised-activation-sprint-handoff | Nachalah next sprint | sha256:ce355b7759e4ed56 | path: genesis/docs/superpowers/plans/2026-09-05-nachalah-supervised-activation-sprint-handoff.md"
---

Exact signed grant record authentication is verified for #1b-a only. Parent #1b
and #1b-b remain open; this result grants no activation authority.
No files staged or committed. Base: `7f95ac47fda5ee3f6f5176b747145a76aa9e19f1`.

The coordinator now resolves one exact ActionHash and returns its existing signed
Record, checking Create, the scoped Commitment entry definition, public app-entry
presence, entry-size budget, and recomputed action/entry identities. Legacy
EntryHash readback remains available; its one-record-per-entry claim is corrected.

The storage adapter reuses HcClient's Mishpat cell. Acting-node verification
requires independently provisioned action hash, entry hash and Holochain author,
checks a 256 KiB wire budget before decode/hash/crypto, recomputes both identities,
and verifies the actual Ed25519 signature on Holochain serialized Action bytes.
It returns private-field, read-only authenticated content without Deserialize.
No dependencies added; ark's pure core is untouched.

Independent signing fixtures use two real keys. Adversarial tests cover identical
entry bytes from a different author, all independent pin mismatches, mutated
action/entry/signature/cached hash, wrong zome/type/visibility, Update/Delete,
missing entries, malformed and oversized wire input. A separate source-backed
contract detects Commitment entry-index or integrity-zome ordering drift.
The eighth test covers correctly signed but malformed Commitment content; all
eight passed inside the final full owning gate.

Gate evidence:

- `env CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev RUSTC_WRAPPER= cargo test --manifest-path elohim/elohim-storage/Cargo.toml --lib services::commitment_record -j1` — `EXIT=0`; 7 tests passed, 3564 filtered out, 2m48s compile. Log: `/tmp/nachalah-grant-record-tests.log`.
- Initial focused compile — `EXIT=101`: current Holochain 0.7 EntryHash uses `with_data_sync`, not `with_data(...).await`; corrected before the passing rerun.
- `PATH=/tmp/nachalah-bin:$PATH just gate manifest-hygiene` — `EXIT=0`; 9 tests passed. Log: `/tmp/nachalah-grant-manifest-gate.log`.
- `PATH=/tmp/nachalah-bin:$PATH just gate schema-dna` — `EXIT=0`; all DNA checks passed. Log: `/tmp/nachalah-grant-schema-gate.log`.
- Both owning seam registries validated against `seam-registry.schema.json` with Python jsonschema — `EXIT=0`.
- `PATH=/tmp/nachalah-bin:$PATH just gate elohim-storage` — `EXIT=1`, intentionally interrupted while waiting for another session's target lock before our full tests started. Log: `/tmp/nachalah-grant-storage-gate.log`. Migration hygiene, formatting and clippy passed. Only our waiting cargo PID 2940340 was terminated to prevent overlapping full suites; the other session's process was untouched. Berth released; the subsequent non-overlapping rerun below passed.
- Final non-overlapping `PATH=/tmp/nachalah-bin:$PATH just gate elohim-storage` — `EXIT=0`; migration hygiene, formatting, clippy and full test execution passed. Across 169 result batches: 4386 passed, 0 failed, 59 ignored (including docs). Unit batch: 3569 passed, 0 failed, 3 ignored; all 8 grant-record tests passed. Build took 17m55s with normal I/O-guard pauses; no guard was bypassed. Log: `/tmp/nachalah-grant-storage-gate-final.log`.
- `env RUSTFLAGS= RUSTC_WRAPPER= CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__holochain__dna__mishpat/dev cargo test --manifest-path elohim/holochain/dna/mishpat/Cargo.toml -p mishpat commitment_record -- --nocapture` — `EXIT=0`; 1 exact-read coordinator shape test passed, 66 filtered out (17.07s compile). Log: `/tmp/nachalah-grant-mishpat-tests.log`.
- `PATH=/tmp/nachalah-bin:$PATH just gate sweettest-check` — `EXIT=0`; manifest-selected test compilation completed (4.23s). This is the declared compile-only gate; actual execution is proved by the focused coordinator test and storage suite above. Log: `/tmp/nachalah-grant-sweettest-check.log`.
- Root preserved-prerequisite app gate — `EXIT=0`: lint, dependencies, Angular build, 223 files / 4570 tests. Log: `/tmp/nachalah-continuation-app-gate.log` (reported by coordinating agent).

`epr flow context` reports no storage gate, but the authoritative
`elohim/holochain/build-manifest.json` registers `elohim-storage`; the gate-runner
resolved that owner. DNA context names manifest-hygiene/schema-dna/sweettest-check.

C0–C14 are registered for the verifier, authenticated boundary, exact-read verdict
and private shape predicate. C12 full authority remains explicitly unbound at
#1b-b: issuer/recipient/scope/validity, identity lineage and fresh non-revocation
are not authenticated here. Counters, residual RCA and supervisor wiring remain
open. Candidate Agent EPR CID and Holochain author key remain distinct namespaces;
independent provisioning does not imply a binding between them. This prerequisite
never confers activation permission or claims fresh non-revocation. Parent #1b,
activation, automatic rollback and both household upgrade runs remain open.
Earned-arc actuation remains disabled. Independent source review reported no
actionable issue through the coordinating agent; final review disposition remains
with the root reviewer. All cargo leases were released after verification.
