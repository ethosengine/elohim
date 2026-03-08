/**
 * ImagoDei Interfaces — Abstract contracts for identity services.
 *
 * These interfaces enable inversion of control:
 * - IPresenceLifecycle + PRESENCE_LIFECYCLE token → swap presence management strategy
 * - IStewardshipPolicy + STEWARDSHIP_POLICY token → swap capability management strategy
 */

export { PRESENCE_LIFECYCLE } from './presence.interface';
export type { IPresenceLifecycle } from './presence.interface';
export { STEWARDSHIP_POLICY } from './stewardship-policy.interface';
export type { IStewardshipPolicy } from './stewardship-policy.interface';
export { IDENTITY_API } from './identity.interface';
export type { IIdentityApi } from './identity.interface';
