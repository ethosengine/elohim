---
name: Alpha cluster topology — 6 peers, bootstrap pair, k8s nodes as household stand-ins
description: Alpha test fabric layout — adam+matthew across the node-split deliberately to give seeder bulk-upload bandwidth; the k8s node split represents household boundaries
type: project
originSessionId: 872c2e1c-02fe-453a-93b3-e69dac1e54e3
---
Alpha is a 6-peer test fabric where the k8s node split represents household boundaries (a simulacrum of the real-world household-level network):

| Peer | k8s node | Role |
|---|---|---|
| adam | shem | genesis peer; bulk seed receiver (paired w/ matthew) |
| frank | shem | secondary peer (intra-household w/ adam) |
| pete | shem | secondary peer (intra-household w/ adam) |
| matthew | ethosengine | other half of bootstrap pair; bulk seed receiver |
| jessica | intel-nuc | separate household |
| timothy | thinkc-p0h | separate household |

Doorway runs on intel-nuc (`elohim-doorway-alpha`).

**Bootstrap pair = adam + matthew across the node-split.** Both have device archetypes capable of bulk-upload; they're the two seeder targets that share load. Other peers settle the dataplane between themselves at their own pace via P2P (substrate replication — currently Plan 1 partial-ship).

`shem` k8s node is the multi-tenant "expand more peers as needed" surface; the others are family-scale single-pod nodes. The node split is deliberate — it lets us test cross-node flows (real P2P) rather than intra-pod simulations.

**Why:** This was clarified after a debugging session where the prior frame assumed alpha was 3 peers (adam/matthew/frank) and matthew was the singular doorway storage URL. Reality is 6 peers; STORAGE_URLS is fully populated; STORAGE_URL singular = matthew; only frank is steward-registered (separate bug to investigate).

**How to apply:** When debugging alpha symptoms involving "where is this byte / why isn't this peer reachable", consult this topology first. Don't assume single-peer or 3-peer. The bootstrap pair (adam+matthew) is the upload entrypoint; replication-from-genesis-peer to the other 4 peers is currently aspirational pending Plan 2/3 (verifier + reconstruction).
