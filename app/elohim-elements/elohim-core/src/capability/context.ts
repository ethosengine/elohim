/**
 * Lit contexts for Capability Profile and Content Certainty.
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §3.1, §4.2
 */

import { createContext } from '@lit/context';

import type { CapabilityProfile } from './profile.js';
import type { ContentCertainty } from './certainty.js';

export const capabilityProfileContext = createContext<CapabilityProfile>(
  Symbol.for('elohim-capability-profile'),
);

export const contentCertaintyContext = createContext<ContentCertainty>(
  Symbol.for('elohim-content-certainty'),
);
