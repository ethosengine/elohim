/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: enums/substrate-signal.schema.json -- DO NOT EDIT */

/**
 * Protocol substrate signal categories. Every app signal maps to one of these primitives. These are the resource dimensions the protocol tracks and governs.
 */
export type SubstrateSignal =
  | 'attention'
  | 'compute'
  | 'storage'
  | 'bandwidth'
  | 'energy'
  | 'time'
  | 'resource';
