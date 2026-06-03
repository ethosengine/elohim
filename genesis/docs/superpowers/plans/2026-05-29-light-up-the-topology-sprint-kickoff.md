---
id: light-up-the-topology-sprint-kickoff
status: Draft
cites:
  - ../specs/2026-05-29-durability-topology-felt-resilience.md   # the related doc this derives from
---

# Sprint Kickoff — Light Up the Topology: Felt Resiliency, Peer-Hosted EPR-Apps & the Reach/Hosting Boundary

> **Kickoff prompt** for the next sprint intensive, captured 2026-05-29 while shaking out
> EPR-app delivery. Grounds the two vision specs into an executable epic set with per-epic
> built-vs-gap state (verified by a 3-scout sweep, file-level). Run in parallel with the
> live `alpha.elohim.host` / `elohim.host` delivery shake-out.
>
> **Read first:** `2026-05-29-epr-reachability-economics.md` (reach + delivery axes, doorway
> two-hat model, thin-fediverse) and `2026-05-29-durability-topology-felt-resilience.md`
> (persistence axis, built-vs-endgame map, the three gaps).

## The frame (one paragraph)

The substrate is **~70% built and sitting on a solid deterministic floor.** The gap to "it
just works" is overwhelmingly **connective tissue, committed-accounting readers, and joining
the elohim ceiling to the floor** — NOT new DHT entry types. Three axes graduate along one
intimacy gradient (self → dwelling → collective → commons): **Reach** = permission (who may
see), **Delivery** = economics (who pays to move bytes now), **Persistence** = durability (who
guarantees survival), plus **Recovery** (who restores). The sprint makes these *felt*: a
person can log in cleanly, see their replication/delivery and free-vs-stewarded storage, work
EPR-link context menus, navigate peers, and read the reach↔hosting boundary intuitively — on
top of a floor where gates/determinism stay in Rust and the elohim `.ts` is sense-and-respond,
never the evaluator.

## Doctrine constraints (hard gates for every epic)

- **Gates/determinism in Rust; elohim `.ts` is sense-and-respond.** No agent becomes an evaluator.
- **No new DHT entry types** unless a `p2p-design-gate` pass proves it (most epics are wiring/readers/UI).
- **snake_case never leaves Rust;** views are camelCase via `views.rs`. No transforms in TS.
- **`project-epr` is a sponsorship / named-front-door primitive**, not a per-content sitemap.
- **Doorway holds no canonical data** (thin edge): projection + cache + web2 service contract only.
- **governable ≠ seizable** for any doorway/node governance work (graduated, consent-based, anti-capture).

---

## Epics (each: outcome · built today · the gap · doctrine note)

### Epic A — Light up the topology (connective tissue + posture UI)  ·  *durability gap 1*
**Outcome:** a person can *find* and *read* the topology without knowing URLs.
- **Built:** `MyClusterComponent` `/shefa/cluster` + `PeerTopologyComponent` `/shefa/peers` (both now in Shefa sidenav); `DeviceTileComponent` free/used/stewarded triptych (data live); per-EPR `ResilienceView` "Network" tab (`api/resilience.rs`); progressive icon `●◐○` on EPR titles; `ConnectionIndicatorComponent` ambient "network up" chip.
- **Gap:** top-level posture view — `ResilienceSnapshotView`/`NetworkPostureView`/`TopologyOverviewView` wire types exist with **no consuming component**; free/used rendered as `<dl>` text **not a clickable bar**; peer/device cards are **not links** (no drill-down).
- **Note:** mostly Angular wiring on live wire-types; no new substrate.

