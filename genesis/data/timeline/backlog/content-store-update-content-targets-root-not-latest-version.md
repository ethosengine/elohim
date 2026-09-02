---
id: "backlog-content-store-update-content-targets-root-not-latest-version"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "content_store::update_content targets the FIRST IdToContent link (the root), so a content id's version chain is a star — release lineage (and any version order) cannot be proven from the L2 chain"
slug: "content-store-update-content-targets-root-not-latest-version"
written: "2026-09-02"
author: "session-2026-09-01-adoption-ceremony"
status: "in-tree"
priority: "high"
jobs: [elohim-holochain, elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "backlog-task-runtime-upgrade-a2o-receipt"
  - "backlog-lamad-dna-workspace-hc-rna-cdylib-link-breaks-coordinator-build"
tags: [upgrade-propagation, rung5, zome, coordinator-only, lineage]
---

## Measured (2026-09-02, local mesh, `release-lineage-probe.ts`)

Channel `runtime:coordinators:elohim:receipt-20260901-r2`: release O then release P were each
published via `update_content` on the channel's own id. On both conductors P's
`ContentHeadWire.supersedes` (= the update's `original_action_address`) is the CHANNEL ROOT
record (`metadata_json.kind = release-channel`), not O. Cause:
`content_store/src/lib.rs` `update_content` step 1 does `get_links(IdToContent).first()` and
updates THAT action; step 6 adds a fresh id→action link but the old links remain, so `first()`
keeps returning the root and every version becomes a sibling child of the root.

## Consequence

The spec's lineage rule ("the body's `lineageParentCid` is a hint that must match the L2 version
chain") cannot hold for any second release: the chain never carries release order. The adoption
controller (fix 5, 2026-09-02) falls back to proving a declared parent by EXISTENCE as a
release-manifest on the same channel — weaker than chain order. Any other consumer of
`supersedes` / version order (head heal, forward-ordering proofs) inherits the same star.

## Fix (coordinator-only — no DNA-hash move)

`update_content` must target the LATEST version: pick the `IdToContent` link with the newest
timestamp (or delete the previous link in step 6 so exactly one remains). Then
`head.supersedes` names the prior version and the controller's strict-equality arm applies.
This is the ideal first REAL coordinator-only release for the rung-5 ceremony — blocked on the
DNA workspace build (see the hc-rna cdylib atom).

## DoD

Publish two releases on a fresh channel; `resolve_content_head_local` reports
`supersedes == <first release cid>`; the controller verifies the second with the chain arm (no
existence fallback); sweettest covers update-of-update ordering.

## Fix landed in-tree (2026-09-02)

`content_store::latest_id_to_content_link(links)` selects the newest `IdToContent` link by
(timestamp, create-link action hash) — the same deterministic order `newest_canonical_link`
already uses — and replaces `links.first()` / `links[0]` in `update_content`,
`get_content_by_id` and `get_blobs_by_content_id`. Old links are left on the DHT (no
delete-link write on the update path; readers never rely on `get_links` order again).
Coordinator-only: `content_store_integrity.wasm` byte-identical before/after
(`d1c4e709…`), `content_store.wasm` moved. Sweettest added:
`update_of_update_supersedes_previous_version_not_root` (create → update₁ → update₂ ⇒ head =
update₂, `supersedes` = update₁, `get_content_by_id` serves update₂). NOT run locally (the
sweettest slot is cold and the resident mesh holds the RAM budget); it runs in the DNA
pipeline (`--run-ignored all`) and in the pre-push `sweettest-check` on a `dev` push. Do not
push mid-wave: a same-wave dispatch bakes the previous happ. This is the first genuine
coordinator-only release candidate for the rung-5 election on alpha.
