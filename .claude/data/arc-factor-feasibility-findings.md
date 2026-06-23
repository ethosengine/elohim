# Arc-factor feasibility findings (for Pillar 3 spec)

VERDICT: target_arc_factor = STATIC-at-conductor-start, re-derive-on-restart-only.
It is a u32 PARTICIPATION SWITCH ({0,1}), NOT a fractional size dial.
The arc ITSELF is runtime-dynamic (gossip grows cur→tgt), but the TARGET it
chases is always FULL — kitsune2 does NOT shrink target arc as N grows.
Peer-count-based auto-sharding is NOT implemented in the deployed line.

## Versions
- Deployed conductor: holochain 0.6.0 / holochain_p2p 0.6.0 / kitsune2 0.3.2
  (sweettest Cargo.lock 2120/2390/3431; edgenode Dockerfile base
  ghcr.io/holo-host/edgenode:v0.0.8-alpha31-hc0.6.0).
- Steward (tauri-plugin-holochain): holochain_conductor_api 0.7.0-dev.21 /
  kitsune2 0.4.1. VERSION SKEW worth flagging in spec.

## Config field (static)
- holochain_conductor_api .../config/conductor.rs:302-306 — pub target_arc_factor: u32,
  doc: "target arc factor to apply when receiving hints from kitsune2. In normal
  operation leave as default 1. For leacher nodes ... set to zero."
- default_target_arc_factor() = 1 (conductor.rs:362).
- Set NOWHERE in repo except steward mobile (lib.rs:140 = 0). edgenode
  conductor-config.yaml network: block has NO target_arc_factor -> defaults to 1.

## No runtime resize path
- process_manager.rs spawns conductor as child: Command::new(binary)
  .arg("--config-path").arg(config_path) (lines 64-66). Field read from YAML at start.
- AdminWebsocket (holochain_client 0.9.0-dev.22 admin_websocket.rs:193-557) has
  NO arc/network-config-update/set_target_arc method. update_coordinators(506)
  exists but does NOT touch arc. No runtime resize RPC.
- elohim-storage Cargo.lock has ONLY kitsune2_api/bootstrap_client/core — no
  holochain meta, no holochain_p2p, no gossip/dht. Storage does NOT embed a
  conductor; the real sharding code lives in the external conductor binary.

## The arc mechanism (two levels)
- kitsune2_api LocalAgent trait (agent.rs:55-76): get/set_cur_storage_arc +
  get_tgt/set_tgt_storage_arc_hint. cur "initially zero on join, gossip updates
  as data collected"; tgt set by sharding module, "may update to FULL or true target".
- core_space.rs:451 set_cur_storage_arc(DhtArc::Empty) on join.
- apply_arc_factor (holochain_p2p local_agent.rs): len = (arc.arc_span()+1)*factor;
  len==0 -> DhtArc::Empty; factor>1 -> ERROR LOG + forced to 1
  ("multi-factor sharding isn't yet implemented"). So effective domain = {0,1}.
- gossip update_storage_arcs only grows cur toward tgt on NoDiff/in-sync; NO
  peer-count target computation; agents join with DhtArc::FULL. (kitsune2 gossip src
  + holochain issue #220 / dev docs concepts/4_dht.)

## Implication for the design
- The spec's "target_arc_factor < 1 -> bounded shard" lever DOES NOT EXIST at
  this field (u32, clamped {0,1}). f=1 already = FULL.
- The real continuous size lever is the LocalAgent tgt-arc HINT (a DhtArc range),
  fed by a sharding policy. Reaching it requires either (a) a kitsune2 `advanced`
  config block (NetworkConfig.advanced: Option<serde_json::Value>, conductor.rs:312-317)
  or (b) upstream sharding-policy work / a custom kitsune2 module. Either way:
  set at conductor construction -> still re-derive-on-RESTART, not live-resizable.
- RESIDUAL RISK: I could not read kitsune2 0.3.2 dht/gossip source locally (not
  vendored; 0.4.1 read instead). Trait + factor semantics are stable across the
  versions, but the exact 0.3.2 sharding target value is inferred from upstream
  main + docs, not byte-read from the pinned crate.
