/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/render-capability-profile.schema.json -- DO NOT EDIT */

/**
 * Kind of server-side renderer a doorway carries. Reserved values are valid claim values; only `angular-ssr` is implemented in elohim-render today. Source of truth: doorway runtime (Category C operational). Not a DHT entry type.
 */
export type RendererKind = 'angular-ssr' | 'react-rsc' | 'vue-ssr' | 'svelte-ssr' | 'lit-ssr' | 'static-html';
/**
 * Kind of server-side renderer a doorway carries. Reserved values are valid claim values; only `angular-ssr` is implemented in elohim-render today. Source of truth: doorway runtime (Category C operational). Not a DHT entry type.
 */
export type RendererKind1 = 'angular-ssr' | 'react-rsc' | 'vue-ssr' | 'svelte-ssr' | 'lit-ssr' | 'static-html';

/**
 * Source of truth: auto-derived at doorway startup from on-disk bundles intersected with elohim-storage's manifest of SSR-eligible routes (Operational, Category C). doorway-config.toml may reduce the claim but never inflate it. Layered into PeerStatusView via build_peer_status_view, mirroring the elohimCapability pattern. NOT a DHT entry.
 */
export interface RenderCapabilityProfile {
  /**
   * @minItems 1
   */
  bundles: [
    {
      /**
       * Bundle name (e.g. lamad-app, qahal-app)
       */
      name: string;
      /**
       * Semver bundle version
       */
      version: string;
      renderer: RendererKind;
      /**
       * Optional sha256 hash of the bundle file
       */
      digest?: string | null;
    },
    ...{
      /**
       * Bundle name (e.g. lamad-app, qahal-app)
       */
      name: string;
      /**
       * Semver bundle version
       */
      version: string;
      renderer: RendererKind;
      /**
       * Optional sha256 hash of the bundle file
       */
      digest?: string | null;
    }[],
  ];
  /**
   * Distinct renderer kinds (deduplicated bundles[].renderer). Cheap-to-query summary.
   *
   * @minItems 1
   */
  renderers: [RendererKind1, ...RendererKind1[]];
  /**
   * Auth modes this doorway honors for SSR. 'anonymous' must always be present.
   *
   * @minItems 1
   */
  authModes: [
    'anonymous' | 'doorway-hosted' | 'steward-presence',
    ...('anonymous' | 'doorway-hosted' | 'steward-presence')[],
  ];
  /**
   * Operator-declared concurrency budget. CSR fallback fires when reached.
   */
  maxConcurrentRenders: number;
  /**
   * Operator-declared memory ceiling for the renderer (informational; null = cgroup-managed)
   */
  memoryBudgetMib?: number | null;
}
