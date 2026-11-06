Legend: ✅ directly stated in a source/spec; ◐ synthesis or accepted current-state input not independently re-derived. No scenarios were executed.

## A. Why custody exists

The lineage consistently separates authority, consent, physical possession, and proof:

1. ✅ “Resilience IS resilience; visibility of resilience is what builds trust, safety, and acceptance.” `genesis/docs/superpowers/specs/2026-05-02-blob-custody-reconciliation-design.md:21-28`
2. ✅ “Inventory gossip can run cleanly while bytes never actually replicate.” `genesis/docs/superpowers/specs/2026-05-02-blob-custody-reconciliation-design.md:217-225`
3. ✅ “Salvage authors intent; Phase-2 moves bytes.” `genesis/docs/superpowers/specs/2026-06-24-blob-custody-phase3-xor-salvage-placement-design.md:104-110`
4. ✅ “A peer is never silently conscripted.” Opt-in is false by default; only fresh advertised capacity enters the candidate pool. `genesis/docs/superpowers/specs/2026-06-24-blob-custody-phase3-xor-salvage-placement-design.md:114-121`
5. ✅ “Custody may exist … but confers no authority whatsoever.” `genesis/docs/superpowers/specs/2026-08-05-ownership-custody-inalienable-red-team-design.md:75-81`
6. ✅ “For an inalienable subject, freely transferable custody is precisely the attack.” `genesis/docs/superpowers/specs/2026-08-05-ownership-custody-inalienable-red-team-design.md:104-113`
7. ✅ “The manifest is derived, never notarized — and derivation is verified, not trusted.” `genesis/docs/superpowers/specs/2026-08-23-swarm-curve-and-blind-custody-design.md:131-140`
8. ✅ “Custody commitments are the only notarized objects.” Possession hints and shard manifests are operational or derived, never truth. `genesis/docs/superpowers/specs/2026-08-23-swarm-curve-and-blind-custody-design.md:141-147`
9. ✅ “Custodians are never inspection agents and are never asked to unseal.” `genesis/docs/superpowers/specs/2026-08-23-swarm-curve-and-blind-custody-design.md:413-424`
10. ✅ Carrying capacity is “one byte number per device, derived, with a configurable override”; it is capacity after the runtime floor, not raw disk. `genesis/docs/superpowers/specs/2026-10-06-device-footprint-residency-carrying-capacity-design.md:182-199`

◐ The combined principle is: replication supplies availability, not authority or truth. Authority comes from commitments; evidence comes from verified serving; consent determines whether a peer may carry; placement and capacity determine where and how much.

## B. Custody vocabulary

| Term | Grounded meaning and home |
|---|---|
| ✅ `custody-blob` | Existing Class-A REA commitment: provider promises custody of a composite/blob to its receiver; one per manifest × custodian. `2026-05-02-blob-custody-reconciliation-design.md:34-38`; `2026-08-23-swarm-curve-and-blind-custody-design.md:250-270` |
| ✅ `replicates-content` | Content-tier Mishpat commitment naming an EPR/reach; counted as a replication promise but pledges zero capacity bytes by design. `genesis/data/timeline/backlog/capacity-tier-pledge-producer.md:20-33` |
| ✅ Self-custody | Runtime self-placement authors a deterministic active commitment only after verified placement, through the conductor rather than local SQL. `genesis/data/timeline/backlog/2026-07-30-custody-blob-first-commitment-auto-producer.md:106-118` |
| ✅ Salvage | An opted-in peer adopts an under-replicated blob by authoring new intent; ordinary reconcile then fetches the bytes. `2026-06-24-blob-custody-phase3-xor-salvage-placement-design.md:88-112` |
| ✅ Placement | A pluggable deterministic ranking of eligible holders; XOR was explicitly an MVP, not the final policy. `2026-06-24-blob-custody-phase3-xor-salvage-placement-design.md:54-86` |
| ✅ `placement-gap` | Durable REA observation that a custody promise remains unhonored beyond grace; an economic signal, not punishment. `2026-05-02-blob-custody-reconciliation-design.md:149-160,244-252` |
| ✅ Hold / arrival / release | A hold names why bytes remain; every arrival records how and for whom bytes arrived; another peer’s placement is released only by that placing peer’s withdrawal. `2026-10-06-device-footprint-residency-carrying-capacity-design.md:101-146,177-180` |
| ✅ Blind custody | Custodians store and serve ciphertext shards but cannot read them; admission requires a commitment, and reader keys/rings remain separate. `2026-08-23-swarm-curve-and-blind-custody-design.md:380-411` |
| ✅ Swarm curve | Throughput grows with distinct useful shard holders, approximately `min(down, min(h,k)·u)`; per-shard deltas make incomplete peers immediately useful sources. `2026-08-23-swarm-curve-and-blind-custody-design.md:156-176` |
| ✅ Carrying capacity | Derived usable bytes divided into own/dwelling/collective/commons reservations, with admission before movement. `2026-10-06-device-footprint-residency-carrying-capacity-design.md:182-228` |
| ✅ Residency | Private device-pin state describing bytes kept locally; it is never broadcast and is distinct from following or carrying. `2026-10-06-device-footprint-residency-carrying-capacity-design.md:69-99` |

