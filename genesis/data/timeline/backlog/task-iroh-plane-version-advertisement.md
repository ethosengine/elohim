---
id: "backlog-task-iroh-plane-version-advertisement"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: iroh-plane parity for peer version advertisement — the libp2p leg advertises service/version+commit via identify; the iroh leg advertises nothing"
slug: "task-iroh-plane-version-advertisement"
written: "2026-08-31"
author: "session-2026-08-31-velocity-snowball"
status: "wip"
priority: "medium"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-task-runtime-passport-endpoint"
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "habit:dataplane-convergence"
tags: [observability, iroh, transport-parity, mixed-version, delegable]
claimedBy: "codex"
---

**Claimable by any implementation agent. The exchange seam is grounded below;
preserve its signed-v1 compatibility contract.**

## Why

On the libp2p leg, peers already learn each other's storage build: the
identify protocol carries `elohim-storage/x.y.z+<commit>` as user-agent
(`elohim/elohim-storage/src/p2p/behaviour.rs` ~line 496). On the iroh leg
there is no identify equivalent, so an iroh-only peer (the deployment
direction of travel) is version-anonymous to its peers. The 2026-08-31
frame-cap incident is the concrete cost: an old-reader/new-sender split
had to be diagnosed by log archaeology instead of by asking peers their
versions. Evidence: commit `f1d504317` (`fix(dataplane): cure the iroh
view-federation frame-cap drift — liberal reader, conservative sender`) records
the live `536378 > 262144` refusal and the deployed-reader-floor cure.

## P2P design-gate decision

- **Classification:** Ephemeral (C), T2 peer-hoster observability metadata.
  The running binary is the source; the receiver's last-seen value is an
  in-memory projection rebuilt by the recurring transport-manifest exchange.
  It creates no DHT entry/link, persistent row, head-plane item, coordinator
  function, Automerge projection, or HTTP route; therefore it is
  DNA-hash-neutral and has no head-plane cost.
- **Identity/address:** attach the observation to the already-resolved iroh
  `NodeId`/peer-book entry. It introduces no content address and must never be
  joined to `agent_cid` by raw string equality.
- **Network stakes:** the same advisory observation behaves at every declared
  stage. No constitutional/local-relationship/counter-evidence floor is in
  play, and the value may not grant authority or negotiate capabilities.
- **Concern canon:** C0 is answered by T2/operational placement; C4 by explicit
  unknown/last-seen absence; C5 by treating the claim as evidence, never
  authority; C7 by deriving it from the running service's BuildInfo; C8 by
  existing accepted/rejected manifest observations; C10 by the serde-default
  and byte-pin tests below; C14 by retaining an observable decode/verify
  failure. C1, C2, C3, C6a, C6b, C9, C11, C12, and C13 are n-a because this
  adds no election, state machine, work loop/effect, lineage claim, ingress
  policy, authorization, or authority scaffold.

## Scope

1. Ride the existing `TransportManifestAnnouncement` exchange in
   `src/p2p/transport_manifest_gossip.rs`, built by
   `src/p2p_iroh/announcer.rs`, verified by `src/p2p/gossip_dispatch.rs`, and
   projected into `src/p2p_iroh/peer_book.rs`. Do not invent a new
   protocol/ALPN; the identity-handshake client has no production caller.
2. Add an OPTIONAL, additive `userAgent` (or equivalent) field:
   `#[serde(default, skip_serializing_if = "Option::is_none")]`. Preserve the
   signed-v1 bytes exactly. Either keep the field explicitly advisory and
   outside the v1 signature, or add a separately optional extension signature;
   never add it to the existing signing bytes, which would make an old reader
   ignore the map key and then reject the signature.
3. Derive the value from the running service's BuildInfo user-agent and store
   the last-seen optional value on the existing `IrohPeerEntry`. Enrich an
   existing accepted-manifest log/debug observation if available; do not add
   an HTTP route or infer feature/capability support from the value.
4. Pin both compatibility directions: old message → new reader, and new
   message → old reader semantics/signature verification. Pin byte identity
   for the unchanged/v1 signing shape and round-trip the populated field.
5. Register the changed boundary in
   `elohim/elohim-storage/seam-registry.yaml` if the implementation introduces
   a new boundary answer type or decision point, using the concern answers
   above and independent contract tests.

## Disjointness contract

- The delegated implementation agent (Codex or equivalent) MAY edit only
  `src/p2p/transport_manifest_gossip.rs`, the directly participating
  `src/p2p_iroh/{announcer,peer_book}.rs` files, narrowly required receive-side
  handling in `src/p2p/gossip_dispatch.rs`, focused tests, and the crate-local
  `seam-registry.yaml` when the birth rule requires it.
- It MUST NOT edit `src/p2p/view_federation.rs` or its frame-cap constants;
  `http.rs` (including the `/version` match arm); `happ_manager.rs`;
  `reconcile_peers` PRODUCTION logic; any Jenkinsfile; any
  deployment/orchestrator manifest; or `hc-mesh.sh`. Those are the rung
  lane's surfaces this week.
- AMENDMENT (rung lane, 2026-09-01): a compile-FORCED `#[cfg(test)]`
  fixture touch in `reconcile_peers` is permitted — the minimal edit only
  (add the new field with its default to existing struct literals; no
  fixture restructuring, no style changes, no other lines). If the field
  addition forces any non-test line in that file, stop and report instead.
  Preferred long-term shape (optional, in the task's own scope): a
  `Default` impl or test-constructor for the gossiped entry in its home
  module, so future additive fields stop rippling into other files'
  fixtures.

## DoD + verification

- `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --manifest-path elohim/elohim-storage/Cargo.toml --features "p2p p2p-iroh" p2p_iroh; echo "EXIT=$?"` is green.
- `just gate elohim-storage` is green, including the default-features build
  (the 2026-08-31 cross-reference trap: never reference `crate::p2p_iroh`
  from always-compiled code).
- Wire-compat tests prove both decode directions, old-reader signature
  acceptance, and byte-identical v1 signing bytes.

## Implementation checkpoint (2026-09-01)

The additive advisory field, BuildInfo announcement, verified receive log,
last-seen peer-book projection, and bidirectional v1 compatibility tests are
implemented. Feature-enabled evidence is green: the `p2p_iroh` filter passed
111/111 tests (including populated peer-book storage) with Cargo exit 0.

The task remains `wip` because `just gate elohim-storage` is currently blocked
at `cargo fmt --check` by the disjoint runtime-config lane's untracked
`src/runtime_config.rs` and concurrent `src/main.rs` edits. No reported format
diff is in this task's write set; this lane did not modify those files.
