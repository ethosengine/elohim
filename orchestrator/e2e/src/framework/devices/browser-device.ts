/**
 * BrowserDevice — simulates a browser interacting with a doorway via HTTP.
 */

import { Device, type DeviceType } from '../device.js';
import {
  DoorwayClient,
  type AuthResponse,
  type RegisterRequest,
  type LoginRequest,
} from '../api/doorway-client.js';

export class BrowserDevice extends Device {
  readonly type: DeviceType = 'browser';
  readonly label: string;
  readonly client: DoorwayClient;

  private authResponse?: AuthResponse;

  constructor(
    label: string,
    private readonly doorwayUrl: string,
  ) {
    super();
    this.label = label;
    this.client = new DoorwayClient(doorwayUrl);
  }

  get isAuthenticated(): boolean {
    return !!this.authResponse;
  }

  get token(): string | undefined {
    return this.authResponse?.token;
  }

  get agentPubKey(): string | undefined {
    return this.authResponse?.agentPubKey;
  }

  get humanId(): string | undefined {
    return this.authResponse?.humanId;
  }

  async register(req: RegisterRequest): Promise<AuthResponse> {
    const res = await this.client.register(req);
    this.authResponse = res;
    this.client.setToken(res.token);
    return res;
  }

  async login(req: LoginRequest): Promise<AuthResponse> {
    const res = await this.client.login(req);
    this.authResponse = res;
    this.client.setToken(res.token);
    return res;
  }
}
