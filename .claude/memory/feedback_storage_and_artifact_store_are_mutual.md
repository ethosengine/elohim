---
name: storage-and-artifact-store-are-mutual
title: "Storage and artifact store are mutual"
description: "elohim-storage and the artifact store hold and shape each other; send gaps to storage as its own work, never re-invent its primitives"
metadata:
  node_type: memory
  title: Storage and artifact store are mutual
  type: feedback
  originSessionId: ab4528a1-72ba-4517-856c-4277b174922c
  modified: 2026-11-11T18:03:15.697Z
---

Operator, 2026-11-11: elohim-storage and the artifact store are mutual, not separate. Storage's blob plane holds the artifact store's bytes. The artifact store holds and builds storage's own binaries (the F2r receipt is elohim-storage moving through the artifact store). Each one's needs shape the other's primitives. Example: the artifact store's per-leaf custody need became storage's own backlog row (arch-dataplane-refactor-backlog row 23).

**Why:** treating the seam as a boundary to defend leads to duplicated mechanisms (the artifact store re-inventing custody) or to findings that never flow back to the component that should change.

**How to apply:** when artifact-store work reveals a gap in a storage primitive, send it to storage as storage's own work (a backlog row on the right cluster, a flow note on its habit), and let the artifact store inherit the result. Code ownership still decides who implements what; the concern is shared. The seam exists for our own internal discipline and mastery ("know thyself"): it is how we know what each part is for, so we can master it, not a contract between strangers. It is also an empathetic exercise: knowing and loving ourselves well is what lets us fully know and love others, so the clarity we build at our own seams is what we can then offer the protocols, peers and systems beyond us. Use it to ask "inside or beyond the protocol", not "whose problem". Related: [[feedback_bridge_native_toolchains_not_lcd]], [[feedback_inherit_substrate_ontology]].
