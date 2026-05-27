/**
 * Lamad-local identity utilities.
 *
 * P-disposition (Slice 2.3 residual): isNetworkMode is a pure function that only
 * depends on the identity mode string union. Copying it here keeps lamad free of
 * the @app/imagodei/models cross-pillar import while keeping the logic in one small
 * place that's easy to audit for drift against the imagodei original.
 *
 * If the IdentityMode enum ever gains new network-connected modes, update both
 * the imagodei original and this copy together.
 */

import type { LamadIdentityMode } from '../interfaces/cross-pillar.interface';

/**
 * Returns true when the identity mode represents a network-authenticated state
 * (hosted — custodial keys on edge node; steward — local device keys).
 */
export function isNetworkMode(mode: LamadIdentityMode): boolean {
  return mode === 'hosted' || mode === 'steward';
}
