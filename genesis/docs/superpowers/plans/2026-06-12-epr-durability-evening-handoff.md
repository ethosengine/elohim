# EPR Durability Arc — evening 2026-06-12 handoff (signal-leg trace)

Continuation of `2026-06-12-epr-durability-overnight-handoff.md`. The custody-convergence
chain advanced four links tonight; ONE link remains dark. Frozen measure unchanged:
`bash .claude/shifts/2026-06-11T12-15-epr-durability-cluster-validation.measure.sh` (0 = green).

## DO NOT RE-DERIVE (verified live tonight, with evidence)

- **Conductor-plane root cause FOUND and half-cured: the doorway was bootstrap-blind.**
  Doorway spoke only kitsune1 (`POST /bootstrap` + X-Op); HC 0.6 conductors speak kitsune2
  (`PUT /bootstrap/{spaceB64url}/{agentB64url}`, `GET /bootstrap/{space}`) — every put 404'd
  ("No registry match" paired with each PUT in doorway logs), so every conductor was its own
  DHT island the entire arc. Fixed in `doorway/doorway-service/src/bootstrap/k2.rs`
  (83fbc920c) + a live-contact follow-up: conductors re-put agent infos with `created_at`
  3-30 min old, so the reference server's 3-minute freshness floor rejected EVERY real put —
  floor dropped (3d04d24f9; `expires_at > now` + 30-min span cap already bound staleness).
- **Bootstrap leg now VERIFIED working:** store holds ~102 agent infos for the lamad space
  (`GET https://elohim.host/bootstrap/K8-okzuNgktao5W1TEP5kNJ_6vvLIvK96DzATk2RDsA`),
  rejects zero, "Bootstrap overloaded" spam zero (was ~1500/hr).
- **peer_statuses LIT in production** — the msgpack decode fix (d33b0e1f5: holo_hash fields
  arrive as BYTE ARRAYS; the old rmp→serde_json::Value pre-pass + String mirror dropped every
  signal at debug level). Real row with dhtAnchorHash on matthew. Class warning: the
  REA/mishpat/content subscribers still use the broken pre-pass (memory:
  conductor-signal-msgpack-decode-class).
- **Demo honesty LIVE:** `/api/v1/resilience/elohim-host-landing/household` returns
  `distributionState:"unmeasured"` + `onlinePeers:{live,known}` (11f401452); card renders
  "not yet distributed" themed + contrast-passing (05827e5ed).
- **First cross-leg heal observed then explained:** jessica `healed:1 local_total:1` is her
  OWN authored commitment (no network needed). Everything authored by others still
  `conductor_missing:33`.
- Genesis #1135 ABORTED, #1136 propagation artifact missing — measure 3/1 readings tonight
  are the lossy-measure trap, not regressions.

## FIRST ACTION — the signal/dial leg (the ONE dark link)

State: conductors HAVE each other's agent infos advertising
`wss://signal.elohim.host:443/<peer-token>` — yet jessica logs
`kitsune2_core core_space "Broadcast new agent info to 0 peers"` (no connections), zero
tx5/sbd errors at INFO, and the doorway logs NO `WS /signal/{pubkey}` connection arrivals.
Two candidate causes, in trace order:

1. **Ingress/path mismatch (same class as bootstrap):** sbd clients dial
   `wss://signal.elohim.host/<pubkey>` (ROOT path); the doorway's signal route matches
   `p.starts_with("/signal/")` (`server/http.rs` ~2445). Unless the signal.elohim.host
   ingress rewrites `/` → `/signal/`, the upgrade falls through the registry. Read the
   orchestrator-rendered ingress for host `signal.elohim.host` (Jenkins MCP / rendered
   manifests — NOT kubectl). A raw upgrade probe to `https://signal.elohim.host/test123`
   returns 400 (something websocket-aware answers — identify WHAT).
2. **tx5 never dials:** if ingress is fine, bump one conductor to
   `RUST_LOG=tx5=debug,kitsune2_gossip=debug` via the edge env (elohim/holochain Jenkinsfile
   env placeholder) and watch the dial attempts.

Also verify the doorway SBD implementation against the CURRENT sbd-server protocol once
connections arrive (`doorway/doorway-service/src/signal/` implements lbrt/lidl/areq/srdy —
looks current-shaped, but live contact bit us once already today: the bootstrap freshness
floor was reference-faithful and still wrong for live conductors).

## Then

- Heal watch: jessica/adam `projection-reconcile: sweep complete` flips
  `conductor_missing 33→0`; then genesis run → measure 0 → 3 consecutive greens.
- Init-authoring design (Layer 2) is DECIDED + spec'd:
  `2026-06-12-init-authoring-native-seeding-design.md` (da7605e19 + aa8c16b12 + 18c694a1c)
  — adam stewards genesis per-corpus routing; §b.1 story-derived collective graph; stage A
  is local-stack and does NOT wait on convergence. Provenance-manifest spec is SUPERSEDED.

## Rails (unchanged + tonight's additions)

- Single-dispatcher before every push; wait for the orchestrator run to EXIT before pushing
  again (abort-previous); ABORTED genesis runs read as full-failure measures — judge only
  complete runs.
- Hook bypass rule (operator, tonight): the pipeline-owning agent doing integration shakeout
  may push --no-verify ONLY with gates already green and real hook findings fixed first
  (memory: hook-bypass-integration-shakeout).
- Loki labels: pods `elohim-<name>-alpha-0`, container `elohim-node`; doorway pods
  `elohim-doorway-alpha*`. The a2o ESLint is environmentally broken (ts-api-utils missing).
