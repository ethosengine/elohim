# eprfs

`eprfs` is the native Elohim filesystem projection layer.

It does not replace `elohim-storage`, and it is not a `bridges/` adapter.

- `elohim-storage` owns EPR records, blobs, custody, replication, DHT/notary, and transport.
- `eprfs` owns local filesystem projection semantics: paths, sparse materialization, byte presence, projection manifests, and writeback/attestation seams.
- Consumers such as `brit` own domain interpretation: git commits, trees, blobs, refs, build attestations, and repository reach.

The intended composition is:

```text
brit / other domain adapters
  -> eprfs projection contract
  -> elohim-storage data plane
  -> iroh/libp2p/Holochain substrate
```

## Crates

- `eprfs-core` — pure model + traits. No git, no FUSE, no storage HTTP client.
- `eprfs-host` — host filesystem capability profiles for Linux, macOS, Windows, portable directories, and peer-managed projections.
- `eprfs-local` — materializes projection manifests into ordinary filesystem trees.
- `eprfs-meta` — parses, resolves, and natively evaluates authored `.epr-meta` cascades and version-pinned policies.
- `eprfs-storage` — storage-facing adapter scaffolding and test doubles for `elohim-storage` integration.
- `elohim-epr-cli` — the `epr` onboarding, local coherence, path explanation, and reach-readiness interface.

## Repository Governance CLI

The native `epr` tool keeps two moments distinct:

- `epr check` advises on local working-tree authoring and always leaves local agency intact.
- `epr ready` evaluates the committed change set that is asking for reach and exits non-zero when a governance-floor finding blocks that boundary.

For repository contributors and adapter developers: start at the **Elohim
repository root**, with Git, a native Rust toolchain compatible with this
workspace, and the repository's Cargo registry access configured. The checked-in
pre-push gate also uses Python 3, Node.js, pnpm and Just. First contact is local:

```bash
env RUSTFLAGS= CARGO_TARGET_DIR=/tmp/eprfs-onboarding-target \
  cargo run --manifest-path elohim/eprfs/Cargo.toml -q -p elohim-epr-cli -- setup
# Make the binary just built available in this shell; no global installation.
export PATH="/tmp/eprfs-onboarding-target/debug:$PATH"
epr doctor
```

`setup` only points this clone's local Git configuration at the checked-in
`.husky/pre-push` gate. It does not publish, install global state, or change the
working tree. The remaining commands are `doctor`, `explain <path>`, `check`,
and `ready [--target <ref>] [--deep]`; every report is also available as JSON.

## Native feature entry and repository attribution

A repository declares its own flow container in `.epr-meta/repository.yaml`:

```yaml
version: 1
agent: repo:example/project
```

This is local attribution, not a human identity or network authority. Flow writes
refuse missing or malformed declarations. Elohim declares its historical
`repo:ethosengine/elohim` value so existing record addresses remain unchanged;
Brit declares its own value. No remote URL or sibling checkout supplies identity.

A **recipe** declares intended work and its steps; a **habit** names a behavior
with a runnable check; a **commitment** records an actor's claim on a step.
Projecting recipes creates or refreshes private local work records, without
claiming or accepting work.

From the Elohim repository root, these are existing inputs, not placeholders:

```sh
epr flow project --root . --recipes .claude/epr-meta/recipes.yaml
epr flow context genesis/docs/superpowers/plans/2026-09-28-brit-governed-readiness.md --root .
epr flow context genesis/docs/superpowers/plans/2026-09-28-brit-governed-readiness.md --root . --json
```

For other work, replace the plan path with an existing plan or
`.epr-meta/<id>.habit.md` in that repository. Successful context output identifies
the target, its applicable gate and the evidence or unresolved work it finds;
its `actionable` section names the next useful step. Missing evidence is a
reported state, not a promise that the feature is ready.

`brit context` delegates to this same installed evaluator. The additive
`actionable` section explains covenant rank, authored order, current owners,
environment/seal blockers and the next claim, resume, review or revalidation
step. Missing identity/habit evidence is explicit. A habit atom as the target
follows its declared `refs` and accounted commitments, with a disclosed scope
limit. Context reads never claim work; source changes and rejected reviews
remain visible independently of production and acceptance.

Habit declarations under each directory's `.epr-meta` are authoritative and
repository-local. The parent does not walk submodule habits. Generated legacy
registers remain a compatibility input for undeclared archives, not a fallback
that can resurrect a deleted modern declaration. `.eprfs/status` carries private
local projections and flow records; keep it out of version control. Bounded
memory recall can find further sources; similarity does not grant acceptance.

## Host Profiles

`eprfs` must be able to collapse a projection onto many host filesystems without
making the host the source of truth. `eprfs-host` models what the target surface
can preserve: native symlinks, executable bits, case sensitivity, xattrs, atomic
rename, and whether the directory is peer-managed.

Peer-managed projection directories are the Elohim-native analogue to familiar
sync-folder clients: they expose normal files where possible and sidecar markers
where the host cannot preserve projection semantics.

## Projection Source Identity

Projection entries may carry a domain-neutral source identity:

- `content` — byte-bearing content such as a file blob.
- `container` — an object that organizes child entries, such as a tree.
- `link` — link content whose bytes name another path.
- `external` — a boundary to another resource, repository, or projection.

Domain adapters decide the namespace and source id. For example, `brit` uses
git object ids, but `eprfs-core` only validates the projection shape.

## EPR Meta Head Coupling

`.epr-meta` is the authored source form for EPR meta around a path or subtree.
`eprfs` treats it as the seed of a broader head-coupling model, not merely as
directory-local governance.

An EPR meta record can carry:

- `story` — knowledge/story context for the path or resource.
- `value` — value or REA context.
- `governance` — who/policy/rules/validators for the path or subtree.
- `place` — place or locality context.
- `attestations` — attestations that stand around the subject.
- `claims` — claims made by or about the subject.

`eprfs-meta` resolves the `.epr-meta` ancestor cascade into `EprMetaResolution`,
answering "what EPR head/meta applies to this file?" Governance is one leg in
that answer, alongside story, value, place, attestations, and claims.

## EPR Cards and Projection Awareness

Static projection manifests remain the truth of a snapshot. Mutable protocol
state lives beside that truth as awareness:

- `EprCard` — protocol-facing summary of subject, byte presence, resiliency,
  peer visibility, verification, and local overlay state.
- `ProjectionAwareness` — root card plus per-entry awareness for filesystem
  status surfaces, sidecars, CLIs, FUSE, WinFsp, Finder, or Explorer views.
- `ProjectionAwarenessProvider` — trait for Elohim protocol/storage services to
  supply observed peer and resiliency state without making `eprfs` own that
  truth.

`eprfs-local` can persist awareness under `.eprfs/status/`:

```text
.eprfs/status/projection.json
.eprfs/status/entries.jsonl
```

This keeps the boundary explicit:

```text
manifest = static snapshot truth
awareness = mutable protocol/local status around that truth
overlay = local changes relative to that truth
```

## Design Rule

`eprfs` knows how EPR-governed data collapses into a filesystem tree on this machine. It does not know what that tree means. A repository, household archive, learning path bundle, or application source tree should all be able to use the same projection contract.
