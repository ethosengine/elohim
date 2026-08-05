---
title: "Wave 2 relay custody — self-hosted iroh-relay, the transport flip, and the tx5 retirement"
id: wave2-relay-sovereignty-design
date: 2026-08-05
status: Draft
class: substrate
domain: substrate (conductor transport — kitsune2 relay plane; distinct from the elohim-storage iroh dataplane)
author: rust-architect (Opus), Wave 2 Task E1 of the holochain-iroh convergence campaign
cites:
  - holochain-iroh-convergence-upgrade-campaign | the governing campaign whose Wave 2 Task E1 this design closes — and whose two stated assumptions it corrects (the flip is an image-feature change, not a config change; the household-first staging is not executable on the live DHT) | sha256:ddab91172812e446 | path: genesis/docs/superpowers/plans/2026-08-04-holochain-iroh-convergence-upgrade-campaign.md
  - holochain-iroh-dep-verification-pack | the Wave 1 Lane D config-mapping ground truth this design builds on — relay_url to irohTransport.relayUrl, signal_url/webrtc_config as tx5-only leftovers, and the unauthenticated-relay boundary imposed by doorway lacking /authenticate and /relay/register | sha256:36835df9cebacd31 | path: genesis/docs/content/elohim-protocol/history/2026-08-04-holochain-iroh-dep-verification-pack.md
  - substrate-trust-contract-runbook | the invariant/probe contract the transport flip must preserve — its seam-smoke, conductor-diagnostics and canonical-head probes carry over unchanged, seam-smoke[signal-bus] is retired by the flip and replaced by this design relay-reachability and n0-contamination probes | sha256:cb76e9f0ae6bacfc | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
  - genesis/orchestrator/manifests/infra/alpha-iroh-relay.yaml
  - genesis/orchestrator/manifests/infra/alpha-coturn-operations.yaml
  - genesis/orchestrator/manifests/infra/alpha-coturn-shem.yaml
  - genesis/data/timeline/backlog/sovereign-turn-relay-transport-commons.md
memory_anchors:
  - project_two_premises_dns_beacon_owned
  - project_alpha_topology_bootstrap_pair
  - feedback_household_nodes_is_the_stable_floor
  - feedback_k8s_is_not_the_architecture
  - project_iroh_dataplane_actual_state
---

# Wave 2 relay custody — self-hosted iroh-relay

**Word note.** Throughout this document *sovereignty* carries the campaign's
existing sense: **custody of infrastructure** — who owns the box that our bytes
transit. It is never an identity-tier claim. The canon's stewardship ontology is
untouched here; a relay is a piece of plumbing a community stewards, and no
claim about persons or identity tiers is made or implied by hosting one.

**What this decides.** Where the conductor fleet's iroh relay lives, which
version of it, how it is exposed, how the transport flip is actually performed
(it is *not* the config-only flip the campaign plan assumed), what proves it
worked, and in what order the tx5-era infrastructure retires.

**What this does not do.** No template edits, no cluster actions, no commits of
conductor config. The manifest deliverable is inert until the operator applies
it — the repo is the reconcile surface; the live cluster is the operator's.

---

## 0. The four findings that reshaped the task

Before the decisions, the four things the evidence changed:

1. **The flip is an image-feature change, not a config change.** kitsune2 0.4.1
   selects tx5 whenever the tx5 feature is compiled; `transport-iroh` is only
   reachable when tx5 is *absent* (§4.1). "tx5 stays compiled for one wave"
   cannot mean one dual-feature image — that image is tx5-only and iroh-dead.
2. **A partial flip partitions the namespace.** tx5 and iroh peer URLs are
   different schemes over different transports; a tx5 conductor cannot dial an
   iroh peer URL and vice versa (§5.2). The campaign's "M/J/J household proves
   the mechanism first" cannot be done *on the live alpha DHT* — the trio would
   prove the mechanism by severing itself from adam.
3. **Coturn retirement is not clean.** The `relay-addr-beacon` sidecar living
   *inside* both coturn pods owns the `elohim.host` apex and `alpha.elohim.host`
   A records — the dynamic-WAN anchors that every other name in the zone CNAMEs
   to, including both doorway ingresses and the App pipeline's E2E target
   (§6.2). Retiring coturn without re-homing the beacon staleness-breaks the
   zone.
4. **There are two n0 leaks, not one.** The conductor's unset `relay_url` is the
   known hazard. The storage dataplane has its own: `IrohConfig.use_n0_relays`
   defaults to `true` → `RelayMode::Default` → n0's public relays
   (`elohim/elohim-storage/src/p2p_iroh/config.rs:59-61,91`,
   `endpoint.rs:40-48`). Wave 2's Task E3 lights that path. The layer guard says
   these are different config surfaces — it does not say only one of them phones
   home.

---

## 1. Decision record — reject the n0 public relay default

### D1. Explicit self-hosted `relay_url` everywhere a transport-iroh conductor can boot

**Decision.** Every conductor-config surface that a transport-iroh build could
ever read carries an explicit `network.relay_url` pointing at our relay. No
surface is left to the default.

**Why the default is unacceptable.** At Holochain 0.6.3, `network.relay_url`
maps to `irohTransport.relayUrl`, and when that is `None`
`kitsune2_transport_iroh` builds the endpoint with `RelayMode::Default` —
n0's public relay fleet (`kitsune2_transport_iroh-0.4.1/src/lib.rs:248-253,
463-465`). The failure mode is the worst kind: **silent and green**. Peers
connect, gossip flows, nothing logs an error, and every NAT-traversed
conductor-to-conductor byte in the fleet transits a third party's box. There is
no "we forgot" state that announces itself; the only way to see it is to read
the relay hostname out of the agent-info URLs (§5.3).

This is the same posture we already hold for bootstrap and signal — both are
already self-hosted. The relay is the last outsourced transport leg, exactly as
the coturn manifests' own header argues for TURN. A relay sees only encrypted
QUIC bytes; it is not a confidentiality dependency. It *is* an availability and
traffic-metadata dependency, and that is precisely why the community holds it.

**Scope of D1 — both layers.** The decision binds two independent config
surfaces:

| Layer | Surface | Current default | Wave 2 requirement |
|---|---|---|---|
| Conductor transport (kitsune2) | `network.relay_url` in conductor-config | placeholder already set to `https://relay.elohim.host`, inert under tx5 | live and reachable before any transport-iroh image ships |
| Storage dataplane (p2p_iroh) | `IrohConfig.use_n0_relays: bool` | `true` → `RelayMode::Default` → n0 | must not light dual mode on the n0 default; needs a `relay_url` field (§4.4) |

