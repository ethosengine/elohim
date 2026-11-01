/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/chrome-context.schema.json -- DO NOT EDIT */

/**
 * device-agent: the person's own key is on this device (grants live on their chain). hosted-session: a validated session on a doorway that hosts their agent (grants held by the browser until a chain write exists). visitor: no validated identity (grants held by the browser).
 */
export type ViewerKind = 'device-agent' | 'hosted-session' | 'visitor';
/**
 * Which medium mounted the chrome. web2-doorway: an ordinary browser through a doorway (only this medium needs DNS for isolation). peer-native: the person's own storage peer served the shell with no doorway. desktop: the desktop shell.
 */
export type ChromeMedium = 'web2-doorway' | 'peer-native' | 'desktop';
/**
 * Content format for rendering. Core formats are DNA-notarized. Extended formats are storage-level rendering hints.
 */
export type ContentFormat =
  | 'markdown'
  | 'html'
  | 'video'
  | 'audio'
  | 'interactive'
  | 'external'
  | 'epr-composite'
  | 'plaintext'
  | 'text'
  | 'plain'
  | 'gherkin'
  | 'perseus'
  | 'perseus-json'
  | 'perseus-quiz-json'
  | 'video-embed'
  | 'audio-file'
  | 'html5-app'
  | 'spa-bundle'
  | 'human-json'
  | 'organization-json'
  | 'json'
  | 'sophia'
  | 'sophia-quiz-json'
  | 'element-registry-manifest';
/**
 * Data, never inferred from bytes: trustful when the content has an Elohim Protocol record (every record served through /apps today); trustless is reserved for non-protocol content projected inward. Narrowed reach never demotes a record to trustless.
 */
export type ContentTrustMode = 'trustful' | 'trustless';
/**
 * The record's declared reach as projected (display only; reach gates are enforced elsewhere).
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
 * Derived, never written: green iff the record's dht_anchor_hash is present in the projection, else amber (served as soon as the bytes verify, awaiting the network's witness). Amber content still runs in trustful mode. The chrome re-reads it when page info opens.
 */
export type WitnessState = 'amber' | 'green';
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
 * origin-pair: content on the public name's paired apps origin (web2). scheme-per-record: a per-record custom scheme (desktop). separate-webview: content in its own webview (desktop). opaque-sandbox: an opaque origin (trustless). none: content shares the chrome's origin.
 */
export type IsolationBoundary =
  | 'origin-pair'
  | 'scheme-per-record'
  | 'separate-webview'
  | 'opaque-sandbox'
  | 'none';
/**
 * One hop. A peer is named only by its key; human labels come from the person's own roster, never from a server's configuration.
 */
export type DeliveryHop = SelfHop | HolderHop | DoorwayHop;
/**
 * Which party in the path reported this hop. A hop reported by a party other than itself is hearsay, and the Delivery card says so.
 */
export type HopReporter = 'self' | 'holder' | 'doorway';
export type DeliveryTransport = 'iroh' | 'libp2p' | 'http';

/**
 * Source of truth: rebuilt per request from the EPR record's projection and the facts of this one delivery (Ephemeral, Category C). Never persisted, never notarized, never stored by any medium. The context island the EPR chrome (<elohim-omni>, elohim/elohim-chrome-asset/src/omni-element.js) reads from <script type="application/json" id="elohim-omni-context">. Every medium mounts the same element from this one shape: the web2 doorway shell, a storage peer's own /apps shell (peer-native), the native Angular app through the EPR router, and the desktop shell. Authority: nothing here is decided by whichever server wrote the island. Every reading is a projection of what the record and the network say (the record, its reach, its declared capabilities, its DHT witness, the holders reading); delivery facts are each reported by the hop they describe and confer no authority. Unknown facts are OMITTED, never guessed. Page info: the chrome's rows and the EPR page's sections share one vocabulary (../enums/page-info-section.schema.json); every card ends at /epr/<recordId>#<section>. Grants: a person's narrowing or granting of a frame capability is never part of this island. Where the key is not on the person's device (visitor, hosted session) the chrome keeps grants in the browser on the chrome origin under the key `elohim-grants:<contentAddress>` (see $defs/grantStorageKey), as a JSON object `{ "<frameCapability>": true | false }` — `false` narrows an ordinary capability, `true` grants a device one. Keying by content address means a new bundle (a new declared set) starts from the record's declaration again. A device-agent keeps grants on its own chain (Private, Category B), not in this key. Rust producer: elohim-chrome-asset (ChromeContext / ContentIsland), validated by that crate's schema-contract test; TS consumer: generated chrome-context.ts.
 */
