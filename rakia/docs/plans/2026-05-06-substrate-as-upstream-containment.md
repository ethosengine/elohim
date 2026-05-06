# Substrate as Upstream Containment

**Status:** Design sketch (side-thread capture, not yet a phase)
**Date:** 2026-05-06
**Related:**
- `elohim/brit/docs/plans/phases/phase-3-libp2p-transport.md` (git over libp2p)
- `elohim/brit/docs/plans/phases/phase-2-contentnode-adapter.md` (commits/trees as ContentNodes)
- `elohim/brit/docs/plans/phases/phase-5-dht-discovery.md` (DHT-based repo discovery)
- `genesis/orchestrator/Jenkinsfile` (the four `git fetch --depth=200` aborts on 2026-05-06 that surfaced the cost)

## Origin

A `/deliver` continuation for "light up the topology" bailed when four consecutive orchestrator builds (#860, #861, #862, #864) failed in `git fetch --depth=200` against `github.com`. Build #858 had succeeded with the identical fetch ~7h prior, confirming intermittent upstream unavailability. Two retriggers (`f8f6be97`, `6df687bc`) didn't recover.

The cluster already insulates itself from `npmjs.org` via `nexus.ethosengine.com` — pulls cached packages, refreshes on demand, builds keep running when upstream blinks. **Same shape applies to every upstream we touch.** The most painful gap right now is git/GitHub.

## Two Framings

There are two ways to approach this. We should be explicit about which one we're committing to, because the migration story is different.

### Framing 1 — Web2 Mirror Layer (incremental)

Add a cluster-internal git mirror (gitea / gitlab CE / `git daemon` on a fileserver) that fetches and caches dev branch on a cron. Jenkins agents clone from `git.ethosengine.com`. Mirror→GitHub sync is async; CI never blocks on GitHub.

**Pros:** Deployable today. Standard pattern. Same shape as nexus.
**Cons:** Yet another web2 service to operate. Doesn't compose with the protocol substrate. When that mirror service goes down, we've moved the problem, not dissolved it.

This is the **out-of-scope nuance** captured in `project_upstream_proxy_pattern_brit_rakia` memory: "the proxy layer is a stewardship contract too, not just plumbing." If we go this route, name it explicitly and treat it as a household-cluster gift, not infrastructure debt.

### Framing 2 — Substrate-Native Upstream (MVP target)

The protocol substrate already has the primitives:

- **ContentNode adapter (brit Phase 2)** — git commits and trees become content-addressed nodes
- **Git over libp2p (brit Phase 3)** — `/brit/fetch/1.0.0` carries pack-protocol over the P2P network
- **DHT discovery (brit Phase 5)** — peers discover who has which repo
- **elohim-storage replication** — content replicates per stewardship commitments
- **Doorway as web2 projection** — already the only web2 surface in the three-layer truth model (per `project_three_layer_truth_model` memory)

So the MVP is not "add a mirror service." It is:

> Matthew installs an app via elohim-storage. The app declares an `elohim-protocol` git-repo ContentNode. Matthew's household-cluster commits to replicate it (per his social-graph stewardship commitments). The doorway projects a GitHub-compatible HTTP surface (`git.matthew.elohim.host/elohim/protocol`). Jenkins clones from there. When github.com hiccups, nothing notices, because the bytes were already on Matthew's blades.

This is the same inversion as everywhere else in the protocol: instead of building a web2 service that mirrors GitHub, **the substrate already replicates the bytes**, and the doorway is just the web2 projection.

GitHub becomes one possible upstream — peer to a peer — instead of the source of truth. Recovery from github.com outage is automatic because nobody was depending on GitHub being available; they were depending on the *content*, which is content-addressed and replicated.

## Why This Matters for the brit→rakia Dimension

The brit→rakia continuum already has axes (build attestation, deployment state, reach gating, succession). **Add "upstream containment" as a first-class axis.**

For each external dependency a build manifest declares, brit can surface:

- **Containment level:**
  - 0 — direct (build hits live upstream every time)
  - 1 — mirrored (cluster-local web2 cache; nexus pattern)
  - 2 — substrate-replicated (replication is a household stewardship commitment; no web2 service in the path)

- **Replication breadth:** how many household clusters carry it (one stewardship commitment vs. many)
- **Last-resort source:** is there still a path to fetch from upstream if all replicas are down?

This is the same shape as reach: rakia can refuse to promote a build whose upstreams are at containment level 0 to environments that need 99.9% availability. Containment becomes a quality signal.

## What the MVP Actually Looks Like

A minimum implementable cut:

1. **brit Phase 2 + Phase 3 land** (prerequisite — without ContentNode addresses for git objects and a libp2p fetch protocol, the substrate has nothing to replicate)
2. **A repo-replication app** runs in elohim-storage. Its manifest declares: "this household replicates `elohim/protocol` (and any forks the operator stewards)." Replication policy is a REA commitment.
3. **Doorway adds a git-projection route** — `/git/<owner>/<repo>` speaks the dumb-HTTP git protocol on top of substrate-stored objects. Same pattern as the existing `/blob/<hash>` surface, just with git's expected URL shape.
4. **Jenkins points its `git url:` at the doorway** instead of github.com. Or the orchestrator does — graph-walker reads from the doorway path. (Configurable per pipeline.)
5. **Sync from upstream is its own concern** — a separate process that pulls from github.com when reachable and writes new commits/trees into the substrate. Pre-stocks the cache; not on the build's critical path.

Step 5 is the only piece that ever touches github.com directly, and it doesn't run during a build. Build-time path: Jenkins → doorway → substrate. No external network access during the build.

## Why "Matthew's Operator Story" Frames This Well

The household-cluster narrative is already canonical for elohim-storage (see `project_household_fabric` memory). Matthew operates a household; nodes join/leave fluidly; the cluster is the unit of resilience. Upstream containment naturally belongs to that operator story:

- Matthew installs the elohim repo replicator the same way he installs anything else on his household
- Replication is a stewardship commitment he authors (REA), not a config flag
- His doorway projects the mirror surface that his Jenkins (or anyone else's, with reach) consumes
- When his household has the bytes, his builds are decoupled from github.com
- When other households commit to replicate too, the substrate gains redundancy *as a side effect* of multiple operators sharing the cost

This dissolves the question "who pays for the mirror?" — the mirror is the household's contribution to the protocol, the same way DHT participation is.

## What This Is Not

- **Not a federated GitHub replacement.** Forge governance (issues, PRs, reviews) is brit Phase 6 (fork governance). This sketch is only about *replication of bytes*.
- **Not a substitute for brit Phase 3.** This sketch *consumes* Phase 3 — it's not a parallel transport.
- **Not a runtime decision.** Containment level is declared at manifest time; the build doesn't pick which path to use.
- **Not pinning the orchestrator to a single doorway.** Per `project_multi_doorway_human_registration` memory, operations should be doorway-agnostic. The graph-walker can fetch from any of N doorways the cluster reaches; doorway selection is a separate concern.

## Open Questions for the Real Plan

1. **Does this need a new brit phase, or extend Phase 3?** Phase 3 lands the transport; this sketch is a doorway projection on top. Probably its own phase number after Phase 5.
2. **Where does the "sync from upstream" worker live?** Doorway? elohim-storage? A new `brit-sync` daemon? Should be the smallest thing that can run as a household stewardship commitment.
3. **How does the doorway know which repos it should project?** Manifest-driven (per `project_doorway_manifest_driven_routes` memory) — but what's the manifest shape for a "replicate this git repo" declaration?
4. **What's the minimum reach for a build to consume a substrate-replicated repo?** A solo replicator is fine for the operator's own builds; promotion-blocking environments may need diversity.
5. **GitHub → substrate write-back path** — when a developer commits, do they push to GitHub (and a sync worker imports) or push to the substrate directly (and a sync worker exports)? The first is incremental; the second is the long-arc target.

## Action

Park as a design sketch. Pick up after brit Phase 3 lands and we have working substrate-replicated git. At that point this becomes a concrete rakia integration: the orchestrator reads upstream-containment from build manifests and refuses to schedule level-0 builds onto reach-gated environments.

For the immediate term: add the "upstream containment" axis to brit→rakia dimension planning notes wherever those live, so it doesn't get rediscovered the next time GitHub blinks.
