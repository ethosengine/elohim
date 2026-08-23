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
| `--sink` | — | — | Enable a sink (`cloudflare`, `pkarr`, `coturn`). Repeatable; sinks compose. At least one required. |
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

##### Shared doorway-set mode (sibling-safe multi-A "logical anycast")

The exclusive lane above assumes ONE beacon owns `--record-name` outright.
Shared lanes let **multiple beacon instances — one per WAN — each maintain
their OWN A/AAAA record under a shared hostname**, so a doorway set (e.g.
`doorways.elohim.host`) resolves to several relays without any instance ever
clobbering a sibling's record. One beacon can contribute to several shared
hostnames by repeating `--shared-record <name>=<owner>`; order is preserved.
This is the protocol primitive "address-set contribution with ownership +
freshness"; Cloudflare DNS is its current projection. Shared lanes run
**alongside** the exclusive lane, not instead of it.

| Flag | Env | Default | Meaning |
|------|-----|---------|---------|
| `--shared-record <name>=<owner>` | `BEACON_SHARED_RECORDS` (comma-separated) | — | Preferred repeatable shared lane. Each value binds its hostname and owner atomically; duplicate hostnames are rejected. |
| `--shared-record-name` | `BEACON_SHARED_RECORD_NAME` | — | Legacy single shared hostname. Still accepted unchanged; requires the legacy `--record-owner` pair. |
| `--record-owner` | `BEACON_RECORD_OWNER` | — | Legacy owner paired with `--shared-record-name`. |
| `--shared-refresh-secs` | `BEACON_SHARED_REFRESH_SECS` | `300` | Max age of our own freshness stamp before we re-PATCH even with an unchanged IP. |
| `--shared-stale-secs` | `BEACON_SHARED_STALE_SECS` | `900` | Age beyond which a SIBLING's record is considered abandoned and reaped (DELETEd). Must be greater than `--shared-refresh-secs` (validated at startup). |

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
  stamping.
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