The layer guard holds: operational relay-hosting experience transfers, code does
not, and the two configs must never be conflated. But the *stance* is one
stance, and it applies to both.

### D2. One relay name — `relay.elohim.host` — not per-env

**Decision.** Keep the already-placeholdered single name. Do not per-env it the
way `signal_url` is per-env.

**Why this is not the same shape as `signal_url`.** SBD signal is per-doorway
because a signal session is *doorway-scoped state* — `signal.alpha.elohim.host`
and `signal.elohim.host` exist because matthew's doorway and adam's doorway hold
different session tables. A relay holds no such state: it is a stateless byte
forwarder keyed by endpoint id. Two envs sharing a relay cannot see each other's
traffic and cannot join each other's DHT (different DNA hashes, different
spaces). There is nothing to isolate.

Three further reasons:

- **kitsune2 0.4.1 accepts exactly one relay URL.** `IrohTransportConfig.relay_url`
  is `Option<String>`, singular (`kitsune2_transport_iroh-0.4.1/src/lib.rs:253`).
  Per-env names buy no redundancy because a conductor can hold only one home
  relay regardless.
- **The placeholder is already in five config surfaces** (`elohim/holochain/edgenode/conductor-config.yaml:34`,
  `che-devworkspaces/containers/elohim-edgenode/conductor-config.yaml`, and the
  three edgenode/human manifests). Renaming costs five sed-templated edits for
  zero functional gain.
- **Only alpha flips this wave.** Prod and staging conductors stay on tx5 images;
  their `relay_url` value remains inert. When they flip, they can point at the
  same relay or a new one — the decision stays open at no cost.

**Reserved for later, with the trigger named.** `relay-alpha.elohim.host` /
`relay-prod.elohim.host` are reserved. The one condition that forces the split
is a *capacity or blast-radius* argument, not an isolation one: when prod's
relayed traffic would degrade alpha's, split the name. Not before.

### D3. Single relay is a known resilience regression — recorded, not hidden

The coturn design deliberately runs a **dual-WAN pair** (operations + shem)
because a single relay is a single point of failure for the genesis pair. iroh's
`RelayMap` supports multiple relays, but **kitsune2 0.4.1 exposes only one**.
Wave 2 therefore ships strictly less relay redundancy than the tx5 stack it
replaces.

This is a real cost, accepted for one wave, with two mitigations:

- iroh holepunches; tx5+coturn relayed far more traffic. A relay outage degrades
  a fraction of paths rather than all of them — *if* holepunching works, which
  §3.4 says is reduced in Phase A. Measure it (§5.3), do not assume it.
- Named follow-up: **upstream `IrohTransportConfig.relay_url: Option<String>` →
  a list**, so the dual-WAN commons pattern the coturn manifests already model
  is expressible on iroh. This is the successor to
  `genesis/data/timeline/backlog/sovereign-turn-relay-transport-commons.md`, not
  a new idea — that backlog item's "N-node federated relay commons" is exactly
  what the singular field blocks.

### D4. Unauthenticated relay this wave

Per D1 verdict 3 of the verification pack: doorway's custom k2 bootstrap
(`doorway/doorway-service/src/bootstrap/k2.rs`) implements neither
`/authenticate` nor `/relay/register`. `base64_auth_material_bootstrap` and
`base64_auth_material_relay` stay **unset**, and the relay runs
`access = "everyone"`.

Two independent reasons this is the right call now and not a shortcut:

1. Setting `base64_auth_material_relay` against a base that does not serve both
   endpoints does not degrade — it makes the endpoint fail to come up
   (`kitsune2_transport_iroh-0.4.1/src/lib.rs:492-530` authenticates and
   registers *before* inserting the relay).
2. Upstream's own decode path is inconsistent with its own docs: the 0.6.3
   builder uses `BASE64_STANDARD` while the field comment says base64url-no-pad.
   Adopting auth material before that is reconciled buys a debugging session,
   not security.

**What unauthenticated actually exposes.** Anyone who learns the hostname can
use our relay to forward their own iroh traffic. They cannot read ours, cannot
join our DHT, and cannot enumerate our peers from the relay. The cost is
bandwidth and abuse surface. Two cheap bounds are in the manifest: per-client rx
rate limiting is available (left unset — see §3.5) and `access` can move to
`allowlist` of endpoint ids at any time without a client change. **Named
follow-up:** authenticated relay requires the upstream integrated relay surface
(or `/authenticate` + `/relay/register` implemented deliberately in doorway) —
it is a Wave-3-or-later item, and the `access = "http"` callback mode is a
lighter middle path worth evaluating first (it POSTs each connecting endpoint id
to a URL we control — doorway could answer it from the bootstrap store without
implementing the kitsune2 auth protocol at all).

---

## 2. Relay version and protocol compatibility

All evidence below is from crate sources on disk
(`/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`) and the
vendored conductor submodule (`elohim/holochain-conductor`, branch
`elohim-0.6.3`, HEAD `da823fc6a`). No web access was used.

### 2.1 The client we must serve

`holochain 0.6.3` → `kitsune2 0.4.1` → `kitsune2_transport_iroh 0.4.1` →
`iroh-holochain 0.95.1` → `iroh-relay-holochain 0.95.1` (relay client).
Verified from `elohim/holochain-conductor/Cargo.lock` dependency edges.

### 2.2 Verdict: `iroh-relay 0.95.1` (stock, crates.io) is the correct server — CONFIRMED

Four independent lines of evidence:

1. **The republish is byte-identical on the protocol surface.**
   `diff iroh-relay-0.95.1/src/http.rs iroh-relay-holochain-0.95.1/src/http.rs`
   → identical. Same `RELAY_PROTOCOL_VERSION = "iroh-relay-v1"`, same
   `RELAY_PATH = "/relay"`, same `CLIENT_AUTH_HEADER`.
2. **Upstream Holochain itself pairs them.** `holochain_p2p 0.6.3` and
   `holochain 0.6.3` both take a dev/optional dependency on **stock**
   `iroh-relay = { version = "0.95.1", features = ["server", "test-utils"] }`
   (`crates/holochain_p2p/Cargo.toml:67`, `crates/holochain/Cargo.toml:107`).
