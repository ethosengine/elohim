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

### 2026-08-07 ~04:15Z hoot-owl addendum — instrument live, mechanism narrowed past H2

The kitsune2 debug instrument deployed (edge #1314) and REFUTES simple-H2: with
`blocking_put_auth` logging "Putting agent info to bootstrap server" unconditionally at
ENTRY (before any network I/O), zero such lines exist — so the PUT is never attempted at
the client layer, not attempted-and-swallowed. GET polls are chatty per space; per-space
`do_insert_relay` fires each boot. Every logged branch of the join-callback chain is
silent: no "Not updating agent info" (no-URL branch), no "failed to sign agent info", no
peer-store insert failure, no "Bootstrap overloaded". Code audit of the full chain is
clean (fork BootWrap::put forwards; register_cb/invoke_cb correct; core_space callback
logs both branches). Conclusion: the spawned join-callback task stalls silently before
its first branch. Candidate mechanisms (log-indistinguishable):
(a) CoreSpace.inner RwLock read parked forever (write-holder wedged — new_url ×
    iroh-transport synchronous new_listening_address interplay);
(b) MetaLairClient::sign() hang without timeout (sign-warn fires only on Err, never on
    hang; serving traffic does NOT falsify this — zome reads skip conductor-lair and
    doorway zome calls sign storage-side).
Discriminators (ceiling): entry-log in the fork's join callback (push to
ethosengine/holochain elohim-0.6.3, operator-gated) or stack-dump/tokio-console on a
live pod (operator kubectl).

### 2026-08-07 ~05:50Z — UNIFIED: defect 1 cascades into a fleet-wide hidden /db outage

The relay_url skew is not just an admin-seam annoyance — it wedges storage BOOT:
`main.rs` retries `loop { conductor start → wait_for_ready → ensure_happ_installed }`
(main.rs:849) forever on the list_apps failure (all 7 pods, ~12 retries/hour, uniform —
Loki), and `HttpServer::new` sits AFTER the loop (main.rs:3012). Since the flip restart,
**no alpha storage pod has bound HTTP :8090**: the doorway route registries (populated
by fetching `{storage_url}/manifest` from the primary peer at boot) are empty of storage
routes, so the ENTIRE `/db/*` surface 404s on both doorways (live-verified; POST
/admin/steward-peers/refresh fails with connection errors to matthew/adam :8090).
Conductors serve regardless — launched as children before the wedge, cells auto-start
from persisted enable-state, and the doorway ZomeCaller dials conductor websockets
directly — which is why the federation facade stayed green while the data surface was
dark. "No visible outage window" was a facade-route measurement.

Fix-forward consequence: the client-pin fix (717bbad23, in dev-latest) unblocks the boot
loop → storage HTTP binds → /db heals → happ manager completes its enable/sync pass.
Whether bootstrap publish then lights (defects collapse to one root) or stays dark
(defect 2 genuinely independent — silent join-callback stall, discriminators above) is
adjudicated by the bootstrap-coherence agents measure on the post-fix deploy.

Evidence-hygiene correction: the overnight "peer-store FAIL total=0 addressed=0" reads
are UNTRUSTWORTHY — /db/p2p/conductor-diagnostics 404s through the doorway (doorway's
own not_found_response despite is_diagnostic_probe listing the path), and the seam-smoke
parser renders that 404 as "0 0". The reliable partition signals remain: doorway
access-log PUT absence, bootstrap-coherence agents=0, zero P2P connections.

Delivery path (confirmed against the build machinery): the iroh anchor is a binary-swap
wrapper (elohim-storage:dev-latest + fork conductor bin; che-devworkspaces
Jenkinsfile-elohim-edgenode). Sequence: (1) client fix → dev `[build:edge]` (refreshes
dev-latest; interim deploy re-enters the same broken state — accepted); (2) operator
fires elohim-edgenode job iroh lane (HC_FEATURES=…,transport-iroh +
BUILD_STORAGE_CANARY=true) to re-wrap; (3) fresh-sha `[build:edge]` push moves
DEPLOY_VERSION so ALL seven STSs (incl. genesis pair) restart onto the moved anchor;
(4) convergence probe + read the new debug lines for defect 2.
