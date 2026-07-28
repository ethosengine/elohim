# SHEM — remote-premises node: intent, network topology, ports, and DNS

**Status:** living document. Last verified **2026-07-28**.
**Audience:** anyone touching coturn, the beacon, doorway routing, DNS, or debugging why
the shem-side conductors won't converge.

---

## 1. Why shem exists — the intent

`shem` is **not overflow capacity**. It models a second, independent premises.

The alpha fleet is a two-premises deployment:

| premises | node(s) | doorway | public name | represents |
|---|---|---|---|---|
| **operations** (on-prem LAN) | `ethosengine`, `intel-nuc`, `hp-micro10`, `thinkc-*` | doorway-A | `alpha.elohim.host` | **matthew's** side |
| **remote** (adam's premises) | `shem` | doorway-B | `elohim.host` | **adam's** side |

`elohim-adam-alpha` is the **shem-side genesis anchor** — the peer whose fetches toward
the household are the ones that matter. adam is meant to be a real peer operating a
doorway *from his own premises*, reachable at *his own* WAN.

**The load-bearing consequence:** shem must function as an **ingress point**. WAN traffic
for adam's doorway should enter the cluster *at adam's premises*, not enter at the
operations premises and hairpin back over the WireGuard tunnel.

> ⚠️ **Superseded guidance.** Older notes said *"treat shem as backend-only; don't put
> Ingress-served services there"* (a workaround for a cross-node host→pod bug). That is
> **no longer true and no longer desirable.** Verified 2026-07-28: shem runs its own
> nginx ingress controller and serves `doorway-alpha.elohim.host` — whose pod lives on
> `intel-nuc` — with HTTP 200. Cross-node host→pod works. Do not re-apply the old rule.

---

## 2. Topology

| node | k8s InternalIP | LAN | WAN | role |
|---|---|---|---|---|
| `ethosengine` | 192.168.86.100 | 192.168.86.0/24 | **136.51.77.49** | performance; **terminates WireGuard** (`wg0` = 10.99.0.1/24) |
| `intel-nuc` | 192.168.86.102 | 192.168.86.0/24 | (same WAN) | operations; router port-forward target for the ops premises |
| `shem` | **10.99.0.2** | **192.168.1.100** | **136.50.16.133** | remote premises; `node-type: remote`, tainted `remote-wan=true:NoSchedule` |

- shem's k8s node IP (`10.99.0.2`) is the **WireGuard tunnel address**, not a LAN address.
- shem's WAN is **residential NAT with a dynamic IP**. Nothing may hardcode it. (This is
  the entire reason `relay-addr-beacon` exists — see §5.)
- WG overhead ⇒ `wg0` MTU **1420**. Calico VXLAN adds 50 B ⇒ `FELIX_VXLANMTU=1370`
  cluster-wide. Do not revert without re-checking every path.

### Reaching shem

| you are on | use | note |
|---|---|---|
| operations LAN / `ethosengine` | **`10.99.0.2`** | routes straight down `wg0`; survives WAN-IP churn — **preferred** |
| shem's own LAN | **`192.168.1.100`** | direct; router not in the path |
| anywhere public | `136.50.16.133` | dynamic; only for external reachability *testing* |

> 🪤 **NAT loopback trap.** From *inside* shem's LAN, connecting to `136.50.16.133` gives
> **"connection refused"** even when the forward is correct — consumer routers don't
> hairpin, so the packet hits the router's WAN interface, DNAT is never applied, nothing
> listens, RST. **Never verify a port-forward from inside its own LAN.** Test from
> `ethosengine`, which is a genuinely different WAN.

### Pod placement (alpha)

- **on `shem`:** `elohim-adam-alpha-0`, `elohim-eve-alpha-0`, `elohim-gertrude-alpha-0`,
  `elohim-susan-alpha-0`, `elohim-doorway-alpha-b`, `coturn-shem`, an nginx ingress controller
- **on `ethosengine`:** `elohim-matthew-alpha-0`, `elohim-jessica-alpha-0`, `elohim-james-alpha-0`
- **on `intel-nuc`:** `elohim-doorway-alpha`, `coturn-ethosengine`

`elohim-adam-alpha-0`'s `openebs-hostpath` PVCs pin it to shem (~1.4 GB, mostly
`/data/blobs`). Moving it is a data migration — **and would defeat the model.** Don't.

