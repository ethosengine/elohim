/**
 * Human model — represents a person with identity and devices.
 *
 * In test scenarios, a Human has a name (for readability), credentials,
 * and one or more Devices they use to interact with doorways.
 */

import type { Device } from './device.js';

export interface HumanCredentials {
  identifier: string;
  password: string;
  displayName: string;
}

export class Human {
  readonly name: string;
  readonly credentials: HumanCredentials;
  readonly devices: Device[] = [];
  readonly tokens = new Map<string, string>();

  agentPubKey?: string;
  humanId?: string;

  constructor(name: string, credentials: HumanCredentials) {
    this.name = name;
    this.credentials = credentials;
  }

  addDevice(device: Device): void {
    this.devices.push(device);
  }

  /** Store a JWT for a specific doorway */
  setToken(doorwayId: string, token: string): void {
    this.tokens.set(doorwayId, token);
  }

  getToken(doorwayId: string): string | undefined {
    return this.tokens.get(doorwayId);
  }
}
