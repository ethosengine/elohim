# relay-addr-beacon

Keeps a sovereign relay's **dynamic residential WAN IP** fresh in the places
that need it: DNS (Tier-1, consumed today), pkarr (Tier-2, published for a
future resolution bridge), and coturn's `external-ip` mapping (local).

It is **N-generalizable** — run one beacon per relay node. In the alpha
dual-WAN testbed there are two: one beside the `turn.elohim.host` coturn on the
ethosengine node, one beside `turn-shem.elohim.host` on the shem node. Both TURN
relays sit behind Google-Fiber residential NAT with a *dynamic* public WAN IP;
this beacon is what keeps their DNS records and coturn `external-ip` honest.

## Why

Behind residential NAT the host interface holds a private LAN address
(e.g. `192.168.1.100`) while the public WAN IP is dynamic. Two things break
unless something actively republishes the WAN IP:

1. **DNS** — `turn.elohim.host` must point at the current public WAN IP so ICE
   candidates can reach the relay.
2. **coturn** — coturn must advertise `external-ip=<wan>/<lan>` or it hands out
   the useless private candidate. coturn does **not** discover its own WAN IP.

## Prerequisites and first successful run

Builds require a current Rust toolchain and outbound access to the locked crate
registry. Runtime needs outbound HTTPS to at least one configured egress echo
endpoint plus the selected remote sink. The state file and any coturn output
path must be writable by the beacon process.

Sink-specific requirements:

- Cloudflare: a token scoped to the target zone with Zone Read and DNS Edit.
- pkarr: outbound HTTPS to the configured relay and a writable `PKARR_KEY_FILE`.
- coturn: readable base config and writable output config. An
  `--on-change-exec` restart additionally needs the process privileges described
  in the coturn section below.

From the repository root, build the native binary with the WASM-only flag
cleared:

```
cd relay-addr-beacon
RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/relay-addr-beacon-target cargo build --release
/tmp/relay-addr-beacon-target/release/relay-addr-beacon --help
```

For a first run that does not mutate DNS or contact pkarr, render a coturn file
under `/tmp` once. It still contacts an egress echo endpoint to discover the
public WAN address:

```
printf 'listening-port=3478\n' > /tmp/turnserver.base.conf
/tmp/relay-addr-beacon-target/release/relay-addr-beacon \
  --once --sink coturn --lan-ip 192.0.2.10 \
  --state-file /tmp/relay-addr-beacon-state.json \
  --coturn-base-conf /tmp/turnserver.base.conf \
  --coturn-out-conf /tmp/turnserver.conf
```

Success exits zero and logs `detected address snapshot`, `sink publish ok`, and
`persisted published snapshot`. Confirm `/tmp/turnserver.conf` contains one
`external-ip=<detected-wan>/192.0.2.10` line and inspect the JSON state file.
The next useful action is to configure the production sink you need, run once,
and verify its projection: query DNS for Cloudflare, fetch the pkarr packet, or
start coturn with the rendered config. The deployment examples below show the
long-running composition.

## Interface

```
relay-addr-beacon [OPTIONS] --sink <SINK> [--sink <SINK> ...]
```

Precedence is **flag > env > default**.

### Core

| Flag | Env | Default | Meaning |
|------|-----|---------|---------|
| `--interval-secs` | `BEACON_INTERVAL_SECS` | `30` | Poll interval for the detect→publish loop. |
| `--once` | — | `false` | Detect + run every sink once, then exit (runs sinks regardless of change). Ideal as an initContainer. |
| `--enable-v6` | `BEACON_ENABLE_V6` | `false` | Also publish public IPv6 (AAAA / pkarr aaaa). This phase is v4-first. |
| `--record-name` | `BEACON_RECORD_NAME` | — | DNS record name for the Cloudflare sink (e.g. `turn.elohim.host`). |
| `--lan-ip` | `BEACON_LAN_IP` | auto | Override the detected LAN IPv4 for coturn's `<wan>/<lan>` mapping. |
| `--state-file` | `BEACON_STATE_FILE` | `/var/lib/relay-addr-beacon/state.json` | Persisted last-published snapshot for change detection. |
| `--sink` | — | — | Enable a sink (`cloudflare`, `pkarr`, `coturn`, `file`). Repeatable; sinks compose. At least one required. |
| `--egress-endpoint` | `BEACON_EGRESS_ENDPOINTS` | ipify / ifconfig.me / icanhazip | Ordered public-IP echo endpoints; first success wins. |

