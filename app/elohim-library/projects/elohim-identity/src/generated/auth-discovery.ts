/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/auth-discovery.schema.json -- DO NOT EDIT */

/**
 * An origin-relative path. Must begin with a single '/' — a protocol-relative '//host/x' names ANOTHER origin while passing a naive leading-slash check, so it is excluded by pattern rather than by convention.
 */
export type RelativePath = string;

/**
 * Source of truth: doorway operational configuration (Operational, Category C). GET /.well-known/elohim-auth — the unauthenticated document an app reads INSTEAD of carrying auth configuration, so a page needs to know only the origin it was served from. Reconstructed per request from the doorway's own route table; never persisted, never notarized. EVERY location it names is an origin-RELATIVE path: a discovery document that could name another origin would be an open-redirect primitive, and this shape makes a foreign origin unexpressible rather than merely rejected at the client. Deliberately excludes portalHostUrl — a graduated steward's portal does live on another origin, but it is per-human, arrives on GET /auth/me, and must be attested in that human's own record rather than advertised publicly. Rust wire authority: doorway/doorway-service/src/routes/auth_discovery.rs AuthDiscovery (validated by that crate's tests/schema_contract.rs).
 */
export interface AuthDiscovery {
  /**
   * Document shape version. Bump when a client could misread an older body.
   */
  version: number;
  /**
   * Which doorway answered. Omitted (never null) when the doorway carries no configured id, so a client can branch on presence without a null check.
   */
  doorwayId?: string;
  /**
   * Where to SEND a human to sign in — the doorway-hosted portal page.
   */
  portal: string;
  /**
   * The auth endpoints this doorway serves. Every entry is an exact member of AUTH_OWNED_PATHS in doorway-service/src/server/http.rs; advertising a path that is not owned would answer with the SPA shell instead, which is the advertise/serve asymmetry this document must never introduce.
   */
  endpoints: {
    register: RelativePath;
    login: RelativePath;
    logout: RelativePath;
    refresh: RelativePath;
    me: RelativePath;
    /**
     * An origin-relative path. Must begin with a single '/' — a protocol-relative '//host/x' names ANOTHER origin while passing a naive leading-slash check, so it is excluded by pattern rather than by convention.
     */
    authorize: string;
    /**
     * An origin-relative path. Must begin with a single '/' — a protocol-relative '//host/x' names ANOTHER origin while passing a naive leading-slash check, so it is excluded by pattern rather than by convention.
     */
    token: string;
    /**
     * An origin-relative path. Must begin with a single '/' — a protocol-relative '//host/x' names ANOTHER origin while passing a naive leading-slash check, so it is excluded by pattern rather than by convention.
     */
    sessionToken: string;
    /**
     * An origin-relative path. Must begin with a single '/' — a protocol-relative '//host/x' names ANOTHER origin while passing a naive leading-slash check, so it is excluded by pattern rather than by convention.
     */
    exchangeSession: string;
    /**
     * An origin-relative path. Must begin with a single '/' — a protocol-relative '//host/x' names ANOTHER origin while passing a naive leading-slash check, so it is excluded by pattern rather than by convention.
     */
    portalHost: string;
  };
}
