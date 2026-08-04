---
id: "backlog-dual-address-blob-router-unmanifested-blake3-400"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "seed_e2e_dual_address red: unmanifested GET by blake3 address 400s — alias lookup empty + manifest-gated iroh path + 400-vs-404 wire-shape drift, three layers behind one failing test"
slug: "dual-address-blob-router-unmanifested-blake3-400"
written: "2026-08-04"
author: "holochain-iroh convergence campaign (Wave 1 Lane A baseline triage)"
status: "backlog"
priority: "high"
tags: [dataplane, blob-router, dual-address, blake3, transport-manifest, iroh, known-red, ci-blindness]
cites:
  - elohim/elohim-storage/src/http_blob_router.rs
  - elohim/elohim-storage/tests/seed_e2e_dual_address.rs
  - genesis/data/timeline/backlog/2026-07-30-blobhash-serverblobhash-duality-canonical-join-key.md
  - genesis/docs/superpowers/plans/2026-08-04-holochain-iroh-convergence-upgrade-campaign.md
---

# Dual-address blob fetch red — suspect a stack, not a miss

`tests/seed_e2e_dual_address.rs::seeded_blob_is_fetchable_by_both_addresses`
(committed 2026-05-10, Plan 3 dual-write seeder) fails at HEAD: GET by the
`blake3-` address returns **400** instead of 200. Found 2026-08-04 as the third
pre-existing red in the Lane-A iroh-pin-lift baseline (`--features p2p-iroh
--tests` — a surface CI never runs, so it rotted silently). PARKED as a named
known-red exclusion so the pin lift can proceed; this entry owns the concern.

## The three stacked layers (each independently wrong or suspicious)

1. **Alias lookup came back empty.** The blob was seeded via the dual-write
   PUT, so `peer_blob_inventory` should know the blake3↔sha256 pairing; with
   `sha256_alias_for_blake3 = Some`, even the libp2p-only degradation serves
   the bytes → 200. The observed 400 implies the alias row wasn't found at
   read time. Sibling of the canonical-join-key duality item (see cites) and
   possibly of the household_id-NULL identity-coherence gaps
   (`project_dataplane_next_lens_diversity_placement`). Question: did this
   test EVER pass, and what regressed it? (`git log` the router + inventory
   write path; candidate suspects include the affinity column work
   `82d2e2538` and later inventory changes.)
2. **Manifest-gated iroh read path.** `choose_backend`
   (`src/http_blob_router.rs:121-196`) only attempts iroh when BOTH
   `self_manifest` AND `caller_manifest` are present (`negotiated_iroh`,
   `_ => false`); an unauthenticated/unmanifested caller asking for an
   unambiguous `blake3-` content address can never reach the iroh store even
   when the node has the bytes. Design question: should an explicit
   blake3-form address be servable without transport negotiation (content
   addressing as identity — P2P-native reading), or is negotiation-gated
   read the intended contract (and then the test must wire manifests)?
   Do NOT resolve this ad hoc — it touches the reach/serve contract.
3. **400 vs 404 wire-shape drift.** The router's own comment promises the
   blake3-only degradation "produce[s] the existing 404 wire shape"
   (`http_blob_router.rs:132-136`), but the legacy parser 400s on a
   blake3-form address as malformed. Whichever way layer 2 resolves, the
   degradation's actual wire shape contradicts its documented intent.

## Exclusion contract (while parked)

Campaign Lane A (and any later lane touching elohim-storage) treats exactly
this one test as KNOWN-RED: post-change gates must show it failing with the
SAME signature (400 on the blake3 GET) and everything else green. A change in
its failure mode = new information, re-triage. Fixing it goes through this
entry, starting with layer 1 (was the alias ever written / when did it stop).

## DoD

Root-cause layer 1 with a dated regression commit or a never-passed verdict;
explicit design ruling on layer 2 (cite the reach/serve contract owner);
align layer 3's wire shape with the ruling; un-exclude the test; note the
CI-blindness angle in the Wave-1 exit review (this is the third `p2p-iroh`
test-surface rot found on 2026-08-04 alone).