### Detection

- **Public WAN IP** — HTTP GET to each egress endpoint in order; the first that
  returns a parseable IP of the wanted family wins. Defaults:
  `https://api.ipify.org`, `https://ifconfig.me/ip`, `https://icanhazip.com`.
- **LAN IPv4** — a UDP socket is `connect`-ed toward `8.8.8.8:80` (no datagram is
  sent); the kernel picks the egress interface and we read its local address.
  This is std-only — no interface-enumeration dependency. Override with
  `--lan-ip`.

Only on a **change** from the last-published snapshot (persisted to
`--state-file`) are the sinks invoked. `--once` always runs them. State is
persisted **only when every sink succeeds**, so a transient failure retries on
the next cycle instead of being suppressed.

### Sinks

#### `cloudflare` (Tier-1 — consumed today)

Upserts an `A` (and `AAAA` when `--enable-v6`) record for `--record-name` in
`--cf-zone`, `ttl=60`, **`proxied=false`** (the Cloudflare proxy cannot carry
UDP, so a proxied record would break STUN/TURN).

| Flag | Env | Meaning |
|------|-----|---------|
| `--cf-zone` | `CF_ZONE` | Zone name that owns the record (e.g. `elohim.host`). |
| `--cf-token` | `CF_API_TOKEN` | Cloudflare API token (Bearer). |
| `--cf-token-file` | `CF_API_TOKEN_FILE` | File containing the token (trailing whitespace trimmed); used if the flag/env is unset. |

Flow: `GET /zones?name=<zone>` → zone id; `GET /zones/{zid}/dns_records?type=A&name=<rec>`
→ PATCH if it exists else POST create.

