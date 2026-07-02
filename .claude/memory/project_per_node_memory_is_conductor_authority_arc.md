---
name: project_per_node_memory_is_conductor_authority_arc
title: Per-node GB memory = conductor full-arc authority
description: "Per-node 2→4GB+ climb (james OOM) = conductor full-arc DHT working set; target_arc_factor defaults to 1 so RAM ∝ corpus; arc-factor<1 is the scale lever."
metadata: 
  node_type: memory
  type: project
  originSessionId: 9d34ac8f-8455-4a08-8ccd-166c8171e257
---

Alpha storage agents (the `elohim-node` container = `elohim-storage` Rust process + an embedded Holochain 0.6/kitsune2 conductor child sharing **one cgroup**) show load-correlated memory growth: loaded nodes (matthew=doorway read/proxy target, jessica, james=largest corpus 3654 docs) climb monotonically ~2→4GB+ with retained ~0.5–1.7GB step-jumps every ~3–5 min; the quiet bootstrap node (adam) plateaus ~2.2GB. james OOM-crashlooped every ~9 min at a 3Gi limit; bumping it to 8Gi only moved the ceiling (it climbed back to 3.3GB and kept going).

**Root cause (2026-06-13 leak-hunt, 4 parallel hunters): the conductor's DHT+gossip working set at `network.target_arc_factor=1` (full authority arc), which is set NOWHERE across deployed configs so it defaults to 1.** At full arc every node holds a DHT region/op-hash/fetch-pool working set **proportional to the whole corpus** → per-node RAM ∝ total corpus. This is the "every node mirrors everything" model — the thing that does NOT scale, and the antithesis of the tiered-quilt / RS-sharding design. "8GB to stabilize james" = symptom of carrying a full arc. The scale lever (and what makes "laptop = full participant" true) is **arc-factor < 1 → node holds a bounded shard**. Flipping it is a resilience↔memory trade (operator's architectural call, NOT a transparent fix); `disable_gossip`/`disable_publish` are test-utils-only, absent from the prod config schema.

**Our Rust code is NOT the GB driver.** Three *real* but minor unbounded structures (hygiene, won't fix the OOM): `services/provide_reconcile.rs:261` latch (insert-only HashMap, ~150B/entry), `p2p/mod.rs:4513` `pending_epr_resolves` (only `pending_*` map missing its OutboundFailure cleanup), `reconcile/controller.rs:101` `observed_kinds` (appends per signal). Latent/dead (fix before wiring, can't leak today): `observation/log.rs:24` ObservationLog, `signals.rs:69` received_chunks, conductor_agent_info_gossip last_seen (gated off).

**Unconfirmed: leak vs bounded-large-arc.** Container `working_set_bytes` can't attribute RSS between the conductor child and storage parent, and "dies before it would plateau" looks identical to "climbs forever." No Pyroscope datasource exists. Cheap discriminators (all operator-side — dev env can't kubectl/exec): (1) per-proc RSS split via `ps -o rss,comm` in the container (conductor child vs storage parent); (2) raise one loaded node 3→6Gi and watch for plateau; (3) `target_arc_factor:0` on one loaded node — if the climb vanishes, conductor working set confirmed. See [[project_hub_optional_floor]], [[feedback_household_nodes_is_the_stable_floor]], [[project_alpha_substrate_probe_rails]].
