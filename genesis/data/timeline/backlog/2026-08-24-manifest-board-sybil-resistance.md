---
title: "transport-manifest bootstrap board needs Sybil resistance before it goes live on a public doorway"
date: 2026-08-24
status: needs-brainstorm
class: security-design
habit: dataplane-convergence
requires_env: none
---

`POST /p2p/manifests` (doorway, aef9dc203) admits any self-signed TransportManifestAnnouncement:
the signature verifies against the announcement's OWN iroh_node_id, so there is no identity
COST. An attacker mints 64+ fresh ed25519 keypairs (free), signs manifests with attacker-
controlled addrs + announced_at_ms=now, POSTs faster than the 30 s re-announce cadence, and
evicts every real peer → GET returns 100% attacker addresses → pure-iroh bootstrappers dial only
the attacker. (Adversarial review 2026-08-24, CRITICAL.)

SHIPPED MITIGATION (this batch): the endpoint is gated behind DOORWAY_MANIFEST_BOARD_ENABLED
(default FALSE). Live in localdev (mesh sets it on; T0' proven there — pure-iroh formed a mesh in
52 s via the board), DORMANT on the fleet (which runs dual, not pure-iroh, so nothing is lost).
Route 404s when off; storage client then gets nothing and stays inert.

DESIGN QUESTION (brainstorm before flipping the flag on any public doorway): what is the right
Sybil resistance for a bootstrap directory? Candidates — (a) accept only manifests whose agent_cid
already resolves via the bootstrap/kitsune2 layer (ties iroh identity to an existing DHT-known
agent); (b) first-seen grace period before an entry becomes eviction-eligible; (c) per-source-IP
eviction rate bound; (d) PoW/stake. (a) is the most protocol-native (reuses the existing agent
identity plane as the cost) — lead candidate. Route to /brainstorm with the p2p-design-gate.

Related MEDIUMs from the same review (mesh-member-reachable only while the board is gated off, so
not newly-public — fix alongside the flag flip):
- iroh AnnounceChange frame allocates to the 16 MiB codec cap before the 64 KiB semantic check
  (sync_backend.rs:115 vs codec.rs DEFAULT_MAX_FRAME_SIZE) — lower the cap for AnnounceChange or
  correct the doc comment.
- spawn_announce_pull (sync_backend.rs:195) has no per-(peer,doc) debounce/dedup/concurrency cap —
  a flood of bogus-hash announces spawns unbounded dial-back tasks. Coalesce per (peer,doc).

## 2026-08-30 — gated board reads as "empty" to a joiner (C4 honest-absence)

A workspace storage joiner (iroh, relay.alpha attached) bootstrapping against doorway-alpha with the
board OFF counted `elohim_iroh_doorway_bootstrap_reads_total{phase="boot",result="empty"}` ×5 and
`{phase="watch",result="empty"}` — the doorway's 404 (`DOORWAY_MANIFEST_BOARD_ENABLED` unset) is
indistinguishable from a board with no peers. A joiner that will never seed should say so: add a
`result="gated"` (HTTP 404/403) outcome so the boot sentinel does not print a green "empty board".
Measured while staging the alpha enablers (storage-side `ELOHIM_IROH_RELAY_URL` + board ON for
doorway-alpha); see stewarded-device-sync.feature station 3.
