# lvi — Devspace Peer-Runtime Roadmap

> **Framing:** This is a **high-level roadmap**, not an executable TDD plan. Each milestone below
> is decomposed into its own dated, bite-sized plan (`docs/plans/YYYY-MM-DD-<milestone>.md`) **at
> pickup** — the brit/rakia spec → plan → phase cadence. Do not implement directly from this file.
> Governing spec: `docs/specs/2026-07-20-elohim-native-devspace-design.md`.

**Goal:** Build lvi — the Elohim Protocol devspace peer-runtime — from a walking skeleton (a
one-click, doorway-projected openvscode-server on a real household node) toward the full
"k8s-powers-over-p2p" vision, one honestly-scoped slice at a time.

**Architecture:** lvi composes four shipped substrates — brit (covenant + source-closures), rakia
(build), eprfs (mount/materialize), doorway (ingress) — and adds only the composition layer: the
`DevspaceSeed` runtime shape, the COLD→WARM→reap→rematerialize lifecycle, sandboxed process
supervision, and doorway-registration glue. The one genuinely net-new subsystem is the **actuator**
(process supervision + containment over a live eprfs mount).

**Tech stack:** Rust (cargo workspace, tokio) · eprfs `ProjectionManifest`/`LocalMaterializer` ·
brit `NodeSeed`/`BritCid` · steward/node `pod` (`ActionKind`) · doorway `HostRegistry`/proxy ·
Mishpat `delegates-compute` + `bounds_validator` · podman-rootless · openvscode-server · a2o
(Gherkin) scenarios · `pnpm look` for URL verification.

## Global Constraints

*Every milestone's work implicitly includes these (verbatim from the spec + `CLAUDE.md`).*

- **Native build hygiene:** `RUSTFLAGS=""`; `CARGO_TARGET_DIR` set to the cargo-pool slot; plain
  cargo (lvi is native, no WASM getrandom flag).
- **Self-contained toward submodule:** everything lvi needs lives inside `elohim/lvi/`; designed to
  graduate to `ethosengine/lvi`.
- **Compose, never re-implement:** consume brit/rakia/eprfs/doorway; the `elohim/.epr-meta`
  interface-first-reuse gate + lvi's four `.epr-meta` invariants are binding.
- **Co-resident safety is the non-negotiable floor:** the sandbox hard quota (`--memory --cpus
  --pids-limit`, disk ceiling, `--network=none` default) must isolate a devspace from the host's
  live conductor/DHT participant.
- **Admit ≠ contain ≠ bound:** trust-graph admits, sandbox contains, TTL bounds. Revocation is
  **lease-expiry, not interrupt**.
- **Mount, don't ship** · **EPR Derivation** (input-address for dedup, output-diverge for trust) ·
  **authorization = `delegates-compute` + copy-of-`replicates-commons` `bounds_validator`** ·
  **no derived bytes in commons** (source→`NodeSeed`, caches→lease, stateful→declared+verified).
- **Reach-as-promotion** is the north star; its convergence mechanism is a deferred track, never a
  v1 obligation.
- **Confirmed operator calls:** Q1 — accept the artifact-cache v1 loss. Q2 — household-trust: a
  bounded-TTL run of a trusted peer's unvetted code, contained by quota, no artifact review. Q3 —
  placement-fixed v1; placement later as a market (offer→bid→pick), **never** a master-electing
  scheduler.
