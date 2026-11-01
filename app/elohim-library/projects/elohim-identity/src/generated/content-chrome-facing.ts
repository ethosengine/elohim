/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/content-chrome-facing.schema.json -- DO NOT EDIT */

/**
 * Content reach/visibility level. Ordered from most restrictive to most open. Source of truth: DNA-notarized CORE_REACH_LEVELS constant in content_store_integrity zome. Category A — enumeration values are part of the protocol vocabulary enforced by gateways without parsing payload.
 */
export type Reach =
  | 'private'
  | 'self'
  | 'intimate'
  | 'trusted'
  | 'familiar'
  | 'community'
  | 'public'
  | 'commons';
/**
 * What content framed by the EPR chrome may be given beyond running its own scripts. A record declares the capabilities it requests (html5-app contentBody.capabilities); the chrome gives the declared set within the standing ceiling, as the person narrows or grants it. Scripts are never a capability: a trustful frame always runs scripts, and whether it also keeps its own origin (allow-same-origin) is decided by the chrome from mode and isolation, never declared here. Each value carries three annotations the generators project into the chrome's table and the doorway's Permissions-Policy ceiling: x-sandbox (the iframe sandbox token it adds, or null), x-allow (the Permissions-Policy feature it enables in the iframe allow attribute, or null), x-device (true when it reaches the person's device and so is only ever given by the person's own grant). Source of truth: this schema (vocabulary only, Category C) — it stores nothing and is not notarized; the declared set lives in the html5-app record's contentBody, which the integrity zome treats as opaque, so this vocabulary never moves a DNA hash.
 */
export type FrameCapability =
  | 'forms'
  | 'popups'
  | 'downloads'
  | 'fullscreen'
  | 'clipboard-write'
  | 'camera'
  | 'microphone'
  | 'geolocation';

/**
 * Every reading the protocol chrome (chip and page-info cards) shows for one EPR content record — GET /api/v1/epr/{id}/chrome, and the island of the storage peer's own /apps shell. Folded by elohim_facings::folds::content_chrome from the record and the projections it composes (holders: the resilience snapshot via build_felt_status; claims: attestations by subject; legs: epr_coupling; steward: stewardship allocations). Source of truth: the Notarized content record (DHT, Category A) and those projections; this view itself is Ephemeral (Operational, Category C) — recomputed per request, never stored. Not-selected contract: a lens the facing did not compute is ABSENT from the wire, never null and never zero. That absence has one provenance — this peer did not take that reading (holders: no snapshot; legs: no EPR atom names the record; app: not an html5-app) — so a bare optional is honest (CONVENTIONS rule 11). claims and steward are always selected: an empty list is a measured 'none recorded'.
 */
export interface ContentChromeFacingView {
  /**
   * The record id — the EPR address the chrome links back to (/epr/<id>).
   */
  recordId: string;
  title: string;
  /**
   * The record's contentFormat, verbatim (a lamad manifest format, e.g. html5-app).
   */
  format: string;
  /**
   * Mirrors chrome-context contentIsland.mode: data, never inferred from bytes. trustful for every protocol record this facing reads.
   */
  mode: 'trustful' | 'trustless';
  reach: Reach;
  /**
   * Mirrors chrome-context contentIsland.witness: green once a conductor other than the record's author holds the record as valid (and the record carries a non-blank dht_anchor_hash); amber until then, and whenever the serving peer cannot read that in time. The anchor alone is not a witness. Derived on every read, never written.
   */
  witness: 'amber' | 'green';
  /**
   * RFC 3339 time this peer folded the facing.
   */
  computedAt: string;
  /**
   * The bundle's content address as CIDv1 (bafkrei…, the X-Content-Address spelling). ABSENT (never null) when the record names no bytes.
   */
  contentAddress?: string;
  app?: ContentChromeAppLens;
  holders?: ContentChromeHoldersLens;
  claims?: ContentChromeClaimsLens;
  /**
   * The record's EPR coupling legs (epr_content fold_coupling_legs; the shape of epr-raw-view coupling). ABSENT (never null) when no EPR atom names the record; within it a leg with no coupling is null.
   */
  legs?: {
    knowledge?: string | null;
    value?: string | null;
    governance?: string | null;
  };
  steward?: ContentChromeStewardLens;
}
/**
 * An app bundle's body declarations (html5-app, and spa-bundle — a wrapper's format, whose body is the same app-bundle shape), read by lamad-types Html5AppBody's lenient parser (an unknown capability is never granted). presents marks a wrapper. ABSENT (never null) for any other format.
 */
