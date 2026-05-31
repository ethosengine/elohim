# Track B — DDNS + ACME Crate Design

**Date:** 2026-05-31
**Status:** Design spec + scaffold (Track B of cluster-outage resilience)
**Umbrella:** `2026-05-31-cluster-outage-resilience-decomposition-design.md` §4
**Crate:** `crates/elohim-edge-presence/` (name rationale in §2)
**Author:** Rust Architect agent (worktree `agent-a90c1d6039f3d8213`)

---

## 1. Context

### 1.1 Why this crate exists

The live cluster outage of 2026-05-31 exposed that `elohim.host` / `alpha.elohim.host`
cannot remain WAN-reachable when half the on-prem microk8s cluster goes down. The root
dependency is that TLS termination and DNS are currently handled by k8s cert-manager /
external-dns — components that stop working when the cluster control plane loses quorum.

Track B removes that dependency at the runtime layer: any `doorway-service` or
`steward/node` process can self-register a stable public hostname (dynamic DNS) and
self-obtain/renew TLS certificates via ACME, with no reliance on k8s ingress or
cert-manager.

This crate is the **enabler** for the (future, out-of-scope-now) Shem failover front-door
— a lightweight ingress in front of Shem's microk8s with its own WAN IP/DNS that keeps
orphaned-but-running Shem pods reachable when cluster comms drop (umbrella spec §6).

### 1.2 Relation to pkarr (resolving the tension)

pkarr is already in-tree
(`doorway/doorway-service/src/routes/pkarr_resolver.rs`,
`doorway/doorway-service/src/services/pkarr_resolver.rs`) as the peer-discovery layer.
It is the `external-dns` replacement for **peers finding each other**, and it operates
at the protocol layer (iroh discovery + self-hostable resolvers, umbrella §3.1).

This crate is **complementary**, not competing:

| Layer | Mechanism | For whom |
|-------|-----------|----------|
| **Peer discovery** | pkarr (signed records on DHT) | Protocol peers (elohim nodes, P2P transports) |
| **Browser-facing HTTPS** | Traditional DNS A/AAAA + TLS (this crate) | Browsers, curl, anything that speaks standard DNS + TLS |

Browsers cannot resolve pkarr names. A household doorway serving `https://myfamily.elohim.host`
to a browser requires a DNS A record pointing to the household's WAN IP and a certificate
trusted by the browser's root store. pkarr cannot satisfy either requirement today. The
two mechanisms are not in tension — they serve different consumers of the same box.

**A peer MUST register both.** pkarr announces "I am node X at transport address Y" to
protocol peers; DNS+cert announces "hostname H resolves to WAN IP Z with valid TLS" to
browsers and the wider internet. The `EdgePresence` orchestrator type in this crate
handles the DNS+cert side; pkarr registration remains in the caller's domain (steward or
doorway setup).

### 1.3 What exists today (stubs to fill)

`steward/node/src/dashboard/setup.rs` contains:

- `DnsProvider` enum — `None | Cloudflare { api_token, zone_id } | DuckDns { token, domain } | NoIp { username, password, hostname } | Ddclient { config }`. Type shapes are correct; function bodies are empty `Ok(())` TODO match arms.
- `configure_ddns(&DnsProvider) -> Result<(), String>` — empty match.
- `configure_https(hostname, email) -> Result<(), String>` — empty body.

This crate extracts and implements these stubs. The caller in `setup.rs` will replace its `TODO` bodies with `elohim_edge_presence::EdgePresence::new(cfg).run()`.

### 1.4 Doorway TLS today