- **Commit-only:** commit on the shift branch; the integrator is the single push/merge authority.
- **P2P Design Gate: PASSED** (full classification in the spec's *P2P Design Gate* section) — lvi
  creates **zero new Holochain entry types**; every route (`/lvi/*`, `/devspace/{id}`) projects
  *operational* running-instance state or serves a *content-addressed* seed, authorized by the
  notarized `delegates-compute` commitment. Routes follow the DHT design; they never precede it.

---

## Milestone map (dependency-ordered)

The roadmap follows the **onboarding & graduation ladder** (see the spec section of the same name).
**Operator scope decision (2026-07-21): v1's definition of done is the web-convenience headline —
reach your devspace from a library computer through a corporate firewall (M4).** That makes the full
M2→M3→M4 span the v1 scope, not a stop at device-local. The identity/auth spine is consumed wholesale
from the doorway at every rung; lvi builds only the named devspace delta.

Because the **WebRTC browser↔blade tunnel is the single biggest lift and *gates* M4**, it is
de-risked EARLY as a spike (**S1**), in parallel with the eprfs proof — before the actuator path is
built on top of an unproven transport.

```
            ┌─ S1: WebRTC transport spike ─┐   (gating de-risk, runs early)
  M0 ──▶ M1 ┤                              ├──▶ M2 ──────▶ M3 ────────▶ M4   = v1  (DoD: firewall access)
 scaffold eprfs                                 actuator +   auth +       household-rack
          proof                                 containment  flywheel     backend + WebRTC
                                                (Rung 1)     (devspace     firewall access
                                                             OAuth+bind)   (Rung 3)
   ┊──▶ Horizon: H1 dep-substrate · H2 cross-peer placement-market · H3 reach-promotion · H4 R4 meter · H5 untrusted tier · H6 generalized Rung-3 self-service
```

**v1 = M0 → M1 (+ S1) → M2 → M3 → M4**, finishing at the firewall-access headline. M2 (actuator +
containment) and M3 (auth + flywheel) are the necessary build-up to M4, proven at `localhost` first
before the WebRTC/rack topology is layered on. The Horizon is the named north-star; each becomes real
only when its precondition lands (some need `shem`/cross-DHT, out of the household floor).

---

## M0 — Module scaffold

**Goal:** `elohim/lvi/` is a real, building, self-contained cargo workspace with its module identity
in place.

**Workstreams**
- Cargo workspace (`elohim/lvi/Cargo.toml`) with member stubs: `lvi-core`, `lvi-seed`,
  `lvi-actuator`, `lvi-ingress`, `lvi-cli` (following the `rakia-*`/`eprfs-*` convention).
- `README.md` — the module manifesto in the brit/rakia voice (the love letter; the lions; the
  homage; what lvi is and is not).
- Build/CI wiring: `build-manifest.json` (watch paths, gate projects) and the matching `run_gate`
  fallback `case` so a push touching lvi doesn't hit the `*) Unknown project` abort.
- Path-dep wiring for the sibling crates lvi will consume (workspace deps on eprfs/brit crates;
  Dockerfile `COPY` + manifest watch-path implications noted for later edge work).

**Exit criteria:** `cargo build` + `cargo fmt --check` + `cargo clippy -- -D warnings` green on the
empty workspace (against the pool `CARGO_TARGET_DIR`); the module README + `CLAUDE.md` + `.epr-meta`
cohere; a no-op push detection maps lvi to a known gate.

**Note:** M0 is the natural "lvi module born" commit — it folds in the already-planted `CLAUDE.md`,
`.epr-meta`, spec, and this roadmap.

---

## M1 — Slice 0: the eprfs proof (zero stubs; provable on shipped code)

**Goal:** Prove **COLD → WARM → teardown → rematerialize + mutable-state** end-to-end using only
shipped eprfs machinery — no actuator, no `delegates-compute` executor, no doorway projection. This
de-risks the Bet-4 spine (the most-available win) before any net-new subsystem.

**Workstreams**
- `lvi-seed`: define the `DevspaceSeed` shape (extend eprfs `ProjectionManifest` with the `runtime`
  + `bounds` fields per the spec). The **first real design task inside it** is the concrete
  toolchain-marker-blob encoding — how a seed names "obtain the Rust toolchain" without Nix and
  without a Dockerfile (the EPR-Derivation shape, hand-authored inputs for v1).
- `lvi-core`: the lifecycle state machine (COLD/HYDRATING/WARM/REAPING/TORN-DOWN) as pure types +
  transitions (no process spawn yet).
- The proof itself: author a `DevspaceSeed`-shaped manifest (repo + toolchain marker blobs),
  custody-commit it, then across two local eprfs stores: `Sparse` → `FetchMissing` → edit → reap-seal
  the edit to a brit `NodeSeed` → discard → rematerialize from the **same manifest CID** → assert the
  edit survived with `verify_projection` showing **zero drift**.

**Exit criteria:** an integration test exercising the full round-trip is green in CI on shipped
eprfs/brit code; the `DevspaceSeed` toolchain-encoding decision is documented in the M1 plan;
zero-drift `verify_projection` on rematerialize is asserted, not assumed.

**Risk to measure (do not assert away):** first-warm latency and the receiving device's disk ceiling
on a realistic (multi-GB) toolchain closure — instrument it here, report the numbers.

---

## S1 — Spike: the WebRTC browser↔peer transport (the gating de-risk)

**Goal:** Prove — *before* building the actuator path on top of it — that a plain browser can reach a
peer-hosted HTTP/WS server through a firewall over a doorway-signaled WebRTC data channel. This is the
single biggest lift and it gates M4; spike it early and independently. Runs in parallel with M1.

**Workstreams**
- Stand up a throwaway HTTP/WS server (a stand-in for openvscode-server) on a peer.
- Use the doorway's *existing* WebRTC **signal** function to broker a browser↔peer connection.
- Build the minimal **client shim** that tunnels HTTP/WS over the WebRTC data channel.
- Test from a genuinely foreign network (outbound-only, behind a firewall); measure direct-vs-relay
  traversal.

**Exit criteria:** a browser on a foreign network loads a page + holds a WebSocket from a peer-hosted
server, tunneled over doorway-signaled WebRTC, with measured connection-establishment latency and a
working relay-of-last-resort fallback. **If this cannot be made to work acceptably, M4's topology is
reconsidered before M2/M3 are built on it** — that is the whole point of spiking it first.

**Note:** S1 is transport-only — no devspace, no actuator, no auth. It isolates the one unknown that
gates all of v1.

---

## M2 — Rung 1: device-local (conductor + IDE co-resident + containment) — the walking skeleton

**Goal:** On your own device, one-click a devspace: the IDE co-launches with your conductor,
sandboxed, accessed at `localhost` — no doorway, no WebRTC, no remote auth — and prove it cannot
harm your co-resident conductor. This is "an IDE that launches with a conductor on someone's
device," the cheapest real proof, riding the SHIPPED `steward/device` pattern.

**Workstreams**
- `spawn_devspace_actuator()` — ~150 lines mirroring `steward/device`'s `spawn_storage_sidecar()`:
  spawn-if-not-healthy, Tauri-managed child, non-fatal degrade, gated on `holochain://setup-completed`.
- `lvi-actuator`: pod `ActionKind::Devspace{Materialize, HealthCheck, Reap}` — `LocalMaterializer::
  FetchMissing` against a minimal `DevspaceSeed`, podman-rootless spawn of openvscode-server under
  supervision, TTL-reap into M1's edit-seal.
- `lvi-core` sandbox profile: the derived `reach → podman-flags` **hard quota** (`--memory --cpus
  --pids-limit`, disk ceiling, `--network=none` default) — the co-resident safety floor.
- Authorization: one hand-authored `delegates-compute` `Mishpat::Commitment` authorizing *your own*
  devspace, validated by the **copy-of-`replicates-commons`** validator (reuse the 7 `bounds_validator`
  checks).

**Exit criteria — the three things M2 must prove:**
1. **One-click warm in (measured) seconds** at `localhost` — sparse mount; editor interactive before
   the toolchain finishes hydrating.
2. **Co-resident safety** — an a2o **containment scenario**: an adversarial build (fork-bomb /
   disk-filler / memory-hog) is quota-killed while *your* conductor's health probe stays green.
   *The load-bearing regression guard; runs on a real device, not a stub.*
3. **Mutable state survives teardown** — TTL-reap seals to a CID that rematerializes with zero
   `verify_projection` drift.

Plus: gates green (lint/fmt/clippy on the touched tree).

---

## M3 — The flywheel: conductor-less try-it through a doorway (auth spine reused wholesale)

**Goal:** A conductor-holder invites a conductor-less person *through a doorway* to try a devspace,
zero-install. The onboarding/auth spine is consumed wholesale (custodial keys + chaperone
`POST /hc/connect` + `AgentProvisioner` onto a pooled conductor); lvi adds only the devspace client
+ binding + a pooled try-it devspace.

**Workstreams**
- **Devspace OAuth client** — register one `client_id` in the doorway registry, but **with PKCE +
  real consent, NOT `trusted:true`** (it fronts a live shell with filesystem + secret access). This
  is the one place lvi must not inherit elohim-app's lower-stakes posture.
- **Per-devspace auth binding** — a token scoped to the *owning* devspace, so an authenticated user
  cannot reach *another* user's shell (`isAuthenticated()` alone will not stop that).
- Reuse `identityGuard`/`sessionOrAuthGuard` on `/lvi/*` routes; reuse the chaperone + provisioner
  for the conductor-less agent; a **pooled try-it `DevspaceSeed`** under containment.

**Exit criteria:** a conductor-less browser user, invited through a doorway, gets a working try-it
devspace with no install, authed via the *reused* OAuth flow; a security test proves the auth
binding blocks reaching another user's shell; containment holds for the pooled devspace.

---

## M4 — Rung 3: household-rack backend + firewall access (the web-convenience headline)

**Goal:** Reach *your* devspace from any browser — a library computer through a corporate firewall.
**Thin doorway (auth + WebRTC signal) + household-rack-hosted compute (contracted per-blade) +
browser↔blade WebRTC transport.** The doorway never proxies the shell.

**Workstreams**
- **Household-scoped per-blade contracting:** a `delegates-compute` commitment allocates a blade in
  the steward's household rack; "which blade" is a household-internal placement decision (the near-
  term, tractable placement problem — cross-peer market stays deferred, H2).
