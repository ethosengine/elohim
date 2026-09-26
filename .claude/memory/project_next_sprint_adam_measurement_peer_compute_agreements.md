---
name: project_next_sprint_adam_measurement_peer_compute_agreements
title: Next sprint — adam as measurement peer, compute agreements
description: "Operator 2026-09-25: next sprint = adam on shem accepts declared rakia compute agreements (run, store, attest), measure-class runs offload from the dev berth, ark/berth limits make pod equivalents for epr-apps."
metadata:
  node_type: memory
  title: Next sprint — adam as measurement peer, compute agreements
  type: project
  originSessionId: 9adf9f01-3f4d-4b27-8758-d78fda7cada3
  modified: 2026-09-25T13:21:01.318Z
---

Named by the operator 2026-09-25 after the K0 A/B held the dev berth ~16 h. Captured in the
native-delivery plan ("Next sprint captured", commit on dev 2026-09-25); the brainstorm + spec is the
entry point, composing four existing Draft specs rather than minting a new one:
`2026-09-08-peer-executed-stage-design`, `2026-09-02-compute-envelope-tevah-design`,
`2026-09-05-k8s-bridge-runtime-envelope-render-design`, `2026-09-03-workspace-berth-carrying-capacity-design`.

The four intents: (1) overhead on shem so arks/berths provision within adam's k8s budget (k8s bridge
render is the declared home; cluster change is the operator's); (2) the rakia job is a declared
compute agreement — an REA Commitment inherited from epr-rea/elohim-compute, never a new table — that
adam (k8s-aware native runtime) accepts, runs, stores for an agreed retention and optionally attests
via brit; (3) measure/profile-class runs travel to the peer holding the stores so development stays
non-blocking; (4) tame CPU/RAM first (per-hosted-agent publish selector/COUNT/ScheduledFunction cost ∝
table size; WAL checkpoints only 90 s after boot; heartbeat writer; ~786 MB heap per hosted human)
so ark/berth limits become the pod equivalent for installing, running, scaling, archiving epr-apps.

**How to apply:** when this sprint opens, run the P2P design gate on the agreement + receipt before any
route; start from the plan's capture and the four specs, not from Jenkins. See
[[feedback_measure_class_runs_off_dev_berth]], [[project_compute_envelope_three_homes_k8s_bridge]],
[[feedback_k8s_is_not_the_architecture]], [[project_native_delivery_sprint_2026_09_24]],
[[reference_node_hardware_ethosengine_shem]].

**Grounding corrections (2026-09-25 composition pass, plan `/projects/.claude-config/plans/we-ran-into-a-cryptic-robin.md`):** the "compute agreement" already exists as a wired DHT chain — `delegates-compute` grant (mishpat) → `work` Commitment `rakia-compute-request-v1` (elohim DNA, `content_store::compute_task`) → provider `accept`/`complete` EconomicEvents with the receipt CID — measured on the household 2026-09-07 with jessica standing in for adam. The primitive lives in **mishpat + elohim-storage** (`api/compute_grants.rs`, `compute_tasks.rs`), NOT the `elohim-compute` crate (that is reporting + actuation). Nothing binds matthew and adam today (different fixture households; trust seam inert). Rulings R1–R11 in the plan: no new entity/route/zome; scope `measure-stage`; admission stays by the requester's key with the grant enforced at launch and at the completion read; the berth lease becomes the router (refuse `measure` on the dev berth + declared override that emits pain); rung H (household) proves the chain, rung A (adam) proves offload and is blocked on operator items (executor pin, image/key/Secret, slice numbers — the pod is already over its envelope — and a checkout+mesh on adam's berth).