That default behaviour is the **exclusive lane**: one beacon instance owns
`--record-name` outright and keeps it correct. `--record-name` is always
required by the cloudflare sink, including when the lane you actually care
about is a shared one — so point it at a per-instance diagnostic name you want
anyway. It must NOT also be one of the shared names: a shared lane and an
exclusive owner cannot both own one name (the exclusive upsert patches the
first record it finds at that name, which may be a sibling's). With
`--serving-probe-url` configured that collision is refused at startup; without
a probe nothing catches it, so keep the names distinct.

##### Shared doorway-set mode (sibling-safe multi-A "logical anycast")

The exclusive lane above assumes ONE beacon owns `--record-name` outright.
Shared lanes let **multiple beacon instances — one per WAN — each maintain
their OWN A/AAAA record under a shared hostname**, so a doorway set (e.g.
`doorways.elohim.host`) resolves to several relays without any instance ever
clobbering a sibling's record. One beacon can contribute to several shared
hostnames by repeating `--shared-record <name>=<owner>`; order is preserved,
and duplicate hostnames (case- and terminal-dot-insensitive) are rejected at
startup. This is the protocol primitive "address-set contribution with
ownership + freshness"; Cloudflare DNS is its current projection. Shared lanes
run **alongside** the exclusive lane, not instead of it.

**Lanes are independent.** Each lane carries its OWN membership state — its own
join/leave counters, its own applied-projection marker, its own publish gate.
One serving probe decides one thing ("is *my* doorway serving?") and every lane
is reconciled against that same verdict, but a lane whose projection fails (a
Cloudflare 503, a wedged document lock) retries on its own and is never
recorded as applied because a sibling lane succeeded on the same tick. That is
the difference between two lanes and one lane written twice.

| Flag | Env | Default | Meaning |
|------|-----|---------|---------|
| `--shared-record <name>=<owner>` | `BEACON_SHARED_RECORDS` (comma-separated) | — | Preferred repeatable shared lane. Each value binds its hostname and owner atomically; duplicate hostnames are rejected. |
| `--shared-record-name` | `BEACON_SHARED_RECORD_NAME` | — | Legacy single shared hostname. Still accepted unchanged; requires the legacy `--record-owner` pair. |
| `--record-owner` | `BEACON_RECORD_OWNER` | — | Legacy owner paired with `--shared-record-name`. |
| `--shared-refresh-secs` | `BEACON_SHARED_REFRESH_SECS` | `300` | Max age of our own freshness stamp before we re-PATCH even with an unchanged IP. |
| `--shared-stale-secs` | `BEACON_SHARED_STALE_SECS` | `900` | Age beyond which a SIBLING's record is considered abandoned and reaped (DELETEd). Must be greater than `--shared-refresh-secs` (validated at startup). |

**Membership is earned from the serving probe** when `--serving-probe-url` is
configured (the flags are tabled under the [`file` sink](#file-household-ownable-membership-projection),
and they govern BOTH projections identically). A lane joins after
`--serving-join-after` consecutive HTTP 200s — `POST`/`PATCH`ing its record —
and withdraws after `--serving-leave-after` consecutive non-200s, `DELETE`ing
only the records it owns. While a lane is withdrawn, an address-change publish
cannot re-advertise it. The probe is OPTIONAL for the cloudflare sink: with no
probe, every configured lane publishes unconditionally, which is what the
deployed doorway-set legs do today. It is REQUIRED for the `file` sink.

**Ownership rides the Cloudflare record `comment` field** —
`beacon-owner=<slug>; ts=<unix-seconds>`, tolerant-parsed (unknown keys
ignored; a missing/garbled stamp never resolves to an owner). Our OWN
freshness check (whether to re-`PATCH` an unchanged IP) also uses that
comment `ts`, since it's compared against our own local clock — same-clock
by construction. A **sibling's** reap-staleness is judged differently: from
Cloudflare's server-side `modified_on` on the record, never the sibling's
self-reported comment `ts`. Fleet clocks skew by hours, so trusting a
sibling's own `ts` would reap a live-but-behind-clock sibling every cycle
(permanent flap) while never reaping an ahead-clock dead one; `modified_on`
is a clock every instance agrees on.

For each configured shared lane and enabled record type (A, and AAAA with
`--enable-v6`), every cycle `GET`s all records at that name and partitions them:

- **mine** (comment parses with `owner == --record-owner`) — absent: `POST`
  create with a fresh stamp. Present: `PATCH` iff the content changed OR our
  stamp is older than `--shared-refresh-secs` (this is what keeps a beacon
  whose WAN IP never changes from ever looking abandoned); otherwise left
  untouched. Every record whose comment parses to OUR OWN owner slug is
  skipped in the reap pass below, not just the one record chosen as "mine".
- **siblings** (comment parses with a *different* owner) — **never patched**.
  Reaped (`DELETE`) only once the sibling record's `modified_on` is older
  than `--shared-stale-secs`; a reap is logged at `warn` with the owner and
  age. Below that age, left strictly alone. A sibling with a missing or
  unparseable `modified_on` is treated as fail-safe **not reapable** (logged
  at `warn`) — a garbled/absent Cloudflare timestamp must never be mistaken
  for staleness.
- **unowned** (missing/garbled comment) — **never touched** automatically;
  its presence is logged at `warn` so an operator can investigate. This is
  the safety rule that makes the mode sibling-safe: a beacon only ever
  mutates the record it can prove is its own, and only ever deletes a record
  it can prove is a stale peer's.

**Main-loop interaction.** The address-detect loop normally skips every sink
when the WAN address is unchanged. When shared mode is configured, an
unchanged cycle runs the Cloudflare freshness pass, which verifies the
exclusive lane and iterates every shared lane; other sinks remain untouched.
That is enough to keep freshness stamps and stale-sibling reap active without
reintroducing churn elsewhere (see the `cycle` doc comment in `src/main.rs`).

#### `file` (household-ownable membership projection)

Shared membership is a **set of origins eligible to serve one public name**.
The Cloudflare shared lane above projects that set as `A`/`AAAA` records for a
zone you rent; the `file` sink projects the **same set**, decided by the **same
`reconcile_membership` call, the same serving probe and the same join/leave
hysteresis**, into a JSON document on a filesystem you own.

It exists because a household cannot certify public-name transition against DNS
it does not control, and a test-only proxy that never executes the real routing
decision certifies nothing. Two legs — one per doorway — writing their own entry
into one document *is* the routing apparatus.

| Flag | Env | Default | Meaning |
|------|-----|---------|---------|
| `--membership-file` | `BEACON_MEMBERSHIP_FILE` | — | Where this leg's membership document(s) live. Sibling legs share it. One document per public name — see the derivation rules below. |
| `--member-origin` | `BEACON_MEMBER_ORIGIN` | — | The origin (`scheme://host:port`) this leg advertises. Required because household doorways can differ by PORT on one host, which no address snapshot can distinguish. |

The serving-probe flags below are shared with the Cloudflare shared lane (both
decide membership from the same probe); the `file` sink **requires** the URL:

| Flag | Env | Default | Meaning |
|------|-----|---------|---------|
| `--serving-probe-url` | `BEACON_SERVING_PROBE_URL` | — | Health URL polled to decide whether this leg's doorway is serving. Only HTTP 200 serves — a redirect, an error and silence do not. |
| `--serving-probe-interval-secs` | `BEACON_SERVING_PROBE_INTERVAL_SECS` | `15` | Probe cadence, independent of address discovery. Also bounds each probe's timeout (`min(interval, 15s)`). |
| `--serving-join-after` | `BEACON_SERVING_JOIN_AFTER` | `2` | Consecutive serving probes required to join, including after a restart. |
| `--serving-leave-after` | `BEACON_SERVING_LEAVE_AFTER` | `3` | Consecutive non-serving probes required to withdraw. |

The public name and this leg's owner slug come from the **same**
`--shared-record <name>=<owner>` (or legacy `--shared-record-name` /
`--record-owner`) lanes the Cloudflare sink uses; at least one is required.
`--serving-probe-url` is **required** — membership is earned from the probe,
never assumed.

**One document per public name.** A document's `name` field names ONE public
name, so a leg contributing to two lanes writes two documents, each with its
own entry for this owner. Which file each lane lands in is derived from
`--membership-file`, in this order:

| Configured `--membership-file` | Lane `elohim.host` lands at | When to use it |
|--------------------------------|------------------------------|----------------|
| contains `{name}` — e.g. `/var/lib/beacon/{name}/members.json` | `/var/lib/beacon/elohim.host/members.json` | any lane count; the explicit spelling, and the one that does not depend on the directory already existing |
| directory-shaped — a trailing `/` (e.g. `/var/lib/beacon/membership/`), or a path that already exists as a directory | `/var/lib/beacon/membership/elohim.host.json` | the usual two-lane shape |
| a plain file path — e.g. `…/membership/elohim.local.json` | that path, **verbatim** | a single lane (every existing single-lane invocation is unchanged) |

A plain file path with **two or more** lanes is refused at startup, naming both
fixes: deriving sibling documents beside a file the operator already named
would leave that name meaning nothing, and writing one lane there while
inventing paths for the rest is exactly the last-value-wins class repeatable
lanes exist to remove. A terminal dot is DNS-equivalent and never reaches a
file name (`elohim.host.` becomes `elohim.host.json`); a lane name containing a
path separator is refused (no DNS name has one).

Document shape:

```json
{
  "name": "elohim.local",
  "members": [
    { "owner": "alpha", "origin": "http://localhost:8888", "updated_at": "2026-09-12T12:52:27Z" },
    { "owner": "apex",  "origin": "http://localhost:8889", "updated_at": "2026-09-12T12:52:27Z" }
  ],
  "updated_at": "2026-09-12T12:52:27Z"
}
```

Rules, identical in spirit to the Cloudflare shared lane:

- **Exact-owner writes.** A leg only ever adds, refreshes or removes the entry
  whose `owner` is its own; a sibling's entry is never rewritten, reordered into
  a different value, or reaped. There is deliberately **no stale-sibling reap**:
  on one host the legs share a clock and a filesystem, so an absent sibling is
  an absent *process* — a household fact to read, not a record to collect.
- **Exactly one entry per owner**, and `members` is sorted by owner so the
  document is deterministic.
- **Freshness.** A serving leg re-stamps its own `updated_at` once it is older
  than `--shared-refresh-secs`.
- **Restart starts withdrawn**, then earns membership back with
  `--serving-join-after` consecutive serving probes.
- **Concurrency.** Every read-modify-write is taken under an `O_EXCL` lock file
  (`<path>.lock`, broken after 30 s if abandoned) and committed by `rename`, so
  a reader never sees a partial document and two legs never clobber each other.

A `file`-only leg needs **no address detection at all** (membership is
origin-keyed, not address-keyed), so it runs on a loopback mesh with no public
IP and no internet — the address loop is skipped entirely.

Two legs, one host, two doorway ports:

```
# leg A — the alpha doorway
relay-addr-beacon --sink file \
  --shared-record elohim.local=alpha \
  --membership-file /tmp/elohim-local-mesh/membership/elohim.local.json \
  --member-origin http://localhost:8888 \
  --serving-probe-url http://127.0.0.1:8888/health \
  --serving-probe-interval-secs 3

# leg B — the apex doorway
relay-addr-beacon --sink file \
  --shared-record elohim.local=apex \
  --membership-file /tmp/elohim-local-mesh/membership/elohim.local.json \
  --member-origin http://localhost:8889 \
  --serving-probe-url http://127.0.0.1:8889/health \
  --serving-probe-interval-secs 3
```

`app/elohim-app/scripts/hc-mesh.sh` stages exactly these two legs at
`just mesh start` (opt out with `MESH_MEMBERSHIP=0`), and the household fixture
declares the document so acceptance scenarios resolve the public name through
it.

The same two legs staging **two** public names — the household rehearsal of the
apex shape, where both doorways serve the apex *and* the doorway set. Each leg
now writes two documents under the directory-shaped path:

```
# leg A — the alpha doorway, contributing to BOTH names
relay-addr-beacon --sink file \
  --shared-record elohim.local=alpha \
  --shared-record doorways.elohim.local=alpha \
  --membership-file /tmp/elohim-local-mesh/membership/ \
  --member-origin http://localhost:8888 \
  --serving-probe-url http://127.0.0.1:8888/health

# leg B — the apex doorway, the same two names under its own owner slug
relay-addr-beacon --sink file \
  --shared-record elohim.local=apex \
  --shared-record doorways.elohim.local=apex \
  --membership-file /tmp/elohim-local-mesh/membership/ \
  --member-origin http://localhost:8889 \
  --serving-probe-url http://127.0.0.1:8889/health
```

That writes `membership/elohim.local.json` and
`membership/doorways.elohim.local.json`, each holding one entry per serving
owner. Freezing leg B's doorway withdraws `apex` from **both** documents after
`--serving-leave-after` probes and leaves leg A's entries byte-identical in
both.

#### `pkarr` (Tier-2 — published, not yet consumed) — OFF by default

Signs a `SignedPacket` with an `A` (+ `AAAA` when `--enable-v6`) record at the
signer's **apex** (`.`) and PUTs the relay payload to `{relay}/{z32}`.

| Flag | Env | Default | Meaning |
|------|-----|---------|---------|
| `--pkarr-key-file` | `PKARR_KEY_FILE` | `/var/lib/relay-addr-beacon/pkarr.key` | Dedicated pkarr secret key (hex, `0600`). Generated if absent. **Do not reuse an iroh/libp2p key.** |
| `--pkarr-relay` | `PKARR_RELAY` | `https://elohim.host/pkarr` | Relay endpoint **including** the `/pkarr` path; the z-base-32 public key is appended. |

> **Tier split — honest status.** ICE **cannot yet consume pkarr names.** The
> conductor ICE config and tx5 resolve ordinary DNS (Tier-1) today. Turning a
> pkarr key into a usable TURN host requires the Tier-2 resolution bridge, which
> is **not built yet**. This sink publishes the records so that bridge has
> something to resolve when it lands — they are *not* consumed by any relay
> client today. Hence it is off by default.
>
> The `signed_packet` feature set does **not** compile `pkarr::Client`, so the
> PUT is done directly with `reqwest`.

#### `coturn` (local)

Writes `--coturn-out-conf` = the contents of `--coturn-base-conf` with an
appended `external-ip=<wan>/<lan>` line (or bare `<wan>` when the LAN IP is
unknown), then optionally runs `--on-change-exec`.

| Flag | Env | Meaning |
|------|-----|---------|
| `--coturn-base-conf` | `COTURN_BASE_CONF` | Base config, copied verbatim before the appended line. |
| `--coturn-out-conf` | `COTURN_OUT_CONF` | Output path for the rendered config. |
| `--on-change-exec` | `COTURN_ON_CHANGE_EXEC` | Command run via `sh -c` after the config changes. |

> **coturn reload caveat.** coturn does **not** hot-reload `external-ip` on
> `SIGHUP` — the value is read at startup. So `--on-change-exec` must **RESTART**
> coturn, not signal it. No reload mechanism is hardcoded; the operator supplies
> the appropriate command for their environment. Recommended values:
>
> - Kubernetes sidecar/pod: `--on-change-exec 'pkill -TERM -x turnserver'`
>   (signal coturn so it exits and the kubelet restarts the container).
> - systemd host: `--on-change-exec 'systemctl restart coturn'`.
> - Docker: restart the coturn container by whatever supervisor owns it.
>
> **Requirements for the `pkill -TERM -x turnserver` mechanism** (beacon and
> coturn in the same pod, beacon signalling coturn across containers):
>
> - The pod must set `shareProcessNamespace: true` so the beacon container can
>   see coturn's process.
> - The beacon image must contain `pkill` — provided by the `procps` package,
>   which this crate's `Dockerfile` installs.
> - coturn must be **PID 1** of its container and handle `SIGTERM` (exit
>   cleanly) so the container restarts and re-reads `external-ip`.
> - The beacon must run as **root** or hold **`CAP_KILL`** to signal a process
>   in another container.
>
> Do **not** use `kill -TERM 1` — with `shareProcessNamespace: true` PID 1 is the
> shared pause/namespace process, and even without it that signals the beacon
> itself, not coturn.

## Examples

Cloudflare + coturn, looping (typical relay-node daemon):

```
relay-addr-beacon \
  --sink cloudflare --sink coturn \
  --record-name turn.elohim.host \
  --cf-zone elohim.host \
  --coturn-base-conf /etc/coturn/turnserver.base.conf \
  --coturn-out-conf  /etc/coturn/turnserver.conf \
  --on-change-exec 'pkill -TERM -x turnserver'
# CF_API_TOKEN supplied via env/secret.
# Requires shareProcessNamespace + procps + CAP_KILL — see the coturn reload caveat.
```

One-shot initContainer that writes coturn's config before coturn starts:

```
relay-addr-beacon --once --sink coturn \
  --coturn-base-conf /config/turnserver.base.conf \
  --coturn-out-conf  /config/turnserver.conf
```

Two beacons contributing to a shared doorway set, each behind its own WAN —
`operations` and `shem` both publish under `doorways.elohim.host` alongside
their own exclusive record, without clobbering each other:

```
# instance A
relay-addr-beacon --sink cloudflare \
  --record-name turn.elohim.host --cf-zone elohim.host \
  --shared-record doorways.elohim.host=operations

# instance B (different WAN, different owner slug)
relay-addr-beacon --sink cloudflare \
  --record-name turn-shem.elohim.host --cf-zone elohim.host \
  --shared-record doorways.elohim.host=shem
```

To contribute one beacon leg to two shared sets, repeat the atomic flag. This
only configures mechanism; adding a production hostname remains an explicit
operator-owned DNS decision:

```
relay-addr-beacon --sink cloudflare \
  --record-name turn.elohim.host --cf-zone elohim.host \
  --shared-record doorways.elohim.host=operations \
  --shared-record doorway-canary.elohim.host=operations
```

**The apex two-lane shape** (2026-09-12 ruling: the apex `elohim.host` is
served by BOTH doorways, so each beacon leg contributes an owner record to two
shared names). Copy this into a manifest's `args` — two `--shared-record`
values, one per name, the same owner slug on both, and the apex name must NOT
also be that leg's exclusive `--record-name` (validation refuses it: a shared
lane and an exclusive owner cannot both own one name):

```
# leg "operations"
relay-addr-beacon --sink cloudflare \
  --record-name alpha.elohim.host --cf-zone elohim.host \
  --shared-record doorways.elohim.host=operations \
  --shared-record elohim.host=operations

# leg "shem" (different WAN, different owner slug, same two shared names)
relay-addr-beacon --sink cloudflare \
  --record-name turn-shem.elohim.host --cf-zone elohim.host \
  --shared-record doorways.elohim.host=shem \
  --shared-record elohim.host=shem
```

Note the shem leg's `--record-name`: today that leg owns `elohim.host`
exclusively, so moving the apex into a shared lane means giving that leg a
DIFFERENT exclusive name in the same edit (above, `turn-shem.elohim.host`) —
otherwise one name is both exclusively owned and shared-contributed by the same
process. The apex flip itself, and the doorway ingress/TLS SAN that must accept
the apex host before it answers, remain operator-owned decisions; this crate
only supplies the mechanism.

As YAML `args` (the deployed spelling; the legacy
`--shared-record-name`/`--record-owner` pair the coturn manifests still use
stays accepted, and is equivalent to a single `--shared-record` value):

```yaml
- "--shared-record"
- "doorways.elohim.host=operations"
- "--shared-record"
- "elohim.host=operations"
```

## Development gate

Native crate (no Holochain WASM flag). In constrained environments point the
target dir at a writable `/tmp` slot:

```
just gate    # cargo fmt --check && cargo clippy -D warnings && cargo test
```

## Verification status (honest)

- `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` are the
  gate. See the slice hand-off for the actual recorded results of the run in
  this environment.
- Unit tests cover: repeatable/legacy shared-lane parsing and validation, IP
  parse/validation, egress-endpoint resolution,
  change-detection state (save/load/diff), Cloudflare request-body shape
  (`proxied=false`, `ttl=60`, and — shared lane — `comment`), the owner-comment
  parse/format round-trip (tolerant of unknown keys, reordering, and garbled
  input), and coturn config rendering (`<wan>/<lan>`, bare `<wan>`, newline
  handling). Tests do **not** touch the real network.
- The Cloudflare shared lane additionally has `wiremock`-backed integration
  tests (a `dev-dependency` only) driving a local mock HTTP server: create when
  absent, patch-only-mine among mine/fresh-sibling/unowned, zero mutating
  calls on unchanged-IP-plus-fresh-stamp, stamp-refresh on unchanged-IP-plus
  stale-stamp, exact-record reap of a stale sibling (per `modified_on`), no
  reap of a fresh sibling, a garbled-comment record never touched, a
  clock-skew regression proving an hours-stale comment `ts` with a FRESH
  `modified_on` is not reaped, a missing-`modified_on` sibling treated as
  fail-safe not-reapable, a same-owner duplicate record never reaped, an
  AAAA-lane create proving the shared list call is type-scoped with a correct
  AAAA body, ordered multi-lane PATCH fan-out, and exclusive-lane ownership
  stamping. Two further production-cycle tests drive `serving_cycle` with TWO
  Cloudflare lanes: both lanes join on the second serving probe and withdraw on
  the third non-serving one with a sibling's apex record untouched, and — with
  record removal broken for one lane only — the failing lane neither marks
  itself applied nor suppresses the lane that succeeded.
- The `file` membership sink has unit tests (document materialisation,
  exact-owner writes with a byte-identical sibling entry across a withdrawal, no
  duplicate owner on rejoin, hand-edited duplicate collapse, corrupt-document
  rebuild, an origin change touching only our own entry, 25 rounds of concurrent
  two-leg writes leaving a complete document and no lock/temp residue, and an
  abandoned lock being broken) plus production-cycle tests that drive
  `serving_cycle` itself: join2/leave3 across two legs sharing one document, a
  restarted leg withdrawing then re-adopting exactly one entry, a dead probe
  endpoint withdrawing the same way a shedding one does, two lanes on one leg
  writing two derived documents and withdrawing from both together (with the
  sibling leg's entries byte-identical in both), the `{name}` placeholder
  deriving the same pair, and a single-lane leg still writing the verbatim
  configured document with nothing derived beside it. Config tests cover the
  per-lane path derivation, the refusal of two lanes into one named document,
  and the refusal of a lane name carrying a path separator.
- The `Dockerfile` and live DNS/coturn integration are **not** exercised by the
  test gate; they are provided for the operator to build and deploy.

## Deps

crates.io only (via the Nexus mirror): `tokio`, `reqwest` (rustls-tls, json),
`serde`/`serde_json`, `clap`, `tracing`/`tracing-subscriber`, `anyhow`,
`pkarr` (`default-features = false`, `features = ["signed_packet"]`), `bytes`,
`time` (`default-features = false`, `features = ["parsing"]` — RFC3339
parsing of Cloudflare's `modified_on`, used for clock-skew-safe sibling reap
staleness; no `chrono`/hand-rolled date parsing).
Dev-only: `wiremock` (Cloudflare shared-lane HTTP-mock tests).
Zero internal path-deps; own `[workspace]` stanza and `Cargo.lock`.
