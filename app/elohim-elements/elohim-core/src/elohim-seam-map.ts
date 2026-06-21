import { msg } from '@lit/localize';
import { css, html, LitElement, nothing, type TemplateResult } from 'lit';
import { property, state } from 'lit/decorators.js';

import { CapabilityAwareElement } from './capability/index.js';

// ---------------------------------------------------------------------------
// Types — operational UI data (no ts-rs view exists for this visualization)
//
// DATA-SHAPE NOTE: The SeamMap is a static architecture-visualization fixture
// consumed by Storybook / developer tooling. There is no backend view for this
// data. The eventual home is the `architecture-atlas` contentFormat per the
// p2p-design-gate: a ContentNode of that format would carry this as its
// `content` payload, served through the lamad renderer for architecture docs.
// Until that contentFormat and its renderer exist, this element is Library A
// substrate consuming a local interface declared below.
// ---------------------------------------------------------------------------

/** Participation tracks from Figure B of the seam atlas. */
export type ParticipationTrack = 'T1-dht' | 'T2-substrate' | 'T3-spoke' | 'T4-doorway';

/**
 * Device spectrum rung — a row in the §2 device-spectrum table.
 * `id` matches the `device-*` archetype from `genesis/data/devices/devices.json`.
 */
export interface SeamDevice {
  id: string;
  label: string;
  capabilityLevel: 0 | 1 | 2 | 3 | 4 | 5;
  formFactor: string;
  tracks: ParticipationTrack[];
}

/**
 * Applicability of a seam at a given capability level:
 * - `full`  — seam is fully active at this level
 * - `partial` — seam is partially active (e.g. spoke side only, or optional)
 * - `host`  — this device HOSTS the seam (T3/T4)
 * - `none`  — seam does not apply at this level
 */
export type SeamApplicability = 'full' | 'partial' | 'host' | 'none';

/** Per-level applicability map. Keys are capability levels 0–5. */
export type SeamApplicabilityMap = Record<0 | 1 | 2 | 3 | 4 | 5, SeamApplicability>;

/** Group heading for the seam rows in the vertical composition stack. */
export type SeamGroup =
  | 'hardware'
  | 'os-packaging'
  | 'runtime'
  | 'plugin'
  | 'sdk-bridge'
  | 'app-manifest'
  | 'client'
  | 'role-doorway'
  | 'role-peer-hoster'
  | 'role-aggregation'
  | 'role-hub-cluster'
  | 'plane-confidentiality'
  | 'plane-temporal'
  | 'plane-resource-governance';

/**
 * One seam from the §3 seam catalog.
 */
export interface SeamEntry {
  id: string;
  title: string;
  group: SeamGroup;
  /** One-line problem-class summary from §3. */
  problemClass: string;
  /** Path for "add a new X" — the action a developer takes at this seam. */
  addNewX: string;
  /** Canonical home — where to read/edit (file paths, docs). */
  home: string;
  /** Adjacent-seam confusion to avoid. */
  confusion: string;
  /** Applicability per capability level. */
  applicability: SeamApplicabilityMap;
}

/** One concern-routing row from §4. */
export interface RoutingRow {
  concern: string;
  seamId: string;
  deviceScale: string;
  home: string;
}

/** Track span definition for Figure B bands. */
export interface TrackBand {
  id: ParticipationTrack;
  label: string;
  description: string;
  /** Which capabilityLevel range spans full involvement. */
  fullFrom: 0 | 1 | 2 | 3 | 4 | 5;
  fullTo: 0 | 1 | 2 | 3 | 4 | 5;
  /** Optional partial span (spoke side vs host side). */
  partialLabel?: string;
}

/** Root data shape for the seam map visualization. */
export interface SeamMap {
  devices: SeamDevice[];
  seams: SeamEntry[];
  tracks: TrackBand[];
  routing: RoutingRow[];
}

/** Content / load state of the element. */
export type SeamMapState = 'idle' | 'loading' | 'empty' | 'error';

// ---------------------------------------------------------------------------
// Group display metadata
// ---------------------------------------------------------------------------

const GROUP_LABELS: Record<SeamGroup, () => string> = {
  'hardware': () => msg('Hardware'),
  'os-packaging': () => msg('OS / Packaging'),
  'runtime': () => msg('Runtime / Binary'),
  'plugin': () => msg('Mod / Plugin'),
  'sdk-bridge': () => msg('SDK / Bridge'),
  'app-manifest': () => msg('App-Manifest / Domain'),
  'client': () => msg('Client Surface'),
  'role-doorway': () => msg('Role: Doorway (T4)'),
  'role-peer-hoster': () => msg('Role: Peer-Hoster (T2+T3)'),
  'role-aggregation': () => msg('Role: Aggregation (T2)'),
  'role-hub-cluster': () => msg('Role: Hub-Cluster'),
  'plane-confidentiality': () => msg('Plane: Confidentiality'),
  'plane-temporal': () => msg('Plane: Temporal'),
  'plane-resource-governance': () => msg('Plane: Resource Governance'),
};

const TRACK_LABELS: Record<ParticipationTrack, () => string> = {
  'T1-dht': () => msg('T1 DHT — notary floor'),
  'T2-substrate': () => msg('T2 Substrate — storage-running'),
  'T3-spoke': () => msg('T3 Spoke — HTTP/WS bridge'),
  'T4-doorway': () => msg('T4 Doorway — web2 projection'),
};

const APPLICABILITY_LABELS: Record<SeamApplicability, () => string> = {
  full: () => msg('Active'),
  partial: () => msg('Partial'),
  host: () => msg('Host'),
  none: () => msg('N/A'),
};

// ---------------------------------------------------------------------------
// Default fixture — the full atlas from §2–§4
// ---------------------------------------------------------------------------