## C. Design intent versus wired state

| Spec | Decision | Wired/open |
|---|---|---|
| ✅ Phase 2, 2026-05-02 | Manifest/reality/diff reconciliation; fresh inventory; verified race fetch; placement-gap events; shared fetch path. `:30-44,126-215` | ◐ The accepted current state says inventory, custody reconciliation and verified race-fetch landed. Stage-2 signatures remain open. The document itself was pre-implementation. `:3,263-270,304-310` |
| ✅ Phase 3, 2026-06-24 | Opt-in whole-blob salvage through a placement-strategy seam; XOR as coordination-free MVP. `:27-39,88-125` | ◐ Accepted current state says salvage with configured target replicas landed. Intentional household/failure-domain policy was P3-8 deferred. `:127-137,155-168` |
| ✅ Inalienable red-team, 2026-08-05 | Ownership of bodies/intimate subjects is personal and non-transferable; custody gives no authority; `subjectStanding` should veto transfer. `:51-113` | ✅ Governance design, not wired ontology: the inalienable frame was explicitly not applied; outside-holon guardianship challenge remains undesigned. `:83-85,326-337` |
| ✅ Swarm/blind custody, 2026-08-23 | Derived manifests, composite commitments, bitfield inventory, k-of-n completion, encryption-before-store, blind custody and cross-plane quarantine. `:119-154` | ✅ At that date only underlying shard/race primitives and one-host private-replica proof existed. S0/S1/S4/S5/C6/T2 rows were future work. `:59-117,467-493` |
| ✅ Footprint, 2026-10-06 | Separate follow/reside/carry; universal hold resolver; arrival provenance; admission and capacity bands. `:69-228` | ✅ Existing retention is narrow: release-ledger blobs plus content and live custody checks. General holds, arrival records, withdrawal, capacity derivation and admission remain missing nodes/slices. `:108-146,294-355` |

Requested backlog-item status:

- ✅ First commitment: `done`, but its own residual says salvage-authored commitments were created without activation. `2026-07-30-custody-blob-first-commitment-auto-producer.md:106-142`
- ✅ Chunked >16 MiB restart durability: `resolved`, 7/7 restart proof; >64 MiB RS ingest, shard inventory and cross-peer manifest residuals remain separate. `chunked-blob-over-16mb-not-durable-mesh-repro.md:216-251`
- ✅ Peer fallback: `superseded-in-code`; pointer propagation/heal landed, while retirement of per-host staging remains open elsewhere. `dataplane-peer-fallback-and-blob-replication.md:10-12`
- ✅ Direction decision: still marked `proposed`; later work effectively chose substrate pull rather than fan-out seed. `epr-blob-replication-direction.md:62-72`
- ✅ Workspace artifacts: `open`, blocked on the steward-channel T4 gate. `workspace-artifacts-over-steward-link.md:10,20-33`
- ✅ Native Track E blob store: `backlog`; local ≤1 MiB CID chunks, daemon-down put, scoped serving and zero-peer failure tests are unbuilt acceptance. `native-dev-track-e-tier-blob-store-slice1-local-chunked-put.md:10,19-38`

The borrows backlog is globally `status: backlog`; no row is wholly completed. ✅ Open rows are 1–6, 8–9, 12–13, 15, 17–18 and 21–22; row 7 is refactor-blocked, row 10 vision-deferred, row 11 awaiting a driver, row 14 is partial (L0 landed/L2 open), row 16 is partial (storage half cured, doorway half open), row 19 is unverified research, and row 20 is a draft with nothing built. `arch-dataplane-borrows-backlog.md:34-58,73-179,214`

## D. Replication semantics today

- ◐ Replica target: one configured scalar is applied per blob; there is no authority-bearing heterogeneous per-blob target in this lineage. Phase 3 specifies `salvage_target_replicas`, default 2. `2026-06-24-blob-custody-phase3-xor-salvage-placement-design.md:92-110`
- ✅ Failure domains: Phase 3 explicitly warns that two peers are not necessarily two households; household/failure-domain placement was the highest-priority deferred strategy. `:127-137,166`
- ✅ Freshness: Phase 2 specifies a 600-second inventory freshness horizon and TTL eligibility; honored counts use fresh rows. `2026-05-02-blob-custody-reconciliation-design.md:65,138-147,316-325`
- ✅ Repair bounds: Phase 2 specifies 10 kicks/peer/minute, five-second peer timeout, parallelism three, 120-second sweeps and placement-gap cooldown. These are global operational controls, not a per-blob repair budget. `:304-325`
- ✅ Future home: failure-domain policy belongs behind `PlacementStrategy`; capacity admission belongs in device bands/commitments; shard repair should be bounded by k-of-n, inflight limits and pledged capacity. `2026-06-24…md:54-86,127-137`; `2026-08-23…md:141-176`; `2026-10-06…md:201-228`

