---
name: Upstream containment as brit→rakia axis — MVP is substrate-native, not web2 mirrors
description: Generalize nexus-shape proxying for all CI upstreams; MVP target is substrate-replicated repos (Matthew's household replicates the elohim repo; doorway projects GitHub-shape) rather than yet-another web2 mirror service
type: project
originSessionId: e9127493-544b-44b2-8f81-c61c1fa5cbb6
---
The pattern: `nexus.ethosengine.com` already insulates the cluster from npmjs.org availability. The same shape should extend to every upstream — but the MVP framing has shifted.

**Why:** /deliver light-up-the-topology continuation 2026-05-06 hit four consecutive orchestrator-build aborts in `git fetch --depth=200` against github.com (#860, #861, #862, #864). Build #858 succeeded with the identical fetch ~7h prior. CI was hostage to GitHub weather; whole shift bailed.

**Two framings — be explicit which we commit to:**

1. **Web2 mirror layer** (incremental): cluster-internal git mirror (gitea / `git daemon`) at `git.ethosengine.com`. Standard, deployable today, but yet-another-service to operate.
2. **Substrate-native upstream** (MVP target): the protocol substrate already has the primitives (brit Phase 2 ContentNode adapter, Phase 3 git-over-libp2p, Phase 5 DHT discovery, elohim-storage replication, doorway as web2 projection). The mirror IS the substrate. Matthew installs a repo-replicator app via elohim-storage; his household commits to replicate `elohim/protocol`; doorway projects a GitHub-shape HTTP surface; Jenkins clones from there; github.com is just one possible upstream replica that a sync-worker pre-stocks the cache from. **Build-time path never touches github.com.**

**How to apply:** Add **upstream containment** as a first-class axis in brit→rakia dimension planning, alongside reach/build-state/deployment-state. For each manifest-declared external dependency, surface:

- Containment level (0 direct / 1 web2-mirrored / 2 substrate-replicated)
- Replication breadth (how many households steward it)
- Last-resort upstream source

Containment becomes a quality signal — rakia can refuse to promote a level-0 build to a reach-gated environment.

**Captured in:** `rakia/docs/plans/2026-05-06-substrate-as-upstream-containment.md` (design sketch, not yet a phase). Open questions parked there: where the sync-worker lives, what the manifest shape is for a replicate-this-repo declaration, GitHub→substrate write-back path.

**Connections:**
- Composes with `project_household_fabric` (Matthew operates a household cluster; replication is a stewardship commitment, not infra debt)
- Composes with `project_three_layer_truth_model` (doorway as web2 projection only; substrate carries the bytes)
- Composes with `project_doorway_manifest_driven_routes` (doorway exposes the GitHub-shape via manifest declaration)
- Composes with `project_doorway_is_federation_surface_atproto` (federation-flavor interop — including Git-over-HTTP — lives at doorway, not at the substrate)
- Should be doorway-agnostic per `project_multi_doorway_human_registration` (graph-walker fetches from any of N doorways)

**Other upstream candidates** (same axis applies): cargo crates, docker FROM bases, Holochain DNA artifacts. Each gets its own containment-level decision; substrate-native is preferred where the ContentNode shape fits naturally.

**Out-of-scope alternatives that are worse:** retry-with-backoff (hides symptom), self-hosted runners with warm cache (partial; no help on cold pods), move CI to a different cloud (same problem, different upstream).

**Status:** design observation 2026-05-06; rakia design sketch written; activates after brit Phase 3 lands.