export interface ContentChromeAppLens {
  /**
   * The entry file, declared or index.html.
   */
  entry: string;
  /**
   * Declared frame capabilities in declared order; [] when none.
   */
  capabilities: FrameCapability[];
  /**
   * true when the bundle is a wrapper that presents other content and fulfils the chrome's claims.
   */
  presents: boolean;
  /**
   * Origins (scheme://host[:port]) the record declares the app may reach beyond its own (html5-app-body sources, normalized by lamad-types) — what its content is observed against with a Report-Only policy; not a grant. ABSENT (never null, never []) when the record declares none.
   *
   * @minItems 1
   */
  sources?: [string, ...string[]];
  themeHint?: ContentChromeThemeHint;
  /**
   * Where the same work lives outside the protocol. ABSENT (never null) when undeclared.
   */
  fallbackUrl?: string;
}
/**
 * The ambient theme the bundle asks for. ABSENT (never null) when undeclared.
 */
export interface ContentChromeThemeHint {
  colorScheme?: 'light' | 'dark';
  themeColor?: string;
}
/**
 * The holders reading, worded by the resiliency fold (build_felt_status) — the same snapshot the trust-mark dial reads. ABSENT (never null) when this peer could not take the snapshot.
 */
export interface ContentChromeHoldersLens {
  /**
   * Households holding the record now (0 is a reading, never absent).
   */
  holders: number;
  /**
   * Households the record's resilience floor wants.
   */
  target: number;
  /**
   * The fold's own sentence, verbatim (FeltStatusView.headline).
   */
  headline: string;
  reassurance: 'protected' | 'watching' | 'needs-help' | 'not-yet-seen';
  /**
   * RFC 3339 time this peer took the reading.
   */
  asOf: string;
  /**
   * Distinct collectives holding the record under an ACTIVE provide commitment covering its reach — held by notarized promise, not only by bytes (the trust mark's sealed state). The same snapshot's resilience-snapshot-view commitmentBackedCollectives.
   */
  commitmentBackedCollectives: number;
  /**
   * The collectives holding the record (the shape of resilience-snapshot-view feltStatus.heldBy items).
   */
  heldBy: {
    id: string;
    kind: string;
    /**
     * Display label; absent when the collective has none.
     */
    label?: string;
    /**
     * Distinct agents within the collective holding the record; absent when not selected.
     */
    intraHubPeers?: number;
  }[];
}
/**
 * The Claims reading: attestations whose subject is the record (asked by record id and by bundle CID, as the EPR page asks), newest first. Always selected; [] is a measured 'none recorded'.
 */
export interface ContentChromeClaimsLens {
  attestations: ContentChromeClaim[];
}
export interface ContentChromeClaim {
  /**
   * The attestation's CID.
   */
  id: string;
  /**
   * attestation:<subtype>.
   */
  kind: string;
  /**
   * Who made the claim.
   */
  issuerCid: string;
  /**
   * witness | self-attest | audit-signature | computational.
   */
  proofClass: string;
  title: string;
  createdAt: string;
  /**
   * true when withdrawn; kept listed so the withdrawal is visible.
   */
  revoked: boolean;
  /**
   * Expiry, when the attestation carries one; absent otherwise.
   */
  expiresAt?: string;
}
/**
 * Who stewards the record: active stewardship allocations, largest share first. Always selected; [] is a measured 'none recorded'.
 */
export interface ContentChromeStewardLens {
  stewards: ContentChromeSteward[];
}
export interface ContentChromeSteward {
  /**
   * The steward's contributor-presence id.
   */
  presenceId: string;
  contributionType: string;
  allocationRatio: number;
  /**
   * The allocation's governance state (disputed when contested).
   */
  governanceState: string;
  /**
   * The presence's display name when this peer holds it; absent otherwise.
   */
  displayName?: string;
}
