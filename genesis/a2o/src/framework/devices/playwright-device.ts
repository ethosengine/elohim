/**
 * PlaywrightDevice — drives a real Chromium browser for full UI testing.
 *
 * Wraps Playwright's Browser + BrowserContext + Page. Also holds a
 * DoorwayClient for API-level setup/fallback (e.g. registering users
 * via API before driving the UI).
 *
 * Observability: captures console logs, uncaught JS errors, and failed
 * network requests throughout the page lifecycle. Use `getErrors()` to
 * retrieve captured data for assertions or artifact reports.
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
export interface PWPage {
  goto(url: string, options?: Record<string, unknown>): Promise<unknown>;
  screenshot(options?: Record<string, unknown>): Promise<Buffer>;
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  evaluate(fn: (...args: any[]) => unknown, ...args: unknown[]): Promise<unknown>;
  url(): string;
  title(): Promise<string>;
  waitForLoadState(state?: string): Promise<void>;
  waitForTimeout(ms: number): Promise<void>;
  waitForURL(
    url: string | RegExp | ((url: URL) => boolean),
    options?: Record<string, unknown>
  ): Promise<void>;
  getByText(text: string, options?: Record<string, unknown>): PWLocator;
  getByRole(role: string, options?: Record<string, unknown>): PWLocator;
  locator(selector: string): PWLocator;
  reload(options?: Record<string, unknown>): Promise<unknown>;
  on(event: string, handler: (...args: unknown[]) => void): void;
  fill(selector: string, value: string): Promise<void>;
}

export interface PWLocator {
  click(options?: Record<string, unknown>): Promise<void>;
  count(): Promise<number>;
  first(): PWLocator;
  nth(index: number): PWLocator;
  waitFor(options?: Record<string, unknown>): Promise<void>;
  textContent(): Promise<string | null>;
  fill(value: string): Promise<void>;
  getByText(text: string, options?: Record<string, unknown>): PWLocator;
  isVisible(): Promise<boolean>;
}

interface PWBrowserContext {
  newPage(): Promise<PWPage>;
  close(): Promise<void>;
  tracing: PWTracing;
}

interface PWTracing {
  start(options?: Record<string, unknown>): Promise<void>;
  stop(options?: Record<string, unknown>): Promise<void>;
}

interface PWBrowser {
  newContext(options?: Record<string, unknown>): Promise<PWBrowserContext>;
  close(): Promise<void>;
}

/** Captured console message. */
export interface CapturedConsoleLog {
  level: string;
  text: string;
  url: string;
  timestamp: number;
}

/** Captured uncaught JS error. */
export interface CapturedPageError {
  message: string;
  stack?: string;
  url: string;
  timestamp: number;
}

/** Captured failed network request. */
export interface CapturedFailedRequest {
  url: string;
  method: string;
  status?: number;
  failure?: string;
  timestamp: number;
}

/** All captured browser errors for a device session. */
export interface CapturedErrors {
  console: CapturedConsoleLog[];
  page: CapturedPageError[];
  network: CapturedFailedRequest[];
}

export class PlaywrightDevice extends Device {
  readonly type: DeviceType = 'playwright';
  readonly label: string;
  readonly client: DoorwayClient;

  private context?: PWBrowserContext;
  private _page?: PWPage;
  private authResponse?: AuthResponse;

  /** Captured console messages (all levels). */
  consoleLogs: CapturedConsoleLog[] = [];

  /** Captured uncaught JS errors. */
  pageErrors: CapturedPageError[] = [];

  /** Captured failed network requests. */
  failedRequests: CapturedFailedRequest[] = [];

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

  /** Create a fresh browser context and page with observability wired up. */
  async init(): Promise<void> {
    this.context = await this.browser.newContext({
      viewport: { width: 1280, height: 720 },
      ignoreHTTPSErrors: true,
    });

    // Start tracing if requested
    if (process.env['E2E_TRACE'] === 'true') {
      await this.context.tracing.start({ screenshots: true, snapshots: true });
    }

    this._page = await this.context.newPage();

    // Console capture — all levels (log, warn, error, info, debug)
    this._page.on('console', (...args: unknown[]) => {
      const msg = args[0] as { type(): string; text(): string };
      this.consoleLogs.push({
        level: msg.type(),
        text: msg.text(),
        url: this._page?.url() ?? '',
        timestamp: Date.now(),
      });
    });

    // Uncaught JS errors (window.onerror / unhandled rejections)
    this._page.on('pageerror', (...args: unknown[]) => {
      const err = args[0] as { message: string; stack?: string };
      this.pageErrors.push({
        message: err.message,
        stack: err.stack,
        url: this._page?.url() ?? '',
        timestamp: Date.now(),
      });
    });

    // Failed network requests (DNS failures, connection refused, etc.)
    this._page.on('requestfailed', (...args: unknown[]) => {
      const req = args[0] as {
        url(): string;
        method(): string;
        failure(): { errorText: string } | null;
      };
      this.failedRequests.push({
        url: req.url(),
        method: req.method(),
        failure: req.failure()?.errorText,
        timestamp: Date.now(),
      });
    });
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

  /** Get all captured errors/warnings from the browser session. */
  getErrors(): CapturedErrors {
    return {
      console: this.consoleLogs.filter(l => l.level === 'error' || l.level === 'warning'),
      page: [...this.pageErrors],
      network: [...this.failedRequests],
    };
  }

  /** Quick check: were any errors or warnings captured? */
  hasErrors(): boolean {
    const errors = this.getErrors();
    return errors.console.length > 0 || errors.page.length > 0 || errors.network.length > 0;
  }

  /** Reset all capture arrays (call between scenarios). */
  clearCapture(): void {
    this.consoleLogs = [];
    this.pageErrors = [];
    this.failedRequests = [];
  }

  /** Stop tracing and save to a zip file. Returns path or undefined if tracing not enabled. */
  async saveTrace(name: string): Promise<string | undefined> {
    if (!this.context || process.env['E2E_TRACE'] !== 'true') return undefined;
    const path = `reports/traces/${name}.zip`;
    await this.context.tracing.stop({ path });
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
