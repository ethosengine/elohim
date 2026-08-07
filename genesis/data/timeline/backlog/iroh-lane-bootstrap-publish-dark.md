---
id: "backlog-iroh-lane-bootstrap-publish-dark"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Post-flip: iroh conductors do not republish agent-infos to bootstrap — possible 7-island partition behind a serving facade"
slug: "iroh-lane-bootstrap-publish-dark"
written: "2026-08-07"
author: "agentic-developer"
status: "open"
priority: "critical"
area: "dataplane"
domain: "code"
tags: [iroh, wave2, bootstrap, kitsune2, partition, soak, code-domain]
---

# iroh-lane bootstrap publish is dark

Evidence (2026-08-07 00:15-01:20Z, seam-smoke direct runs post-flip):
- Deploy-time (#1313, mid-restart): bootstrap-sharing OK "5 16 spaces/agents" — the OLD tx5 entries.
- +30m and +80m post-flip: bootstrap-sharing FAIL "A=0 0 B=0 0"; peer-store FAIL total=0 addressed=0
  on BOTH doorways; n0/tx5 seams cannot run ("no peer URLs to inspect").
- Meanwhile: conductors ARE on iroh and registered with the sovereign relays (do_insert_relay at
  23:51Z, all peers), and the federation route serves 200 on both doorways.

Reading: the old bootstrap entries expired (~20min) and the iroh conductors never republished.
On a full-arc fleet every zome read is local-only, so serving routes prove LOCAL data, not mesh —
**the fleet may be 7 DHT islands behind a working facade** (recurrence of the kitsune2
bootstrap-gap class from the doorway-ops incidents, now on the transport-iroh lane).

Decisive next probe (cheap, run first): cross-peer convergence — author a new action on peer A,
watch it arrive on peer B (`✓ canonical head propagated` on the next deploy, the dht-fetch seam
A=/B= heads, or an a2o @concern:content-sync run). If it converges, discovery is degraded but the
mesh holds (relay-held connections / peer stores populated somewhere the doorway view doesn't
read). If it does NOT converge, the flip broke peer discovery and the options are: (a) fix the
bootstrap publish leg on the iroh lane (does kitsune2 transport-iroh wire coreBootstrap the same
way? config key drift? the bootstrap_url flow was tx5-era-verified only), or (b) D9 rollback
(one-line STS repoint to hc-elohim-0.6.3) while (a) is fixed off-fleet.

Suspect surfaces: conductor NetworkConfig bootstrap wiring under transport-iroh (the dep-verification
pack history doc covered bootstrap wire compat — re-read it); doorway /admin/bootstrap-coherence +
conductor-diagnostics read legs (could be reading a store the iroh lane writes elsewhere — verify
the probe against a conductor's own admin API before trusting the zero).

Divergence risk while open: writes on different islands diverge until heal; heal is fills-never-moves
so nothing is lost, but the reconcile grows with time. Alpha-only (staging/prod are tx5).

## 01:15Z evidence — root-cause candidate: admin-seam version skew, fix-forward shaped

Conductor logs (james, recurring): `Error: list_apps failed: Websocket error: Received a message
that did not deserialize: Deserialize("unknown field relay_url, expected one of name, description,
roles, allow_deferred_memproofs, bootstrap_url, signal_url")` — the fork's iroh conductor now
emits `relay_url` in the app-manifest wire shape; the monorepo's pinned holochain_client
(elohim-storage's embedded manager) has deny_unknown_fields and rejects the response. The
storage-side happ_manager cannot list_apps against its own conductor.

Also confirmed in the same window: kitsune2 coreBootstrap.serverUrl IS configured
(https://doorway-alpha.elohim.host/bootstrap; the "may be unused" WARN is the known-benign
Stage-0 census class). Zero P2P `Connection established` lines fleet-wide in 60m.

Morning fix path (ordered):
1. Bump/patch the monorepo's holochain_client (or the manifest struct) to tolerate `relay_url`
   — one-field skew; check the fork's holochain_client crate for the matching version and pin it.
   Then REBUILD the iroh anchor (edgenode job iroh lane) + repoint/redeploy.
2. Separately verify the bootstrap publish leg: grep kitsune2 core_bootstrap / doorway /bootstrap
   access logs for PUT traffic — if absent even before the list_apps failure, the publish break is
   independent (two defects, not one).
3. Decisive convergence probe still stands (cross-peer head propagation) after the fix lands.
D9 rollback remains one line if the operator prefers stability over fix-forward at morning triage.

## 2026-08-07 morning triage — TWO independent defects confirmed

**Defect 2 (bootstrap publish dark) verified independent of the list_apps skew.**

Loki evidence (doorway access logs, elohim-alpha): PUT `/bootstrap/*` traffic died at
23:39:56Z — the same minute conductors rebooted onto iroh (Launching HolochainP2p with
irohTransport at 23:40:39Z on james/jessica) — while GET `/bootstrap/*` polls continue
normally and the store reads `0 agents in 0 spaces` continuously. Zero PUTs in 80k+
scanned doorway lines post-flip.

Source review (kitsune2 0.4.1 at the Cargo.lock checksum + fork holochain_p2p):
- `CoreBootstrapFactory` is registered UNCONDITIONALLY in `default_builder()` — no
  transport gating; the fork wraps (not replaces) it via BootWrapFact (actor.rs:554-558).
  Bootstrap publish IS wired under transport-iroh.
- The `config.rs:165` "may be unused" WARN is a benign assembly-time lint whose
  `debug_path` dumps the ENTIRE incoming config blob per call — "all four subtrees"
  is a misreading; only tx5Transport is legitimately unclaimed on the iroh build.
- `list_apps` failure cannot suppress publish: `local_agent_join → bootstrap.put()` runs
  inside the conductor's own kitsune2, not the storage admin loop. happ_manager's early
  return (happ_manager.rs:53-56) only blocks storage-side re-verification.

Mechanism narrowed to H2 — publish attempted, failing below the log floor:
- H1 (no current_url) ELIMINATED: `"Not updating agent info"` (core_space.rs:523,
  info-level) — zero hits across 186k conductor lines post-flip.
- `"Bootstrap PUT returned HTTP error"` (warn-level) — zero hits: not an HTTP-status
  failure.
- kitsune2_bootstrap_client swallows transport-level PUT failures (conn/TLS/DNS) into
  a `debug!`-only path (core_bootstrap.rs push_task); GET has the identical asymmetry,
  so live GETs don't clear PUT.
- Instrument shipped with the fix batch: alpha conductor RUST_LOG gains
  `kitsune2_core=debug,kitsune2_bootstrap_client=debug` (template + adam yaml, marked
  temporary) — next deploy names the PUT failure or proves no attempt fires.

Delivery path (confirmed against the build machinery): the iroh anchor is a binary-swap
wrapper (elohim-storage:dev-latest + fork conductor bin; che-devworkspaces
Jenkinsfile-elohim-edgenode). Sequence: (1) client fix → dev `[build:edge]` (refreshes
dev-latest; interim deploy re-enters the same broken state — accepted); (2) operator
fires elohim-edgenode job iroh lane (HC_FEATURES=…,transport-iroh +
BUILD_STORAGE_CANARY=true) to re-wrap; (3) fresh-sha `[build:edge]` push moves
DEPLOY_VERSION so ALL seven STSs (incl. genesis pair) restart onto the moved anchor;
(4) convergence probe + read the new debug lines for defect 2.