3. **And runs the exact pairing in its conductor test harness.**
   `crates/holochain/src/sweettest/sweet_conductor_config_rendezvous.rs:100-110`
   spawns `iroh_relay::server::Server` (stock 0.95.1) and points
   transport-iroh conductors at it. This *is* the compatibility test, written
   upstream, and our Lane-B gate ran it green (`cargo test -p holochain_p2p`,
   51/0 — which compiles the dev-dep, proving the server crate builds in our
   toolchain).
4. **It publishes a lockfile.** `iroh-relay-0.95.1/Cargo.lock` exists, and its
   crypto chain (`curve25519-dalek 5.0.0-pre.1`, `ed25519-dalek 3.0.0-pre.1`,
   `digest 0.11.0-rc.3`, `sha2 0.11.0-rc.2`) is **identical** to what the
   conductor lock resolves. So `cargo install iroh-relay --version 0.95.1
   --locked --features server` reproduces a resolution we have already compiled.

> The `--locked` flag is load-bearing, not hygiene. `iroh-base 0.95.1`
> exact-pins `curve25519-dalek =5.0.0-pre.1` — the pre-release whose published
> source carried the `digest::crypto_common` path bug that froze the storage
> dataplane on iroh 0.92. The bug is resolution-conditional (it bites against a
> `digest` that moved the item); the published lock pins the `digest 0.11.0-rc.3`
> that works. An unlocked build is a coin flip.

### 2.3 Verdict: the storage dataplane's iroh 0.92 clients CAN share the same relay — CONFIRMED at the wire

The relay protocol is **wire-identical between iroh-relay 0.92.0 and 0.95.1**.
The 0.92→0.95.1 diff is a rename (`Node*`→`Endpoint*`) plus an error-library
migration (`snafu` → `n0-error`). Specifically:

