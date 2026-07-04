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
- `eprfs-meta` — parses authored `.epr-meta` files and resolves what EPR meta/head coupling applies to a projected path.
- `eprfs-storage` — storage-facing adapter scaffolding and test doubles for `elohim-storage` integration.

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