/** Default SeamMap fixture derived from the 2026-06-21 atlas. */
export const DEFAULT_SEAM_MAP: SeamMap = {
  devices: [
    {
      id: 'biometric-fob',
      label: msg('Wearable / Fob'),
      capabilityLevel: 0,
      formFactor: 'fob',
      tracks: ['T1-dht', 'T3-spoke'],
    },
    {
      id: 'observer-camera',
      label: msg('Civic Camera'),
      capabilityLevel: 0,
      formFactor: 'iot-sensor',
      tracks: ['T1-dht', 'T3-spoke'],
    },
    {
      id: '2019-android-phone',
      label: msg('Phone'),
      capabilityLevel: 2,
      formFactor: 'phone',
      tracks: ['T1-dht', 'T3-spoke'],
    },
    {
      id: 'chromebook-edu',
      label: msg('Chromebook'),
      capabilityLevel: 2,
      formFactor: 'laptop',
      tracks: ['T1-dht', 'T3-spoke'],
    },
    {
      id: 'recycled-laptop',
      label: msg('Laptop'),
      capabilityLevel: 3,
      formFactor: 'laptop',
      tracks: ['T1-dht', 'T2-substrate', 'T3-spoke'],
    },
    {
      id: 'raspberry-pi-4',
      label: msg('Raspberry Pi 4'),
      capabilityLevel: 3,
      formFactor: 'sbc',
      tracks: ['T1-dht', 'T2-substrate'],
    },
    {
      id: 'home-nuc',
      label: msg('Home NUC'),
      capabilityLevel: 4,
      formFactor: 'mini-pc',
      tracks: ['T1-dht', 'T2-substrate', 'T4-doorway'],
    },
    {
      id: 'gaming-desktop',
      label: msg('Gaming Desktop'),
      capabilityLevel: 4,
      formFactor: 'desktop',
      tracks: ['T1-dht', 'T2-substrate'],
    },
    {
      id: 'family-node-base',
      label: msg('Family Node (base)'),
      capabilityLevel: 5,
      formFactor: 'rack-module',
      tracks: ['T1-dht', 'T2-substrate', 'T3-spoke', 'T4-doorway'],
    },
    {
      id: 'family-node-extended',
      label: msg('Family Node (store-anator)'),
      capabilityLevel: 5,
      formFactor: 'rack-module',
      tracks: ['T1-dht', 'T2-substrate', 'T3-spoke', 'T4-doorway'],
    },
    {
      id: 'k8s-pod-256mb',
      label: msg('K8s Pod'),
      capabilityLevel: 5,
      formFactor: 'container',
      tracks: ['T1-dht', 'T2-substrate', 'T4-doorway'],
    },
  ],
  seams: [
    // §3.1
    {
      id: 'hardware',
      title: msg('Hardware / Capability Gradient'),
      group: 'hardware',
      problemClass: msg('What is this peer able/allowed to do, and how do operations adapt?'),
      addNewX: msg('Add a device-*.md fixture + devices.json entry; wire deviceArchetype in deployments.json'),
      home: 'genesis/plans/2026-04-13-device-archetypes-design.md; genesis/data/devices/*',
      confusion: msg('capability_level (L0–5) ≠ stage (1–4, journey) ≠ Tier (1/2/3, product class). nodeTypes is k8s placement, NOT capability.'),
      applicability: { 0: 'full', 1: 'full', 2: 'full', 3: 'full', 4: 'full', 5: 'full' },
    },
    // §3.2
    {
      id: 'os-packaging',
      title: msg('OS / Packaging / Deploy Target'),
      group: 'os-packaging',
      problemClass: msg('Will this code physically run on this device, and how does it get there?'),
      addNewX: msg('Add a Dockerfile / Tauri bundle target / nginx image; edit the edgenode template.yaml'),
      home: 'CLAUDE.md §Deployment Contexts; steward/device/CLAUDE.md; Dockerfiles; devfile.yaml',
      confusion: msg('Packaging ("which image runs") ≠ the network role it then plays. Editing a manifest is a packaging act; live cluster reconciliation is operator-owned.'),
      applicability: { 0: 'none', 1: 'none', 2: 'partial', 3: 'partial', 4: 'full', 5: 'full' },
    },
    // §3.3
    {
      id: 'runtime',
      title: msg('Runtime / Binary + Footprint'),
      group: 'runtime',
      problemClass: msg('Which native binary (or none) runs here, in what compositional shape — thin vs fat, which cargo features?'),
      addNewX: msg('Flip cargo [features]; embed thin (steward/node/Cargo.toml); a brittle sed strips V8/SSR from the storage image'),
      home: 'elohim/elohim-storage/CLAUDE.md; steward/node/CLAUDE.md; doorway/doorway-service/CLAUDE.md',
      confusion: msg('The "elohim-node" name overload: steward/node binary ≠ k8s container named elohim-node. Footprint tuning is native-binary-only — never tweak WASM/DNA per device.'),
      applicability: { 0: 'none', 1: 'none', 2: 'partial', 3: 'partial', 4: 'full', 5: 'full' },
    },
    // §3.4
    {
      id: 'plugin',
      title: msg('Mod / Plugin'),
      group: 'plugin',
      problemClass: msg('Loading/extending native code into a running node dynamically and integrity-bound.'),
      addNewX: msg('No canonical path today (loader/host absent). Integrity: bounds_validator + delegates-compute + coordinator hot-swap.'),
      home: '.claude/data/modular-runtime-plugin-integrity-feasibility-2026-06-21.md',
      confusion: msg('The bridge is the compile-time form of native extension. @capability* tags are UI render profiles, NOT authorization — that is Mishpat delegates-compute.'),
      applicability: { 0: 'none', 1: 'none', 2: 'none', 3: 'partial', 4: 'partial', 5: 'full' },
    },
    // §3.5 + §3.6 combined row
    {
      id: 'sdk-bridge',
      title: msg('SDK Grammar + Bridge'),
      group: 'sdk-bridge',
      problemClass: msg('SDK: declarative domain composition with integrity-by-construction. Bridge: imperative interop facing the non-elohim world.'),
      addNewX: msg('SDK: edit manifest → schema:validate → codegen. Bridge: bridges/CLAUDE.md 4-step (crate + dep + arm + recompile).'),
      home: 'elohim/sdk/CLAUDE.md; bridges/CLAUDE.md; elohim/sdk/domains/lamad/manifest.json',
      confusion: msg('SDK (compose inward) ≠ Bridge (translate outward) ≠ Plugin (extend downward). Three different artifacts, three different bind-times.'),
      applicability: { 0: 'none', 1: 'none', 2: 'none', 3: 'partial', 4: 'full', 5: 'full' },
    },
    // §3.7
    {
      id: 'app-manifest',
      title: msg('App-Manifest / Domain'),
      group: 'app-manifest',
      problemClass: msg('Add a new domain app — declarative vocabulary: content types/formats, renderer mappings, relationships, signals, coupling rules.'),
      addNewX: msg('domains/<app>/manifest.json + domains/<app>/types/ wire-types + codegen (pnpm run lamad:codegen)'),
      home: 'elohim/sdk/domains/CLAUDE.md; elohim/sdk/domains/{lamad,qahal,shefa}/; root CLAUDE.md §Schema',
      confusion: msg('Core-vs-extensible formats: a core "interactive" has no renderer → raw-JSON fallback. Use sophia-quiz-json, html5-app, not core protocol formats.'),
      applicability: { 0: 'none', 1: 'none', 2: 'full', 3: 'full', 4: 'full', 5: 'full' },
    },
    // §3.8
    {
      id: 'client',
      title: msg('Client Surface'),
      group: 'client',
      problemClass: msg('Presentation + user-intent capture only. Render host-provided state; emit intent events. Nothing else.'),
      addNewX: msg('Add a Lit element to elohim-elements (render + a11y + emit, bind nothing) → bind brand in graphos Library B → compose in the elohim-app shell.'),
      home: 'app/elohim-elements/CLAUDE.md; app/elohim-library/CLAUDE.md; skill looking-at-frontend',
      confusion: msg('Client ≠ doorway projection. A "404 / blob missing" is substrate replication, not a UI bug. Capability-Profile lens gradient ≠ hardware-tier detection.'),
      applicability: { 0: 'partial', 1: 'partial', 2: 'full', 3: 'full', 4: 'full', 5: 'full' },
    },
    // §3.9
    {
      id: 'role-doorway',
      title: msg('Role: Doorway Projection (T4)'),
      group: 'role-doorway',
      problemClass: msg('Make canonical substrate truth legible to browsers and the web2 world — HTTP, OAuth, manifest-driven routes, single-target proxy + cache.'),
      addNewX: msg('Add the match arm AND is_service_path (else EPR router shadows it). Add an is_service_path unit test.'),
      home: 'doorway/doorway-service/CLAUDE.md; 2026-05-02-elohim-hub-boundaries-design.md',
      confusion: msg('Hub ≠ doorway. "Doorway projects outward to web2; hub projects inward to nearby peers." Doorway is NOT a P2P participant.'),
      applicability: { 0: 'none', 1: 'none', 2: 'none', 3: 'none', 4: 'partial', 5: 'host' },
    },
    // §3.10
    {
      id: 'role-peer-hoster',
      title: msg('Role: Peer-Hoster Dataplane (T2+T3)'),
      group: 'role-peer-hoster',
      problemClass: msg('Durable availability of records across disconnect — async store-and-forward CRDT sync (T2) + spoke HTTP/WS (T3), addressed by peerId.'),
      addNewX: msg('Add a libp2p request-response behaviour + codec; CRDT engine is Automerge.'),
      home: 'elohim/elohim-storage/CLAUDE.md; .claude/data/peer-hoster-async-sync-readiness-2026-06-21.md',
      confusion: msg('peerId ≠ hostname. Peer-hoster (durable availability of records) ≠ aggregation (value rollup), though both ride T2.'),
      applicability: { 0: 'none', 1: 'none', 2: 'none', 3: 'partial', 4: 'full', 5: 'full' },
    },
    // §3.11
    {
      id: 'role-aggregation',
      title: msg('Role: Aggregation / Recursive Rollup (T2)'),
      group: 'role-aggregation',
      problemClass: msg('Recursive signal aggregation (region ← councils ← households) over CoverageRollup engine; REA value-flow accounting.'),
      addNewX: msg('Invoke p2p-design-gate FIRST. Rollup engine: recursion.rs; consumer path: graph_views/shefa/coverage.rs.'),
      home: 'elohim/elohim-storage/CLAUDE.md; .claude/data/commons-compute-aggregation-readiness-2026-06-21.md',
      confusion: msg('Flat CoverageRollup (single-level fold) ≠ recursive multi-level rollup. Case-A all-zeros is a Track-1 identity-coherence failure, not an aggregation bug.'),
      applicability: { 0: 'none', 1: 'none', 2: 'none', 3: 'none', 4: 'partial', 5: 'full' },
    },
    // §3.12
    {
      id: 'role-hub-cluster',
      title: msg('Role: Hub Cluster Ops + Enablement'),
      group: 'role-hub-cluster',
      problemClass: msg('k8s-class concerns absorbed inside a hub — blade-to-blade mDNS, leader election, pod consensus, replica/PVC placement. Plus the "hubbiness dial".'),
      addNewX: msg('steward/node/src/{cluster,pod,p2p}/; hub-internal swarm is private to HouseholdHub/CollectiveHub impl, mDNS-first.'),
      home: 'steward/node/CLAUDE.md; 2026-05-02-elohim-hub-boundaries-design.md; steward/node/src/',
      confusion: msg('Hub-internal swarm (steward/node, 3.12) ≠ Track-2 hub-to-hub federation (elohim-storage, 3.10/3.11). Also: hub-as-role ≠ Tier-3 hardware.'),
      applicability: { 0: 'none', 1: 'none', 2: 'none', 3: 'partial', 4: 'partial', 5: 'full' },
    },
    // §3.13
    {
      id: 'plane-confidentiality',
      title: msg('Plane: Confidentiality / Encryption'),
      group: 'plane-confidentiality',
      problemClass: msg('Keeping content readable only by intended readers — encryption-at-rest, private-replica encryption, reader-key envelopes, secret/key custody.'),
      addNewX: msg('A KeyEnvelope reader-key wrap (X25519 reader set) over the blob/quilt plane.'),
      home: '.claude/data/commons-compute-aggregation-readiness-2026-06-21.md (R6); BlobStore::store; tiered-quilt-stewardship-design.md',
      confusion: msg('Confidentiality ≠ integrity ≠ authorization. A notarized entry is world-readable. "Make this private" is the encryption plane, not a permission flag.'),
      applicability: { 0: 'none', 1: 'none', 2: 'partial', 3: 'partial', 4: 'full', 5: 'full' },
    },
    // §3.14
    {
      id: 'plane-temporal',
      title: msg('Plane: Temporal / Scheduling'),
      group: 'plane-temporal',
      problemClass: msg('When things happen across the fabric — durable timers, recurring ticks, deferred/scheduled work, validity horizons, expiry→obligation.'),
      addNewX: msg('time is a substrate signal; aggregate-tick services/aggregator.rs::aggregate_and_emit; validity horizons in the feedback/claims model.'),
      home: 'elohim/sdk/CLAUDE.md §signal list; services/aggregator.rs; the feedback/claims model',
      confusion: msg('time as a substrate signal (economic, accounted) ≠ a durable scheduler (operational, when-to-fire). "Re-quilt every N hours" is this seam, not aggregation.'),
      applicability: { 0: 'none', 1: 'none', 2: 'none', 3: 'partial', 4: 'full', 5: 'full' },
    },
    // §3.15
    {
      id: 'plane-resource-governance',
      title: msg('Plane: Resource Governance / Admission'),
      group: 'plane-resource-governance',
      problemClass: msg('How the system protects itself and allocates scarce capacity — quotas, rate-limits, circuit-breakers, load-shedding, admission control.'),
      addNewX: msg('bounds_validator (7-check, rate/ceiling per commitment); doorway 503-shed / admission; self-heal circuit breakers; pantry backpressure; arc_factor.'),
      home: 'elohim/elohim-storage/src/services/bounds_validator.rs; doorway shed/watchdog /metrics (M1-M5); the self-heal runtime-findings ledger',
      confusion: msg('The unifying owner of these guards is the elohim-operator (per-hub allocator). "The phone is getting flooded" or "breaker stuck open" is this seam, not hardware or dataplane.'),
      applicability: { 0: 'full', 1: 'full', 2: 'full', 3: 'full', 4: 'full', 5: 'full' },
    },
  ],
  tracks: [
    {
      id: 'T1-dht',
      label: msg('T1 DHT'),
      description: msg('Notary floor: identity · stewardship contracts · REA commitments · reach. All devices. tx5/WebRTC.'),
      fullFrom: 0,
      fullTo: 5,
    },
    {
      id: 'T3-spoke',
      label: msg('T3 Spoke'),
      description: msg('HTTP/WS bridge: wearable/IoT/phone route through a dwelling hub. Spoke side: L0–L2; host side: recycled-laptop+.'),
      fullFrom: 0,
      fullTo: 2,
      partialLabel: msg('→ hub host'),
    },
    {
      id: 'T2-substrate',
      label: msg('T2 Substrate'),
      description: msg('Storage-running: laptop+ run elohim-storage. libp2p-primary → iroh-primary (thick).'),
      fullFrom: 3,
      fullTo: 5,
    },
    {
      id: 'T4-doorway',
      label: msg('T4 Doorway'),
      description: msg('Browsers CONSUME (default: doorway.elohim.host). Thick nodes may operate a doorway as a separate opt-in deploy.'),
      fullFrom: 5,
      fullTo: 5,
      partialLabel: msg('← browsers consume →'),
    },
  ],
  routing: [
    { concern: msg('A device is too small to run a conductor'), seamId: 'hardware', deviceScale: 'L0–L1 smartwatch/fob', home: 'device-archetypes-design :33-41; Track3Bridge' },
    { concern: msg('A phone keeps OOM\'ing / sync floods it'), seamId: 'runtime', deviceScale: 'L2 phone', home: 'device-archetypes :105; cargo features thin flavor' },
    { concern: msg('Render simpler for a child / small screen'), seamId: 'client', deviceScale: 'thin (any)', home: 'elohim-elements/CLAUDE.md:80-98 (lens gradient)' },
    { concern: msg('A page 404s / a blob won\'t load'), seamId: 'role-doorway', deviceScale: 'thin browser', home: 'doorway/CLAUDE.md "No Blob Fan-Out"' },
    { concern: msg('Add a new domain app (new pillar surface)'), seamId: 'app-manifest', deviceScale: 'device-invariant', home: 'domains/<app>/manifest.json; elohim/sdk/domains/CLAUDE.md' },
    { concern: msg('Add a new capability to an existing app'), seamId: 'sdk-bridge', deviceScale: 'device-invariant', home: 'signal_kind / Commitment action / Governor; p2p-design-gate' },
    { concern: msg('We need to talk to Mastodon / ActivityPub'), seamId: 'sdk-bridge', deviceScale: 'thick (doorway host)', home: 'bridges/CLAUDE.md; doorway consumes web2 bridges' },
    { concern: msg('Extend the runtime with a native capability'), seamId: 'plugin', deviceScale: 'mid–thick', home: 'modular-runtime-plugin-integrity assessment' },
    { concern: msg('Tune the binary for a Raspberry Pi (shrink footprint)'), seamId: 'runtime', deviceScale: 'Pi/NUC L3-4', home: 'cargo [features] Cargo.toml:267-273' },
    { concern: msg('Package the app for a desktop user'), seamId: 'os-packaging', deviceScale: 'laptop L2-3', home: 'Tauri .deb/AppImage; steward/device/CLAUDE.md' },
    { concern: msg('A laptop should host content for others'), seamId: 'role-peer-hoster', deviceScale: 'laptop→hub L3', home: 'peer-hoster-async-sync-readiness; CRDT store' },
    { concern: msg('A non-tech steward wants to turn a laptop into a hub'), seamId: 'role-hub-cluster', deviceScale: 'recycled-laptop L3', home: 'hub-enablement-dial-readiness; steward/node daemon' },
    { concern: msg('Aggregate signals to commons scale'), seamId: 'role-aggregation', deviceScale: 'always-on→thick', home: 'recursive rollup over recursion.rs' },
    { concern: msg('Make this content private / readable only by X'), seamId: 'plane-confidentiality', deviceScale: 'any', home: 'KeyEnvelope over the blob/quilt plane' },
    { concern: msg('Schedule recurring work / expire a claim'), seamId: 'plane-temporal', deviceScale: 'always-on', home: 'time signal + durable scheduler home' },
    { concern: msg('A peer is being flooded / a breaker is stuck open'), seamId: 'plane-resource-governance', deviceScale: 'any', home: 'bounds_validator + doorway shed + self-heal' },
  ],
};

