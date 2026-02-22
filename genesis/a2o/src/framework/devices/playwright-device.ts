/**
 * PlaywrightDevice — drives a real Chromium browser for full UI testing.
 *
 * Wraps Playwright's Browser + BrowserContext + Page. Also holds a
 * DoorwayClient for API-level setup/fallback (e.g. registering users
 * via API before driving the UI).
 */

import {
  DoorwayClient,
  type AuthResponse,
  type LoginRequest,
  type RegisterRequest,
} from '../api/doorway-client.js';
import { Device, type DeviceType } from '../device.js';

/**
 * Minimal Playwright type stubs to avoid requiring playwright at compile time.
 * The real Playwright types are used at runtime via dynamic import.
 */
interface PWPage {
  goto(url: string, options?: Record<string, unknown>): Promise<unknown>;
  screenshot(options?: Record<string, unknown>): Promise<Buffer>;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  evaluate(fn: (...args: any[]) => unknown, ...args: unknown[]): Promise<unknown>;
  url(): string;
  title(): Promise<string>;
  waitForLoadState(state?: string): Promise<void>;
  waitForTimeout(ms: number): Promise<void>;
  getByText(text: string, options?: Record<string, unknown>): PWLocator;
  getByRole(role: string, options?: Record<string, unknown>): PWLocator;
  locator(selector: string): PWLocator;
  reload(options?: Record<string, unknown>): Promise<unknown>;
}

interface PWLocator {
  click(): Promise<void>;
  count(): Promise<number>;
  first(): PWLocator;
  waitFor(options?: Record<string, unknown>): Promise<void>;
  textContent(): Promise<string | null>;
  getByText(text: string, options?: Record<string, unknown>): PWLocator;
}

interface PWBrowserContext {
  newPage(): Promise<PWPage>;
  close(): Promise<void>;
}

interface PWBrowser {
  newContext(options?: Record<string, unknown>): Promise<PWBrowserContext>;
  close(): Promise<void>;
}

export class PlaywrightDevice extends Device {
  readonly type: DeviceType = 'playwright';
  readonly label: string;
  readonly client: DoorwayClient;

  private context?: PWBrowserContext;
  private _page?: PWPage;
  private authResponse?: AuthResponse;

  constructor(
    label: string,
    private readonly appUrl: string,
    doorwayUrl: string,
    private readonly browser: PWBrowser
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

  get page(): PWPage {
    if (!this._page) throw new Error('PlaywrightDevice not initialized — call init() first');
    return this._page;
  }

  /** Create a fresh browser context and page. */
  async init(): Promise<void> {
    this.context = await this.browser.newContext({
      viewport: { width: 1280, height: 720 },
      ignoreHTTPSErrors: true,
    });
    this._page = await this.context.newPage();
  }

  /** Navigate to a path relative to the app URL. */
  async navigate(path: string): Promise<void> {
    const url = path.startsWith('http') ? path : `${this.appUrl}${path}`;
    await this.page.goto(url, { waitUntil: 'networkidle' });
  }

  /** Register via API, then inject the auth token as a cookie/localStorage. */
  async register(req: RegisterRequest): Promise<AuthResponse> {
    const res = await this.client.register(req);
    this.authResponse = res;
    this.client.setToken(res.token);
    await this.injectAuth(res);
    return res;
  }

  /** Login via API, then inject the auth token into the browser. */
  async login(req: LoginRequest): Promise<AuthResponse> {
    const res = await this.client.login(req);
    this.authResponse = res;
    this.client.setToken(res.token);
    await this.injectAuth(res);
    return res;
  }

  /** Take a screenshot and save to reports/screenshots/. */
  async screenshot(name: string): Promise<string> {
    const path = `reports/screenshots/${name}.png`;
    await this.page.screenshot({ path, fullPage: true });
    return path;
  }

  /** Clean up browser context. */
  async close(): Promise<void> {
    if (this.context) {
      await this.context.close();
      this.context = undefined;
      this._page = undefined;
    }
  }

  /**
   * Inject auth state into the browser context so the Angular app
   * recognizes the session. Uses localStorage which the app reads on init.
   */
  private async injectAuth(auth: AuthResponse): Promise<void> {
    // Navigate to the app origin first so we can set localStorage
    if (!this._page?.url().startsWith(this.appUrl)) {
      await this.page.goto(this.appUrl, { waitUntil: 'domcontentloaded' });
    }

    await this.page.evaluate(
      (authData: { token: string; agentPubKey: string; humanId: string }) => {
        // Keys must match elohim-app/src/app/imagodei/models/auth.model.ts
        localStorage.setItem('elohim-auth-token', authData.token);
        localStorage.setItem('elohim-auth-agent-pub-key', authData.agentPubKey);
        localStorage.setItem('elohim-auth-human-id', authData.humanId);
      },
      auth
    );
  }
}
