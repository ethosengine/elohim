---
name: Elohim operator as household fabric manager
description: Long-term vision — the elohim operator manages a local cluster of family nodes/blades that join and leave fluidly, with operator-driven replication and optimization distinct from the P2P dataplane.
type: project
originSessionId: d59b6174-405f-478d-a6fc-567fd30edc74
graduated_to: "experience-story-james-son--as-stewardee--stewarded-device-sync"
graduated_on: "2026-05-14"
---

> **GRADUATED 2026-05-14** — Story `james-son--as-stewardee--stewarded-device-sync` carries the household-fabric lesson via lived narrative (Jessica's spoke open/close, Matthew's family node, James's Chromebook joining/leaving). The story is now the canonical surface for this knowledge. This entry preserved in the graduated archive for traceability. See `genesis/data/stories/james-son--as-stewardee--stewarded-device-sync.md`.


Household nodes are not static. Family members bring hardware when they arrive (grandma moves in with two blades) and take hardware when they leave (James takes a node to college). The elohim operator is expected to:

- Detect new nodes joining the household cluster
- Decide what to replicate/optimize locally (storage synergies within the LAN)
- Distinguish local-cluster replication from WAN P2P replication
- Handle graceful node departure (data rebalance, re-stewardship)

**Why:** This is the household operator's job, not k8s's and not the P2P dataplane's. The P2P layer gives cross-WAN resilience; the local operator gives LAN-level performance/convenience (shared cache, local shards, faster sync between family devices). Today's single-pod-per-human + `openebs-hostpath` decision is correct for now but will need revisiting once multi-node households exist.

**How to apply:** When designing storage, scheduling, or sync features, ask "is this operator-layer (local cluster) or dataplane-layer (P2P)?" Don't collapse them. Avoid decisions that assume one node per human or a static node roster.

**Household topologies currently in play (2026-04-15):**
- Matthew's household: multi-node LAN (Matthew is already running multinode in real life). Models operator-driven local fabric across ethosengine/intel-nuc/thinkc-p*/hp-micro10.
- Adam's household: single-node on shem, shared with Eve. Models a couple where both humans' data lives on one family node — distinct from solo operators (Frank, Pete).

Eve is not yet in the manifests; add her as a human persona co-located on shem when the adam+eve household scenario is needed.
