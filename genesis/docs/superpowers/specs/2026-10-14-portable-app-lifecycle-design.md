---
id: portable-app-lifecycle-design
status: Draft
class: architecture
context-tier: disclosed
steward: human:matthew
serves: release-retention
date: 2026-10-14
graduation-trigger: Che, Rakia and platform integration stories pass against real authorized runtimes and cold offline clients
cites:
  - "live-session-plane-design | Owns ephemeral passing-state sessions; app lifecycle composes its SDK rather than opening another channel | sha256:d1adbf86e46fd487 | path: genesis/docs/superpowers/specs/2026-10-14-live-session-plane-design.md"
  - "live-session-slice-1-own-devices-direct | Keeps this-machine-only SDK routes peer-owned and explicitly forbids their exposure through a Che proxy | sha256:d256fb581707f08d | path: genesis/docs/superpowers/plans/2026-10-14-live-session-slice-1-own-devices-direct.md"
  - "device-footprint-residency-carrying-capacity-design | Owns follow, reside, carry, pins, retention windows and resource admission reused by portable apps | sha256:99ab8f8330cc3f24 | path: genesis/docs/superpowers/specs/2026-10-06-device-footprint-residency-carrying-capacity-design.md"
  - "working-version-sdk-standard-design | Owns private drafts and publication; continuation references that state rather than creating a second document | sha256:66ba00931d6dd97e | path: genesis/docs/superpowers/specs/2026-10-04-working-version-sdk-standard-design.md"
  - "sync-state-contract | Owns epoch and position comparison and unknown freshness used by bounded dashboard refresh | sha256:cea95b17140d2fd6 | path: genesis/docs/superpowers/specs/2026-08-29-sync-state-contract-design.md"
---

# Portable app lifecycle

Recovered from `0ee5dfde8` on 2026-10-28. This remains a draft migration contract; its foundation contracts (the SDK lifecycle interfaces, wire views and environment adapters) were ported separately on 2026-10-29. The newer [What every app inherits](../specs/2026-10-20-what-every-app-inherits-placement-contract-design.md) governs placement and manifest needs. A need becomes declarable only with its first reader; this recovery does not reopen undeclarable fields.

## Delivery contract

Apps open on an available authorized runtime, acquire verified compatible capabilities locally,
and switch execution only after an explicit continuation is ready. Held bytes, permission,
runtime availability and offline readiness are independent facts. A browser without the optional
connector retains the web experience. The native shell and ordinary browser tabs consume the
same contracts. A protocol link bootstraps a connection; it is never lasting authority.

This design serves `release-retention`, `workspace-device-arrival`, `epr-atom-home`,
`sync-scale-honesty` and `attention-witnessed-privately`. The foundation supplies contracts, not
a claim that the runtime migrations below have shipped. The acceptance chain is
`genesis/a2o/features/devflow/portable-app-lifecycle.feature`.

## Ownership

| Concern | Producer | Consumer |
|---|---|---|
| Lifecycle vocabulary and wire shapes | SDK schemas; `crates/elohim-views` conforms and generates TS | Rust SDK and `storage-client-ts` |
| Resolution, execution and continuation interfaces | Rust SDK; matching framework-free TS interfaces | Domain apps, shell, hub |
| Verified bytes, elected release heads, holds and authorized sync | Storage | Hub and resident node |
| Peer-addressed app delivery and readiness | Hub | Native peers and doorway |
| Resident processes and resource admission | Node composition using Ark | App runtime adapters |
| Browser session and HTTPS service attachment | Doorway | Browser and workspace runtime |
| Browser/native translation | `bridges/browser` library | Native messaging host; replaceable extension |
| Environment discovery | `bridges/workspace` library | Che adapter in node; local adapter |

The live-session plane and its own-devices slice own passing-state channels and the shared SDK
session object. This lifecycle introduces no competing session/channel API. The peer runtime owns
the app surface and admission; doorway mounts it through browser ingress. In particular, slice 1's
`caller_is_local` routes are intentionally loopback-only and reject trusted proxies/forwarding
headers. The Che bridge MUST NOT tunnel those routes or spoof a local caller. A later explicitly
authorized peer ingress adapter composes the same session implementation with its own admission;
until then, remote passing-state access is unsupported. Presence is distinct from the durable
private observation journal and its low-priority reconciliation lane.

Bridges have no identity store, analytics authority, new supervisor or app policy. Admission
belongs to the hosting runtime. Native artifacts are optional by app; browser and server artifact
support today does not imply an OS/architecture native distribution resolver exists.

## Local-first and P2P gate

Alone with a device key, held verified bytes and private working versions remain usable; only
acquisition of missing bytes and other parties' acts wait. With a doorway-held key, local reads
and preparation remain possible but signing waits visibly. Visitors may read verified retained
public bytes; account durability is not implied. Browser selection and scrolling remain local.
An explicit private continuation references the existing working version rather than replacing it.

