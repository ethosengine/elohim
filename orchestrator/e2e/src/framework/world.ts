/**
 * Cucumber World — shared state across step definitions.
 *
 * Holds references to doorway clients, registered humans, and
 * content created during test scenarios.
 */

import { World, type IWorldOptions } from '@cucumber/cucumber';
import { DoorwayClient } from './api/doorway-client.js';
import { Human } from './human.js';

export interface DoorwayEntry {
  id: string;
  url: string;
  client: DoorwayClient;
}

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

  addDoorway(id: string, url: string): DoorwayEntry {
    const entry: DoorwayEntry = { id, url, client: new DoorwayClient(url) };
    this.doorways.set(id, entry);
    return entry;
  }

  getDoorway(id: string): DoorwayEntry {
    const d = this.doorways.get(id);
    if (!d) throw new Error(`Unknown doorway: "${id}". Known: ${[...this.doorways.keys()].join(', ')}`);
    return d;
  }

  addHuman(name: string, human: Human): void {
    this.humans.set(name, human);
  }

  getHuman(name: string): Human {
    const h = this.humans.get(name);
    if (!h) throw new Error(`Unknown human: "${name}". Known: ${[...this.humans.keys()].join(', ')}`);
    return h;
  }

  onCleanup(fn: () => Promise<void>): void {
    this.cleanupCallbacks.push(fn);
  }

  async runCleanup(): Promise<void> {
    for (const fn of this.cleanupCallbacks.reverse()) {
      try {
        await fn();
      } catch {
        // best-effort cleanup
      }
    }
    this.cleanupCallbacks = [];
  }
}