| Surface | 0.92.0 | 0.95.1 | Verdict |
|---|---|---|---|
| `RELAY_PROTOCOL_VERSION` | `iroh-relay-v1` | `iroh-relay-v1` | identical |
| `RELAY_PATH` / `RELAY_PROBE_PATH` | `/relay` / `/ping` | same | identical |
| `CLIENT_AUTH_HEADER` | `x-iroh-relay-client-auth-v1` | same | identical |
| `FrameType` discriminants 0-12 | 0-12 | 0-12 (only tag 8's *name* changed `NodeGone`→`EndpointGone`) | identical |
| Handshake frame structs (`ServerChallenge`, `ClientAuth`, `ServerConfirmsAuth`, `ServerDeniesAuth`) | field-for-field | field-for-field | identical |
| Handshake frame TAGs | `ServerChallenge`=0…`ServerDeniesAuth`=3 | same | identical |
| QAD ALPN | `/iroh-qad/0` | `/iroh-qad/0` | identical |
| Client dial | wss to `/relay`, `Sec-Websocket-Protocol: iroh-relay-v1` | same | identical |

`protos/disco.rs` and `protos/streams.rs` are byte-identical between the two
versions; `protos/common.rs`, `protos/handshake.rs`, `protos/relay.rs`, and
`client.rs` differ only in the two churn classes above.

**Layer guard, stated precisely.** Sharing the relay *instance* is permitted and
correct — one host, two client populations, keyed independently by endpoint id;
the relay has no notion of spaces or DNAs. Sharing the *config* is forbidden:
the conductor's relay is set via `network.relay_url` in conductor-config, the
dataplane's would be set via a new `IrohConfig` field (§4.4). Two keys, one
hostname. Never one key.

**Not yet wired.** Storage's `IrohConfig` has no relay-URL field today — only
the `use_n0_relays: bool`. §4.4 names the small change.

### 2.4 Wave 3 (iroh 1.0.3): forward-compatible, one caveat

`iroh-relay 1.0.3` replaces the `RELAY_PROTOCOL_VERSION` constant with a
negotiated `ProtocolVersion` enum: `V1` (`iroh-relay-v1`) and `V2`
(`iroh-relay-v2`, added in iroh 0.98.0 — removed the `Health` frame id 11, added
`Status` frame id 13). `ProtocolVersion::ALL = &[V2, V1]` — **a 1.0.3 server
still speaks V1**, and the `Status` frame carries the comment "may not be sent to
`iroh-relay-v1` clients."

So the Wave-3 sequencing is favorable in one direction and hostile in the other:

- **Upgrade the relay first, then the clients** — a 1.0.3 relay serves both the
  0.95.1 conductor clients and 1.0.3 clients during the transition. Safe.
- **Do not upgrade clients first.** A 1.0.3 client against a 0.95.1 relay
  negotiates down to V1 (the client sends its list; the 0.95.1 server matches on
  the bare `iroh-relay-v1` string). This *should* work but is the untested
  direction — see STILL-UNKNOWN U3.

### 2.5 STUN: not a thing on iroh — the question dissolves

The task asked whether relay-side STUN should be on now that coturn retires.
**iroh 0.9x has no STUN.** `net_report` probes are exactly three:
`Probe::Https`, `Probe::QadIpv4`, `Probe::QadIpv6`
(`iroh-holochain-0.95.1/src/net_report/options.rs:77-84,131`,
`report.rs:144-148`). The NAT-assist mechanism is **QUIC Address Discovery**
(QAD) — a QUIC connection to the relay's UDP port with ALPN `/iroh-qad/0`, from
which the endpoint learns its observed public address. There is no STUN server
to enable and no STUN URL to configure. The `iceServers`/`stun:` entries in the
conductor configs are tx5-only and become inert at the flip.

---

## 3. The relay deployment

Manifest: `genesis/orchestrator/manifests/infra/alpha-iroh-relay.yaml`.

### 3.1 D5. Image: build `iroh-relay 0.95.1` from crates.io into Harbor

**Decision.** `harbor.ethosengine.com/ethosengine/iroh-relay:0.95.1-dev-latest`,
built from crates.io with `--locked`, following the `relay-addr-beacon`
precedent exactly (self-contained build context, `rust:1-slim-bookworm`
builder, `debian:bookworm-slim` runtime, `RUSTFLAGS=""`, crates.io direct — not
through Nexus, per the 2026-07-30 decision recorded in
`relay-addr-beacon/Dockerfile`).

**Why not n0's published container.** Three reasons, in order of weight:

1. **The fleet pulls Harbor.** Every other image in these manifests does. A
   third-party registry in the pull path is a new availability dependency for
   the one component whose entire purpose is removing a third-party dependency.
2. **Whether a `0.95.1` container tag exists upstream is unverified** and
   unverifiable without web access (U1). 0.95.x is an unusual point on iroh's
   release line — the one Holochain republished from — and n0's container
   publishing cadence is not something to assume.
3. **A sovereign relay pulled from someone else's registry is a half-measure.**
   The point of the exercise.

The build is small: one crates.io crate, one `[[bin]]`, no path-deps, no CGo, no
Go toolchain. Materially cheaper than the edgenode conductor build.

**Operator mirror step** (also in §7): build + push once; the tag is pinned in
the manifest and only moves on a deliberate version bump. A build recipe is in
the manifest header comment so the operator does not have to reconstruct it.

### 3.2 D6. TLS at the ingress; the relay runs plain HTTP

**Decision.** ingress-nginx terminates TLS with a cert-manager
`letsencrypt-production` certificate for `relay.elohim.host`; the relay pod runs
with **no `[tls]` section**, serving everything on one plain-HTTP port (3340).

**Why this is not a compromise.** In no-TLS mode the relay registers *all*
services on the single `http_bind_addr` listener — `/relay` (the websocket
plane), `/ping` (the Https latency probe), `/generate_204` (the captive-portal
probe), `/`, `/robots.txt` (`iroh-relay-0.95.1/src/server.rs:380-383,441-447`).
One Service port, one Ingress rule, exactly the doorway pattern. If the relay
held TLS itself, `/generate_204` would move to a *separate* plain-HTTP listener
and we would need two ports and two ingress paths for no benefit.

**The one thing this costs, verified.** The relay client's fast-path
authentication uses TLS **exported keying material** (`KeyMaterialClientAuth`),
which binds to the TLS session the client established. Behind a terminating
proxy the relay has no TLS session and cannot verify it. This does **not**
break: the server-side handshake explicitly falls back to a challenge/response
round trip, with the comment *"Verification not succeeding is part of normal
operation: The TLS exporter isn't required to match"*
(`iroh-relay-0.95.1/src/protos/handshake.rs`, `serverside()`). Cost: one extra
RTT per relay connection establishment. Not per frame.

**Two ingress details that are load-bearing:**

- `nginx.ingress.kubernetes.io/ssl-redirect: "false"`. The captive-portal probe
  is issued as `http://{host}/generate_204`
  (`iroh-holochain-0.95.1/src/net_report/reportgen.rs:611`). A 301 to https is
  not a 204; forcing redirect makes every conductor believe it is behind a
  captive portal.
- The websocket annotation set from `doorway/prod.yaml` (proxy-http-version 1.1,
  read/send timeout 3600, `websocket-services`, the Upgrade/Connection
  configuration-snippet). The relay connection is long-lived by design; a short
  read timeout produces a reconnect storm that looks like relay instability.

### 3.3 D7. Placement: `node-type=operations`, `relay.elohim.host` CNAME → `alpha.elohim.host`

**Decision.** Hard-pin the relay to `node-type=operations` (the always-on
on-prem Intel NUC where mongodb, nats, and doorway-A are already hard-pinned),
and make `relay.elohim.host` a CNAME to `alpha.elohim.host`.

**Why operations and not shem.** shem is the capability the scope gate turns
off; `feedback_household_nodes_is_the_stable_floor` is the standing read of
which leg is dependable. Pinning the fleet's *single* relay (D3) to the leg that
gets suspended would make relay availability track the least reliable premise.
doorway-B is hard-pinned to `node-type=remote` with no fallback precisely
because it *is* adam's doorway; the relay is not premise-specific and should sit
where uptime is.

**Why CNAME rather than a new A record.** `alpha.elohim.host` is already the
beacon-maintained A record for the operations WAN, and that WAN IP is **dynamic**
(residential Google Fiber). A CNAME inherits the beacon's drift-healing for free
— no second record to keep synced, no second thing to go stale. This is the
existing machinery doing one more job, not new machinery.
(`project_two_premises_dns_beacon_owned`.)

**Consequence, named honestly.** Household conductors on the operations WAN will
resolve `relay.elohim.host` publicly and hairpin out through the router and back
in through the ingress to reach a pod on their own premise. It works; it is
inefficient. Not worth solving in Wave 2 — revisit only if the hairpin shows up
in the latency numbers.

### 3.4 D8. QAD off in Phase A; the UDP plane is Phase B

**Decision.** `enable_quic_addr_discovery = false`. Phase A serves the relay
plane only.

**Why it cannot be Phase A.** QAD is a raw QUIC/UDP listener on port 7842 with
its own TLS certificate (`enable_quic_addr_discovery` errors without a `[tls]`
section). ingress-nginx cannot proxy it, **and this cluster has no
LoadBalancer/MetalLB** — the reason the coturn manifests reach for
`hostNetwork: true` in the first place (`alpha-coturn-operations.yaml`, the
hostNetwork rationale comment). Serving QAD means the relay pod holds a real
certificate and binds the host's UDP 7842 with a matching router forward — a
different exposure pattern from the HTTPS ingress.

**Why QAD-off is a supported topology, not a broken one.** Upstream's own
conductor rendezvous harness runs the relay with `relay.tls = None` **and**
`relay_config.quic = None` — no QAD at all — and transport-iroh conductors work
against it (`sweet_conductor_config_rendezvous.rs:100-110`). Absent QAD is a
first-class configuration, not a degraded one.

**What it actually costs.** kitsune2 builds the relay map via
`RelayMap::from_iter([relay_url])`, and `From<RelayUrl> for RelayConfig` sets
`quic: Some(RelayQuicConfig { port: 7842 })` **by default**
(`iroh-relay-holochain-0.95.1/src/relay_map.rs:162-176,200-214`). So conductors
*will* probe UDP 7842 and those probes will fail. The probe failure is benign
(net_report simply records no QAD report); the cost is that endpoints do not
learn their observed public address from the relay, so hole-punching has fewer
candidates and more traffic stays relayed. **This is measurable, not
speculative** — see the `direct=` sentinel in §5.3. Phase B is justified by that
number, not by theory.

**Phase B shape, when the number justifies it.** cert-manager `Certificate` CRD
→ secret → mounted into the relay pod → `[tls] cert_mode = "reloading"` reading
the mounted PEM pair → `enable_quic_addr_discovery = true` → a raw UDP:7842 path
to the pod. This is the **one** place the coturn `hostNetwork: true` precedent is
the right model rather than the wrong one: coturn's hostNetwork pattern is wrong
for an HTTPS plane and right for a raw-UDP plane. Note that enabling relay-side
TLS also moves `/generate_204` onto a separate listener (§3.2) — Phase B changes
the ingress shape, so it is a genuine second design pass, not a config toggle.

### 3.5 Sizing, limits, and what is deliberately unset

- **Resources:** requests 64Mi/50m, limits 512Mi/500m — matched to the NATS
  neighbor. `key_cache_capacity = 8192` (upstream default is 1 Mi entries, sized
  for a million clients — ~56 MB for a fleet of seven; 8192 costs ~450 KB).
- **Rate limits: deliberately unset.** `[limits]` supports per-client rx
  byte/burst caps. A cap set below gossip's real burst profile does not error —
  it silently slows the DHT and reads as a transport fault. Set it after we have
  a bytes/sec baseline from the metrics surface, not before.
- **`access = "everyone"`** — stated explicitly rather than defaulted, so the
  posture is a recorded decision (D4) rather than an omission.
- **Metrics on** (`:9090`), with a PodMonitor in the manifest. It will read DOWN
  until the operator adds port 9090 to `allow-metrics-from-observability` in
  `genesis/orchestrator/manifests/infra/alpha-metrics-networkpolicy.yaml`
  (elohim-alpha runs `default-deny-cross-env`). That is on the operator
  checklist (§7) — the PodMonitor is shipped rather than omitted so the gap is
  visible and closeable rather than forgotten.
- **Replicas: 1.** A second replica does not add relay redundancy the way it
  would for a stateless HTTP service: relay clients hold a long-lived connection
  and two peers must be on the *same* relay instance for it to forward between
  them. Load-balancing two replicas round-robin would silently break relaying
  for the pairs that land on different pods. If replicas are ever wanted, they
  need session affinity on endpoint id — which the ingress cannot express.
  **One replica is a correctness choice, not a cost choice.**

---

## 4. Config wiring plan (E2 — design only)

### 4.1 The flip is a build-feature change

`kitsune2::default_builder()` at 0.4.1:

```rust
#[cfg(feature = "transport-tx5-backend-go-pion")]
transport: Tx5TransportFactory::create(),
#[cfg(all(not(feature = "transport-tx5-backend-go-pion"), feature = "transport-iroh"))]
transport: IrohTransportFactory::create(),
```
(`kitsune2-0.4.1/src/lib.rs:60-68`)

**tx5 wins whenever it is compiled.** There is no runtime selector, no config
key, no environment variable. The transport is chosen at compile time by feature
absence.

Therefore the flip is:

| | Current fleet image | Flip image |
|---|---|---|
| `HC_FEATURES` build arg | `sqlite-encrypted,wasmer_sys,transport-tx5-backend-go-pion,jemalloc` | `sqlite-encrypted,wasmer_sys,transport-iroh,jemalloc` |
| Build flags | `--no-default-features --features "${HC_FEATURES}"` | unchanged |
| Harbor tag | `hc-elohim-0.6.3` (live) | `hc-elohim-0.6.3-iroh` (new) |
| tx5 vendor patch (`7cc927e`, ethosengine/tx5 zombie-fix) | required | **unused** — the `[patch.crates-io]` block stays in the fork but compiles nothing |
| Go toolchain / CGo in the build | required (tx5-go-pion-sys) | **not required** — the iroh variant Dockerfile can drop the Go install |

Both Dockerfiles already parameterize this:
`che-devworkspaces/containers/elohim-edgenode/Dockerfile:39` and
`elohim/holochain/edgenode/Dockerfile.zombie-fix:39` take `HC_FEATURES` as an
`ARG`. **The iroh variant is a build-arg override, not a Dockerfile fork** —
build the same Dockerfile twice with two `--build-arg HC_FEATURES=...` values
and two tags. (Dropping the Go toolchain from the iroh variant is a later
optimization; do not fork the Dockerfile in the same change that flips the
transport.)

### 4.2 Which config keys become live, which become inert

Nothing is deleted at flip time. The keys change meaning:

| Key | Under tx5 today | Under transport-iroh |
|---|---|---|
| `network.bootstrap_url` | live | **live, unchanged** — transport-independent; stays on doorway's k2 bootstrap |
| `network.relay_url` | inert placeholder | **BECOMES LIVE** — `irohTransport.relayUrl`; top-level wins over `advanced.irohTransport.relayUrl` |
| `network.signal_url` | live (SBD) | **inert** — tx5-only |
| `network.webrtc_config` (STUN + TURN iceServers, credential `alpha-turn-commons-2026`) | live | **inert** — tx5-only |
| `network.request_timeout_s` | live (tx5 timeouts) | **inert for iroh** — it maps to `tx5Transport.timeoutS`; iroh's connect timeout is `advanced.irohTransport.connectTimeoutS` |
| `network.advanced.k2Gossip.*` (roundTimeoutMs 60000, maxConcurrentAcceptedRounds 4) | live | **live, unchanged** — gossip module, transport-independent. Keep the slow-WAN tuning. |
| `network.base64_auth_material_bootstrap` / `_relay` | absent | **stays absent** (D4) |

**One addition to consider at flip time, not required.**
`advanced.irohTransport.connectTimeoutS` defaults to 60s and `maxFrameBytes` to
100 MiB. The 60s default is the analogue of the slow-WAN tuning already applied
to k2Gossip; whether it needs the same treatment is an observation question, not
a pre-flip one. Do not tune blind.

**Config validation must move with the flip.**
`validate-conductor-config.sh` is a GATE on every human-manifest render and
today it validates that the ICE config parses into tx5's contract. Under iroh
that check validates a dead key. It must be extended (not replaced — prod and
staging stay on tx5) to assert, for an iroh-featured target: `relay_url` present,
scheme `https`, host **not** matching `*.iroh.network`. That last assertion is
the mechanical form of D1.

### 4.3 Deletion order (retirement time, not flip time)

Nothing in this list happens in the same change as the flip. Order and gates in
§6.

1. `network.webrtc_config` (STUN entries + TURN iceServers + the shared
   credential) — after coturn is gone.
2. `network.signal_url` — after the SBD module is gone.
3. `network.request_timeout_s` — with (2); it configures nothing else.
4. `signal.*.elohim.host` ingress hosts on doorway alpha/alpha-b/prod.
5. The doorway `signal/` module and its mounts.
6. The coturn manifests.

### 4.4 The dataplane's own relay wiring (E3's prerequisite)

Wave 2 Task E3 lights `ELOHIM_TRANSPORT_BACKEND: "dual"` on the storage
dataplane. Today that path builds its iroh endpoint with `RelayMode::Default`
because `IrohConfig.use_n0_relays` defaults to `true`
(`elohim/elohim-storage/src/p2p_iroh/config.rs:59-61,91`;
`endpoint.rs:40-48`).

**Required before E3, or E3 lights an n0 dependency:** extend `IrohConfig` with
`relay_url: Option<String>` and make `build_endpoint` select
`RelayMode::Custom(RelayMap::from_iter([url]))` when it is set, keeping
`use_n0_relays` as the (now clearly-named) fallback and `RelayMode::Disabled` as
the loopback/test path the parity harness already uses
(`parity_harness.rs:26`). Small, mechanical, additive, and it is the same
hostname from §3 — a different key on the same box.

If that change cannot land in Wave 2, the fallback posture is
`use_n0_relays: false` (`RelayMode::Disabled`) for the dual-mode enablement:
LAN/direct only, no relay assist, no n0. Degraded reach is acceptable; a silent
n0 dependency is not.

---

## 5. Rollback and sequencing

### 5.1 D9. Rollback is an image-tag repoint

Because the transport is compile-time (§4.1), "tx5 stays compiled for one full
wave" resolves to: **both image tags stay live in Harbor for one full wave, built
from the same conductor commit.** Rollback is repointing the env's image tag and
redeploying — config keys never move, because `relay_url` and `signal_url`
coexist harmlessly in every config (both are just fields on the one
`NetworkConfig`; the unused one is ignored, and `deny_unknown_fields` is
satisfied by both being *known* keys).

That is a better rollback than a config flip would have been: it is a single
pinned artifact reference, it is atomic per StatefulSet, and it cannot half-apply.

The tx5 vendor patch (`7cc927e`) and the `[patch.crates-io]` block stay in the
fork for the wave. They retire only when the tx5 image tag does.

### 5.2 D10. The flip is per-namespace-atomic, not per-peer

**A partial flip partitions the DHT.** Peer URLs are transport-shaped:

- tx5: `wss://{sbd-host}/{pubkey}` (from tx5's own local_url;
  `kitsune2_transport_tx5-0.4.1/src/lib.rs:58,325`)
- iroh: `https://{relay-host}:443/{endpoint_id}`
  (`kitsune2_transport_iroh-0.4.1/src/url.rs:22-52`)

A tx5 conductor cannot dial an `https://` peer URL and an iroh conductor cannot
dial a `wss://` one. They share a bootstrap server and a DNA hash and are still
unable to exchange a single byte. **This is the same atomicity rule as the
genesis-pair `ALLOW_DNA_REINSTALL` requirement, arriving from a different
direction:** hash-partition there, transport-partition here.

**This corrects the campaign plan.** "M/J/J household mesh proves the mechanism
first" is not executable on the live alpha DHT — the trio would prove the
mechanism by severing itself from adam and the rest. Revised sequence:

**Stage 0 — mechanism proof, off the live DHT.** Two conductors on the
`-iroh` image, own bootstrap space, pointed at the live
`https://relay.elohim.host`. Proves the image boots, the relay is reachable from
outside, registration happens, and two peers exchange ops. Zero risk to alpha.
The upstream sweettest rendezvous harness is the cheaper first form of this —
run it with the relay URL overridden to ours.

**Stage 1 — alpha, whole namespace, one window.** All alpha conductors flip in
one rollout. The genesis pair (adam + matthew) must land in the same window by
the standing rule; but so must every other alpha peer, for the transport reason.
`@requires:alpha-cluster-6peer` for the soak, but the *flip* is not gated on
6-peer availability — it is gated on all live peers moving together. A peer that
is down during the window comes up on the new image (the StatefulSet spec is the
truth) and rejoins.

**Stage 2 — soak.** Full 6-peer soak against the trust-contract probes (§5.3)
before staging or prod are considered. The `direct=` ratio from Stage 2 is what
decides whether Phase B QAD (§3.4) is needed.

Prod and staging do not flip in Wave 2.

### 5.3 Probes

**Existing, from the substrate trust-contract runbook** — these carry over
unchanged and are the primary evidence that the flip preserved the substrate:

- `seam-smoke[bootstrap-sharing]` — must stay green; bootstrap is
  transport-independent and a red here means we broke the wrong thing.
- `seam-smoke[peer-store]` — each doorway's PRIMARY conductor holds addressed
  agent-infos.
- `seam-smoke[dht-fetch]` — landing canonical head identical on A and B.
- `✓ canonical head propagated` on every APP deploy console.
- `GET {doorway}/db/p2p/conductor-diagnostics` and
  `GET {doorway}/admin/bootstrap-coherence`.

**Retired by the flip:** `seam-smoke[signal-bus]` becomes meaningless — it probes
SBD frame delivery via `doorway/doorway-service/tools/sbd-cross-relay-probe.py`.
It must be *replaced*, not merely dropped, by the relay-reachability smoke below.

**New, relay-specific.** These are the ones that answer *"our relay, not n0's."*

| Probe | How | What it proves |
|---|---|---|
| **The n0-contamination test** | `GET {doorway}/db/p2p/conductor-diagnostics` → every `agents[].url` matches `https://relay.elohim.host:443/{endpoint_id}` | THE sovereignty probe. kitsune2 canonicalizes a peer URL as `{scheme}://{host}:{port}{path}/{endpoint_id}` (`transport_iroh/src/url.rs:31-52`), so the agent-info URL *is* the relay hostname. n0 contamination reads as `*.relay.iroh.network`. Anything not `relay.elohim.host` fails the gate. |
| **Home-relay registration sentinel** | Loki, `container=elohim-node`: `Received a new listening address from relay server` (`transport_iroh/src/lib.rs:636`) | the conductor actually got a home relay from the relay server, as opposed to sitting relay-less |
| **Direct-vs-relayed ratio** | Loki: `Connection established ... direct=true\|false` (`transport_iroh/src/lib.rs:842`) | the QAD-absence cost meter (§3.4). A `direct=false` majority is the number that justifies Phase B. |
| **Relay liveness from outside** | `curl -s -o /dev/null -w '%{http_code}' https://relay.elohim.host/ping` → `200`; `curl http://relay.elohim.host/generate_204` → `204` (not a 301) | the ingress serves both the Https latency probe and the captive-portal probe correctly; the 301 case is the `ssl-redirect` trap (§3.2) |
| **Relay-side census** | relay `/metrics` on :9090 — connected-client count should equal the flipped conductor count | every conductor found the relay; a short count names which peer did not |
| **No lingering tx5** | Loki: absence of `tx5 send error` (the current live warning) and absence of any `wss://signal.` peer URL in `conductor-diagnostics` | the flip is complete rather than partial (§5.2) |

**Config gate:** `validate-conductor-config.sh` extended per §4.2 — it is the
render-time form of the n0-contamination test, catching the mistake before it
deploys rather than after.

---

## 6. Retirement plan

Staged **inert → verified → removed**, with the gate stated before each removal.
Nothing here happens during the flip.

### 6.1 SBD signal server — retire in two pieces, not one

The module is `doorway/doorway-service/src/signal/` — 1607 lines across six
files. It is **not** uniformly tx5 scaffolding:

| Piece | Lines | Nature |
|---|---|---|
| `mod.rs`, `bus.rs`, `bus_mongo.rs`, `cmd.rs`, `store.rs` | 1167 | the SBD signal server proper — tx5 conductor signaling |
| `media.rs` | 440 | a **WebRTC media-session layer** (SDP offer/answer, ICE batching, session lifecycle) for human audio/video calls, layered on SBD |

`media.rs` has **zero dispatch sites**. Its types are re-exported from
`signal/mod.rs:39` and referenced only in a doc comment in
`services/recording.rs:31`. No route consumes them; no app code references them.
It is un-wired aspiration, not a live consumer — but it is also not tx5 debt, and
deleting it as "dead tx5 code" would silently discard a stated capability.

**Decision:** delete both, with the media layer's removal recorded as its own
decision — the human A/V media-signaling capability's future home is the peer
plane (iroh/EPR), not a doorway SBD extension. If that is contested, it is
contested *before* the delete, not discovered after.

**Removal inventory** (all verified):
`src/signal/` (6 files); `server/http.rs` mounts (:2035 reserved path, :4693-4702
dispatch, :5511-5519 handler, :1764-1778 startup + bus consumer, :36 import,
:127-128 state field, :490 construction); `config.rs` `signal_url` arg;
`health.rs` and `auth_routes.rs` surfacing; the hardcoded fallback at
`services/federation.rs:1276` (`"wss://signal.elohim.host"`); the
`signal.alpha.elohim.host` / `signal.elohim.host` / `signal.doorway*.elohim.host`
ingress hosts in `doorway/{alpha,alpha-b,prod}.yaml`; and
`tools/sbd-cross-relay-probe.py` with its `seam-smoke[signal-bus]` wiring.

**Gate before removal:** no conductor in any env speaks SBD — i.e. prod and
staging have also flipped. The doorway serves all envs; the SBD module cannot go
while a single tx5 conductor exists anywhere. **This means SBD retirement is a
post-Wave-2 item** — Wave 2 flips alpha only. Mark it inert in Wave 2
(documented as tx5-legacy at each site), remove when the last env flips.

### 6.2 Coturn — the beacon must be re-homed FIRST

**The blocker.** The `relay-addr-beacon` sidecar
(`harbor.ethosengine.com/ethosengine/relay-addr-beacon:dev-latest`) runs *inside*
both coturn pods and owns the dynamic-WAN DNS for the whole zone:

- coturn-shem's beacon → the `elohim.host` **apex** A record (shem WAN)
- coturn-operations' beacon → `alpha.elohim.host` A record (operations WAN)

Every other name in the zone CNAMEs to one of those two — including
`doorway.elohim.host`, `doorway-alpha.elohim.host`, `storybook`, `staging`, the
App pipeline's E2E target, and (per D7) `relay.elohim.host` itself. The beacon
also does per-cycle freshness verification and self-heals external clobbers in
≤30s (`project_two_premises_dns_beacon_owned`).

**Delete coturn without re-homing the beacon and the zone goes stale on the next
WAN IP change** — which is not a hypothetical on residential Google Fiber.

**Order (each step gated on the previous):**

1. **Re-home the beacon.** Two options, operator's call: (a) a standalone
   two-replica beacon Deployment (one per premise, same node pins as the coturn
   legs); (b) fold the operations-leg beacon into the iroh-relay pod as a
   sidecar and the shem-leg beacon into a standalone Deployment on shem. (a) is
   cleaner — it stops coupling DNS freshness to whatever service happens to be
   the relay this year, which is the coupling that created this problem. **(a) is
   the recommendation.**
   *Gate:* both A records observed self-healing from the new home (the beacon
   logs `exclusive record DRIFTED` and re-writes within 30s).
2. **Mark the TURN config inert.** After the alpha flip, `webrtc_config`'s TURN
   iceServers configure nothing on alpha. Leave the keys, add a comment.
   *Gate:* alpha soak green (§5.2 Stage 2).
3. **Remove the TURN iceServers from conductor configs** once prod and staging
   have flipped too. *Gate:* no tx5 conductor anywhere.
4. **Remove `alpha-coturn-operations.yaml` and `alpha-coturn-shem.yaml`.**
   *Gate:* (1) and (3) both done; the coturn pods have zero connected allocations
   for a full soak period. Also retire the shared credential
   `alpha-turn-commons-2026` and the router port-forwards documented in the
   coturn headers (3478/udp+tcp, 49152-49999/udp ops, 49160-49200/udp shem) — the
   operator owns the router side.

**Verified clean otherwise:** nothing besides the humans template and
adam-firstman wires `turn:` URLs; edgenode manifests are STUN-only; no other
manifest or service consumes the coturn pair.

### 6.3 The tx5 vendor patch

`7cc927e` (ethosengine/tx5 zombie-fix, `[patch.crates-io]` in the conductor fork)
retires when the tx5 image tag does (§5.1) — one full wave after the alpha flip,
and not before prod/staging flip. It costs nothing to keep and everything to
remove early.

---

## 7. Operator ratification checklist

**Already decided by the campaign plan — no ratification needed:**

- Wave 2 is the transport-sovereignty flip; alpha first; prod/staging later.
- tx5 stays available for one full wave after the flip.
- The n0 public relay is rejected (this doc records the *how*, the campaign
  already recorded the *whether*).
- Cluster ops are operator-owned; this manifest is inert until applied.

**Requires operator ratification — these change what infrastructure exists:**

| # | Decision | Why it is the operator's |
|---|---|---|
| 1 | **DNS:** create `relay.elohim.host` CNAME → `alpha.elohim.host` (Cloudflare, `proxied: false` — the beacon hard-writes unproxied) | zone change; and the proxied flag interacts with the beacon's exclusive-record lane |
| 2 | **Harbor mirror:** build + push `harbor.ethosengine.com/ethosengine/iroh-relay:0.95.1-dev-latest` (recipe in the manifest header) | new image in the registry the fleet trusts |
| 3 | **Apply** `genesis/orchestrator/manifests/infra/alpha-iroh-relay.yaml` (whichever pipeline applies `infra/*.yaml`) | cluster action |
| 4 | **NetworkPolicy:** add port 9090 to `allow-metrics-from-observability` so the relay PodMonitor scrapes | touches a policy file carrying a live-drift warning |
| 5 | **Ratify the placement:** relay on `node-type=operations` rather than shem (D7), and **single relay** with the redundancy regression accepted for one wave (D3) | availability posture; contradicts the deliberate dual-WAN choice made for coturn |
| 6 | **Ratify the unauthenticated posture** (D4) — anyone who learns the hostname can relay their own traffic through it | security posture |
| 7 | **Ratify the atomic flip window** (D10) — all alpha conductors in one rollout; the household-first staging in the campaign plan is not executable on the live DHT | changes the campaign's stated sequencing |
| 8 | **Ratify the two image tags** (D9) — `hc-elohim-0.6.3` and `hc-elohim-0.6.3-iroh` both live in Harbor for the wave | registry footprint + the rollback contract |
| 9 | **Beacon re-homing before any coturn removal** (§6.2), and which option (standalone Deployment recommended) | DNS for the whole zone depends on it |
| 10 | **Ratify deleting the un-wired `media.rs` A/V signaling layer** (§6.1) with the record that its future home is the peer plane | discards a stated (if unbuilt) capability |
| 11 | **Router port-forwards:** none needed for Phase A (ingress-only). Phase B QAD would need UDP 7842 forwarded on the operations router | premise network |

---

## 8. STILL-UNKNOWN — with verification steps

Each of these is unresolvable from local evidence. None blocks writing the
manifest; each blocks a specific claim.

**U1 — Does n0 publish an `iroh-relay:0.95.1` container image?**
Unverifiable without web access. *Moot by design:* D5 builds from crates.io into
Harbor regardless, so nothing depends on the answer. If the operator prefers an
upstream image, verify the tag exists AND that it is 0.95.1 (not "latest" on the
1.0 line, which would be a protocol-version question — see 2.4) before adopting.

**U2 — Does `cargo install iroh-relay --version 0.95.1 --locked --features server` build standalone?**
High confidence yes: the published lock's crypto chain is identical to the
conductor's, and our Lane-B `cargo test -p holochain_p2p` compiled that exact
chain green. But that build happened inside the conductor's workspace
resolution, not standalone. *Verification:* the image build itself is the probe
— `docker build` the manifest's recipe; a red here is a dependency-resolution
problem, not a design problem, and the fallback is to vendor the conductor's
resolved pins for the dalek/digest/sha2 chain.

**U3 — Does a `1.0.3` relay serve `0.95.1` clients, and does a `1.0.3` client negotiate down to a `0.95.1` server?**
The first direction is strongly supported (`ProtocolVersion::ALL = &[V2, V1]`;
V2's `Status` frame is documented as not-sent-to-V1-clients). The second is
untested — a 1.0.3 client sends a multi-version subprotocol list and a 0.95.1
server matches on the bare `iroh-relay-v1` string; whether the list form is
accepted by the older matcher is not verified. *Verification (Wave 3, before the
family move):* stand up a 1.0.3 relay alongside the 0.95.1 one and point a
single 0.95.1 conductor at it; then a 1.0.3 storage endpoint at the 0.95.1 relay.
*Consequence if the second direction fails:* Wave 3 must upgrade relay-first,
never client-first — which is the recommended order anyway.

**U4 — What fraction of conductor connections are relayed with QAD off?**
Upstream's harness proves QAD-absent *works*; it says nothing about hole-punch
success rates across two real residential NATs and a cloud NAT. *Verification:*
the `direct=true|false` sentinel (§5.3) over the Stage-2 soak. This is the sole
input to the Phase-B QAD decision — do not build Phase B before reading it.

**U5 — Do the conductor's failing QAD probes produce log noise or backoff that looks like a fault?**
The probes will be attempted (`RelayMap::from_iter` sets `quic: Some(7842)`
unconditionally) and will fail. Benign in principle; the question is whether
they generate warn-level noise that will be misread during the soak.
*Verification:* Stage 0 (off-DHT two-conductor proof) — read the full log of a
conductor against the QAD-less relay and record the exact lines, so the soak's
readers know what normal looks like.

**U6 — Does ingress-nginx pass `Sec-Websocket-Protocol: iroh-relay-v1` through unmodified?**
Standard behavior, and the doorway's existing websocket annotation set is the
same shape. But the relay hard-requires the subprotocol echo — the server
matches on it (`http_server.rs:484`) and the client requires
`SWITCHING_PROTOCOLS`. *Verification:* Stage 0, or directly:
`curl -i -H 'Connection: Upgrade' -H 'Upgrade: websocket' -H 'Sec-WebSocket-Version: 13' -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==' -H 'Sec-WebSocket-Protocol: iroh-relay-v1' https://relay.elohim.host/relay`
→ expect `101` with `Sec-WebSocket-Protocol: iroh-relay-v1` echoed.

**U7 — Which pipeline applies `genesis/orchestrator/manifests/infra/*.yaml`?**
The `alpha-doorway-podmonitor.yaml` header records this as an open question as
of 2026-06-17 ("If the PodMonitors are not yet live, the apply-path for
`infra/*.yaml` is the separate gap to close"). If that gap is still open, the
relay manifest is authored-but-never-applied. *Verification:* operator confirms
the apply path before Stage 0; a relay that exists only in the repo blocks the
whole wave.
