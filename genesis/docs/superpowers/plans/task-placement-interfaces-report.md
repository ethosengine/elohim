---
id: placement-interfaces-report
title: The composed interfaces of the placement contract — report
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#3
actor: agent:architect@claude-opus-5-5
habits: [materialize-what-is-read]
commits: []
cites:
  - "placement-interfaces-brief | The composed interfaces of the placement contract | sha256:f953a81da54d3247 | path: genesis/docs/superpowers/plans/task-placement-interfaces-brief.md"
  - "placement-contract-architecture-plan | The placement contract's architecture | sha256:2148e5a4de372761 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
  - "placement-contract-interfaces-design | The placement contract's interfaces | sha256:626e6405dcb35854 | path: genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md"
---

# The composed interfaces of the placement contract — report

Seat: I1 of `2026-10-20-placement-contract-architecture-plan` (gap `#3`). Output:
`genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md` (status Draft),
synthesized from the judges' winner (developer-ergonomics) with every graft the three judges named, and
each listed conflict resolved in the document's §5. Nothing staged, committed or pushed (architecture
before landing). No code and no schema file written.

Gate evidence (the context gate line was ambiguous across doc gates; the brief and plan name the cite
check):

```
$ epr flow cites seal genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md
   id: present  ·  cites: 4 sealed to envelope, 49 legacy-kept, 0 path: locator(s) stamped
   ✅ gate: all cites content-addressed + resolvable
$ epr flow cites verify genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md
epr flow cites verify …: ✅ all cites content-addressed + resolvable
EXIT=0
```

## What the document fixes

Crate graph and rails (§1); seams A–I with signatures, test contracts, implementors and callers,
extension points, first instances and one worked example each (§2); the reuse map and a retire table
with retire-when conditions (§3); performance notes (§4); conflict resolutions and operator decisions
(§5).

## Signatures checked against the code (findings that changed the winner)

- `BlobCid` derives no `Ord` (`address.rs`); `cid::Cid` 0.11 is `Copy + Ord`, so `BlobCid` gains it
  additively. `FetchPolicy` is `Clone` only and gains `Copy`.
- `AgentRef` (`epr-rea/src/model.rs`) and `Reach` (`epr/src/reach.rs`, deliberately) have no `Ord`; no
  derive in the document orders over them; ranking tie-breaks on the agent string.
- `DirectoryStorage::has_blob` calls `read_object` (whole read and hash); the chain never calls it.
- `GRANT_SCOPES` is a private const (`compute_grants.rs:46`); the winner's `scope: &'static str` under
  `Deserialize` would not compile; replaced by a closed `GrantScope` with an SDK enum home.
- `elohim-compute` pulls chrono, dashmap and parking_lot, but `actuation.rs` uses only serde; ark-core
  already depends on it. placement-core takes it with default features off after a follow-on feature
  split.
- The slice-1 walk is in the working tree with `Placement` and a non-`#[non_exhaustive]`
  `AbsentReason`; slice 1's five `TreeError` variants already break brit's registry pin `0.1.2`, so the
  first publish carrying `walk` is `0.2.0`.
- Wire view types now live in `elohim/elohim-views`; elohim-storage and eprfs are separate workspaces
  from `elohim/Cargo.toml`, and elohim-storage already path-depends on `ark/core` (precedent for ark-core
  path-depending on eprfs-core).
- The existing evidence vocabulary is `fixture | real` (`evidence-class-descends`); the probe uses it.
- The spec says leases are "renewed by nothing periodic" and a revert is a new declaration; all three
  proposals carried renewal timers and a backward-moving head. Both corrected.

## Concerns (why DONE_WITH_CONCERNS)

1. Signatures are checked by reading, not compiled. I2 (the adversarial seam check) is the next item and
   should compile the eprfs-core additions against the working-tree `walk.rs` first.
2. Operator or spec-owner rulings are needed before the seats that build on them land (§5 items 3, 6, 8,
   15): elohim-storage depending on eprfs-core and eprfs-serve; incumbency scoped to the parameter set in
   force as the reconciliation of hysteresis with revert; the built-in parameter set under R5; plan S2's
   wording against spec §17 slice 9's "each field lands with its reader".
3. Lane E must confirm the slice-1 renames (`Placement` → `Coverage`, `#[non_exhaustive]` on
   `AbsentReason`) before E1 lands; the document writes the target names.
4. Follow-ons outside the covenant's write-set, named and not written: elohim-placement-core
  (`elohim/placement/core`), ark-core `task.rs`, the elohim-storage adapters (`BlobStoreTier`,
  `PeerTransportTier`, `RootReachGate`, custody `Ports`, the `PlacementStrategy` shim, the
  `GRANT_SCOPES` equality test), the elohim-compute `runtime` feature split and the additive
  `LimitOwner::Charter`, seam-contracts `probe.rs` and `check_ordering`, `verifyAddress` in
  `@elohim/epr`, storage-client-ts `HttpTier` and placement surface, and `apps-sw.ts`. Inside the
  write-set and assigned to lanes: eprfs-core modules, `eprfs-serve`, `@elohim/eprfs` (lane E); the
  schemas of §2.G (lane S).
5. No rule floor was needed for this read-and-write-a-doc work and none is claimed.