### Epic B — Deliver REAL resiliency (committed accounting + prioritizer wiring)  ·  *durability gap 2*
**Outcome:** the free/used bar shows **pledged-vs-held with the storage-premium visible**, and commitments actually shape what peers cache.
- **Built:** `cluster_view.rs` reconstructs observed free/used + stewarded per-request; `constitutional_ratio_registry` (donut clamp) LIVE; `bounds_validator` (7 checks) LIVE; RS(4,3) encode/decode (`sharding.rs`) LIVE; `replication.rs` state machine wired; `shard_manifests`/`shard_locations`/`stewardship_allocations` full CRUD; `placement_gaps`/`household_resilience` live projections.
- **Gap:** `peer_capacity_service` **3 reader stubs return 0** (`query_latest_total_raw_bytes`, `aggregate_pledges_by_tier`, `compute_unique_shard_bytes`) → the committed bar reads zero; `replication_prioritizer::score_advertised_blob` is **DEAD CODE** (never wired into `drain_gap_queue`/inventory subscriber) → commitments don't yet shape the cache.
- **Note:** implement the 3 readers (system-sample available_bytes; rea_commitments pledges by tier; DISTINCT peer_blob_inventory bytes) + wire the prioritizer. Pure backend; no new types.

### Epic C — Peer-hosted EPR-apps + doorway Role-2 resolver  ·  *reachability §3*
**Outcome:** a doorway serves **any commons EPR the substrate holds**, not just sponsored front doors; peer-hosted apps light up.
- **Built:** Role 1 (Projector) — `epr_router.rs` named front doors, longest-prefix dispatch, SSE-updated. The current shake-out (eager-projection → `alpha.elohim.host/lamad`; apex via availability-gated seed-stage).
- **Gap:** Role 2 (Resolver) is **ABSENT** — no "EPR head/CID → DHT provider records → pull bytes over libp2p → cache → serve." Single-target dispatch by design today.
- **Note:** **REQUIRES a `p2p-design-gate` pass.** Reach enforced peer-side (serving peer checks standing); doorway enforces only its own sponsorship boundary. This is the "new internet" resolver epic.

