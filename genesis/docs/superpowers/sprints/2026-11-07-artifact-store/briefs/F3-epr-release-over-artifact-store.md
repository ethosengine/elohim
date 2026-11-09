---
id: artifact-store-brief-f3-epr-release-over-artifact-store
status: proposed
habits: [sprint-lands-each-closure-once]
---

# Brief F3 — an epr release travels over the artifact store to the pair device (proposed, not dispatched)

Proposed by ethosengine (ab4528a1) on 2026-11-09 from the operator's framing: "either you or shem could build a
candidate epr, and its binary becomes available as part of that p2p artifact-store, and because we're pair devs
aware of each other's artifacts over auto-update channels we're subscribed to, artifacts can be used as part of
integration on either side." The lead integrator (shem) sequences it; it rolls no alpha (eprfs + device-runtime
only), so it can sit beside the storage closure, after it. Gap evidence: a read-only Codex astra pass recorded in
`genesis/local-dev/epr-converge/result.md` on ethosengine (summarised below with file:line).

## The rung this serves

The epr release climbs the reach ladder: **private** (a session's `EPR_BIN`) → **device** (landed `current`) →
**pair** (the sibling adopts the same bytes, verified, and the record says so) → the steward's devices → commons.
F2 already proves the pair rung for mesh binaries. F3 makes the device's own tools the next artifact on it.

## What exists (no new transport)

- F2 discovers crossed `artifact-produce@1` records by key, fetches from siblings, verifies manifest, chunks and
  the whole output, and installs (`elohim/eprfs/epr-cli/src/artifact.rs:1308`, `elohim/eprfs/eprfs-peer/src/fetch.rs`).
  `mesh-bin` is the daemon's default install allowance, not a transport limit; explicit roots are supported
  (`epr-cli/src/peer.rs:674`, `eprfs-peer/src/fetch.rs:1155`).
- The device follows `bin:epr` in mode `apply`, no pin (`/nix/xdg/config/elohim/device/runtime-manifest.json`);
  `Follow` is a private device choice, not a network head (`elohim/ark/core/src/declaration.rs:47`).
- Local closure ingestion is the receiving boundary (`device-runtime/src/bin_store/mod.rs:210`, `ingest.rs:13`).

## Gaps (the write-set)

1. **Publish.** When a device ingests a clean epr release, it also records the release (both `epr` and
   `bin-launch`, with provenance: source commit, tree-clean, gate run) as artifact Produce records that cross.
2. **Adopt.** `epr bin` (follow/bootstrap), on finding a sibling release for a clean dev-lineage commit it is asked
   to follow, fetches every member through F2 into the bin store, reconstructs and verifies the same release CID,
   and only then hands it to the ordinary landing rules. Today the epr branch builds locally and does not consult
   siblings (`bin_store/lifecycle.rs:222`, `:349`); "followed download" is Brit's GitHub fallback, not this.
3. **Elect across a lineage break.** A distinct, explicit landing intent (`--rebase-lineage`, exclusive with
   `--revert`, `--expect` required) that waives only old→new ancestry and records a truthful ring (e.g.
   `lineage-rebased`, old/new CID and commit, reason, and the sibling attestation when the release was adopted).
   Automatic follow and bootstrap must not be able to take it (a test pins this). Today the only bypass is `--revert`, which mislabels
   (`bin_store/landing.rs:109`, `:204`; `epr-cli/src/bin.rs:106`). Spec rule it extends: head order is dev ancestry,
   backward only by revert election (`genesis/docs/superpowers/specs/2026-10-23-household-follows-latest-design.md:162`).
4. **Re-ingest the pre-migration current.** ethosengine's landed `bafyreibzey…` reads `verifiedCurrent: false`
   (pre-migration `bafkrei…` sums; `genesis/docs/superpowers/plans/task-rg1-eprfs-report.md:178`).

## Tests

Extend `device-runtime/src/bin_store/tests.rs` (landing ~:943, follow/pin ~:2462): explicit rebase succeeds and
the default refuses; stale `--expect`; movement during health; dirty, unpublished, tampered, unhealthy candidate;
pinned follow; conflicting flags; truthful history; unchanged session pins. Adoption: a two-store fixture where
store A publishes and store B adopts the same release CID, plus a tampered-chunk refusal.

## Receipt (the DELTA)

shem builds epr at a dev commit; ethosengine's `epr bin follow` adopts the same release CID from shem (fetched, not
built), landing as `lineage-rebased` once and as an ordinary descendant afterwards; then the reverse direction.
Record fetch vs build wall-clock and bytes in `genesis/orchestrator/.epr-meta/sprint-lands-each-closure-once.habit.md`.

**Gate:** `just gate eprfs` (device-runtime, epr-cli). **Reviewer:** the other device, by running the receipt.
