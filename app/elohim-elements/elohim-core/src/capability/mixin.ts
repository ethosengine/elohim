/**
 * CapabilityAwareElement mixin — opt-in base for elements that observe the Capability Profile.
 *
 * Wires the consumer side of capabilityProfileContext with subscribe=true so consumers
 * re-render when the provider updates.
 *
 * Usage: extend `CapabilityAwareElement(LitElement)` instead of `LitElement`.
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §3.1
 */

import type { LitElement, PropertyDeclarations } from 'lit';

import { ContextConsumer } from '@lit/context';

import { capabilityProfileContext } from './context.js';
import { DEFAULT_PROFILE } from './profile.js';

import type { CapabilityProfile } from './profile.js';

type Constructor<T = object> = new (...args: any[]) => T;

export interface CapabilityAware {
  profile: CapabilityProfile;
}

export function CapabilityAwareElement<TBase extends Constructor<LitElement>>(
  Base: TBase
): TBase & Constructor<CapabilityAware> {
  class Mixed extends Base {
    static properties: PropertyDeclarations = {
      ...(Base as unknown as { properties?: PropertyDeclarations }).properties,
      profile: { attribute: false, state: true },
    };

    profile: CapabilityProfile = DEFAULT_PROFILE;

    constructor(...args: any[]) {
      super(...args);
      new ContextConsumer(this, {
        context: capabilityProfileContext,
        callback: (value: CapabilityProfile) => {
          this.profile = value;
          this.requestUpdate();
        },
        subscribe: true,
      });
    }
  }
  return Mixed as TBase & Constructor<CapabilityAware>;
}