---

## 3. Ports — the full setup

`coturn` runs `hostNetwork: true`, so its ports land **directly on the node's host**.
Forwards at each premises router therefore target the node's LAN IP
(**192.168.1.100** for shem, the ops equivalent for `intel-nuc`).

| port | transport | purpose | required? |
|---|---|---|---|
| **3478** | **UDP** | **TURN/STUN control channel** — `ALLOCATE`, `CREATE_PERMISSION`, `CHANNEL_BIND`, auth | **mandatory — this is the gate** |
| 3478 | TCP | same, TCP candidate — ICE config advertises `?transport=tcp` | yes (else advertised-but-dead) |
| **49160–49200** | **UDP** | **relay media pool** — one port held per live allocation | mandatory, and **too small** (see below) |
| 5349 | TCP | TURN over TLS (`turns:`) | later phase — **not yet functional**, see §3.3 |
| 80 / 443 | TCP | nginx ingress (doorway-B, signal) | yes |
| 22 | TCP | ssh | optional; currently open on shem's WAN |

### 3.1 Why the relay range alone does nothing

TURN splits **control** from **media**:

1. Client sends `ALLOCATE` to **3478**, authenticating with the long-term credential.
2. coturn binds a free port from `min-port..max-port` and returns it as the
   `XOR-RELAYED-ADDRESS`.
3. The remote peer sends media to `<public-WAN-IP>:<that relay port>`.
4. coturn relays it to the client.

**No reachable 3478 ⇒ no `ALLOCATE` ⇒ no relay port is ever assigned ⇒ the forwarded
range sits idle forever.** This was exactly the state before 2026-07-28: the relay range
was forwarded, 3478 was not, and `coturn-shem` had served **zero** sessions since start
while the operations leg had served ~35k. The dual-WAN pair was running single-legged.

**Symptoms that trace back to this** (do not misread as application bugs):

- `tx5_go_pion_sys: Failed to refresh permissions: CreatePermission error response (error 400)`
- `kitsune2_core: could not send publish ops: tx5 send error … (src: timed out)`
- `kitsune2_gossip: Initiated round timed out` — rounds dying at stage `Initiated`
  with `peer_max_op_data_bytes: 0` (the peer never answered)
- `projection-reconcile: sweep exceeded wall-clock budget` (120 s)
- `get_links response channel dropped: likely response timeout` on zome calls
- shem-side peers stuck at `caught_up: false` indefinitely

The conductor config already documents the underlying reason relay is *required* and
STUN alone is insufficient:

> *"the genesis pair straddles shem(cloud NAT) ↔ on-prem(home NAT); srflx↔srflx pairing
> fails silently with STUN only, and app #1604 proved the DHT fetch stays down even with
> fully CONVERGED peer stores — the data channel itself is the seam."*

### 3.2 Status as of 2026-07-28

Verified from `ethosengine` (external vantage, different WAN):

```
136.50.16.133  tcp/22    OPEN
136.50.16.133  tcp/80    OPEN
136.50.16.133  tcp/443   OPEN
136.50.16.133  tcp/3478  OPEN     ← added 2026-07-28
136.50.16.133  udp/3478  STUN Binding Success (0x0101)  ← added 2026-07-28, THE unblock
136.50.16.133  tcp/5349  closed   ← not yet configured, see §3.3
```

`coturn-shem` went from silent to serving. Last 30 min:

```
266  CREATE_PERMISSION
102  error 437 (invalid allocation — stale handles during ICE re-gather, expected)
 46  CHANNEL_BIND
  6  ALLOCATE
  6  error 401 (normal first-round auth challenge)
  0  error 508
```

Sessions show `local 192.168.1.100:3478, remote 136.51.77.49:…` — the operations premises
is now relaying **into** shem. **The dual-WAN pair is finally two-legged.**

> Fleet convergence has **not** landed yet. The shem quartet is still `caught_up: false`;
> divergent-anchor counts are beginning to trend down on adam (3334→3242) and eve
> (3424→2903). Re-measure before drawing conclusions.

### 3.3 Open: relay pool is undersized — and the shem router can't widen it

`min-port=49160` / `max-port=49200` is **41 ports**. On the operations leg, measured over
one hour on 2026-07-28:

```
29  ALLOCATE processed, success
31  ALLOCATE error 508: Cannot create socket   ← 52% failure, pool exhaustion
```

