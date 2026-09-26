---
name: project_delivery_sprints_2026_09_24_25
title: Delivery sprints 2026-09-24/25 (umbrella)
description: "Plans hold sprint state; kept here: pin now e0bfc6c7a, 7e553f9c3 unbisected, fork has no forge CI, K0 store archive, auditor routine, adam compute-chain home."
supersedes: [project_native_delivery_sprint_2026_09_24, project_post_station_4_sprint_2026_09_25, project_next_sprint_adam_measurement_peer_compute_agreements, project_conductor_pin_7e553f9c3_convergence_regression]
metadata:
  node_type: memory
  type: project
---

# Delivery sprints 2026-09-24/25 (umbrella)

The plans and their rulings logs are the record; resume from them, not from here:
`genesis/docs/superpowers/plans/2026-09-24-native-delivery-sprint-plan.md` (lanes, Tasks, rulings,
"Next sprint captured") and
`genesis/docs/superpowers/plans/2026-09-25-post-station-4-participant-classifier-search-attention-sprint.md`
(landed `73a351fd5`, since pushed; result + rulings under `genesis/a2o/reports/post-station-4-2026-09-25/`,
gitignored).

Pointers that bite before anyone opens a plan:
- **Conductor pin:** moved 25dd2d0be → e0bfc6c7a (`ee8c7a78b`) — the sargable arc-read cure alone. It cut
  launch→first zome call from 15–19 min to ~1 min; steady-state CPU and the never-checkpointing WAL are
  unchanged (K2 fork fix, H3 checkpoint cadence). **7e553f9c3 is still unbisected:** it missed household
  doorway-B convergence 3×, but host load dominated, so settle it only with a quiet-host A/B (both pins,
  load < 6). Suspects `0efa40939` (publish waits on local validation), `ff2ea44c6` (iroh receive throttle);
  the prebuilt `hc-fork-61565f320d0e` pair sits between them.
- **The fork has NO GitHub CI** (0 check-runs): a conductor's attestation is the elohim-edgenode Jenkins
  build, and a pin cannot deploy until its SHA is on the fork remote.
- Pre-A/B household stores (the CPU-fix reproduction): `/projects/.claude-config/k0-household-stores-20260925/`.
- Surprise auditor = cloud routine `trig_01Pm5R6UBgRUNFQoPDUWPNAR` (daily 09:17 UTC); it pushes
  `pain/<date>-<seed>` branches that the delivery-stasis pain-sweep station merges.
- **Compute agreement (adam as measurement peer):** already a wired DHT chain — `delegates-compute` grant
  (mishpat) → `work` Commitment `rakia-compute-request-v1` → provider accept/complete events carrying the
  receipt CID — in mishpat + elohim-storage (`api/compute_grants.rs`, `api/compute_tasks.rs`), not the
  `elohim-compute` crate. No new entity, route or zome. Door: `just measure`; design home:
  `genesis/docs/superpowers/specs/2026-09-08-peer-executed-stage-design.md`.

Opus implements, the controller rules and reviews ([[feedback_controller_holds_vision_opus_implements]]).
See [[feedback_local_mesh_first_cadence_and_measure_offload]], [[project_compute_envelope_three_homes_k8s_bridge]].

**Supersedes** (members unchanged):
- [[project_native_delivery_sprint_2026_09_24]] — the native-delivery sprint's pointers and holds.
- [[project_post_station_4_sprint_2026_09_25]] — the post-station-4 sprint's landing state and open
  backlog atoms.
- [[project_next_sprint_adam_measurement_peer_compute_agreements]] — the adam measurement-peer sprint's
  four intents and its grounding corrections.
- [[project_conductor_pin_7e553f9c3_convergence_regression]] — the 7e553f9c3 household A/B, suspects
  and bisect recipe.

**Authorship:** the native-delivery pointers and the next-sprint entry were contributed with
human:matthew as steward of record: no agent author was witnessed writing that content, and the record
does not say human:matthew wrote it. The operator rulings they relay remain the operator's.
