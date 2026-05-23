# Follow-up Prompt 1 — Doorway warm-stream architecture

**Created:** 2026-05-23 from the alpha-landing-page dual-doorway shift.
**For:** A fresh session picking up the doorway projection-cache design work.
**Owner role:** rust-architect (Rust truth-layer) + angular-architect (knows the doorway/CLAUDE.md voice).

---

## Context

While shaking out `alpha.elohim.host`, I (the prior shift) discovered a divergence between what `doorway/CLAUDE.md` says and what the warm-stream code actually does — and you (the operator) corrected my read on the intended architecture.

The corrected intent is:

1. **Seeder seeds storage** (one peer, typically the doorway operator's own steward storage).
2. **P2P data layer syncs** content rows + blobs across all peers per hosting agreement.
3. **Doorway reads hosting-compute agreements from the DHT** — knows which peers have agreed to host which content.
4. **Doorway warms its projection cache from peers** that:
   - Hold the content (per hosting agreement on the DHT)
   - Have enough compute capacity to serve the warm request (load balancer pattern — distribute, don't stampede)
   - Operator's own storage instance is the **likely** first source, but not guaranteed
5. **Ongoing gossip + cache validation** keeps the cache warm post-startup.
6. **Content without a hosting agreement** is accessed via authenticated steward (logged-in) state through P2P gated access — doorway just facilitates the addressing point.
7. **`elohim.host` and `alpha.elohim.host`** are two registered doorway landing pages for the doorway steward of record. Both should be delivered from the projection cache; both can fall back to any capable peer with capacity.

---

## What's currently broken

In **`doorway/doorway-service/src/projection/warm_stream.rs:spawn_stream_task`** + **`doorway/doorway-service/src/routes/admin_cache.rs:cache_warm`**, the warm-stream task **iterates over all entries in `STORAGE_URL` (singular) + `STORAGE_URLS` (plural CSV)** and projects every event from every peer into the shared MongoDB. It is unaware of hosting agreements, peer capacity, or content distribution. Last write wins.

Resolution today:
- `computeStorageUrls()` in `elohim/holochain/Jenkinsfile:376` emits a CSV of every peer in the env (14 peers on alpha).
- That CSV becomes `STORAGE_URLS` on the doorway deployment manifest (`genesis/orchestrator/manifests/doorway/alpha.yaml:128`).
- Doorway fans in 14 streams indiscriminately.

This fan-in is wrong because the **P2P content-row sync (step 2 in the intended flow) doesn't currently propagate** updates from one peer to others. So when CI patches `matthew`'s `elohim-host-landing.blobHash` to the real hash, the other 13 peers still hold the placeholder. The warm-stream then projects the placeholder LAST and the alpha landing page renders broken.

---

## Three failure layers stacked

Treat them as separate problems with separate fixes:

### Layer A — The substrate gap (root cause)

P2P content-row sync between peers in the same env isn't propagating updates. The recently-landed `2026-04-19-self-healing-p2p-dataplane-design.md` spec exists for blob replication but content rows (the `content` SQL table that backs `cache_stream::list_cacheable_content`) aren't covered. Until this is fixed, **any divergence between peers becomes a visibility problem at the doorway projection cache.**

### Layer B — The hosting-agreement awareness gap

Even with sync, warm-stream should ideally **only** consult peers that have a hosting agreement on the DHT for the content it's warming. The current code has zero DHT awareness — it reads `STORAGE_URLS` from env and stops. The hosting-agreement read path (REA stewardship contract for `action='operate-doorway'` or similar) needs to be added.

### Layer C — The load-balancer/stampede gap

Even with hosting-agreement awareness, picking which one of N qualified peers to warm from is a separate concern. Capacity-aware selection (the `system_metrics` projection per `project_node_metrics_vs_hub_aggregation_boundary`) gives the inputs; the warm-stream needs to pick one peer per content unit, not all of them in lockstep.

---

## Open questions for this session

1. **Is the user's architectural intent correct?** Confirm with the operator that the three layers above match their mental model. The `doorway/CLAUDE.md` text saying "single-target dispatch" is about *blob serving on the data path*, not about cache warming — the operator clarified this. Should `doorway/CLAUDE.md` be amended to make this distinction explicit?

2. **Order of fixes.** Layer A is the substrate work and is the **proper** fix; Layer B can be designed independently and shipped as an iteration; Layer C is a refinement once B works. The shortest path to unblocking `alpha.elohim.host` today is to **not** do A/B/C and instead either:
   - (i) Fan-out the CI PATCH across all peers in `stageSpaBlob` so peers stay in agreement by brute force (workaround — fragile, scales linearly with peer count).
   - (ii) Make warm-stream consume only from the singular `STORAGE_URL` env var (the doorway operator's own storage) — quick code change, ships in one Edge rebuild. The flaw with (ii): if the operator's own storage is down or doesn't have the content, the alpha landing page goes blank. Resilience is sacrificed for correctness.

   Which one ships first?

3. **The hosting-agreement read.** The REA structures for hosting agreements are in `elohim-storage` (action='operate-doorway' from prior session memory). Is the data already in the DHT, or does Layer B need DHT entry types added first? Check `elohim/holochain/dna/imagodei/zomes/imagodei/src/lib.rs` for any operate-doorway primitive that already exists.

---

## Where to start

1. Re-read `doorway/CLAUDE.md` "No Blob Fan-Out — Doorway is Single-Target Dispatch" section with this corrected lens — confirm the section is about data-path blob serving, not warm-stream / projection cache.
2. Read `doorway/doorway-service/src/projection/warm_stream.rs` (full file) and `doorway/doorway-service/src/routes/admin_cache.rs:cache_warm`.
3. Read `genesis/docs/superpowers/specs/2026-04-19-self-healing-p2p-dataplane-design.md` for the substrate plan.
4. Read `elohim/elohim-storage/src/cache_stream.rs` and `src/db/cache_queries.rs` for the source-side of the stream.
5. Read `elohim/holochain/Jenkinsfile:376 computeStorageUrls` to see how `STORAGE_URLS` is composed in deployment.
6. Propose: which of the three layers should this session address, and ship the corresponding plan + first iteration.

---

## Constraints

- Do **not** ship a code change that ALSO requires substrate work to be useful. If you go for the "use singular STORAGE_URL" quick fix, ship it alone and surface the substrate gap explicitly in the plan.
- Do **not** silently downgrade resilience without naming the trade-off in the plan.
- Coordinate with the open shift on `alpha.elohim.host` — the credential blocker (next prompt, "Operator Action") and any in-flight Edge rebuilds.
- The user's voice on this domain: "communitarian solarpunk; warm earth + constellation dark." `household` not `user`. `steward` not `owner`. `provision` not `transaction`.

---

## Related artifacts

- Prior shift journal: `.claude/shifts/2026-05-23T05-25-alpha-landing-page-dual-doorway.journal.md`
- Plan that drove the prior shift: `genesis/docs/superpowers/plans/2026-05-23-spa-blob-deploy-drift.md`
- The misread "single-target" quote: `doorway/CLAUDE.md` line citing `project_doorway_single_target_no_fanout`
- Self-healing P2P design: `genesis/docs/superpowers/specs/2026-04-19-self-healing-p2p-dataplane-design.md`
