---
name: Humans register with multiple doorways for resiliency
description: A human is not bound to a single doorway — they register with multiple doorways simultaneously, so any one can service their access; design flows must be doorway-agnostic
type: project
originSessionId: 253292ea-69ea-4e76-86e3-6d87ebdac46c
---
A human registers with multiple doorways (doorway-A, doorway-B, doorway-C...) as a resiliency pattern. No single doorway is the human's "home." Any registered doorway can facilitate that human's access to the network — login, content fetch, recovery ceremony, governance participation.

**Why:** single-doorway binding recreates corporate-custody failure modes (that doorway fails → human is locked out). The protocol commits to never-absolute-lockout (graduated recovery authority), and single-doorway binding would violate that. Multi-registration puts the resilience at the human-level, not the doorway-level.

**Architectural implications:**

- **Flows must be doorway-agnostic.** Recovery ceremonies, login, content operations — none should assume "the human's doorway." The shared substrate (DHT as notary, libp2p mesh as data-ops) is what actually carries state; doorways are interchangeable entry points.
- **Invitations and signals fan out over libp2p mesh**, not via doorway-to-doorway routing. Gossipsub topics reach all elohim-storage pods regardless of which doorway-steward they belong to.
- **Data replicates across doorway-stewards' pods.** Sharded/replicated backups form the view of a human's data across multiple elohim-storage pods, so losing access to one doorway doesn't lose access to the human's history.
- **Hosted-conductor ceremonies can initiate at any registered doorway.** If doorway-A is offline, the claimant walks into doorway-B and starts the ceremony there. The ceremony references the human's DHT state which is shared.

**How to apply:**

- Any design featuring "the human's doorway" is suspect — name the plural. "A doorway the human has registered with" or "any of the human's doorways."
- Cross-doorway scenarios belong in a2o testing. A single-doorway a2o test is insufficient to prove multi-doorway claims.
- Doorway failure is a resiliency test case: if doorway-A goes down mid-ceremony, the claimant can retry at doorway-B and the ceremony state (on DHT) is still valid.
- Do not design doorway-to-doorway routing for claimant flows. Route via libp2p mesh; doorway is a local entry, not a forwarder.
