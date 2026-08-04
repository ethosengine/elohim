---
title: "Holochain/iroh dependency verification pack — dalek, bootstrap wire, and transport config"
id: holochain-iroh-dep-verification-pack
type: history-gotcha
status: noted
tier: history
created: 2026-08-04
author: Codex (Holochain/iroh convergence campaign, Wave 1 Lane D)
topic:
  [
    holochain,
    iroh,
    kitsune2,
    bootstrap,
    curve25519-dalek,
    dependency-verification,
  ]
cites:
  - holochain-iroh-convergence-upgrade-campaign | the governing campaign whose Wave 1 Task D1 this primary-source verification pack closes and whose Wave 2 transport flip consumes these verdicts | sha256:b61c697ad5814c52 | path: genesis/docs/superpowers/plans/2026-08-04-holochain-iroh-convergence-upgrade-campaign.md
  - genesis/data/timeline/backlog/2026-08-04-holochain-iroh-dep-verification-pack.md
---

# Holochain/iroh dependency verification pack

Read-only Task D1 research, using upstream tags and commits as the primary source. The three
headline verdicts are:

| Question                                                                           | Verdict                                                             | Short answer                                                                                                                                      |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Did `curve25519-dalek 5.0.0` final fix the `digest::crypto_common` source bug?     | **CONFIRMED**                                                       | The final tag imports `digest::common::BlockSizeUser`; commit `59305b4e` made the exact correction.                                               |
| Was there no bootstrap-wire change from kitsune2 `0.4.0-dev.2` to `0.4.1`?         | **REFUTED as an absolute claim; core wire compatibility CONFIRMED** | Core agent-info PUT/GET is unchanged and our server remains compatible, but `0.4.1` adds authentication-pending semantics and relay registration. |
| Are `signal_url` and `relay_url` separate tx5/iroh inputs under Holochain `0.6.3`? | **CONFIRMED**                                                       | `signal_url` and `webrtc_config` configure tx5 only; `relay_url` and relay auth configure iroh. Bootstrap auth is separate again.                 |

## 1. `curve25519-dalek 5.0.0` published-source fix

### Verdict: **CONFIRMED**

The final `5.0.0` source fixes the path mismatch that existed in `5.0.0-pre.1`.

Primary evidence:

