---
id: placement-interfaces-brief
title: The composed interfaces of the placement contract — one document every implementer builds against
status: Active
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#3
actor: agent:architect@claude-opus-5-5
habits: [materialize-what-is-read]
cites:
  - "placement-contract-architecture-plan | The placement contract's architecture | sha256:d2a76864bcbbebd1 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
  - "what-every-app-inherits-placement-contract-design | What every app inherits | sha256:2ae965e98bf63d04 | path: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md"
  - "placement-eprfs-slice1-brief | eprfs slice 1 | sha256:46be8b35cc71693b | path: genesis/docs/superpowers/plans/task-placement-eprfs-slice1-brief.md"
---

Item I1 (gap `#3`). The output is a new spec,
`genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md`, that says exactly how
each seam of the placement contract is shaped, so that every later slice fills it in without guessing. The
design spec says what and why; this document says how. It never restates the design spec's reasons; it cites
its sections.

Rulings in force: the design spec §1 rulings 1–6 and §7; architecture before landing; and the orchestrator's
rulings recorded on the plan:

- **R1** granularity per tree root and view, never per blob; long-lived leases; one settlement per commitment per period.
- **R2** below-public placement is household-local, not notarized, until per-space cells exist.
- **R3** a grant is consent and eligibility; a commitment is placement, self-adopted by the carrier.
- **R4** the floor reads only what it holds; method and parameter set named by CID; guarded self-candidacy; idempotent adoption; make-before-break moves.
- **R5** no space-elohim; every parameter set in force is ratified; three authorities (offer or posture, charter, parameter set).
- **R6** named absences; overlays on the writing device; the requester's own store first in every tier list; `fetch_blobs(root, …)`; reach on both sides; device facts never leave the device.
- **R7** (spec ruling 7): every unanswered outcome is one shared type carrying a typed reason, when to come back (a time or the event that changes the answer) and what can be done locally meanwhile; a repeat ask before that time escalates. Every refusal, absence or "not carried" across seams A–I uses it, so a missing part fails to compile. Name the type, its crate (the purest one every seam already depends on), and how each seam constructs it. A pending answer is a channel, not a one-off error: like a reactive stream's demand, backpressure and completion signals, it keeps reporting where it stands (queued, retrying at a given time, carried, refused with its reason) at its lane's quality of service, until the answer arrives or the need is withdrawn; a quiet channel costs nothing at rest. Define the status type, the channel's shape (pull cursor in the pure core, a stream adaptor at the edge), and its terminal states.
- **R6a** (amends R6, operator stance): doorways are plural, thin web2 participants governed as commons facilities by those who ingress through them. A browser reaching the network through a doorway, and the service worker asking one for its delivery peers, is by design. A doorway may supply tier-list entries. What a person relies on is plurality (no single doorway they cannot leave) and direct by-key reach where a peer can address others. Never frame a doorway as a defect or a dependency to minimize.

## What the document must contain

1. **The layer and crate graph**, with the dependency direction stated as checkable rails (like ark-core's
   `boundary::no_runtime_or_io_deps`): which crate owns each trait, what may depend on what, where the pure
   decision cores are (eprfs-core; the placement floor's pure core) and where the adapters are.
2. **One section per seam**, each with the exact Rust signatures (and TypeScript where an app or the service
   worker touches it); the contract every implementor satisfies, stated as a test contract; who implements it
   and who calls it; the extension point (how a new tier, placement kind, derivation method or need plugs in);
   the existing code that becomes its first instance or adapter; and one worked example. Seams:
   - (A) the storage tier and the tier chain, with reach as an injected policy;
   - (B) view, walk and query — adopt the slice-1 brief's API as given;
   - (C) derivation by identity;
   - (D) the placement floor: intents, offers, eligibility, ranking (generalizing `PlacementStrategy` and its
     agreement contract), adoption, the reconcile loop and its ports, the explanation record;
   - (E) the ceiling: draft, ratification reference, parameter-set head (interfaces only);
   - (F) a task placed on a berth;
   - (G) the manifest needs and the SDK schemas;
   - (H) the TypeScript SDK surface, and the service worker as a tier;
   - (I) the probe shape.
3. **The reuse map**: every existing piece of code that becomes an instance of a seam, and every parallel
   mechanism a seam retires.
4. **The performance notes** the shape guarantees: bounded memory, zero-copy bytes, CID short-circuits,
   batched fetch, where caches sit.

Frontmatter as the specs `.epr-meta` requires at birth (`id, status, class, context-tier, steward,
graduation-trigger, cites`). Cites as plain paths, then `epr flow cites seal`. Write no code and no schema file.
Report: `task-placement-interfaces-report.md` with `gap:`, `actor:` and `status:`.
