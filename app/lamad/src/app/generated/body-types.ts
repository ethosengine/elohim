// AUTO-GENERATED from lamad manifest + companion schemas.
// DO NOT EDIT — regenerate with: pnpm run lamad:codegen

export interface Html5AppThemeHint {
  colorScheme?: 'light' | 'dark';
  /** A CSS colour, as the entry HTML's theme-color would declare it. */
  themeColor?: string;
}

export interface Html5AppBody {
  /** The app id: the bundle's URL namespace, /apps/<slug>/<entryPoint>. Distinct from the record id and from the Holochain app id (hAppId). */
  slug: string;
  /** The bundle file a visit enters at, relative to the bundle root. */
  entryPoint?: string;
  /** Where the same work lives outside the protocol, offered as an 'open elsewhere' link. Never framed in its place. */
  fallbackUrl?: string;
  /** What the frame may do beyond running its own scripts, as requested by the record. Scripts are always allowed and are never declared; whether the frame keeps its own origin is the chrome's decision from mode and isolation. Device capabilities (camera, microphone, geolocation) are only ever given by the person's own grant. */
  capabilities?: ('forms' | 'popups' | 'downloads' | 'fullscreen' | 'clipboard-write' | 'camera' | 'microphone' | 'geolocation')[];
  /** Origins the app may reach beyond its own (fetch, scripts, images, sockets), as the record declares them. Trustful content is observed against this list, not blocked: the chrome's web edge serves the content with a Report-Only policy naming these origins, and anything the content reaches outside them is reported to the peer as an undeclared-behaviour signal. Empty or absent means the app declares no outside origin. */
  sources?: string[];
  themeHint?: Html5AppThemeHint;
  /** true when this bundle is a wrapper that presents other content inside its own trust surface and fulfils the chrome's claims for it (as lamad's content viewer does). Only inside a declared wrapper does the chrome come off. */
  presents?: boolean;
}

export interface Section {
  id: string;
  title?: string;
  description?: string;
  level?: 'course' | 'unit' | 'lesson' | 'item';
  estimatedDuration?: string;
  optional?: boolean;
  items?: Item[];
  sections?: Section[];
}

export interface Item {
  /** EPR reference (e.g. 'epr:concept-id' or bare 'concept-id') */
  ref: string;
  role?: 'step' | 'checkpoint' | 'optional' | 'reflection';
  title?: string;
  narrative?: string;
  learningObjectives?: string[];
  completionCriteria?: { type?: 'view' | 'score' | 'time' | 'interaction'; threshold?: number };
}

export interface EprCompositeBody {
  schemaVersion?: number;
  pathType?: string;
  layout?: 'sequential' | 'branching' | 'exploratory';
  sections: Section[];
}