// ---------------------------------------------------------------------------
// Element
// ---------------------------------------------------------------------------

/**
 * `<elohim-seam-map>` — interactive concern-routing seam map visualization.
 *
 * Renders the Elohim architecture atlas as an interactive two-axis grid:
 * - **Rows**: composition-stack seams (hardware → OS → runtime → plugins → SDK/bridge →
 *   app-manifest → client, then role seams, then the confidentiality / temporal /
 *   resource-governance planes)
 * - **Columns**: device spectrum (smartwatch/fob → k8s-pod), colored by capability level
 * - **Bands**: the four participation tracks (T1 DHT / T2 substrate / T3 spoke / T4 doorway)
 *
 * **Interaction**: click/keyboard-select a seam row → detail panel opens showing
 * problem-class · add-a-new-X path · canonical home · confusion-to-avoid.
 * At `standard` lens+, selecting a concern from the routing table cross-highlights the seam.
 *
 * **Lens dispatch**:
 * | Lens     | What's shown                                    |
 * |----------|-------------------------------------------------|
 * | minimal  | Group headings + seam titles only; no detail    |
 * | simple   | Seam titles + applicability dots per device     |
 * | standard | Above + interactive detail panel on select       |
 * | detail   | Standard + routing table panel                  |
 * | debug    | Detail + data-testid annotations visible in DOM  |
 *
 * **Data shape note**: SeamMap is operational visualization config — no ts-rs view
 * exists. The eventual home is the `architecture-atlas` contentFormat (lamad manifest).
 * Until that contentFormat and its renderer are wired, this element consumes the
 * locally-declared `SeamMap` interface above.
 *
 * @element elohim-seam-map
 *
 * @prop {SeamMap} data - The seam map data. Defaults to the built-in atlas fixture.
 * @prop {SeamMapState} mapState - Content state.
 *
 * @slot empty-state - Override the empty state message.
 * @slot loading-state - Override the loading state content.
 * @slot error-state - Override the error state message.
 *
 * @cssprop --elohim-seam-map-bg - Map background color
 * @cssprop --elohim-seam-map-fg - Map foreground text color
 * @cssprop --elohim-seam-map-border - Map outer border
 * @cssprop --elohim-seam-map-radius - Map border-radius
 * @cssprop --elohim-seam-map-padding - Map padding
 * @cssprop --elohim-seam-map-header-bg - Column header cell background
 * @cssprop --elohim-seam-map-header-fg - Column header cell foreground
 * @cssprop --elohim-seam-map-group-bg - Group heading background
 * @cssprop --elohim-seam-map-group-fg - Group heading foreground
 * @cssprop --elohim-seam-map-row-bg - Seam row background (idle)
 * @cssprop --elohim-seam-map-row-bg-hover - Seam row background on hover/focus
 * @cssprop --elohim-seam-map-row-bg-selected - Seam row background when selected
 * @cssprop --elohim-seam-map-row-border - Seam row bottom border
 * @cssprop --elohim-seam-map-cell-full - Applicability dot: full
 * @cssprop --elohim-seam-map-cell-partial - Applicability dot: partial
 * @cssprop --elohim-seam-map-cell-host - Applicability dot: host
 * @cssprop --elohim-seam-map-cell-none - Applicability dot: none/N/A
 * @cssprop --elohim-seam-map-track-t1-bg - T1 track band background
 * @cssprop --elohim-seam-map-track-t2-bg - T2 track band background
 * @cssprop --elohim-seam-map-track-t3-bg - T3 track band background
 * @cssprop --elohim-seam-map-track-t4-bg - T4 track band background
 * @cssprop --elohim-seam-map-detail-bg - Detail panel background
 * @cssprop --elohim-seam-map-detail-border - Detail panel border
 * @cssprop --elohim-seam-map-detail-heading-color - Detail panel heading color
 * @cssprop --elohim-seam-map-detail-body-color - Detail panel body text color
 * @cssprop --elohim-seam-map-detail-label-color - Detail panel field label color
 * @cssprop --elohim-seam-map-meta-color - Debug/trace metadata text color
 * @cssprop --elohim-seam-map-state-color - State banner text color (stale/error/offline)
 *
 * @csspart container - The outer element container
 * @csspart header - Sticky column-header row
 * @csspart grid - The seam-rows × device-columns grid
 * @csspart group-heading - A group separator row
 * @csspart seam-row - One seam's row in the grid
 * @csspart seam-title - The seam title cell
 * @csspart cell - One applicability cell (seam × device)
 * @csspart tracks - The track-band section beneath the grid
 * @csspart track-band - One participation track band
 * @csspart detail-panel - The selected-seam detail panel
 * @csspart routing-table - The concern-routing table (detail+ lens)
 * @csspart state-banner - Loading / error / empty state banner
 *
 * @capabilityMaxLens debug
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual
 * @capabilityRequiredStandings pilot | steward | elohim-support
 * @capabilityContentCertainty canonical
 * @capabilityStates empty:designed, loading:designed, error:designed, stale:not-applicable, contested:not-applicable, offline:not-applicable
 */
