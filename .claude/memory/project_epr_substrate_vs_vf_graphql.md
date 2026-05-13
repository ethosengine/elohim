---
name: EPR substrate is not VF-GraphQL
description: EPR envelope/codec/libp2p work is graph substrate (parent session scope), not shefa-speaks-VF-GraphQL (R&O lessons #4). Do not conflate.
type: project
originSessionId: 6e6b33c4-4e5a-4f96-9d46-9e2f7fc4fcc9
---
EPR Phase 1 (codec), 2A (storage), 2C (libp2p `/elohim/epr-atom/1.0.0`) delivered the graph substrate: signed envelopes with reach/coupling/claims, carried as opaque payloads over a federation transport. This is the foundation several downstream pieces depend on.

This is NOT the same as R&O lessons roadmap #4 "hREA alignment — shefa speaks VF-GraphQL." #4 is the application layer: Apollo client, `@valueflows/vf-graphql-holochain`, Resource/Event/Agent/Plan types, REA CFN mappers, a GraphQL endpoint. As of 2026-04-24 none of that exists.

**Why:** The substrate landed Apr 21–23 in parallel with the R&O lessons campaign. It is easy (I did this once already) to look at "EPR works, it's graph-shaped, it federates" and mark #4 complete. That is wrong and would rot the roadmap.

**How to apply:** When reviewing R&O lessons status, or when brainstorming shefa work: "does the package.json have apollo/valueflows/graphql/hrea, and is there a `.graphql` schema file?" If no — #4 is still 🔴 regardless of how much EPR work has landed. EPR is the primitive #4 will build *on*, not a replacement for it. Roadmap doc: `genesis/docs/plans/2026-04-21-rno-lessons-roadmap-handoff.md` §0.
