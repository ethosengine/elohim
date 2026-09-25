---
epr-meta-version: 1
id: bridges-interop-layer
covers: subtree
purpose: >
  Bridge crates: Rust libraries that translate external protocols to and from elohim's canonical
  EPR-REA substrate. Each bridge is a library in its own Cargo workspace, consumed by the runtime
  whose traffic it absorbs (web2 by doorway-service, protocol-shaped interop by elohim-storage,
  and the orchestrator's CI by a CLI). The current bridges are did, k8s, pkarr and valueflows.
---
# bridges/ — the bridge seam

A bridge translates outward and is added as a crate. A mod or plugin extends the runtime
downward, and an SDK manifest composes inward. `CLAUDE.md` gives the discriminator and the
four-step pattern for adding a bridge: its own workspace, one `<name>-bridge` library with a
`mount` or `handle_request` entry point, a documented consuming runtime, and qahal-authority once
it absorbs external writes.

`covers: subtree` is the ownership claim for the whole seam. Each bridge's own manifest carries
whatever edit-time signal its crate needs (`did/`, `k8s/`, `pkarr/`, `valueflows/`). A new bridge
directory is born with its manifest.