export interface ChromeContext {
  /**
   * Island shape version. Bump when a chrome could misread an older island.
   */
  version: 1;
  /**
   * The EPR address the chrome is mounted for (the record id on a content shell). Always supplied; the element falls back to window.location only when no island exists at all.
   */
  slug: string;
  viewer: ChromeViewer;
  medium: ChromeMedium;
  /**
   * @deprecated
   * DEPRECATED — superseded by viewer.kind and kept for one release so the pre-v1 element keeps working: true exactly when viewer.kind is device-agent or hosted-session. A producer that emits it must derive it from viewer; a consumer must read viewer.
   */
  authenticated?: boolean;
  /**
   * Base URL the chrome resolves its projection fetches against (record, resilience, claims, nav-context). Absent means same-origin with the page. Present on media whose page origin is not the projection's origin (e.g. a desktop shell).
   */
  readingsBase?: string;
  /**
   * The record's human title.
   */
  title?: string;
  /**
   * The ambient theme the chrome adopts: a bare colour scheme ('light' | 'dark' | anything else = auto) or a theme hint object. Absent lets the chrome use its base palette.
   */
  theme?: string | ThemeHint;
  /**
   * Short build stamp of the bundle shown (e.g. its blob hash prefix).
   */
  buildMarker?: string;
  /**
   * Deployment tier label (e.g. alpha, staging). Shown only when showEnv is true.
   */
  envTier?: string;
  showEnv?: boolean;
  /**
   * Where the account link goes for a signed-in viewer. Relative to the page.
   */
  accountHref?: string;
  showThemeToggle?: boolean;
  navBack?: ChromeNavLink;
  navForward?: ChromeNavLink;
  receipt?: ChromeReceiptLink;
  content?: ContentIsland;
}
/**
 * Who is looking, as far as this medium can know without trusting the content. Validated by the producer (a header's presence is not a session).
 */
export interface ChromeViewer {
  kind: ViewerKind;
}
export interface ThemeHint {
  colorScheme?: 'light' | 'dark' | 'auto';
  /**
   * The page's theme-color, as declared by the content.
   */
  themeColor?: string;
  /**
   * Chrome palette tokens (bg, fg, muted, border, accent, shadow, envRing) for the scheme named by colorScheme.
   */
  tokens?: {
    [k: string]: string;
  };
}
export interface ChromeNavLink {
  href: string;
  label?: string;
}
/**
 * Present only when the response being framed accounted for an exchange; an always-present receipt would assert an exchange nobody recorded.
 */
export interface ChromeReceiptLink {
  /**
   * Where the full receipt is read — relative.
   */
  href: string;
  sentence?: string;
  label?: string;
}
/**
 * Present only on a content shell (a page that frames a record's bytes). Absent on the landing page, EPR pages and the app, where page info claims nothing it cannot back: no witness, no Delivery or Permissions rows.
 */
export interface ContentIsland {
  /**
   * The EPR record id; /epr/<recordId> is its governing page.
   */
  recordId: string;
  /**
   * The bundle's content address exactly as the record names it (e.g. sha256-<hex>). Grants are keyed by it.
   */
  contentAddress: string;
  format: ContentFormat;
  /**
   * The bundle's entry file (contentBody.entryPoint, default index.html).
   */
  entry: string;
  mode: ContentTrustMode;
  reach?: Reach;
  witness: WitnessState;
  /**
   * What the record declares (html5-app contentBody.capabilities), in declared order; [] when none. The frame gets declared ∩ admitted ∩ granted.
   */
  capabilities: FrameCapability[];
  /**
   * The holders reading. null when no reading was available to this producer (a single provenance: unknown — zero holders is holders: 0, never null).
   */
  holders: HolderReading | null;
  frame: ContentFrame;
  isolation: ContentIsolation;
  delivery: ContentDelivery;
}
export interface HolderReading {
  /**
   * Households holding the bytes, as read.
   */
  holders: number;
  /**
   * How many the record's resilience asks for.
   */
  target: number;
  /**
   * The fold's own sentence (FeltStatusView.headline) when the reading came with one; clients add only the unknown-state sentence.
   */
  headline?: string;
  /**
   * When the reading was taken. Lets an offline device say how old its knowledge is.
   */
  asOf?: string;
}
/**
 * The chrome builds the iframe from this after it mounts; the shell itself contains no iframe.
 */
export interface ContentFrame {
  /**
   * What the frame loads: on an origin pair, the content origin's /_elohim/boot?to=<entry path>; otherwise the same-origin /apps/<slug>/<entry>.
   */
  src: string;
  /**
   * The frame's accessible title.
   */
  title: string;
}
/**
 * What separates the content from the chrome and the person's session in this medium. The chrome withholds allow-same-origin when boundary is none, and the Permissions card says why.
 */
export interface ContentIsolation {
  boundary: IsolationBoundary;
  /**
   * The origin the content runs on, when it differs from the page's (e.g. https://apps.alpha.elohim.host).
   */
  contentOrigin?: string;
}
/**
 * How these bytes reached this person, hop by hop, each hop reported by a named party. Facts, not authority.
 */
export interface ContentDelivery {
  /**
   * Ordered from the person outward: self first when the person's own peer is in the path, then the doorway (if any), then the holder that served the bytes.
   *
   * @minItems 1
   */
  path: [DeliveryHop, ...DeliveryHop[]];
  checked?: DeliveryCheck;
  /**
   * Route that reads this delivery's receipt back, when one was recorded (relative).
   */
  receipt?: string;
}
export interface SelfHop {
  role: 'self';
  reportedBy: HopReporter;
}
export interface HolderHop {
  role: 'holder';
  /**
   * The holding peer's agent or peer key.
   */
  peer: string;
  transport?: DeliveryTransport;
  reportedBy: HopReporter;
}
export interface DoorwayHop {
  role: 'doorway';
  /**
   * The public name the request came in on (e.g. alpha.elohim.host).
   */
  name: string;
  /**
   * This doorway instance's id.
   */
  id: string;
  /**
   * This doorway's DID, when it serves one.
   */
  did?: string;
  reportedBy: HopReporter;
}
/**
 * Present only when the verifier itself reported checking the bytes; never asserted on its behalf.
 */
export interface DeliveryCheck {
  by: 'peer' | 'worker';
  /**
   * The content address the bytes were checked against.
   */
  against: string;
}