- **WebRTC transport:** the doorway *signals* a browser↔blade WebRTC connection (its existing signal
  function); a **client-side shim tunnels openvscode HTTP/WS over the data channel**; a thin
  relay-of-last-resort only when direct traversal fails. The doorway auth-gates the handshake before
  signaling.

**Exit criteria:** from a browser on a foreign network behind a firewall, authenticate via the
peer-native login, reach a devspace hosted on a household-rack blade over WebRTC, and edit in
openvscode. Measure connection-establishment latency + direct-vs-relay traversal success.

**NOT this milestone:** the *generalized self-service* foreign-browser→household-node handoff and
source-chain graduation-migration. lvi proves the household-rack case; the generalization is a
doorway epic (Horizon H6) — build once, push back to the doorway.

---

## Horizon — the named north-star (deferred; each its own future spec+plan)

These are explicitly **out of the walking-skeleton scope**. Listed so the vision stays legible and
nothing here gets smuggled into v1.

| # | Milestone | What it unlocks | Precondition / note |
|---|---|---|---|
| **H1** | **Track B — dependency substrate** | Replace GitHub (source) + Nexus (coordinate); cache-accelerate the artifact layer. brit `BritCid`→CIDv1 migration, `DevspaceSeed` source-closure resolution (build resolves source entirely from the p2p blob plane, zero web2 hits), then `rakia-executor` + peer-dispatch. | The multi-quarter piece. Q1 (accept cache v1 loss) keeps the *source* plane small and shippable ahead of the executor. |
| **H2** | **Placement-as-market** | Cross-peer hosting: offer→bid→pick over gossipsub. | **Never** a master-electing scheduler (capture smell). Needs gossipsub wired; degrades to "the one peer that answered." |
| **H3** | **Reach-as-promotion / output-convergence** | Reproducibility-as-reach: N diverse stewards build, converge output-CIDs, graduate reach (trusted→community→commons). *The inversion web2 can't make.* | Depends on H1 (`rakia-executor` must exist to converge). The founding differentiator's mechanism — not v1. |
| **H4** | **Realized-compute metering (REA R4)** + interrupt-grade revocation | "Economically coherent" (a real meter, not a grant); SIGKILL-on-signal (a real interrupt, not lease-expiry). | Likely needs `shem`/cross-DHT rollup — out of the household floor. |
| **H5** | **Untrusted-tenant isolation tier** + dynamic multi-tenant ingress | Host code from beyond the trust boundary; many devspaces, dynamic ports. | firecracker/microVM tier; dynamic TTL ingress registry + full port ACL. |