`doorway/doorway-service/src/server/http.rs` runs a plain TCP `TcpListener::bind` (hyper
http1) with no TLS. `DOORWAY_HOSTNAME` is read-only, defaults to `"localhost"`. Outbound
requests use `reqwest` with `rustls-tls` feature but there is no inbound TLS negotiation.
`PublicSurfaceState` in `dashboard_topology.rs` stubs `tls_valid: false` and
`tls_expires_in_days: None` (Phase 4 stubs explicitly marked "Phase 5 wires real
probes").

This crate provides what Phase 5 needs: a cert store and an acceptor the hyper server
can wrap its listener with.

---

## 2. Crate name: `elohim-edge-presence`

**Chosen over** `elohim-edge-cert`, `elohim-ddns`, `elohim-wan-cert`:

- `edge-presence` matches the protocol vocabulary ("presence" = "I am here and
  reachable"). The crate establishes a node's *presence* at the WAN *edge*.
- `elohim-` prefix is consistent with other `crates/` members (`elohim-sdk`,
  `elohim-storage-client`).
- Avoids encoding the implementation detail (DDNS, ACME) in the name; the crate's public
  surface is "I am present at this hostname with valid TLS", not "I ran DNS and ACME".

**Location:** `crates/elohim-edge-presence/` — consistent with the standalone crate
pattern used by `crates/doorway-client`, `crates/elohim-sdk`, `crates/elohim-storage-client`.
Each crate in `crates/` has its own `Cargo.toml` with no shared workspace root; consumers
add the crate via `path = "../../crates/elohim-edge-presence"`.

---

## 3. P2P Design Gate

**Classification of every piece of state the crate manages:**

| State | Classification | Category | Rationale |
|-------|---------------|----------|-----------|
| DNS A/AAAA record (at Cloudflare or DuckDNS) | EXTERNAL operational state | C | Owned by the DNS provider; the crate sets it but does not own the source of truth. Reconstructable by re-running the update. |
| WAN IP address (discovered at runtime) | Ephemeral operational state | C | Transient; re-discovered on each cycle. Not persisted. |
| ACME account key (Ed25519 or ECDSA P-256) | Local operational secret | C | Private to the node; never goes to DHT. Persisted in `~/.elohim/edge-presence/account.key` (or configurable path). |
| Issued TLS certificate (DER chain) | Local operational state | C | Derived from ACME issuance; persisted locally. Reconstructable by re-running ACME. |
| ACME order state (nonce, auth URL, challenge token) | Ephemeral operational state | C | In-memory only; discarded on completion. |
| Hostname → this-node binding | NOT this crate's concern | — | This is pkarr's domain (peer-layer discovery) and the operator's DNS config. The crate writes the DNS record as instructed; the mapping is declared by the operator, not derived. |

**This crate introduces ZERO new DHT entry types.** There is no distributed state.
All state is either local (on-disk key/cert files) or external (DNS provider records).

**No HTTP route is proposed that should have been a DHT entry.** The crate exposes no
HTTP API of its own; it is a library consumed by `doorway-service` and `steward/node`.

---

## 4. Decision points and alternatives

### 4.1 ACME library: `rustls-acme` vs `instant-acme`

**Option A: `rustls-acme` (0.x / 0.7.x)**

- High-level: wraps `tokio-rustls` and the ACME protocol behind a single `AcmeConfig` +
  stream-based `Incoming` adapter. Minimal user code.
- Embeds TLS acceptor: its `Incoming` wraps a `TcpListener` and produces
  `TlsStream<TcpStream>`. Works natively with hyper 1.x via `TokioIo`.
- HTTP-01 and TLS-ALPN-01 challenges supported; DNS-01 not natively (requires a
  custom `ResolvesServerCert` hook).
- Less control over cert lifecycle: renewal is driven by the `Incoming` stream rather
  than an explicit `renew()` call. Harder to unit-test in isolation.
- The stream-adapter pattern makes it awkward to retrofit onto an existing hyper server
  that was built around a plain `TcpListener` — the listener must be replaced with the
  `AcmeConfig::incoming()` adapter.

**Option B: `instant-acme` (0.7.x)**

- Lower-level: exposes `Account`, `Order`, `Authorization`, `Challenge` as typed structs.
  The ACME protocol is explicit; the library does not touch TLS termination at all.
- Cert loading and `tokio-rustls` wiring is the caller's responsibility (adding ~60–80
  lines of explicit `ServerConfig` + acceptor setup), but yields full control.
- DNS-01 challenge is a first-class citizen: the library hands you the `_acme-challenge`
  TXT record value; you call your `DnsProvider` trait to set it.
- `rcgen` (cert generation) and `rustls-pki-types` wire in cleanly alongside it.
- Testable in isolation: you can construct a mock `Account` against a local Pebble
  instance without swapping out the hyper listener.

**Recommendation: `instant-acme`.**

Rationale: DNS-01 challenge is the correct challenge type for this use case (see §4.4).
`instant-acme` treats DNS-01 as first-class; `rustls-acme` does not. The extra 60–80
lines of `tokio-rustls` wiring pay for themselves in testability and clear ownership of
the cert store. The stream-adapter pattern of `rustls-acme` creates a tight coupling
between TLS termination and cert renewal that makes the gating logic in §4.3 harder to
implement cleanly.

Additional: `instant-acme` composes well with the "doorway today has a plain
`TcpListener`" fact. The crate can wrap the listener without modifying the existing `run()`
function's call site (see §4.3 and §8).

**Supporting dependencies:** `rcgen` (CSR generation), `rustls-pki-types` (cert/key
wrappers), `tokio-rustls` (TLS acceptor), `rustls` 0.23.x (already present in the
dependency tree via quinn).

### 4.2 WAN IP discovery

**Option A: Provider API query (ipify.org / ifconfig.me / ipinfo.io)**

Simple HTTP GET to a well-known public IP reflection service. Returns the public IPv4
(and optionally IPv6). No dependencies beyond `reqwest`.
Downside: adds an external service dependency; a single service is a single point of
failure. Mitigated by trying multiple in sequence.

**Option B: libp2p observed-addr / iroh endpoint**

Read the "observed address" reported by a connected peer during the libp2p identify
or iroh endpoint negotiation handshake. No external HTTP call; derived from existing
P2P transport.
Downside: requires an active peer connection; unusable at startup before any peer has
connected. Also: this crate must be usable by `doorway-service` which does not currently
have an in-process libp2p node.

**Option C: STUN (RFC 5389/8489)**

Send a STUN Binding Request to a public STUN server (Google, Cloudflare, coturn).
Returns the reflexive transport address. Requires a STUN client dependency or a
minimal hand-rolled UDP exchange.
Downside: an additional protocol dependency; STUN servers can also be unavailable.

**Recommendation: Option A (multi-provider cascade) as primary, with Option B as
enrichment hook.**

Rationale: `doorway-service` does not embed a P2P runtime, so Option B cannot be
the sole mechanism. The cascade (try providers in order, return first success) is robust
to any single provider outage. The `WanIpDiscoverer` struct exposes an
`inject_observed_addr` method so `steward/node` can hint a peer-observed IP without an
external call — Option B as optional enrichment, not required path.

Default cascade order: `https://api.ipify.org`, `https://ifconfig.me/ip`,
`https://ipinfo.io/ip`. All are HTTPS. The crate does NOT hard-code these; they are
config with sensible defaults.

### 4.3 TLS termination gating (MUST NOT break existing k8s-ingress deployments)

**Option A: `ELOHIM_EDGE_TLS=false` env flag (opt-in at startup)**

The crate adds TLS only when an env var is set. Existing k8s deployments never set it;
local/bare-metal deployments that want self-managed TLS set it.
Simple and explicit. No code path changes for the k8s deployment.

**Option B: Config struct field + feature flag**

`EdgePresenceConfig { tls_mode: TlsMode::Off | TlsMode::TerminateInProcess | TlsMode::DeferToIngress }`.
The `doorway-service` `Args` parser maps CLI/env to `TlsMode`.
More expressive; allows a "defer-to-ingress" mode that explicitly acknowledges k8s-ingress
as the terminator without conflating it with "no TLS at all".

**Option C: Separate listener on a second port (e.g., :8443)**

The existing plain HTTP listener remains untouched on :8080. The crate opens a second
TLS listener on :8443. k8s deployments ignore :8443 (not in the Service/Ingress spec).
Downside: two ports, two listeners, connection handling code split.

**Recommendation: Option B (TlsMode config enum) as the primary switch, with default
`TlsMode::DeferToIngress` in production builds.**

Rationale: Option B is more explicit and safer than Option A (a missing env var in a
production container silently stays in `Off` mode which is correct but undiscoverable in
logs). The `DeferToIngress` variant produces a log line at startup:
`"TLS mode: DeferToIngress — inbound TLS handled by k8s ingress"`, making the intent
auditable. `TerminateInProcess` is the mode bare-metal doorways and steward nodes use.

The `TlsMode::TerminateInProcess` path wraps the hyper `TcpListener` in a `TlsAcceptor`
(from `tokio-rustls`) whose `ServerConfig` is populated from the cert store managed by
`EdgePresence`. The acceptor is hot-reloaded on cert renewal by swapping an `Arc<ServerConfig>`
behind an `ArcSwap` — no restart required.

### 4.4 ACME challenge type: DNS-01 vs HTTP-01

**HTTP-01:** ACME server fetches `http://{hostname}/.well-known/acme-challenge/{token}`.
Requires port 80 to be publicly reachable. Works when the process is already listening
on port 80. Does NOT work through a NAT without port-forwarding; does NOT work for
wildcard certificates.

**DNS-01:** ACME server resolves `_acme-challenge.{hostname}` TXT record. Requires write
access to the DNS zone but NOT port 80 reachability. Works behind NAT. Supports wildcards.
Requires the `DnsProvider` to expose a `set_txt_record` method.

**Recommendation: DNS-01 as primary, HTTP-01 as optional fallback.**

Rationale: the target deployment is a household doorway behind a home NAT/ISP router.
Port 80 may not be publicly reachable (ISPs often block inbound port 80/443). DNS-01
avoids this entirely. The same `DnsProvider` trait that sets A records for DDNS is
extended with `set_txt_record` for DNS-01 challenges — no new abstraction needed.

HTTP-01 remains available as an opt-in (`AcmeChallenge::Http01`) for deployments where
port 80 is reachable and DNS-01 is impractical (e.g., when using a DNS provider whose
API doesn't support programmatic TXT record writes, like ddclient over generic DNS).

---

## 5. Module and trait architecture

```
crates/elohim-edge-presence/
├── Cargo.toml
└── src/
    ├── lib.rs              -- re-exports EdgePresence, EdgePresenceConfig, TlsMode, DnsProvider
    ├── config.rs           -- EdgePresenceConfig, TlsMode, AcmeChallenge, WanIpConfig
    ├── error.rs            -- EdgePresenceError (thiserror)
    ├── wan_ip.rs           -- WanIpDiscoverer, injected-addr support
    ├── dns/
    │   ├── mod.rs          -- DnsProvider trait, DnsRecord types
    │   └── cloudflare.rs   -- CloudflareDnsProvider (reqwest-based; API stubs)
    ├── acme.rs             -- AcmeManager: order → challenge → cert issuance + renewal
    ├── tls.rs              -- CertStore, TlsAcceptor wrapper, hot-reload via ArcSwap
    └── orchestrate.rs      -- EdgePresence orchestrator tying all modules together
```

### 5.1 `DnsProvider` trait (the extensibility seam)

```rust
/// Abstraction over a dynamic DNS provider.
///
/// Implementations: `CloudflareDnsProvider`, `DuckDnsDnsProvider` (future),
/// `NoIpDnsProvider` (future), `DdclientBridge` (future).
///
/// The trait is async and object-safe (via `async_trait`).
#[async_trait::async_trait]
pub trait DnsProvider: Send + Sync + 'static {
    /// Provider name for logging.
    fn name(&self) -> &'static str;

    /// Update the A (IPv4) record for `hostname` to `ip`.
    async fn set_a_record(&self, hostname: &str, ip: Ipv4Addr) -> Result<(), EdgePresenceError>;

    /// Update the AAAA (IPv6) record for `hostname` to `ip`.
    /// Providers that do not support IPv6 may return Ok(()) as a no-op.
    async fn set_aaaa_record(&self, hostname: &str, ip: Ipv6Addr) -> Result<(), EdgePresenceError>;

    /// Set or update a DNS TXT record — used for ACME DNS-01 challenge.
    /// `name` is the full record name (e.g. `_acme-challenge.myfamily.elohim.host`).
    async fn set_txt_record(&self, name: &str, value: &str) -> Result<(), EdgePresenceError>;

    /// Delete a DNS TXT record — called after ACME DNS-01 challenge completion.
    async fn delete_txt_record(&self, name: &str) -> Result<(), EdgePresenceError>;
}
```

### 5.2 `EdgePresence` orchestrator

```rust
/// Top-level orchestrator. Constructed once, run as a background task.
///
/// The caller retains a `TlsAcceptorHandle` to wrap its TCP listener.
pub struct EdgePresence {
    config: EdgePresenceConfig,
    dns: Arc<dyn DnsProvider>,
    wan_ip: WanIpDiscoverer,
    acme: AcmeManager,
    cert_store: Arc<CertStore>,
}

impl EdgePresence {
    pub fn new(config: EdgePresenceConfig, dns: Arc<dyn DnsProvider>) -> Result<Self, EdgePresenceError>;

    /// Start the background renewal loop. Returns a handle for TLS acceptor access.
    /// This future should be spawned with `tokio::spawn`.
    pub async fn run(self) -> Result<(), EdgePresenceError>;

    /// Return a handle to the live cert store. Callers use this to wire their
    /// TLS acceptor. The handle holds a live `Arc<ServerConfig>` that is
    /// hot-swapped on renewal.
    pub fn tls_handle(&self) -> TlsAcceptorHandle;
}
```

### 5.3 `TlsAcceptorHandle`

```rust
/// A cloneable, live reference to the current `rustls::ServerConfig`.
///
/// Wrap a plain `TcpStream` via `handle.accept(stream).await` to produce a
/// `tokio_rustls::server::TlsStream<TcpStream>` that hyper can use.
///
/// The underlying `ServerConfig` is hot-swapped on cert renewal — no restart needed.
#[derive(Clone)]
pub struct TlsAcceptorHandle {
    inner: Arc<ArcSwap<Arc<rustls::ServerConfig>>>,
}

impl TlsAcceptorHandle {
    pub async fn accept(&self, stream: tokio::net::TcpStream)
        -> Result<tokio_rustls::server::TlsStream<tokio::net::TcpStream>, std::io::Error>;
}
```

### 5.4 `CertStore`

```rust
/// Persistent cert store backed by the local filesystem.
///
/// Paths:
///   - `{base_dir}/account.key`     — ACME account private key (PEM)
///   - `{base_dir}/{hostname}.cert` — Certificate chain (PEM)
///   - `{base_dir}/{hostname}.key`  — Certificate private key (PEM)
///
/// Files are written atomically (temp + rename) to avoid partial reads.
pub struct CertStore {
    base_dir: PathBuf,
}

impl CertStore {
    pub fn open(base_dir: impl Into<PathBuf>) -> Result<Self, EdgePresenceError>;
    pub fn load_cert(&self, hostname: &str) -> Result<Option<CertBundle>, EdgePresenceError>;
    pub fn save_cert(&self, hostname: &str, bundle: &CertBundle) -> Result<(), EdgePresenceError>;
    pub fn load_account_key(&self) -> Result<Option<Vec<u8>>, EdgePresenceError>;
    pub fn save_account_key(&self, key_pem: &[u8]) -> Result<(), EdgePresenceError>;
    pub fn needs_renewal(&self, hostname: &str, threshold_days: u32) -> bool;
}
```

---

## 6. Data flow

```
┌─────────────────────────────────────────────────────────────────────┐
│  EdgePresence::run() — background task, loops every renewal_check_interval │
│                                                                     │
│  1. WAN-IP DISCOVERY                                                │
│     WanIpDiscoverer::discover()                                     │
│       → try cascade: ipify → ifconfig.me → ipinfo.io              │
│       → or: return injected observed-addr (steward/node path)      │
│     Result: current_wan_ip: Ipv4Addr                                │
│                                                                     │
│  2. DNS UPDATE (if ip changed or TTL elapsed)                       │
│     DnsProvider::set_a_record(hostname, current_wan_ip)             │
│     → CloudflareDnsProvider: PUT /zones/{zone_id}/dns_records/{id} │
│     or DuckDnsDnsProvider: GET update URL (future)                 │
│                                                                     │
│  3. CERT CHECK                                                      │
│     CertStore::needs_renewal(hostname, threshold_days=30)           │
│     → if false: skip to step 6                                      │
│     → if true: proceed to ACME                                      │
│                                                                     │
│  4. ACME ISSUANCE / RENEWAL (instant-acme)                         │
│     a. Load or create ACME account key                              │
│     b. Submit Order for hostname                                     │
│     c. For each Authorization → DNS-01 challenge:                  │
│          DnsProvider::set_txt_record(_acme-challenge.{hostname}, token) │
│          Poll until ACME server validates                            │
│          DnsProvider::delete_txt_record(_acme-challenge.{hostname}) │
│     d. Generate CSR via rcgen                                       │
│     e. Finalize Order → download cert chain                         │
│     f. CertStore::save_cert(hostname, bundle)                       │
│                                                                     │
│  5. TLS HOT-RELOAD                                                  │
│     Build new rustls::ServerConfig from saved cert+key              │
│     ArcSwap::store(new_config)                                      │
│     → all new connections use the new cert immediately              │
│     → in-flight connections keep their existing TLS session         │
│                                                                     │
│  6. SLEEP until next check (default: 12 hours)                      │
└─────────────────────────────────────────────────────────────────────┘
```

### 6.1 HTTP-01 challenge flow (optional fallback)

When `AcmeChallenge::Http01` is configured, step 4c changes:

```
Instead of DnsProvider::set_txt_record:
  → Provide a challenge token handler (a Fn(path) -> Option<String> callback)
  → Caller must route GET /.well-known/acme-challenge/{token} to this handler
  → doorway-service wires this via a new match arm in handle_request()
  → Poll until ACME server validates
```

The `EdgePresence` struct exposes `challenge_response(path: &str) -> Option<String>` for
doorway's HTTP handler to call.

---

## 7. Error handling, renewal cadence, and backoff

### 7.1 Error types

```rust
#[derive(Debug, thiserror::Error)]
pub enum EdgePresenceError {
    #[error("WAN IP discovery failed after {attempts} attempts: {last_error}")]
    WanIpDiscoveryFailed { attempts: u32, last_error: String },

    #[error("DNS update failed for {hostname}: {source}")]
    DnsUpdateFailed { hostname: String, source: String },

    #[error("ACME order failed for {hostname}: {source}")]
    AcmeOrderFailed { hostname: String, source: String },

    #[error("ACME DNS-01 challenge validation timed out after {elapsed_secs}s")]
    AcmeChallengeTimeout { elapsed_secs: u64 },

    #[error("Certificate store I/O error: {0}")]
    CertStoreIo(#[from] std::io::Error),

    #[error("TLS configuration error: {0}")]
    TlsConfig(String),

    #[error("HTTP client error: {0}")]
    Http(String),

    #[error("Configuration error: {0}")]
    Config(String),
}
```

### 7.2 Renewal cadence

| Check | Interval | Threshold |
|-------|----------|-----------|
| Renewal check loop | 12 hours | Configurable |
| Certificate renewal trigger | 30 days before expiry | Configurable |
| WAN IP re-check | Every renewal loop iteration | — |
| DNS update | Only if IP changed | TTL-based deduplicate |

### 7.3 Retry and backoff

All external calls (WAN IP discovery, DNS API, ACME) use exponential backoff:
- Base delay: 5 seconds
- Multiplier: 2×
- Max delay: 5 minutes
- Max attempts: 5 (configurable)

ACME DNS-01 challenge polling:
- Poll interval: 10 seconds
- Max wait: 5 minutes (DNS TTL propagation)
- If validation times out: mark order as failed, backoff 1 hour before retry.

Cert renewal failure is non-fatal to the service: the existing cert remains in use.
The orchestrator logs an error and retries on the next check interval. An expiry
warning is logged when `days_remaining < threshold_days`. An expiry error is logged
when `days_remaining <= 0` (the cert is expired; TLS connections will fail).

---

## 8. Consumer integration

### 8.1 `doorway-service` integration

**Today:** plain `TcpListener::bind` in `doorway/doorway-service/src/server/http.rs:849`.

**With this crate (TlsMode::TerminateInProcess):**

```rust
// In doorway-service's server/http.rs run() function:
// Gated by args.tls_mode == TlsMode::TerminateInProcess

let tls_handle = state.edge_presence.as_ref().map(|ep| ep.tls_handle());

loop {
    match listener.accept().await {
        Ok((stream, addr)) => {
            if let Some(ref handle) = tls_handle {
                // TLS path: wrap stream in TLS acceptor
                match handle.accept(stream).await {
                    Ok(tls_stream) => {
                        // proceed with TokioIo::new(tls_stream) as before
                    }
                    Err(e) => { warn!("TLS handshake failed from {addr}: {e}"); continue; }
                }
            } else {
                // Plain HTTP path: unchanged from today (k8s-ingress deployments)
                // TokioIo::new(stream) as before
            }
        }
        // ...
    }
}
```

**Wire-up in `doorway-service/src/main.rs`:**

```rust
// New Args field (CLI flag --tls-mode / DOORWAY_TLS_MODE env var):
// TlsMode::DeferToIngress (default) | TlsMode::TerminateInProcess

if args.tls_mode == TlsMode::TerminateInProcess {
    let dns_provider = Arc::new(
        elohim_edge_presence::dns::cloudflare::CloudflareDnsProvider::new(
            cf_api_token, cf_zone_id
        )
    );
    let cfg = elohim_edge_presence::EdgePresenceConfig {
        hostname: derive_doorway_hostname(),
        admin_email: args.acme_email.clone(),
        tls_mode: TlsMode::TerminateInProcess,
        cert_dir: args.cert_dir.clone().unwrap_or_else(|| PathBuf::from("/var/lib/elohim/certs")),
        ..Default::default()
    };
    let ep = elohim_edge_presence::EdgePresence::new(cfg, dns_provider)?;
    let handle = ep.tls_handle();
    tokio::spawn(ep.run());
    state.edge_presence = Some(handle);
}
```

**`PublicSurfaceState` stub resolution:** `build_public_surface()` in
`doorway/doorway-service/src/services/dashboard_topology.rs` stubs `tls_valid: false`.
Phase 5 replaces the stub body with:

```rust
pub fn build_public_surface(hostname: &str, ep: Option<&TlsAcceptorHandle>) -> PublicSurfaceState {
    let (tls_valid, tls_expires_in_days) = ep
        .and_then(|h| h.cert_expiry())
        .map(|expiry| {
            let days = (expiry - OffsetDateTime::now_utc()).whole_days() as i32;
            (days > 0, Some(days))
        })
        .unwrap_or((false, None));
    PublicSurfaceState { dns_resolves: true, dns_target: None, tls_valid, tls_expires_in_days, public_reachable: true }
}
```

### 8.2 `steward/node` integration

`steward/node/src/dashboard/setup.rs` calls `configure_ddns` and `configure_https`.
Replace the TODO bodies:

```rust
// In configure_ddns():
let dns_provider: Arc<dyn elohim_edge_presence::dns::DnsProvider> = match provider {
    DnsProvider::Cloudflare { api_token, zone_id } =>
        Arc::new(elohim_edge_presence::dns::cloudflare::CloudflareDnsProvider::new(api_token, zone_id)),
    DnsProvider::DuckDns { token, domain } =>
        Arc::new(elohim_edge_presence::dns::duckdns::DuckDnsDnsProvider::new(token, domain)), // future
    // ...
};
APP_STATE.edge_presence_dns = Some(dns_provider);

// In configure_https():
let cfg = elohim_edge_presence::EdgePresenceConfig {
    hostname: config.hostname.clone(),
    admin_email: email.map(String::from),
    tls_mode: TlsMode::TerminateInProcess,
    ..Default::default()
};
let ep = elohim_edge_presence::EdgePresence::new(cfg, APP_STATE.edge_presence_dns.clone().unwrap())?;
tokio::spawn(ep.run());
```

The steward can also inject a peer-observed WAN address (from libp2p identify / iroh
endpoint info) without an external HTTP call:

```rust
ep.inject_observed_wan_ip(observed_addr.ip());
```

---

## 9. Testing strategy

### 9.1 Unit tests (no network)

All unit tests use `RUSTFLAGS=""` (native crate; no Holochain WASM backend).

- `wan_ip.rs`: mock `reqwest` responses (use `wiremock` or a channel-based mock
  `WanIpProvider`). Test cascade logic (first succeeds, first fails + second succeeds,
  all fail).
- `dns/cloudflare.rs`: mock Cloudflare API responses for
  `GET /zones/{id}/dns_records` and `PATCH /zones/{id}/dns_records/{record_id}`.
  Test create-new vs update-existing record paths.
- `acme.rs`: unit-test order state machine transitions against a mock `instant-acme`
  backend. Test DNS-01 token extraction and DNS record lifecycle.
- `tls.rs`: verify `CertStore` atomic write (write + rename), `needs_renewal` threshold
  logic, `ArcSwap` hot-reload (load handle, save new cert, verify handle returns new
  cert on next `accept()`).
- `config.rs`: verify `TlsMode::DeferToIngress` is the default; verify config
  deserialization from env vars.

### 9.2 Integration tests (local mock ACME)

Use [Pebble](https://github.com/letsencrypt/pebble) — Let's Encrypt's test ACME server.
Pebble runs as a Docker container locally and provides:
- A real ACME protocol endpoint with configurable challenge validation.
- A DNS resolver stub that can be pre-loaded with TXT record expectations.

Test scenario (run against `pebble` container):

```
Given a Pebble ACME server running on localhost:14000
And a mock DNS provider that captures TXT record writes
When EdgePresence::run() is called for hostname "test.example"
Then DnsProvider::set_txt_record("_acme-challenge.test.example", token) is called
And the TXT record is present before Pebble validates
And a certificate is issued and stored in CertStore
And TlsAcceptorHandle::cert_expiry() returns a future date
And DnsProvider::delete_txt_record("_acme-challenge.test.example") is called after validation
```

Pebble integration tests are marked `#[ignore]` by default and run in CI with
`cargo test -- --ignored` when `PEBBLE_URL` env var is set.

### 9.3 A2O-style regression scenarios

These scenarios belong in `genesis/a2o/features/elohim/` as Track B regression coverage.

**Scenario: Doorway self-registers stable hostname after WAN IP change**
```gherkin
Given a doorway node is running without k8s ingress
And the node's WAN IP has changed since last DNS check
When the EdgePresence renewal loop fires
Then the Cloudflare A record for the doorway hostname is updated to the new WAN IP
And the doorway remains reachable at its hostname
```

**Scenario: TLS certificate is renewed before expiry**
```gherkin
Given a doorway node with a TLS certificate expiring in 25 days
And the renewal threshold is 30 days
When the EdgePresence renewal loop fires
Then a new ACME order is placed for the hostname
And the DNS-01 challenge TXT record is set and then deleted
And the TlsAcceptorHandle serves the new certificate without a restart
```

**Scenario: TLS termination is skipped for k8s-ingress deployments**
```gherkin
Given a doorway-service deployed behind k8s ingress
And DOORWAY_TLS_MODE is not set (defaults to DeferToIngress)
When the doorway-service starts
Then the listener accepts plain TCP connections on :8080
And no TLS acceptor is initialized
And the startup log contains "TLS mode: DeferToIngress"
```

**Scenario: WAN IP discovery falls back when primary provider is unreachable**
```gherkin
Given the primary WAN IP discovery provider (ipify.org) is unreachable
When the EdgePresence renewal loop fires
Then the discoverer tries the fallback provider (ifconfig.me)
And the WAN IP is successfully discovered from the fallback
And a warning is logged indicating fallback was used
```

---

## 10. What the crate does NOT do

- It does not handle pkarr registration (that is the P2P discovery layer, already
  in `doorway-service`).
- It does not manage NGINX, HAProxy, or any external reverse proxy.
- It does not coordinate certificate issuance across multiple doorways (each doorway
  manages its own cert for its own hostname; no shared cert store).
- It does not provide a web UI for certificate management.
- It does not touch `elohim/brit` or `elohim/rakia` submodules.
- It does not modify any existing crate's `Cargo.toml` (it is a new standalone crate;
  consumers add a `path` dependency when they opt in).

---

## 11. Decisions made autonomously

1. **Crate name `elohim-edge-presence`** rather than `elohim-ddns` or `elohim-acme`. The
   name captures the purpose ("establishing presence at the WAN edge"), not the mechanism.
2. **`instant-acme` over `rustls-acme`** because DNS-01 is first-class in `instant-acme`
   and DNS-01 is the correct challenge type for NAT-traversal scenarios.
3. **DNS-01 as primary challenge type** because households behind ISP NATs frequently
   have port 80 blocked; DNS-01 avoids this entirely.
4. **TlsMode::DeferToIngress as default** — existing k8s deployments are not affected.
   New bare-metal deployments opt in explicitly.
5. **Standalone `Cargo.toml`** in `crates/elohim-edge-presence/` — consistent with the
   `crates/` pattern (each crate is a standalone package; no shared workspace root under
   `crates/`). The crate is added to `doorway-service` and `steward/node` via
   `path = "../../crates/elohim-edge-presence"` when those integrations land.
6. **ArcSwap hot-reload** for cert rotation — no restart needed on renewal. This is the
   correct pattern for a long-running server process.
7. **`async_trait`** for `DnsProvider` — the trait must be object-safe for the
   `Arc<dyn DnsProvider>` pattern; async methods need the attribute until `async fn` in
   traits stabilizes widely in MSRV-relevant contexts.
8. **WAN IP cascade** (ipify → ifconfig.me → ipinfo.io) as the default. No single
   external service is required; all are HTTPS.

---

## 12. Open questions needing operator input

1. **Default cert directory path.** What is the canonical directory for cert/key storage
   on a bare-metal doorway? Proposal: `/var/lib/elohim/edge-presence/` for system
   deployments, `~/.elohim/edge-presence/` for user-space steward. Needs operator
   confirmation.

2. **ACME directory URL.** The crate defaults to Let's Encrypt production
   (`https://acme-v02.api.letsencrypt.org/directory`). Should Let's Encrypt staging
   (`https://acme-staging-v02.api.letsencrypt.org/directory`) be the default for dev
   builds, or should both be config-only with no compile-time default distinction?

3. **DNS propagation TTL for DNS-01.** The crate waits up to 5 minutes for DNS-01
   validation. Cloudflare propagates quickly (~30s) but other providers may be slower.
   Should the propagation wait be per-provider-configurable or is 5 minutes sufficient
   for all anticipated providers?

4. **IPv6 (AAAA record) policy.** The `DnsProvider` trait includes `set_aaaa_record`.
   Should the crate attempt AAAA registration when IPv6 is detected, or is IPv4-only
   sufficient for the current cluster-outage-resilience scope?

5. **Cloudflare API token scope.** The `CloudflareDnsProvider` requires a token with
   `Zone:DNS:Edit` permission for both A record writes and TXT challenge writes. Should
   the crate validate the token's permissions at startup (a test API call) or fail
   lazily at first use? Eager validation gives clearer error messages during setup.

6. **Multi-hostname support.** The current design is single-hostname per `EdgePresence`
   instance. A doorway hosting multiple virtual domains would need multiple instances.
   Is multi-hostname in scope before the Shem failover front-door deployment?

7. **Certificate portability.** If a doorway migrates from one machine to another, can
   the operator copy the cert/key files and skip re-issuance? Yes (the cert belongs to
   the hostname, not the machine). Should the `CertStore` support export/import as a
   documented operation? Relevant for the Shem failover scenario.

---

## 13. Implementation checklist (ordered)

### Phase 1 — Crate skeleton (this PR)
- [x] `Cargo.toml` with placeholder dependencies (versions pinned to what's in the
      repo's dependency tree where available)
- [x] `src/lib.rs` with module declarations
- [x] `src/error.rs` — `EdgePresenceError` with all variants
- [x] `src/config.rs` — `EdgePresenceConfig`, `TlsMode`, `AcmeChallenge`
- [x] `src/wan_ip.rs` — `WanIpDiscoverer` struct, cascade logic (stubs)
- [x] `src/dns/mod.rs` — `DnsProvider` trait
- [x] `src/dns/cloudflare.rs` — `CloudflareDnsProvider` struct + method signatures
- [x] `src/acme.rs` — `AcmeManager` struct + method signatures
- [x] `src/tls.rs` — `CertStore`, `TlsAcceptorHandle` struct + method signatures
- [x] `src/orchestrate.rs` — `EdgePresence` struct + method signatures

### Phase 2 — WAN IP and DNS-01 (implement, no live calls)
- [ ] Implement `WanIpDiscoverer::discover()` with real cascade + backoff
- [ ] Add `MockWanIpDiscoverer` for tests
- [ ] Implement `CloudflareDnsProvider::set_a_record()` using `reqwest`
      (Cloudflare API: `GET /zones/{zone_id}/dns_records?name={hostname}&type=A` to
       find existing record ID, then `PATCH` to update or `POST` to create)
- [ ] Implement `CloudflareDnsProvider::set_txt_record()` and `delete_txt_record()`
- [ ] Unit tests for `CloudflareDnsProvider` against a `wiremock` mock server

### Phase 3 — ACME issuance (implement with Pebble)
- [ ] Add `instant-acme`, `rcgen`, `rustls-pki-types` to `Cargo.toml`
- [ ] Implement `AcmeManager::order()` — full DNS-01 flow
- [ ] Implement `AcmeManager::needs_renewal()` — parse cert expiry from PEM
- [ ] Integration test against Pebble (`#[ignore]` + `PEBBLE_URL` gate)

### Phase 4 — TLS termination + hot-reload
- [ ] Add `tokio-rustls`, `rustls`, `arc-swap` to `Cargo.toml`
- [ ] Implement `CertStore` (atomic write, load, expiry check)
- [ ] Implement `TlsAcceptorHandle` + `ArcSwap<Arc<ServerConfig>>`
- [ ] Implement `EdgePresence::run()` orchestration loop with backoff
- [ ] Unit tests for hot-reload (save cert, verify handle serves new cert)

### Phase 5 — doorway-service integration
- [ ] Add `elohim-edge-presence` path dep to `doorway/doorway-service/Cargo.toml`
- [ ] Add `TlsMode` to `doorway/doorway-service/src/config.rs` `Args`
- [ ] Wire `EdgePresence` startup in `doorway-service/src/main.rs`
- [ ] Retrofit `handle_request` for HTTP-01 challenge route
      (`GET /.well-known/acme-challenge/{token}`)
- [ ] Replace `build_public_surface()` stub in `dashboard_topology.rs`
- [ ] E2E test: doorway starts with `DOORWAY_TLS_MODE=TerminateInProcess`,
      obtains cert from Pebble, serves HTTPS, `PublicSurfaceState.tls_valid = true`

### Phase 6 — steward/node integration
- [ ] Add `elohim-edge-presence` path dep to `steward/node/Cargo.toml`
- [ ] Replace `configure_ddns` / `configure_https` TODO bodies in `setup.rs`
- [ ] Wire `inject_observed_wan_ip` from libp2p identify events
- [ ] Sweettest (manual, not automated): steward setup flow obtains cert from Pebble

### Phase 7 — a2o scenarios
- [ ] Write a2o scenarios (§9.3) to `genesis/a2o/features/elohim/track-b-edge-presence.feature`
- [ ] Wire Track B success criterion #5 to a scenario assertion
