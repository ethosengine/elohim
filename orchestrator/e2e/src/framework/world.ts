/**
 * Cucumber World — shared state across step definitions.
 *
 * Holds references to doorway clients, registered humans, content created
 * during test scenarios, and an optional shared Playwright browser instance.
 */

import { World, type IWorldOptions } from '@cucumber/cucumber';

import { DoorwayClient } from './api/doorway-client.js';
import { Human } from './human.js';

export type DeviceMode = 'http' | 'playwright';

export interface DoorwayEntry {
  id: string;
  url: string;
  client: DoorwayClient;
}

/**
 * Minimal Browser interface matching Playwright's Browser.
 * Using a local type avoids requiring playwright at compile time.
 */
interface PlaywrightBrowser {
  newContext(options?: Record<string, unknown>): Promise<unknown>;
  close(): Promise<void>;
}

/** Singleton Playwright browser shared across scenarios in a run. */
let sharedBrowser: PlaywrightBrowser | undefined;

export class E2EWorld extends World {
  /** Named doorway instances (e.g. "alpha", "staging") */
  doorways = new Map<string, DoorwayEntry>();

  /** Named humans participating in the scenario */
  humans = new Map<string, Human>();

  /** Content IDs created during the scenario, keyed by alias */
  contentIds = new Map<string, string>();

  /** Cleanup callbacks to run after each scenario */
  private cleanupCallbacks: (() => Promise<void>)[] = [];

  constructor(options: IWorldOptions) {
    super(options);
  }

  /** Resolve device mode from E2E_DEVICE_MODE env var. Defaults to 'http'. */
  get deviceMode(): DeviceMode {
    const mode = process.env['E2E_DEVICE_MODE'];
    if (mode === 'playwright') return 'playwright';
    return 'http';
  }

  /** Whether to run Playwright in headless mode. Default: true. */
  get headless(): boolean {
    return process.env['E2E_HEADLESS'] !== 'false';
  }

  /**
   * Get or lazily create a shared Playwright browser instance.
   * The browser is shared across all scenarios in a test run and
   * cleaned up via the global AfterAll hook.
   */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  async getBrowser(): Promise<any> {
    if (!sharedBrowser) {
      // Dynamic import — playwright is optional and only loaded in playwright mode
      const pw = await import('playwright');
      sharedBrowser = (await pw.chromium.launch({
        headless: this.headless,
      })) as PlaywrightBrowser;
    }
    return sharedBrowser;
  }

  /** Close the shared browser (called from AfterAll hook). */
  static async closeBrowser(): Promise<void> {
    if (sharedBrowser) {
      await sharedBrowser.close();
      sharedBrowser = undefined;
    }
  }

  addDoorway(id: string, url: string): DoorwayEntry {
    const entry: DoorwayEntry = { id, url, client: new DoorwayClient(url) };
    this.doorways.set(id, entry);
    return entry;
  }

  getDoorway(id: string): DoorwayEntry {
    const d = this.doorways.get(id);
    if (!d)
      throw new Error(`Unknown doorway: "${id}". Known: ${[...this.doorways.keys()].join(', ')}`);
    return d;
  }

  addHuman(name: string, human: Human): void {
    this.humans.set(name, human);
  }

  getHuman(name: string): Human {
    const h = this.humans.get(name);
    if (!h)
      throw new Error(`Unknown human: "${name}". Known: ${[...this.humans.keys()].join(', ')}`);
    return h;
  }

  onCleanup(fn: () => Promise<void>): void {
    this.cleanupCallbacks.push(fn);
  }

  async runCleanup(): Promise<void> {
    for (const fn of [...this.cleanupCallbacks].reverse()) {
      try {
        await fn();
      } catch {
        // best-effort cleanup
      }
    }
    this.cleanupCallbacks = [];
  }
}
