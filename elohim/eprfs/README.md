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
- `eprfs-local` — materializes projection manifests into ordinary filesystem trees.
- `eprfs-storage` — storage-facing adapter scaffolding and test doubles for `elohim-storage` integration.

## Projection Source Identity

Projection entries may carry a domain-neutral source identity:

- `content` — byte-bearing content such as a file blob.
- `container` — an object that organizes child entries, such as a tree.
- `link` — link content whose bytes name another path.
- `external` — a boundary to another resource, repository, or projection.

Domain adapters decide the namespace and source id. For example, `brit` uses
git object ids, but `eprfs-core` only validates the projection shape.

## Design Rule

`eprfs` knows how EPR-governed data collapses into a filesystem tree on this machine. It does not know what that tree means. A repository, household archive, learning path bundle, or application source tree should all be able to use the same projection contract.