Re-measured the same evening, AFTER the shem 3478 fix made the relay genuinely
two-legged (per-leg, last-hour):

```
operations leg:  104 ALLOCATE attempts, 88 error 508   ← 85% failure, worsening
shem leg:         18 ALLOCATE attempts,  0 error 508   ← healthy
```

Correlated: adam's gossip-round timeout ratio was ~85% (213/250) the same hour. The
operations relay pool is now the acute suspect for the convergence stall (§10).

**Constraint discovered 2026-07-28: the pool CANNOT be widened symmetrically.** The
GFiber router in front of shem (Google-account config UI) cannot edit an existing
port-forward range — widening shem's slice would require a router reset. The
49160–49200 forward is therefore the standing contract on the shem side. Decided:
**do not widen shem.** (Harvested as an operator-capacity constraint:
`genesis/a2o/features/dataplane/relay-capacity.feature` — router class bounds how many
peers a doorway premises can service.)

Remaining levers, asymmetric by design:

1. **Operations-only widening** — the operations premises router is not the GFiber
   Google-account UI; if it can forward UDP 49152–49999, widen the router forward and
   `min-port`/`max-port` in `alpha-coturn-operations.yaml` ONLY (router first, then
   manifest — a wider ConfigMap before the router forward advertises unreachable relay
   ports). Shem's ConfigMap stays 49160–49200 to match its router. The legs do not
   need to match each other; each leg's conf must only match ITS OWN router forward.
2. **Lifetime tuning within 41 ports** (either leg, ConfigMap-only, no router
   dependency): shorter `max-allocate-lifetime` / faster stale-allocation reaping so
   ports recycle faster, at the cost of more re-ALLOCATE churn. Measure before
   reaching for this — it trades exhaustion for renegotiation load.

### 3.4 Open: 5349 / TURN-over-TLS is not real yet

Forwarding 5349 alone does nothing. **Neither** coturn leg has `tls-listening-port`,
`cert=`, or `pkey=` — the only listener line is `listening-port=3478`. To land it:

