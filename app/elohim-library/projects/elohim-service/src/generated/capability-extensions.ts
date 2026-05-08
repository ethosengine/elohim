/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/capability-extensions.schema.json -- DO NOT EDIT */

/**
 * Tier-2 capability claims map. Source of truth: each capability's owner declares it via runtime registration (Category C operational). Each key is a kebab-case capability name registered in the protocol's capability registry. Each value carries a schemaRef pointer (so consumers can resolve the profile schema) and an opaque structured profile. The validator checks structural well-formedness only — claim CONTENTS are interpreted by consumers who recognize the capability name. When a Tier-2 capability proves load-bearing, it graduates to Tier 1 with a typed sibling field on peer-status. NOT a DHT entry.
 */
export interface CapabilityExtensions {
  /**
   * This interface was referenced by `CapabilityExtensions`'s JSON-Schema definition
   * via the `patternProperty` "^[a-z][a-z0-9-]{2,30}$".
   */
  [k: string]: {
    /**
     * Schema URI for this capability's profile (e.g., 'epr:schema:view:transcode-capability-profile')
     */
    schemaRef: string;
    /**
     * Capability-specific claim. Shape defined by schemaRef. Validator checks 'is an object'; consumers do deep validation.
     */
    profile: Record<string, unknown>;
  };
}