### Epic D — Clear login flows + the Account-Management surface  ·  *imagodei M5*
**Outcome:** a non-technical person logs in, manages security/sign-in, and sees recovery — end to end.
- **Built:** `LoginComponent`/`AuthService` (93%) /password+OAuth+Tauri providers all LIVE; doorway `auth_routes.rs` full surface incl. recovery handlers; `RecoveryCoordinatorService` (98.7%) + Request/Interview/ElohimVerify components LIVE and DNA-notarized; social Profile surface LIVE.
- **Gap:** **Account-Management surface MISSING in elohim-app** (only a hosted-only stub in doorway-app) — no Security & Sign-in pane; **no post-recovery key rotation** (recovered human regains access but the compromised key isn't revoked — dangerous); graduated-authority tier (intimate→qahal→witness) **not surfaced** to the claimant; passkey/2FA modeled but unimplemented; no password-reset CTA.
- **Note:** build `imagodei/components/account-management/` (Home · Personal Info · **Security & Sign-in** · Third-party Apps · Data & Privacy · People & Sharing). Recovery UX + key rotation live in Security & Sign-in.

### Epic E — EPR-link context menus + the intuitive reach/hosting boundary  ·  *reach/delivery/persistence felt surface*
**Outcome:** clicking an EPR link reveals what you can do with it, and the reach↔hosting boundary reads at a glance.
- **Built (more than expected — the primitive exists, it's the integration that's missing):**
  - `<elohim-context-menu>` Lit primitive — `elohim-elements/elohim-core/src/elohim-context-menu.ts` + spec; full Capability-Profile JSDoc; `ContextMenuItem[]` + `open` contract; **Library A default + Library B designed stories** (`graphos/.../elohim-context-menu.{default,designed}.stories.ts`). **Built and story-covered — but ZERO consumers in `elohim-app/src`.**
  - `<elohim-epr-link>` — built AND integrated into elohim-app via `EprLinkComponent` (thin wrapper); `EprPopoverComponent` (hover, read-only: title/type/tags, lamad metadata, shefa steward count, qahal reach icon `◉◎◍○` + constitutional layer).
  - `EprRelationshipCardComponent` (TEACHES/CONTAINS/REFERENCES/PREREQUISITE + resilience badges); `ContextMenuOnlyComponent` in qahal (flag/challenge/feedback) — both exist, **neither wired to EPR links**.
- **Gap (integration, NOT greenfield):** the `<elohim-context-menu>` primitive is not consumed by any client surface. Wire it into `EprLinkComponent` (open on right-click/long-press), populate its `ContextMenuItem[]` with EPR actions — view, **navigate-to-Network/resilience tab**, steward/replicate, see-relationships, governance (reuse qahal `ContextMenuOnlyComponent`'s flag/challenge/feedback outputs). The reach (permission) vs delivery (economics) vs persistence (durability) **boundary isn't made legible** even though all three glyph sets exist.
- **Note:** this is element→app **integration** — the primitive + stories are done. Pair with `angular-architect` (app wiring) + `component-architect`/`graphos-designer` only if the primitive needs a claimed-action extension. Surface the three-axis boundary as the felt distinction (a commons EPR is *readable* but its bytes may be *metered* and its survival *attested* — three facts, three glyphs).

### Epic F — Replication & delivery stats  ·  *partly B + reachability §4*
**Outcome:** a steward sees replication health AND delivery activity (what they served, for whom).
- **Built:** replication stats LIVE — `api/resilience.rs` (households stewarding, online peers, health), `placement-gaps`, `/p2p/status` (pending/completed/failed/caught_up).
- **Gap:** **delivery stats ABSENT** — no bytes-served / who-pulled / toll-settled endpoint; no `infrastructure:blob-served` aggregation surfaced; finance-bridge/toll settlement absent.
- **Note:** the `blob-served` observation primitive exists conceptually; add the aggregation + endpoint. **Toll/settlement economics is a separate decision** (may defer monetary tolls for v1; serve-stats *visibility* is valuable now and stewardship-legible).

### Epic G — Join the elohim ceiling to the floor  ·  *durability gap 3*
**Outcome:** the elohim can finally "collapse operational complexity" of topology — as sense-and-respond.
- **Built:** deterministic floor (Epic B substrate) + a live wisdom gate (`Phase::DevContext` vs `ElohimActive` observed from real call outcomes).
- **Gap:** `wisdom.rs` input shape accepts only constitution/framing/event_summary → no agent can reason over placement-gaps, resilience snapshots, or inventory advertisements.
- **Note:** grow the wisdom input shape + feed it the live projections (placement signals are shefa inputs). Gates stay in Rust; the elohim only senses and suggests. Sequence LAST (depends on B).

---

## Suggested sequencing

1. **Finish the shake-out** (in flight): eager-projection → `alpha.elohim.host/lamad`=200; apex via availability-gated seed-stage. *This validates the reach path end-to-end first.*
2. **Epic B** (committed-accounting readers + prioritizer wiring) — unblocks the data behind A/F.
3. **Epic A** (connective tissue + posture UI) — makes B visible; cheap, high felt-impact.
4. **Epic E** (EPR context menus + boundary legibility) — parallel with A (both Angular).
5. **Epic D** (account-management + recovery UX + key rotation) — parallelizable (imagodei lane).
6. **Epic C** (Role-2 resolver) — needs its own `p2p-design-gate`; the big "new internet" lift.
7. **Epic F delivery-stats** + **Epic G wisdom-input** — after B lands.

## Operator-lane / infra dependencies to clear alongside
- Alpha conductor runtime: confirm `fa4c6aa0` DNA installed + signal subscriber live (or rely on eager-projection).
- DNA pipeline `/cargo-target` PVC disk-full (#1303) — freezes happ publishes.
- `elohim-doorway-app/dev` Jenkins job missing (orchestrator marks UNSTABLE) — provision or de-register.
- Consider: publish-but-mark-unstable for the packed happ so a sweettest failure doesn't freeze deployability.

## Grounding cross-refs
Both 2026-05-29 specs; memory: `project_substrate_floor_elohim_ceiling`, `project_placement_signals_are_shefa_inputs`, `project_elohim_agent_sense_respond_architecture`, `project_p2p_is_hosting`, `project_three_layer_truth_model`, `project_dwelling_hub_replication_pattern`, `project_graduated_recovery_authority`, `project_recovery_grandma_standard`, `project_m5_reframe_auth_portal_convergence`, `project_imagodei_three_surfaces`, `project_seed_whoever_is_ready`, `project_trust_as_efficiency_signal`.
