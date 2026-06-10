/**
 * BrowserDevice — simulates a browser interacting with a doorway via HTTP.
 *
 * Session bookkeeping lives in the client's session store (the composed
 * @elohim/identity DoorwaySessionClient) — the device exposes read-only views.
 */

import {
  DoorwayClient,
  type AuthResponse,
  type RegisterRequest,
  type LoginRequest,
} from '../api/doorway-client.js';
import { Device, type DeviceType } from '../device.js';

export class BrowserDevice extends Device {
  readonly type: DeviceType = 'browser';
  readonly label: string;
  readonly client: DoorwayClient;

  constructor(label: string, doorwayUrl: string) {
    super();
    this.label = label;
    this.client = new DoorwayClient(doorwayUrl);
  }

  get isAuthenticated(): boolean {
    return this.client.session !== null;
  }

  get token(): string | undefined {
    return this.client.session?.token;
  }

  get agentPubKey(): string | undefined {
    return this.client.session?.agentPubKey;
  }

  get humanId(): string | undefined {
    return this.client.session?.humanId;
  }

  async register(req: RegisterRequest): Promise<AuthResponse> {
    return this.client.register(req);
  }

  async login(req: LoginRequest): Promise<AuthResponse> {
    return this.client.login(req);
  }

  async logout(): Promise<void> {
    await this.client.logout();
  }
}
