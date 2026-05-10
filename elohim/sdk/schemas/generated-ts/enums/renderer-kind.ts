/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: enums/renderer-kind.schema.json -- DO NOT EDIT */

/**
 * Kind of server-side renderer a doorway carries. Reserved values are valid claim values; only `angular-ssr` is implemented in elohim-render today. Source of truth: doorway runtime (Category C operational). Not a DHT entry type.
 */
export type RendererKind =
  | 'angular-ssr'
  | 'react-rsc'
  | 'vue-ssr'
  | 'svelte-ssr'
  | 'lit-ssr'
  | 'static-html';