## E. Inventory trust

✅ Phase 2’s Stage 1 accepts structurally non-empty signatures; Stage 2 is Ed25519 over canonical inventory bytes with verification on ingestion. `2026-05-02-blob-custody-reconciliation-design.md:69-95,304-310`

✅ The later swarm design strengthens the requirement: richer bitfield hints must not ship until the payload `peer_id` is bound to the propagation source; punitive demotion must wait for Stage-2 signatures so a forged claim cannot frame a victim. `2026-08-23-swarm-curve-and-blind-custody-design.md:97-99,199-209`

✅ There is no dedicated inventory-authentication row in `arch-dataplane-borrows-backlog.md`. Adjacent rows are PSI topic privacy (#6), reach-at-replication (#12), blob-inventory negentropy (#21), and unsigned device/person discovery binding (#22). `arch-dataplane-borrows-backlog.md:41,47,55-56`

## F. Ranked inheritance for the artifact sprint

1. ✅ Bind inventory claims to transport source and finish Stage-2 signatures before expanding holder hints. `2026-08-23…md:199-209`
2. ✅ Close large-artifact correctness: composite re-hash, manifest-associated push, manifest-aware custody, >64 MiB RS ingest and shard-only inventory. `2026-08-23…md:467-480`; `chunked-blob…md:216-226`
3. ✅ Repair salvage create→activate so autonomous commitments participate in active-custody folds. `2026-07-30-custody-blob-first-commitment-auto-producer.md:132-142`
4. ✅ Make targets policy-bearing: household/failure-domain diversity, capacity and standing—not merely one global count. `2026-06-24…md:127-137,166`
5. ✅ Land holds/arrival/admission so every artifact has provenance, a retention reason and a charged commitment. `2026-10-06…md:294-349`
6. ✅ Add bounded partial transfer: borrows row 2 per-block verification/range fetch, row 13 priced hedging, and row 16 receiver-granted bulk lanes. `arch-dataplane-borrows-backlog.md:37,48,51`
7. ✅ Keep blind/private artifacts out of public blob/cache paths until reader keys, encrypted store types, custody admission and quarantine actuation exist. `2026-08-23…md:304-341,467-492`
8. ✅ Then connect local artifact storage to the steward link: Track E first, workspace-to-workspace closure fetch second. `native-dev-track-e-tier…md:19-38`; `workspace-artifacts-over-steward-link.md:20-33`

## G. Habits and runnable evidence

- ✅ `blob-durability`: green, inactive, never retires because silent byte loss is irreversible. Latest chronological delta is the 2026-10-18 household flip back to green; the literal last five file-order deltas progress 1/3 → identity-fork diagnosis → 2/3 → 3/3 chaos drills, then name a wedged-but-gossiping salvage gap outside the habit’s loss/churn claim. `blob-durability.habit.md:3-17,19,244-249`
- ✅ `dataplane-convergence`: red, active, never retires because convergence is constitutive. Its last five deltas all preserve red and explicitly make no household/fleet delivery claim. `dataplane-convergence.habit.md:3-21,75-78,1107-1115`

✅ The direct durability scan contains 95 scenarios: 26 lack `@wip`, 69 carry it; after the habit lane’s browser/Shem exclusions, the exact 14 runnable scenarios are:

- App heal: “First request … heals and serves,” “artifact over the erasure-coding threshold,” and “chunked artifact … after restart.” `app-blob-heal-on-read.feature:30,82,94`
- Chaos: “flapping peer,” “cascading peer loss,” and “simultaneous loss of two peers.” `chaos-peer-churn.feature:92,112,149`
- Grandma: node offline, holder lapse, honest unseen album, pro-social shortfall action, seeded album stewardship, and live household distribution. `grandma-photos-survive-node-loss.feature:20,32,43,54,83,98`
- Observable distribution: diversity shortfall and present-zero shortfall. `observable-distribution.feature:368,378`

✅ The five salvage-placement scenarios are all `@wip`: closest opt-in adoption, no conscription, non-closest deferral, no adoption at target, and felt protection after adoption. `salvage-placement.feature:1,31-77`

✅ Supplementary direct custody scenarios: wiped-device recovery, ambient household custody, and both reciprocity scenarios are non-WIP; gossip-independent recovery, projection reconciliation, anchored fixture custody, and degraded XOR strategy are WIP. `federation/peer-recovery.feature:36,54,69`; `qahal/household-formation.feature:56,64`; `resilience/household-reciprocity.feature:21,26`; `trust/trust-legibility-atlas.feature:111`

✅ `dataplane-convergence` names executable checks for session-envelope, federation/version convergence, edge sync/blob/mesh/fallback/inventory, transport parity, trust-priced sync, elected delivery, accountable correction, hub-carried edits, household-follow-head and device-declared-target. Several are intentionally born red, but they are runnable specifications. `dataplane-convergence.habit.md:9-21`