export class ElohimSeamMap extends CapabilityAwareElement(LitElement) {
  static override readonly styles = css`
    :host {
      display: block;
      contain: layout style;
    }

    :host([hidden]) {
      display: none;
    }

    /* ------------------------------------------------------------------ */
    /* Outer container                                                      */
    /* ------------------------------------------------------------------ */
    .container {
      background: var(--elohim-seam-map-bg, Canvas);
      color: var(--elohim-seam-map-fg, CanvasText);
      border: var(--elohim-seam-map-border, 1px solid currentColor);
      border-radius: var(--elohim-seam-map-radius, 0.375rem);
      padding: var(--elohim-seam-map-padding, 1rem);
      font: inherit;
      overflow-x: auto;
    }

    /* ------------------------------------------------------------------ */
    /* State banners                                                        */
    /* ------------------------------------------------------------------ */
    .state-banner {
      padding-block: 2rem;
      text-align: center;
      color: var(--elohim-seam-map-state-color, CanvasText);
      font-style: italic;
    }

    /* ------------------------------------------------------------------ */
    /* Scrollable grid wrapper                                              */
    /* ------------------------------------------------------------------ */
    .scroll-wrapper {
      overflow-x: auto;
      -webkit-overflow-scrolling: touch;
    }

    /* ------------------------------------------------------------------ */
    /* Grid: columns = seam-title + devices                                */
    /* ------------------------------------------------------------------ */
    .grid {
      display: grid;
      min-inline-size: max-content;
      border-block-start: 1px solid currentColor;
    }

    /* ------------------------------------------------------------------ */
    /* Column header row                                                    */
    /* ------------------------------------------------------------------ */
    .header-row {
      display: contents;
    }

    .col-header {
      background: var(--elohim-seam-map-header-bg, ButtonFace);
      color: var(--elohim-seam-map-header-fg, ButtonText);
      padding-block: 0.5rem;
      padding-inline: 0.375rem;
      font-size: 0.6875rem;
      font-weight: 600;
      text-align: center;
      writing-mode: vertical-lr;
      transform: rotate(180deg);
      block-size: 5rem;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      border-inline-end: 1px solid currentColor;
      border-block-end: 1px solid currentColor;
    }

    .col-header-title {
      background: var(--elohim-seam-map-header-bg, ButtonFace);
      color: var(--elohim-seam-map-header-fg, ButtonText);
      padding-block: 0.5rem;
      padding-inline: 0.5rem;
      font-size: 0.75rem;
      font-weight: 600;
      text-align: start;
      border-inline-end: 1px solid currentColor;
      border-block-end: 2px solid currentColor;
      display: flex;
      align-items: flex-end;
    }

    /* ------------------------------------------------------------------ */
    /* Group heading rows                                                   */
    /* ------------------------------------------------------------------ */
    .group-heading {
      background: var(--elohim-seam-map-group-bg, ButtonFace);
      color: var(--elohim-seam-map-group-fg, ButtonText);
      padding-block: 0.25rem;
      padding-inline: 0.5rem;
      font-size: 0.6875rem;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      display: contents;
    }

    .group-heading-cell {
      background: var(--elohim-seam-map-group-bg, ButtonFace);
      color: var(--elohim-seam-map-group-fg, ButtonText);
      padding-block: 0.25rem;
      padding-inline: 0.5rem;
      font-size: 0.6875rem;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      border-block-end: 1px solid currentColor;
      border-inline-end: 1px solid currentColor;
    }

    .group-heading-cell-span {
      background: var(--elohim-seam-map-group-bg, ButtonFace);
      color: var(--elohim-seam-map-group-fg, ButtonText);
      padding-block: 0.25rem;
      padding-inline: 0.5rem;
      font-size: 0.6875rem;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      border-block-end: 1px solid currentColor;
      grid-column: 2 / -1;
    }

    /* ------------------------------------------------------------------ */
    /* Seam rows                                                            */
    /* ------------------------------------------------------------------ */
    .seam-row {
      display: contents;
    }

    .seam-row-cells {
      display: contents;
    }

    .seam-title-cell {
      background: var(--elohim-seam-map-row-bg, Canvas);
      padding-block: 0.5rem;
      padding-inline: 0.5rem;
      font-size: 0.8125rem;
      border-block-end: 1px solid var(--elohim-seam-map-row-border, currentColor);
      border-inline-end: 1px solid currentColor;
      cursor: pointer;
      user-select: none;
      max-inline-size: 14rem;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      position: relative;
    }

    .seam-title-cell:hover,
    .seam-title-cell:focus-visible {
      background: var(--elohim-seam-map-row-bg-hover, Highlight);
      color: var(--elohim-seam-map-fg, HighlightText);
      outline: 2px solid Highlight;
      outline-offset: -2px;
    }

    .seam-title-cell[aria-selected='true'] {
      background: var(--elohim-seam-map-row-bg-selected, Highlight);
      color: var(--elohim-seam-map-fg, HighlightText);
      font-weight: 600;
    }

    .seam-title-cell:focus-visible {
      outline: 2px solid Highlight;
      outline-offset: -2px;
    }

    /* ------------------------------------------------------------------ */
    /* Applicability cells                                                  */
    /* ------------------------------------------------------------------ */
    .cell {
      display: flex;
      align-items: center;
      justify-content: center;
      border-block-end: 1px solid var(--elohim-seam-map-row-border, currentColor);
      border-inline-end: 1px solid currentColor;
      padding: 0.25rem;
      background: var(--elohim-seam-map-row-bg, Canvas);
      min-inline-size: 2.5rem;
      min-block-size: 2rem;
    }

    .cell[data-selected-row='true'] {
      background: var(--elohim-seam-map-row-bg-selected, Highlight);
      opacity: 0.25;
    }

    .dot {
      display: inline-block;
      inline-size: 0.75rem;
      block-size: 0.75rem;
      border-radius: 50%;
      flex-shrink: 0;
    }

    .dot[data-applicability='full'] {
      background: var(--elohim-seam-map-cell-full, LinkText);
    }

    .dot[data-applicability='partial'] {
      background: var(--elohim-seam-map-cell-partial, LinkText);
      opacity: 0.5;
    }

    .dot[data-applicability='host'] {
      background: var(--elohim-seam-map-cell-host, ButtonText);
      outline: 2px solid currentColor;
      outline-offset: 1px;
    }

    .dot[data-applicability='none'] {
      background: var(--elohim-seam-map-cell-none, ButtonFace);
      opacity: 0.2;
    }

    /* ------------------------------------------------------------------ */
    /* Track bands                                                          */
    /* ------------------------------------------------------------------ */
    .tracks {
      margin-block-start: 0.75rem;
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
    }

    .track-label-row {
      display: flex;
      align-items: center;
      gap: 0.5rem;
      font-size: 0.75rem;
      min-block-size: 1.5rem;
    }

    .track-badge {
      font-weight: 700;
      font-size: 0.6875rem;
      padding-block: 0.125rem;
      padding-inline: 0.375rem;
      border-radius: 0.25rem;
      white-space: nowrap;
      flex-shrink: 0;
      min-inline-size: 6rem;
      text-align: center;
    }

    .track-badge[data-track='T1-dht'] {
      background: var(--elohim-seam-map-track-t1-bg, Canvas);
      color: var(--elohim-seam-map-fg, CanvasText);
      border: 1px solid currentColor;
    }

    .track-badge[data-track='T2-substrate'] {
      background: var(--elohim-seam-map-track-t2-bg, Canvas);
      color: var(--elohim-seam-map-fg, CanvasText);
      border: 1px solid currentColor;
      opacity: 0.8;
    }

    .track-badge[data-track='T3-spoke'] {
      background: var(--elohim-seam-map-track-t3-bg, Canvas);
      color: var(--elohim-seam-map-fg, CanvasText);
      border: 1px solid currentColor;
      opacity: 0.7;
    }

    .track-badge[data-track='T4-doorway'] {
      background: var(--elohim-seam-map-track-t4-bg, Canvas);
      color: var(--elohim-seam-map-fg, CanvasText);
      border: 1px solid currentColor;
      opacity: 0.6;
    }

    .track-desc {
      color: var(--elohim-seam-map-fg, CanvasText);
      opacity: 0.75;
    }

    /* ------------------------------------------------------------------ */
    /* Detail panel                                                         */
    /* ------------------------------------------------------------------ */
    .detail-panel {
      margin-block-start: 1rem;
      background: var(--elohim-seam-map-detail-bg, Canvas);
      border: var(--elohim-seam-map-detail-border, 1px solid currentColor);
      border-radius: 0.25rem;
      padding: 0.75rem 1rem;
    }

    .detail-heading {
      font-size: 1rem;
      font-weight: 700;
      color: var(--elohim-seam-map-detail-heading-color, CanvasText);
      margin-block-end: 0.75rem;
    }

    .detail-fields {
      display: grid;
      grid-template-columns: auto 1fr;
      gap: 0.375rem 0.75rem;
      font-size: 0.8125rem;
    }

    .detail-label {
      font-weight: 600;
      color: var(--elohim-seam-map-detail-label-color, CanvasText);
      opacity: 0.75;
      white-space: nowrap;
    }

    .detail-value {
      color: var(--elohim-seam-map-detail-body-color, CanvasText);
    }

    .detail-close {
      margin-block-start: 0.75rem;
      font-size: 0.75rem;
      color: var(--elohim-seam-map-detail-label-color, LinkText);
      cursor: pointer;
      user-select: none;
      background: none;
      border: none;
      padding: 0;
      font: inherit;
      text-decoration: underline;
    }

    .detail-close:focus-visible {
      outline: 2px solid Highlight;
      outline-offset: 2px;
    }

    /* ------------------------------------------------------------------ */
    /* Routing table (detail+ lens)                                        */
    /* ------------------------------------------------------------------ */
    .routing-table {
      margin-block-start: 1.25rem;
    }

    .routing-table-heading {
      font-size: 0.875rem;
      font-weight: 700;
      color: var(--elohim-seam-map-detail-heading-color, CanvasText);
      margin-block-end: 0.5rem;
    }

    table {
      border-collapse: collapse;
      inline-size: 100%;
      font-size: 0.75rem;
    }

    th, td {
      border: 1px solid currentColor;
      padding-block: 0.25rem;
      padding-inline: 0.5rem;
      text-align: start;
      vertical-align: top;
    }

    th {
      background: var(--elohim-seam-map-header-bg, ButtonFace);
      color: var(--elohim-seam-map-header-fg, ButtonText);
      font-weight: 600;
    }

    tr:nth-child(even) td {
      background: var(--elohim-seam-map-group-bg, ButtonFace);
      opacity: 0.5;
    }

    /* ------------------------------------------------------------------ */
    /* Legend                                                              */
    /* ------------------------------------------------------------------ */
    .legend {
      display: flex;
      flex-wrap: wrap;
      gap: 0.75rem 1.25rem;
      margin-block-end: 0.75rem;
      font-size: 0.75rem;
      align-items: center;
    }

    .legend-item {
      display: flex;
      align-items: center;
      gap: 0.375rem;
    }

    /* ------------------------------------------------------------------ */
    /* Reduced motion: no transitions                                       */
    /* No motion is in use by default (stimulus: still).                   */
    /* Wrap any future transitions here:                                    */
    /* @media (prefers-reduced-motion: no-preference) and (update: fast) { */
    /* ------------------------------------------------------------------ */

    /* ------------------------------------------------------------------ */
    /* Reduced transparency                                                */
    /* ------------------------------------------------------------------ */
    @media (prefers-reduced-transparency: reduce) {
      .dot[data-applicability='partial'],
      .dot[data-applicability='none'],
      .track-desc,
      .detail-label {
        opacity: 1;
      }
    }

    /* ------------------------------------------------------------------ */
    /* Forced colors                                                        */
    /* ------------------------------------------------------------------ */
    @media (forced-colors: active) {
      .container {
        background: Canvas;
        color: CanvasText;
        border-color: CanvasText;
      }

      .col-header,
      .col-header-title,
      .group-heading-cell,
      .group-heading-cell-span {
        background: ButtonFace;
        color: ButtonText;
        forced-color-adjust: auto;
      }

      .seam-title-cell:hover,
      .seam-title-cell:focus-visible,
      .seam-title-cell[aria-selected='true'] {
        background: Highlight;
        color: HighlightText;
      }

      .dot[data-applicability='full'] {
        background: LinkText;
        forced-color-adjust: none;
      }

      .dot[data-applicability='partial'] {
        background: LinkText;
        opacity: 1;
        forced-color-adjust: none;
      }

      .dot[data-applicability='host'] {
        background: ButtonText;
        outline-color: ButtonText;
        forced-color-adjust: none;
      }

      .dot[data-applicability='none'] {
        background: ButtonFace;
        opacity: 1;
        forced-color-adjust: none;
      }

      .detail-panel {
        background: Canvas;
        color: CanvasText;
        border-color: CanvasText;
      }

      th {
        background: ButtonFace;
        color: ButtonText;
      }

      .track-badge {
        border-color: CanvasText;
        background: Canvas;
        color: CanvasText;
        opacity: 1;
        forced-color-adjust: none;
      }
    }

    /* ------------------------------------------------------------------ */
    /* Touch targets ≥ 44×44px under pointer:coarse                       */
    /* ------------------------------------------------------------------ */
    @media (pointer: coarse) {
      .seam-title-cell {
        min-block-size: 2.75rem;
        padding-block: 0.75rem;
      }

      .detail-close {
        min-block-size: 2.75rem;
        display: flex;
        align-items: center;
      }
    }

    /* ------------------------------------------------------------------ */
    /* Debug: show data-testid as visible label                            */
    /* ------------------------------------------------------------------ */
    .debug-testid {
      font-size: 0.625rem;
      font-family: ui-monospace, monospace;
      color: var(--elohim-seam-map-meta-color, CanvasText);
      opacity: 0.6;
      display: block;
    }
  `;