- The `5.0.0-pre.1` tag (`cf1c77de`, 2025-09-04) imports
  `crypto_common::BlockSizeUser` from `digest` in
  [`curve25519-dalek/src/edwards.rs` lines 107-111](https://github.com/dalek-cryptography/curve25519-dalek/blob/cf1c77de6e26b0b6ad51cbe921f00640723596d5/curve25519-dalek/src/edwards.rs#L107-L111).
- Upstream commit [`59305b4e` — “Update digest and sha2 deps”](https://github.com/dalek-cryptography/curve25519-dalek/commit/59305b4e26e05710e08c3a0115cf4d17a5bd2045)
  changes that exact import from `crypto_common::BlockSizeUser` to
  `common::BlockSizeUser`.
- The final `5.0.0` tag (`07bef73f`, 2026-07-06) contains the corrected
  [`digest::common::BlockSizeUser` import](https://github.com/dalek-cryptography/curve25519-dalek/blob/07bef73ff85998a206cd2cea7f2605c801d0d1c9/curve25519-dalek/src/edwards.rs#L107-L111),
  depends on stable [`digest = "0.11"`](https://github.com/dalek-cryptography/curve25519-dalek/blob/07bef73ff85998a206cd2cea7f2605c801d0d1c9/curve25519-dalek/Cargo.toml#L48-L54),
  and records the `digest`/`sha2` upgrade in the
  [`5.0.0` changelog](https://github.com/dalek-cryptography/curve25519-dalek/blob/07bef73ff85998a206cd2cea7f2605c801d0d1c9/curve25519-dalek/CHANGELOG.md#L8-L20).

Concrete implication: the published-source defect that justified freezing storage on
`iroh =0.92` is gone from the final dalek release. This answers only source correctness; it does
not override the campaign's later serde-family interlock. The iroh lift still belongs in Wave 3's
Holochain-family move and still requires the prescribed full `cargo test` evidence.

## 2. kitsune2 bootstrap wire, `0.4.0-dev.2` to `0.4.1`

### Verdict: **REFUTED as “no change at all”; core agent-info compatibility CONFIRMED**

The core bootstrap protocol implemented by Doorway did not change, but the broader bootstrap
server/client surface did. Treating the whole surface as byte-for-byte unchanged would therefore
be false.

### Provenance correction: the upstream Holochain `0.6.0` tag does not consume `0.4.0-dev.2`

The question's version association is not borne out by the upstream Holochain tags:

- Holochain `0.6.0` commit `a6d4e805` declares `kitsune2_api`, `kitsune2_core`, and
  `kitsune2_bootstrap_srv` as `0.3.0` in
  [`crates/holochain/Cargo.toml`](https://github.com/holochain/holochain/blob/a6d4e805a0971ccbc0dcb3f3ed6a9e2fac980a3b/crates/holochain/Cargo.toml#L53-L54)
  and [line 107](https://github.com/holochain/holochain/blob/a6d4e805a0971ccbc0dcb3f3ed6a9e2fac980a3b/crates/holochain/Cargo.toml#L107).
  Its lockfile resolves the bootstrap client/server to
  [`0.3.2`](https://github.com/holochain/holochain/blob/a6d4e805a0971ccbc0dcb3f3ed6a9e2fac980a3b/Cargo.lock#L4094-L4113).
- Holochain `0.6.3` commit `448a36ef` declares the kitsune2 family, including both transports and
  bootstrap server, as
  [`0.4.1`](https://github.com/holochain/holochain/blob/448a36efe8c4a95b7f3c6ccca9858d8275ed0a71/crates/holochain_p2p/Cargo.toml#L44-L49).
- `0.4.0-dev.2` is nevertheless relevant to this repository: it is the version in the newer
  client-crate graph at `doorway/doorway-service/Cargo.lock:3150-3192`. The requested
  kitsune2 `v0.4.0-dev.2` to `v0.4.1` comparison is completed below; it just must not be
  described as the upstream Holochain `0.6.0` conductor's dependency transition.

### What remained stable

Across kitsune2 commits `77471c1f` (`v0.4.0-dev.2`) and `b33fa556` (`v0.4.1`):

- Client PUT remains `PUT /bootstrap/{spaceB64Url}/{agentB64Url}`, with base64url-no-pad path
  segments and `AgentInfoSigned::encode()` as the request body. Compare
  [`v0.4.0-dev.2` lines 125-183](https://github.com/holochain/kitsune2/blob/77471c1fb4b6f926609bf82152dc97e26457b4d9/crates/bootstrap_client/src/lib.rs#L125-L183)
  with
  [`v0.4.1` lines 144-216](https://github.com/holochain/kitsune2/blob/b33fa55606f8bd5020cffacf1ffae41f92d4d296/crates/bootstrap_client/src/lib.rs#L144-L216).
- Client GET remains `GET /bootstrap/{spaceB64Url}` and still passes the response bytes to
  `AgentInfoSigned::decode_list`. Compare
  [`v0.4.0-dev.2` lines 191-260](https://github.com/holochain/kitsune2/blob/77471c1fb4b6f926609bf82152dc97e26457b4d9/crates/bootstrap_client/src/lib.rs#L191-L260)
  with
  [`v0.4.1` lines 289-386](https://github.com/holochain/kitsune2/blob/b33fa55606f8bd5020cffacf1ffae41f92d4d296/crates/bootstrap_client/src/lib.rs#L289-L386).
- The reference server keeps the same two route patterns. Compare
  [`v0.4.0-dev.2` lines 230-237](https://github.com/holochain/kitsune2/blob/77471c1fb4b6f926609bf82152dc97e26457b4d9/crates/bootstrap_srv/src/http.rs#L230-L237)
  with
  [`v0.4.1` lines 280-287](https://github.com/holochain/kitsune2/blob/b33fa55606f8bd5020cffacf1ffae41f92d4d296/crates/bootstrap_srv/src/http.rs#L280-L287).
- The signed agent-info wire type is literally the same Git blob at both tags:
  `crates/api/src/agent.rs` resolves to blob `f2c5f3fa87fd3a5cd65b0a6488226e5add434010` in each.
  The `v0.4.1` source shows the outer `agentInfo`/`signature` envelope and
  `decode_list` implementation at
  [`agent.rs` lines 180-258](https://github.com/holochain/kitsune2/blob/b33fa55606f8bd5020cffacf1ffae41f92d4d296/crates/api/src/agent.rs#L180-L258).

Doorway matches those stable parts:

- The routes are dispatched at `doorway/doorway-service/src/server/http.rs:4672-4681` and handled
  at `:5296-5385`.
- `doorway/doorway-service/src/bootstrap/k2.rs:104-165` validates path/body identity and the
  signature; `:275-323` parses the unchanged outer and inner JSON fields; `:210-227` returns the
  stored PUT bodies as the JSON list expected by `decode_list`.

### What changed

Two wire-adjacent additions landed before `0.4.1`:

1. Authentication can now return HTTP `202 Accepted` to mean “credentials received, approval
   pending”; the client turns that into a specific failure rather than trying to parse a token.
   See [`bootstrap_client/src/lib.rs` lines 57-84](https://github.com/holochain/kitsune2/blob/b33fa55606f8bd5020cffacf1ffae41f92d4d296/crates/bootstrap_client/src/lib.rs#L57-L84)
   and [`bootstrap_srv/src/http.rs` lines 487-516](https://github.com/holochain/kitsune2/blob/b33fa55606f8bd5020cffacf1ffae41f92d4d296/crates/bootstrap_srv/src/http.rs#L487-L516).
2. Authenticated iroh relay operation adds `PUT /relay/register` with a 32-byte endpoint public
   key and bearer token. See the client contract at
   [`bootstrap_client/src/lib.rs` lines 219-282](https://github.com/holochain/kitsune2/blob/b33fa55606f8bd5020cffacf1ffae41f92d4d296/crates/bootstrap_client/src/lib.rs#L219-L282)
   and server route at
   [`bootstrap_srv/src/http.rs` lines 305-326](https://github.com/holochain/kitsune2/blob/b33fa55606f8bd5020cffacf1ffae41f92d4d296/crates/bootstrap_srv/src/http.rs#L305-L326).

Doorway's custom k2 server does not implement `/authenticate`, bearer-token checking, or
`/relay/register`. That does **not** break its present unauthenticated role as a core bootstrap
server. It does impose a Wave 2 boundary: leave `base64_auth_material_bootstrap` unset when using
this server, and do not assume that pointing iroh `relay_url` at this Doorway endpoint turns it
into an authenticated iroh relay. An authenticated self-hosted relay needs the upstream integrated
relay surface or equivalent endpoints in a deliberately separate implementation.

## 3. `signal_url` versus `relay_url` under Holochain `0.6.3` iroh transport

### Verdict: **CONFIRMED**

The fields coexist in the YAML because `NetworkConfig` represents both transport builds. They do
not feed the same transport.

### Holochain mapping

Holochain `0.6.3` defines separate bootstrap and relay auth fields, then separate tx5 and iroh
URLs in
[`NetworkConfig` lines 276-336](https://github.com/holochain/holochain/blob/448a36efe8c4a95b7f3c6ccca9858d8275ed0a71/crates/holochain_conductor_api/src/config/conductor.rs#L276-L336).
Its `to_k2_config` mapping is explicit:

| Holochain YAML field                     | Kitsune2 destination                                | Semantics with `transport-iroh`                                                                              |
| ---------------------------------------- | --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `network.bootstrap_url`                  | `coreBootstrap.serverUrl`                           | Live and transport-independent; selects the agent-info discovery server.                                     |
| `network.base64_auth_material_bootstrap` | `Builder.auth_material_bootstrap`                   | Live only for bootstrap authentication; causes `/authenticate` before bootstrap GET/PUT when set.            |
| `network.relay_url`                      | `irohTransport.relayUrl`                            | Live; selects the explicit iroh home relay.                                                                  |
| `network.base64_auth_material_relay`     | `Builder.auth_material_relay`                       | Live for iroh relay registration; with an explicit relay URL, causes `/authenticate` then `/relay/register`. |
| `network.signal_url`                     | `tx5Transport.serverUrl`                            | tx5-only leftover; not read by the iroh transport.                                                           |
| `network.webrtc_config`                  | `tx5Transport.webrtcConfig`                         | tx5-only leftover; not read by the iroh transport.                                                           |
| `network.request_timeout_s`              | `tx5Transport.timeoutS` and `webrtcConnectTimeoutS` | The top-level convenience field configures tx5 timeouts, not iroh's connection timeout.                      |

The direct mappings are visible at
[`conductor.rs` lines 500-568](https://github.com/holochain/holochain/blob/448a36efe8c4a95b7f3c6ccca9858d8275ed0a71/crates/holochain_conductor_api/src/config/conductor.rs#L500-L568).
That method clones `network.advanced` first and then inserts the top-level convenience fields, so
top-level `relay_url` wins over an `advanced.irohTransport.relayUrl` value; the other advanced iroh
knobs remain available.
The two auth values are decoded and passed separately into Holochain P2P at
[`builder.rs` lines 214-232](https://github.com/holochain/holochain/blob/448a36efe8c4a95b7f3c6ccca9858d8275ed0a71/crates/holochain/src/conductor/conductor/builder.rs#L214-L232),
then assigned to the Kitsune builder at
[`actor.rs` lines 496-526](https://github.com/holochain/holochain/blob/448a36efe8c4a95b7f3c6ccca9858d8275ed0a71/crates/holochain_p2p/src/spawn/actor.rs#L496-L526).

The Holochain field comments say “base64 url-safe, with no padding,” but the `0.6.3` builder code
uses `base64::prelude::BASE64_STANDARD.decode` for both values at the linked lines. That is the
actual implementation contract in this release; auth material containing URL-safe-only `-` or `_`
must not be assumed to decode without a targeted test or upstream correction.

### What `kitsune2_transport_iroh 0.4.1` actually consumes

The iroh module's camelCase `network.advanced.irohTransport` fields are defined at
[`transport_iroh/src/lib.rs` lines 241-305](https://github.com/holochain/kitsune2/blob/b33fa55606f8bd5020cffacf1ffae41f92d4d296/crates/transport_iroh/src/lib.rs#L241-L305):

- `relayUrl`: optional explicit home relay. If absent at the raw kitsune2 layer, iroh uses n0's
  default relays.
- `relayAllowPlainText`: must be true for `http://` relay URLs.
- `maxFrameBytes`: endpoint-wide maximum frame size; default 100 MiB.
- `connectTimeoutS`: endpoint-wide peer connection timeout; default 60 seconds.
- `authMaterialRelayBase64`: per-space relay-registration auth override; ignored in global config.

At global creation the factory reads that iroh config, normalizes the relay URL, and separately
pulls `Builder.auth_material_relay` at
[`transport_iroh/src/lib.rs` lines 321-370](https://github.com/holochain/kitsune2/blob/b33fa55606f8bd5020cffacf1ffae41f92d4d296/crates/transport_iroh/src/lib.rs#L321-L370).
With both an explicit relay and relay auth present, it authenticates, registers the endpoint key,
then inserts the relay; without an explicit relay it uses `RelayMode::Default`. See
[`lines 435-530`](https://github.com/holochain/kitsune2/blob/b33fa55606f8bd5020cffacf1ffae41f92d4d296/crates/transport_iroh/src/lib.rs#L435-L530).

By contrast, tx5 owns `server_url`, `webrtc_config`, WebRTC connection timeout, ICE configuration,
and signal-relay fallback in
[`transport_tx5/src/lib.rs` lines 60-146](https://github.com/holochain/kitsune2/blob/b33fa55606f8bd5020cffacf1ffae41f92d4d296/crates/transport_tx5/src/lib.rs#L60-L146)
and [`lines 301-323`](https://github.com/holochain/kitsune2/blob/b33fa55606f8bd5020cffacf1ffae41f92d4d296/crates/transport_tx5/src/lib.rs#L301-L323).
Kitsune2's production builder selects iroh when the tx5 feature is absent and `transport-iroh` is
present, at
[`crates/kitsune2/src/lib.rs` lines 21-27](https://github.com/holochain/kitsune2/blob/b33fa55606f8bd5020cffacf1ffae41f92d4d296/crates/kitsune2/src/lib.rs#L21-L27)
and
[`60-66`](https://github.com/holochain/kitsune2/blob/b33fa55606f8bd5020cffacf1ffae41f92d4d296/crates/kitsune2/src/lib.rs#L60-L66).
Holochain P2P `0.6.3` defaults to `transport-iroh` at
[`Cargo.toml` lines 73-94](https://github.com/holochain/holochain/blob/448a36efe8c4a95b7f3c6ccca9858d8275ed0a71/crates/holochain_p2p/Cargo.toml#L73-L94).

### Wave 2 implications

1. Keep `bootstrap_url` pointed at the current Doorway k2 service; that discovery plane remains
   independent of the transport flip.
2. Set `relay_url` explicitly to the ratified self-hosted iroh relay. Holochain `0.6.3` defaults it
   to n0's public canary relay, so merely deleting the old tx5 settings does not establish the
   desired deployment posture.
3. `signal_url`, `webrtc_config`, and the current coturn/SBD settings can remain during a reversible
   dual-mode period, but they are inert in an iroh-only binary. Their presence does not configure an
   iroh relay.
4. Put iroh tuning under `network.advanced.irohTransport` using the camelCase field names above;
   `request_timeout_s` is not the iroh connection-timeout knob.
5. Leave both auth fields unset when composing the current unauthenticated Doorway bootstrap with
   an open relay. If relay authentication is enabled, `base64_auth_material_relay` requires a relay
   base that serves both `/authenticate` and `/relay/register`; bootstrap auth remains a separate
   decision and is not required by iroh itself.

## Reproduction notes

The source comparisons used:

```text
dalek tags:    curve25519-5.0.0-pre.1 cf1c77de → curve25519-5.0.0 07bef73f
kitsune2 tags: v0.4.0-dev.2 77471c1f → v0.4.1 b33fa556
holochain:     holochain-0.6.0 a6d4e805; holochain-0.6.3 448a36ef
```

No secondary articles or generated summaries were used as evidence.