1. Add `tls-listening-port=5349` + `cert=` / `pkey=` to both ConfigMaps.
2. Issue a cert for the TURN name via a **standalone cert-manager `Certificate` CR** —
   there is no ingress fronting coturn, so ingress-shim will not mint one automatically.
   Mount it into the coturn pod and hook the reload (the beacon already restarts
   turnserver via `--on-change-exec`, so there's an existing path to reuse on renewal).
3. Add the `turns:…:5349` URL to `webrtc_config.iceServers` — otherwise peers never try it.

**Set expectations:** `turns:` is TLS-over-TCP and inherits head-of-line blocking. On an
MTU-1420 WAN link carrying gossip it is the *slow* path. Its value is punching through
restrictive middleboxes — hardening, **not** a substitute for `udp/3478`.

---

## 4. SSRF guard (do not "simplify" this)

Both coturn ConfigMaps carry:

```
allowed-peer-ip=10.1.0.0-10.1.255.255      # Calico pod CIDR — EXCEPTION, checked first
denied-peer-ip=10.0.0.0-10.255.255.255
denied-peer-ip=172.16.0.0-172.31.255.255
denied-peer-ip=192.168.0.0-192.168.255.255
…
```

In coturn, `allowed-peer-ip` is an **exception carved out of** `denied-peer-ip`, not a
global whitelist — non-listed *public* peers stay relayable. An earlier change removed
the carve-out on the belief it was a whitelist; that broke conductor↔conductor relay with
`403: Forbidden IP`. The carve-out is correct and is present on both legs as of
2026-07-28. Residual 403s are LAN ranges (`192.168.x`) being denied **deliberately** —
the static in-repo TURN credential must not expose the LAN admin surface.

---

## 5. DNS — what manages what

### 5.1 `relay-addr-beacon`

A sidecar (**and an init container**) in each coturn pod:
`harbor.ethosengine.com/ethosengine/relay-addr-beacon:dev-latest`, 30 s interval,
`--sink coturn --sink cloudflare`. Auth: secret `relay-addr-beacon-cloudflare` (key `token`),
zone-scoped to `elohim.host`.

It does two jobs:

- **coturn sink** — renders `/base/turnserver.conf` → `/etc/coturn-run/turnserver.conf`
  with a live `external-ip` line, and `--on-change-exec "pkill -TERM -x turnserver"`
  restarts coturn when the WAN IP moves. *This is why nothing may hardcode shem's WAN IP.*
- **cloudflare sink** — writes DNS in two lanes:

| lane | flag | shem leg | operations leg |
|---|---|---|---|
| **owned** | `--record-name` | `turn-shem.elohim.host` | `turn.elohim.host` |
| **shared** | `--shared-record-name` + `--record-owner` | `doorways.elohim.host` (owner=`shem`) | `doorways.elohim.host` (owner=`operations`) |

The `beacon-owner=<x>; ts=<epoch>` **comment** on a record is the coordination mechanism:
it lets two independent beacons share one round-robin A name without clobbering each
other — each finds its own row by that stamp. All beacon-written records use TTL 60 s.

Source of truth: `genesis/orchestrator/manifests/infra/alpha-coturn-{operations,shem}.yaml`,
deployed by **Jenkins** (`app.kubernetes.io/managed-by: jenkins`). Beacon source lives at
`relay-addr-beacon/` in the same repo.

### 5.2 Current record inventory

**API-managed by the beacon** (TTL 60 s, carries a `beacon-owner` comment where shared):

| record | type | value | owner |
|---|---|---|---|
| `doorways.elohim.host` | A | 136.50.16.133 | `beacon-owner=shem` |
| `doorways.elohim.host` | A | 136.51.77.49 | `beacon-owner=operations` |
| `turn-shem.elohim.host` | A | 136.50.16.133 | shem beacon (owned lane) |
| `turn.elohim.host` | A | 136.51.77.49 | operations beacon (owned lane) |

**Hand-managed in the Cloudflare dashboard** (TTL Auto, no stamp, **not in any repo —
there is no DNS-as-code**):

| record | type | value |
|---|---|---|
| `elohim.host` | A | **136.51.77.49** |
| `alpha.elohim.host` | CNAME | `elohim.host` |
| `doorway.elohim.host` | CNAME | `elohim.host` |
| `doorway-alpha.elohim.host` | CNAME | `elohim.host` |
| `doorway-staging.elohim.host` | CNAME | `elohim.host` |
| `signal.elohim.host` | CNAME | `elohim.host` |
| `signal.alpha.elohim.host` | CNAME | `elohim.host` |
| `signal.doorway.elohim.host` | CNAME | `elohim.host` |
| `signal.doorway-alpha.elohim.host` | CNAME | `elohim.host` |
| `signal.doorway-staging.elohim.host` | CNAME | `elohim.host` |

### 5.3 🔴 DNS currently contradicts the premises model

Every CNAME funnels to the apex, and **the apex points at the operations premises**:

```
elohim.host        A      136.51.77.49    ← matthew's side
alpha.elohim.host  CNAME  elohim.host  →  136.51.77.49
signal.*           CNAME  elohim.host  →  136.51.77.49
```

So both doorways resolve to the same WAN, and `elohim.host` — intended as **adam's**
doorway — points at **matthew's** premises.

Concretely: adam's conductor is configured `signal_url: wss://signal.elohim.host`, which
resolves to 136.51.77.49. **adam's WebRTC signalling rendezvous is at matthew's premises**,
entering the cluster there and hairpinning back over the WireGuard tunnel to reach the
doorway-B pod that actually runs on shem. matthew's `signal.doorway-alpha.elohim.host`
also lands there, which for matthew is correct.

### 5.4 Cleanup / target state

> ⚠️ **Do not hand-set the apex A record to `136.50.16.133`.** That is a *residential,
> dynamic* WAN IP. A dashboard-pinned value is a time bomb that fires silently on the next
> ISP lease rotation — and it would take `elohim.host` down, not just TURN.

Give the apex to a beacon lane instead. Target:

| beacon | `--record-name` | effect |
|---|---|---|
| shem | `elohim.host` | apex = adam's premises, auto-tracking his dynamic WAN |
| operations | `alpha.elohim.host` | matthew's side gets its own A instead of a CNAME to apex |

ICE URLs become `turn:elohim.host:3478` (B leg) and `turn:alpha.elohim.host:3478` (A leg)
— role-named, **no node names in public DNS**, both sides dynamic-IP-safe. Naming
principle going forward: **name by premises/role, never by node.** "shem" belongs in
`shem.ethosengine.com` (infrastructure) and nowhere in `elohim.host` (the commons).
(Moving a record to `ethosengine.com` would also require `--cf-zone` to change **and**
the API token re-scoped.)

**Changeover sequence — REVISED 2026-07-28 after reading the beacon source**
(`relay-addr-beacon/src/sinks/cloudflare.rs`). Two facts change the §5.4 plan as
originally sketched:

- The owned lane matches by **name+type only** (no comment/stamp check) and **PATCHes
  an existing record in place**. So the beacon **adopts** a pre-existing hand-set A
  record — no duplicate, no empty-DNS window. Do NOT hand-delete the apex A first.
- The owned-lane lookup is **type-scoped to A**. An existing **CNAME at the same name
  is invisible** to it → the beacon POSTs a new A → Cloudflare rejects the CNAME/A
  conflict → the error is fatal in `--once` mode → **`beacon-init` CrashLoops and the
  coturn container never starts** (the §6.1 failure mode, new root cause). The
  `alpha.elohim.host` CNAME MUST be replaced by hand BEFORE the manifest deploys.

Therefore: **all dashboard prep first (zero traffic change), then ONE atomic repo
deploy.**

*Dashboard prep (Cloudflare, ~5 min, every step is same-IP → zero user-visible change):*

1. Delete the `alpha.elohim.host` CNAME and create in its place an **A record →
   136.51.77.49** (the current operations WAN — the same IP the CNAME resolves to
   today). The operations beacon will adopt and maintain it from deploy onward.
2. Re-point the operations-owned CNAMEs from `elohim.host` to `alpha.elohim.host`:
   `doorway-alpha`, `signal.alpha` (if kept as CNAME), `signal.doorway-alpha`,
   `storybook`, `staging`, `doorway-staging`, `signal.doorway-staging`. Same IP before
   and after — inert until the apex flips. Leave pointing at the apex (they are adam's
   side): `doorway`, `signal.elohim.host`, `signal.doorway`.
   (`test-holostrap` + `staging`: deletion candidates — operator call.)
3. Leave the apex A record exactly as it is (the shem beacon will adopt it).
4. Keep `doorways.elohim.host` untouched — the shared lane already models both premises.

*Then one repo deploy (already prepared in-tree 2026-07-28):* both beacon
`--record-name` renames (each in BOTH beacon-init AND sidecar — §6.1), both ICE URL
blocks in the human manifests (`turn.elohim.host`→`alpha.elohim.host`,
`turn-shem.elohim.host`→`elohim.host` — map by which old record it was, not textual
order), matthew's `signalUrl`/`SIGNAL_URL` → `wss://signal.alpha.elohim.host`, and the
doorway-A ingress gaining the `signal.alpha.elohim.host` host + TLS SAN (the legacy
`signal.doorway-alpha` host stays during transition). On the first beacon cycle
post-deploy the apex flips to shem's WAN; adam's names (`doorway`, `signal.elohim.host`)
follow the apex; matthew's names are already safely re-homed on `alpha.elohim.host`.

*After verifying the flip (`dig elohim.host` → 136.50.16.133; `beacon-init` exited 0 on
both legs; `dig alpha.elohim.host` → 136.51.77.49; **and** `doorway-alpha-tls` was
reissued with the `signal.alpha.elohim.host` SAN —
`openssl s_client -connect 136.51.77.49:443 -servername signal.alpha.elohim.host`
must show a valid LE cert, not self-signed. Until reissue completes, doorwayA-routed
conductors fail the signal TLS handshake; the legacy `signal.doorway-alpha` host is the
rollback path. A silent cert-order failure — e.g. LE rate limit — leaves the old
secret serving and the new host broken: check `kubectl`-side or just probe the SAN):*

5. **Delete** `turn.elohim.host` and `turn-shem.elohim.host` by hand. A beacon rename
   does **not** remove the old record; both would linger unmanaged, pointing at
   dynamic IPs that will silently rot.
6. Refresh the `elohim.host.txt` zone export snapshot.

**TLS caveat:** once the apex moves, cert-manager HTTP-01 challenges for
`elohim-host-apex-tls` land on **shem's** ingress. shem serves 200s today so it should
work — but this is the renewal to watch, same class of quiet failure as the Che cert.
`alpha.elohim.host` already has its own ingress cert (`alpha-elohim-site-tls-cert`,
elohim-app ingress); its renewals stay on the operations side, where its A record
points. On this single-cluster bench HTTP-01 tolerates either ingress answering (both
controllers share the cluster); the caveat matters the day the premises become truly
separate clusters.

---

## 6. Change gotchas

1. **`--record-name` appears TWICE per manifest** — once for `beacon-init`, once for the
   `beacon` sidecar. Change both. Missing one reproduces OPEN backlog item
   `manifest-cli-arg-drift-no-gate-beacon-crashloop`: on 2026-07-17 a `--state-dir` vs
   `--state-file` mismatch CrashLooped beacon-init ~93×, coturn never started, TURN DNS
   never appeared — and `kubectl apply` printed **"unchanged"** throughout. **Nothing goes
   red.** There is still no CI gate validating manifest `args:` against the binary's
   `--help`.
2. **A green image + an "unchanged" apply is a silent non-deploy.** Rollout requires an
   actual manifest diff.
3. **Renaming a beacon record never deletes the old one.** Clean up by hand.
4. **The ICE config and the DNS name must move in the same change.** Otherwise peers
   gather a candidate that resolves to nothing — the advertised-but-dead pattern that cost
   this fleet weeks. `iceServers` lives in
   `genesis/orchestrator/manifests/humans/*.yaml`,
   `genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml`, and
   `elohim/holochain/edgenode/conductor-config.yaml`.
5. **DNS records outside the beacon are not in any repo.** Commit/push through Jenkins
   fixes the TURN records; the apex and the CNAMEs must be changed in the Cloudflare
   dashboard (or brought under DNS-as-code — worth doing).

---

## 7. Not the problem — save yourself the trip

Things investigated on 2026-07-28 and **ruled out** as causes of the shem-side
non-convergence:

- **adam being under-resourced.** Limits 8 CPU / 8 Gi; actual ~4.5 cores / 1.5 Gi.
  `cpu.stat` showed 1940 throttled periods of 118225 (**1.6%**), 28.9 ms total throttle
  across 7.6 h of CPU time. Nothing to right-size.
- **Restarting adam.** The `ca38d4d7` deploy restarted it; 24 h CPU history shows no
  regime change across that boundary. Restarts thrash and have never fixed this class.
- **Arc factor.** All 7 alpha peers advertise the identical **full arc (512/512)**,
  including the three healthy ones. Also, `edgenodeArcFactor: 0` is documented as a relief
  valve for a **NON-ANCHOR** node — and adam is the anchor. Not a lever here.
- **Data volume.** Content inventory is identical (4336) on all seven peers.
- **coturn `403: Forbidden IP`.** The pod-CIDR carve-out is present on both legs;
  residual 403s are deliberately-denied LAN ranges.

The discriminator was always **node placement**, and underneath it, the transport seam:
4/4 shem pods `caught_up: false`, 3/3 operations pods `caught_up: true`. The operations
trio converge because they are co-resident on one node and don't need the relay.

---

## 8. Verification recipes

**External reachability** (run from `ethosengine`, *never* from inside shem's LAN):

```bash
for p in 22 80 443 3478 5349; do
  timeout 4 bash -c "</dev/tcp/136.50.16.133/$p" 2>/dev/null \
    && echo "tcp/$p OPEN" || echo "tcp/$p closed"
done
```

**STUN probe on udp/3478** — note the packet must be exactly 20 bytes:

```bash
# Do NOT build the transaction ID via $(head -c12 /dev/urandom):
# command substitution strips null bytes and silently yields a short, invalid packet.
printf '\x00\x01\x00\x00\x21\x12\xa4\x42\xaa\xbb\xcc\xdd\xee\xff\x11\x22\x33\x44\x55\x66' > /tmp/stun.bin
nc -u -w 4 136.50.16.133 3478 < /tmp/stun.bin | xxd | head -2
# Expect a Binding Success Response: first two bytes 0101, magic cookie 2112a442,
# and the transaction ID echoed back.
```

**coturn health / pool exhaustion:**

```bash
kubectl -n elohim-alpha logs -l app=coturn -c coturn --since=60m \
  | grep -oE 'error [0-9]+: [A-Za-z ]+' | sort | uniq -c | sort -rn
# 508 "Cannot create socket" = relay pool exhausted → widen min-port/max-port
```

**Fleet convergence** — the number that matters:

```bash
for p in adam eve gertrude susan matthew jessica james; do
  kubectl -n elohim-alpha logs elohim-$p-alpha-0 --since=30m 2>/dev/null \
    | grep -a 'projection-reconcile: heal complete' | tail -1 \
    | grep -oE '"caught_up":(true|false)|"content_divergent_anchor":[0-9]+' \
    | tr '\n' ' '; echo " <- $p"
done
```

**Gossip round health** (ratio of timeouts to initiations; high = transport seam):

```bash
kubectl -n elohim-alpha logs elohim-adam-alpha-0 --since=30m \
  | grep -acE 'Initiated round timed out|Initiated gossip with'
```

---

## 9. Action list

| # | action | owner | where |
|---|---|---|---|
| 1 | ~~Forward udp/3478 + tcp/3478 at adam's router~~ **DONE 2026-07-28** | ops | shem router |
| 2 | ~~Widen relay pool symmetrically~~ **RETIRED 2026-07-28** — GFiber cannot edit shem's forward range (§3.3). Replacement: operations-only widening IF the ops router permits (router first, then `alpha-coturn-operations.yaml` only) | ops + dev | router + repo |
| 3 | ~~Re-measure `caught_up` on the shem quartet~~ **DONE 2026-07-28 — verdict STALLED** (§10); acute suspect = operations relay-pool exhaustion (§3.3), standing suspect = signalling hairpin (§5.3) | dev | §8 |
| 4 | ~~Prepare apex/alpha beacon renames + ICE + signal + ingress in repo~~ **PREPARED 2026-07-28** (in-tree, one atomic deploy). Gate: dashboard prep in §5.4 MUST happen first | dev | repo |
| 5 | Dashboard prep: replace `alpha.elohim.host` CNAME with A → 136.51.77.49; re-point operations-owned CNAMEs to it (§5.4 list) | ops | Cloudflare |
| 6 | Push/deploy the prepared change; verify apex flip + beacon-init exit 0 both legs | ops (integrator) | repo → Jenkins |
| 7 | Delete stale `turn.elohim.host` / `turn-shem.elohim.host` after verified flip; decide `staging`/`test-holostrap` deletions; refresh `elohim.host.txt` | ops | Cloudflare |
| 8 | Bring the apex/CNAME records under DNS-as-code | dev | repo |
| 9 | CI gate: validate manifest `args:` against the built binary's `--help` | dev | repo (backlog item is OPEN) |
| 10 | TURN-over-TLS on 5349: ConfigMap + standalone `Certificate` CR + ICE URL | dev | repo |

---

## 10. Current fleet state (2026-07-28 ~17:35 UTC — post-3478-fix re-measure: **STALLED**)

| peer | node | caught_up | divergent anchors (latest heal-complete) |
|---|---|---|---|
| adam | shem | false | 2122 |
| eve | shem | false | 3430 |
| gertrude | shem | false | 1023 |
| susan | shem | false | 3453 |
| matthew | ethosengine | **true** | 549 |
| jessica | ethosengine | **true** | 584 |
| james | ethosengine | **true** | 926 |

Caveat on raw numbers: the reconciler cycles through content-shard subsets (recurring
denominators 12000/8745/10000/6745/1178), so anchor counts swing by shard — compare
same-shard only. Same-shard 6h trend: **flat or slightly rising on nearly every shard**
(eve 8745-shard 3432→3431; gertrude 8745 3520→3520; susan 8745 3450→3453; several
shards dead flat for hours — zero healing). Only adam's 6745-shard improved (−92/4h).
The earlier "two of four trending down" read did not survive shard-aware comparison.

The 3478 fix was **necessary but not sufficient**. Corroborating signals the same hour:
adam gossip-round timeouts ~85% (213/250); operations-leg coturn ALLOCATE error-508 at
~85% (88/104) while the shem leg served cleanly (0/18) — see §3.3; tx5
`CreatePermission 400` continuing fleet-wide (including caught-up ethosengine peers,
so not a shem-specific residual). Next suspects, in order: **(1) operations relay-pool
exhaustion** (§3.3 — acute, measurable, lever identified), **(2) the signalling
hairpin** (§5.3 — fix prepared in-tree, §5.4 sequence). Re-run this section's measure
after each lever lands.