---

## Risks & how each is retired

| Risk | Retired by |
|---|---|
| **The WebRTC browser↔blade tunnel (the M4 gating lift) proving infeasible or too slow through real firewalls** | **Spiked EARLY in S1, independently of the actuator** — if it fails or is too slow, M4's topology is reconsidered *before* M2/M3 are built on it. The #1 v1 risk. |
| Cold-start latency / disk ceiling on multi-GB closures | Measured in M1, M2, and M4 exit criteria — reported as numbers, never asserted. |
| A devspace endangering the host's live protocol participation | M2 containment scenario (the non-negotiable floor); the `.epr-meta` co-resident-safety inject reminds every sandbox edit. |
| Re-implementing what a sibling owns (CID/codec/image/build) | `elohim/.epr-meta` interface-first-reuse + lvi's four invariants fire at edit-time. |
| Over-claiming "replaces Harbor" / "economically coherent" | Language discipline from the verdicts baked into the spec + CLAUDE.md; H1/H4 keep the honest scope. |
| Scope creep (placement/metering smuggled into v1) | Horizon table draws the line explicitly; each Horizon item needs its own spec+plan. |

## Definition of done — v1 (firewall-access headline)

A developer or elohim agent, **from a library computer on a foreign network (behind a corporate
firewall), authenticates via the peer-native OAuth-like login, reaches a devspace hosted on a blade in
the offering steward's household rack over doorway-signaled WebRTC, and edits code in openvscode-server
within (measured) seconds — and the environment tears down and rematerializes from the same CID with
edits intact, while an adversarial build inside it cannot harm the host's live protocol
participation.** **M0 + M1 (+ S1) + M2 + M3 + M4 complete**, gates green, the containment scenario
passing on a real household-rack node, and the firewall-traversal + connection-latency numbers
measured and reported.

## Decomposition note

When a milestone is picked up, decompose it into a dated bite-sized TDD plan
(`docs/plans/YYYY-MM-DD-<milestone>.md`) with per-task failing-test-first steps, then execute via
subagent-driven-development. This roadmap is the map; the per-milestone plans are the turn-by-turn
directions.