Own authorized devices add continuity; a hub adds availability; doorway adds browser reachability.
Their loss does not remove held functionality. Hosting parties see request timing and the plaintext
their role processes. Neither a route lease nor device recognition grants private-history access.

Lifecycle requests, runtime instances, route leases and sync demands are ephemeral C: no new DHT
entries or heads, zero incremental head-plane items at seed and one year. Releases reference
existing Content/Manifest CIDs and elected channels. Private continuation and observations reuse
the B-class working-version and observation paths. Existing integrity entries include Content,
Manifest, private AttentionTending, DevicePolicy and AgentPeerBinding; this foundation adds no
coordinator calls or DNA changes. The roster records attribution, not authorization.

Current observation acknowledgements say `signed: absent` and namespace `as-asserted`. Keep those
facts; a persistence receipt is not a witnessed private source-chain record. Do not implement a
parallel attention system around AttentionTending. Presence is unstored and never replayed.

The person explicitly authorized private raw-history reconciliation between their own admitted
devices. This expands earlier device-only attention guidance; it permits neither pooling nor
cross-person joins. Device-private overrides still suppress discovery and sync.

## Four delivery stages

### Foundation

Compile shared interfaces and generated client contracts; add refusing default adapters, bridge
library boundaries, replacement-adapter tests and bounded synchronization demand coordination.
The generic bounded demand-lease primitive lives below the SDK in `elohim-seam-contracts`;
SDK code only binds app stream targets to it. It is not a second scheduler: Ark and the resident
runtime retain power/resource admission. Python scripts are not an implementation of these APIs.
Declare ownership with `.epr-meta`, register real decision predicates, and wire owning gates.
Contract tests establish interface behavior, never real ingress, custody or offline delivery.

### Che access

Compose device consent, browser session exchange and peer-side scoped resource permission independently.
Add an authenticated outbound WSS service attachment to doorway's existing HTTPS endpoint, with
expiring scoped routes, origin checks, revocation, bounded requests and no shared private caching.
No per-app Kubernetes ingress or arbitrary filesystem/proxy target. The workspace bridge discovers
Che configuration; generic attachment stays in doorway/node. One idempotent run command starts or
reuses the declared runtime and returns its reachable URL. Reconnect rechecks authorization.
Prove direct native and doorway access with the same reader and access refusals. Then retire the
fragment-capability launcher from the ordinary journey. A browser handler launches on the browser's
device, not in Che. A local handler cannot substitute for this transport.

### Rakia adoption

Keep coding semantics in the declaration/adapter and Avodah generic; prove orchard unchanged.
Resolve compatible artifacts and verify their complete dependency set, acquire under standing
permission, start under Ark, and activate a versioned continuation with operation deduplication.
Keep the original runtime active until readiness and handoff succeed; failed acquisition, startup
or activation preserves its usable state. Do not transfer a signing key as a substitute for safe
source-chain authorship. Browser and shell follow the same journey.

Reuse acquisition pins and release retention: follow, reside and carry remain distinct. Offline
availability requires verified executable dependencies plus required data. Running/pinned releases
are protected. Inactivity/pressure policy may reclaim unpinned executable bytes only; identity,
private data, unsynced work and composition references survive. Keep existing release windows;
operator-declared inactivity thresholds belong to the residency policy, not UI constants.

### Platform integration

Inventory first-party app bundles from manifests and migrate Lamad, Avodah, the main client and
shell. Keep identity portals usable for bootstrap. Compose resident services outside app window
lifetimes with Ark and power/network/resource bounds. Browser extension messages are origin-bound,
minimal and replaceable; no app history or credentials in URLs or logs.

Extend ObservationEmitter's existing path with durable local append before acknowledgement,
retry/deduplication and authorized own-device reconciliation. Default to low-priority batches.
Dashboard open/refresh promotes the same scoped queue, immediately shows local readings and freezes
known remote target positions. Concurrent requests coalesce transport work; each requester retains
its own completion target. Epoch changes, unknown remote ends and unreachable peers never become
caught-up to a new unknown target. Retained progress toward a previously observed target remains
visible independently of remote reachability; it never certifies the current remote end.
Completion/cancellation releases priority, not persisted work. Later arrivals do not
extend a refresh forever. Power policy remains authoritative even for foreground work.

## Proof and closure

Run owning SDK, native/hub/node, doorway, bridge and frontend gates as each stage changes them;
Genesis/Gherkin validation; pinned-production browser tests; light/dark review and independent
acceptance-story review. Test invalid origins/grants, revocation, retries, incompatible releases,
failed handoff, interrupted writes, corrupt objects, full storage, cold offline restart and pinned
retention. Every stage ends with its real receipts and a delta in the existing owning habit, then
habit projection. No habit or backlog status changes by intention. Pending stories remain pending.