  /** The seam-map data to render. Defaults to the built-in atlas fixture. */
  @property({ attribute: false })
  data: SeamMap = DEFAULT_SEAM_MAP;

  /** Content state of the element. */
  @property({ attribute: 'map-state' })
  mapState: SeamMapState = 'idle';

  /** The currently-selected seam id. */
  @state()
  private _selectedSeamId: string | null = null;

  /** The selected routing-row index (debug+ lens). */
  @state()
  private _selectedRoutingIdx: number | null = null;

  // -----------------------------------------------------------------------
  // Helpers
  // -----------------------------------------------------------------------

  private _selectSeam(id: string): void {
    this._selectedSeamId = this._selectedSeamId === id ? null : id;
    this._selectedRoutingIdx = null;
  }

  private _handleSeamKeydown(e: KeyboardEvent, id: string): void {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      this._selectSeam(id);
    } else if (e.key === 'Escape') {
      this._selectedSeamId = null;
    }
  }

  private _handleRoutingRowClick(idx: number, seamId: string): void {
    this._selectedRoutingIdx = this._selectedRoutingIdx === idx ? null : idx;
    this._selectedSeamId = seamId;
  }

  private _getSeamById(id: string): SeamEntry | undefined {
    return this.data.seams.find(s => s.id === id);
  }

  /** Group seams by their group property, preserving order. */
  private _groupedSeams(): Array<{ group: SeamGroup; seams: SeamEntry[] }> {
    const groupMap = new Map<SeamGroup, SeamEntry[]>();
    for (const seam of this.data.seams) {
      if (!groupMap.has(seam.group)) {
        groupMap.set(seam.group, []);
      }
      groupMap.get(seam.group)!.push(seam);
    }
    return Array.from(groupMap.entries()).map(([group, seams]) => ({ group, seams }));
  }

  /** Compute the CSS grid-template-columns value. 1 title col + N device cols. */
  private _gridTemplateColumns(): string {
    const deviceCols = this.data.devices.map(() => 'minmax(2.75rem, 3.5rem)').join(' ');
    return `minmax(10rem, 14rem) ${deviceCols}`;
  }

  // -----------------------------------------------------------------------
  // Render helpers
  // -----------------------------------------------------------------------

  private _renderStateBanner(): TemplateResult {
    if (this.mapState === 'loading') {
      return html`
        <div class="state-banner" part="state-banner" role="status" aria-live="polite"
             data-testid="seam-map-loading">
          <slot name="loading-state">${msg('Loading seam map…')}</slot>
        </div>
      `;
    }
    if (this.mapState === 'error') {
      return html`
        <div class="state-banner" part="state-banner" role="alert"
             data-testid="seam-map-error">
          <slot name="error-state">${msg('Seam map data could not be loaded.')}</slot>
        </div>
      `;
    }
    if (this.mapState === 'empty') {
      return html`
        <div class="state-banner" part="state-banner" role="status"
             data-testid="seam-map-empty">
          <slot name="empty-state">${msg('No seam map data.')}</slot>
        </div>
      `;
    }
    return html``;
  }

  private _renderLegend(): TemplateResult {
    return html`
      <div class="legend" aria-label="${msg('Applicability legend')}">
        <span class="legend-item">
          <span class="dot" data-applicability="full" aria-hidden="true"></span>
          <span>${msg('Active')}</span>
        </span>
        <span class="legend-item">
          <span class="dot" data-applicability="partial" aria-hidden="true"></span>
          <span>${msg('Partial')}</span>
        </span>
        <span class="legend-item">
          <span class="dot" data-applicability="host" aria-hidden="true"></span>
          <span>${msg('Host')}</span>
        </span>
        <span class="legend-item">
          <span class="dot" data-applicability="none" aria-hidden="true"></span>
          <span>${msg('N/A')}</span>
        </span>
      </div>
    `;
  }

  private _renderColumnHeaders(devices: SeamDevice[]): TemplateResult {
    const isDebug = this.profile.lens === 'debug';
    return html`
      <div class="col-header-title" part="header" role="columnheader" aria-label="${msg('Seam')}">
        ${msg('Seam')}
      </div>
      ${devices.map(device => html`
        <div class="col-header" role="columnheader"
             title="${device.label} — L${device.capabilityLevel}"
             data-testid="${isDebug ? `col-${device.id}` : nothing}">
          ${device.label}
        </div>
      `)}
    `;
  }

  private _renderGroupHeading(group: SeamGroup): TemplateResult {
    const label = GROUP_LABELS[group]?.() ?? group;
    return html`
      <div class="group-heading-cell" role="rowheader" part="group-heading"
           aria-label="${label}">
        ${label}
      </div>
      <div class="group-heading-cell-span" aria-hidden="true"></div>
    `;
  }

  private _renderSeamRow(seam: SeamEntry, devices: SeamDevice[]): TemplateResult {
    const isSelected = this._selectedSeamId === seam.id;
    const isDebug = this.profile.lens === 'debug';
    const isMinimal = this.profile.lens === 'minimal';

    return html`
      <div
        class="seam-title-cell"
        part="seam-row seam-title"
        role="gridcell"
        tabindex="0"
        aria-selected="${isSelected}"
        aria-label="${seam.title} — ${msg('click to see detail')}"
        data-testid="${isDebug ? `seam-${seam.id}` : nothing}"
        title="${seam.title}"
        @click=${() => this._selectSeam(seam.id)}
        @keydown=${(e: KeyboardEvent) => this._handleSeamKeydown(e, seam.id)}
      >
        ${seam.title}
        ${isDebug ? html`<span class="debug-testid">seam-${seam.id}</span>` : nothing}
      </div>
      ${isMinimal
        ? devices.map(() => html`<div class="cell" part="cell" aria-hidden="true"></div>`)
        : devices.map(device => {
            const level = device.capabilityLevel;
            const applicability = seam.applicability[level] ?? 'none';
            const label = APPLICABILITY_LABELS[applicability]?.() ?? applicability;
            return html`
              <div
                class="cell"
                part="cell"
                role="gridcell"
                data-selected-row="${isSelected}"
                aria-label="${device.label}: ${label}"
                title="${device.label} — ${label}"
              >
                <span
                  class="dot"
                  data-applicability="${applicability}"
                  aria-hidden="true"
                ></span>
              </div>
            `;
          })
      }
    `;
  }

  private _renderDetailPanel(seam: SeamEntry): TemplateResult {
    return html`
      <section class="detail-panel" part="detail-panel" aria-label="${msg('Seam detail')}"
               data-testid="seam-detail-panel">
        <div class="detail-heading">${seam.title}</div>
        <dl class="detail-fields">
          <dt class="detail-label">${msg('Problem-class')}</dt>
          <dd class="detail-value">${seam.problemClass}</dd>

          <dt class="detail-label">${msg('Add a new X')}</dt>
          <dd class="detail-value">${seam.addNewX}</dd>

          <dt class="detail-label">${msg('Home')}</dt>
          <dd class="detail-value">${seam.home}</dd>

          <dt class="detail-label">${msg('Confusion to avoid')}</dt>
          <dd class="detail-value">${seam.confusion}</dd>
        </dl>
        <button
          class="detail-close"
          @click=${() => { this._selectedSeamId = null; }}
          aria-label="${msg('Close seam detail')}">
          ${msg('Close')}
        </button>
      </section>
    `;
  }

  private _renderTrackBands(): TemplateResult {
    return html`
      <div class="tracks" part="tracks" aria-label="${msg('Participation tracks')}">
        ${this.data.tracks.map(track => html`
          <div class="track-label-row" part="track-band" data-testid="track-${track.id}">
            <span class="track-badge" data-track="${track.id}">
              ${TRACK_LABELS[track.id]?.() ?? track.label}
            </span>
            <span class="track-desc">${track.description}</span>
          </div>
        `)}
      </div>
    `;
  }

  private _renderRoutingTable(): TemplateResult {
    return html`
      <div class="routing-table" part="routing-table">
        <div class="routing-table-heading">${msg('Concern-Routing Table')}</div>
        <div style="overflow-x: auto;">
          <table aria-label="${msg('Concern-routing table — find your concern to locate the seam')}">
            <thead>
              <tr>
                <th scope="col">${msg('Concern')}</th>
                <th scope="col">${msg('Seam')}</th>
                <th scope="col">${msg('Device-scale')}</th>
                <th scope="col">${msg('Home')}</th>
              </tr>
            </thead>
            <tbody>
              ${this.data.routing.map((row, idx) => {
                const seam = this._getSeamById(row.seamId);
                const isSelected = this._selectedRoutingIdx === idx;
                return html`
                  <tr
                    aria-selected="${isSelected}"
                    style="cursor: pointer;"
                    @click=${() => this._handleRoutingRowClick(idx, row.seamId)}
                    @keydown=${(e: KeyboardEvent) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault();
                        this._handleRoutingRowClick(idx, row.seamId);
                      }
                    }}
                    tabindex="0"
                  >
                    <td>${row.concern}</td>
                    <td>${seam?.title ?? row.seamId}</td>
                    <td>${row.deviceScale}</td>
                    <td>${row.home}</td>
                  </tr>
                `;
              })}
            </tbody>
          </table>
        </div>
      </div>
    `;
  }

  // -----------------------------------------------------------------------
  // Main render
  // -----------------------------------------------------------------------

  override render(): TemplateResult {
    const lens = this.profile.lens;
    const isMinimal = lens === 'minimal';
    const showApplicability = !isMinimal;
    const showDetail = ['standard', 'detail', 'debug'].includes(lens);
    const showRoutingTable = ['detail', 'debug'].includes(lens);
    const showTracks = !isMinimal;

    if (this.mapState !== 'idle') {
      return html`
        <div class="container" part="container" data-testid="elohim-seam-map">
          ${this._renderStateBanner()}
        </div>
      `;
    }

    const grouped = this._groupedSeams();
    const devices = this.data.devices;
    const selectedSeam = this._selectedSeamId
      ? this._getSeamById(this._selectedSeamId)
      : undefined;

    // Build grid-template-columns
    const gridStyle = `grid-template-columns: ${this._gridTemplateColumns()}`;

    return html`
      <div class="container" part="container" data-testid="elohim-seam-map" tabindex="-1">
        ${showApplicability ? this._renderLegend() : nothing}

        <div class="scroll-wrapper">
          <div
            class="grid"
            part="grid"
            role="grid"
            style="${gridStyle}"
            aria-label="${msg('Elohim seam map — rows are seams, columns are devices. Click a seam row to see its detail.')}"
          >
            <!-- Column headers -->
            <div class="header-row" role="row">
              ${this._renderColumnHeaders(devices)}
            </div>

            <!-- Grouped seam rows -->
            ${grouped.map(({ group, seams }) => html`
              <div class="group-heading" role="row">
                ${this._renderGroupHeading(group)}
              </div>
              ${seams.map(seam => html`
                <div class="seam-row" role="row">
                  ${this._renderSeamRow(seam, showApplicability ? devices : [])}
                </div>
              `)}
            `)}
          </div>
        </div>

        <!-- Participation track bands -->
        ${showTracks ? this._renderTrackBands() : nothing}

        <!-- Detail panel for selected seam -->
        ${showDetail && selectedSeam
          ? this._renderDetailPanel(selectedSeam)
          : nothing}

        <!-- Routing table (detail+ lens) -->
        ${showRoutingTable ? this._renderRoutingTable() : nothing}
      </div>
    `;
  }
}
