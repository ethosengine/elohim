/**
 * Automerge sync helpers for elohim-storage
 *
 * This module provides high-level helpers for syncing Automerge documents
 * via the elohim-storage sync API.
 */

import * as Automerge from '@automerge/automerge';
import { StorageClient } from './client';
import { StorageError } from './types';

/**
 * Sync result
 */
export interface SyncResult<T> {
  /** Updated document */
  doc: Automerge.Doc<T>;
  /** Whether any changes were applied */
  changed: boolean;
  /** New heads after sync */
  heads: string[];
}

/**
 * Helper class for Automerge document sync
 *
 * @example
 * ```typescript
 * import { StorageClient, AutomergeSync } from '@elohim/storage-client';
 *
 * const client = new StorageClient({ baseUrl: 'http://localhost:8080', appId: 'lamad' });
 * const sync = new AutomergeSync(client);
 *
 * // Load or create a document
 * let doc = await sync.load<MySchema>('graph:my-doc');
 *
 * // Make local changes
 * doc = Automerge.change(doc, d => { d.title = 'Updated'; });
 *
 * // Save changes to server
 * await sync.save('graph:my-doc', doc);
 *
 * // Sync with server (bidirectional)
 * const { doc: synced, changed } = await sync.sync('graph:my-doc', doc);
 * ```
 */
export class AutomergeSync {
  private client: StorageClient;
  /** Local cache of known heads per document */
  private knownHeads: Map<string, string[]> = new Map();

  constructor(client: StorageClient) {
    this.client = client;
  }

  /**
   * Load a document from the server
   *
   * Creates a new empty document if it doesn't exist.
   */
  async load<T>(docId: string): Promise<Automerge.Doc<T>> {
    try {
      const response = await this.client.getChangesSince(docId, []);

      if (response.changes.length === 0) {
        // Document doesn't exist, return empty doc
        const doc = Automerge.init<T>();
        this.knownHeads.set(docId, []);
        return doc;
      }

      // Decode and apply all changes
      let doc = Automerge.init<T>();
      for (const changeB64 of response.changes) {
        const changeBytes = this.client.decodeBase64(changeB64);
        doc = Automerge.loadIncremental(doc, changeBytes);
      }

      // Track known heads
      this.knownHeads.set(docId, response.new_heads);

      return doc;
    } catch (error) {
      if (error instanceof StorageError && error.statusCode === 404) {
        // Document doesn't exist
        const doc = Automerge.init<T>();
        this.knownHeads.set(docId, []);
        return doc;
      }
      throw error;
    }
  }

  /**
   * Save local changes to the server
   *
   * Sends only changes since last sync.
   */
  async save<T>(docId: string, doc: Automerge.Doc<T>): Promise<string[]> {
    const knownHeads = this.knownHeads.get(docId) || [];

    // Get changes since known heads
    const changeBytes = this.getChangesSince(doc, knownHeads);

    if (changeBytes.length === 0) {
      // No new changes
      return this.getHeads(doc);
    }

    // Send changes to server
    const response = await this.client.applyChanges(docId, [changeBytes]);

    // Update known heads
    this.knownHeads.set(docId, response.new_heads);

    return response.new_heads;
  }

  /**
   * Bidirectional sync with server
   *
   * 1. Gets server changes since our known heads
   * 2. Sends our changes since server's heads
   * 3. Merges everything locally
   */
  async sync<T>(docId: string, doc: Automerge.Doc<T>): Promise<SyncResult<T>> {
    const localHeads = this.getHeads(doc);
    const knownHeads = this.knownHeads.get(docId) || [];

    // 1. Get server changes since our known heads
    const serverResponse = await this.client.getChangesSince(docId, knownHeads);

    // 2. Apply server changes locally
    let updatedDoc = doc;
    let changed = false;

    for (const changeB64 of serverResponse.changes) {
      const changeBytes = this.client.decodeBase64(changeB64);
      updatedDoc = Automerge.loadIncremental(updatedDoc, changeBytes);
      changed = true;
    }

    // 3. Check if we have local changes the server doesn't have
    const localChanges = this.getChangesSince(doc, knownHeads);
    if (localChanges.length > 0) {
      // Send our changes to server
      await this.client.applyChanges(docId, [localChanges]);
    }

    // 4. Update known heads
    const newHeads = this.getHeads(updatedDoc);
    this.knownHeads.set(docId, newHeads);

    return {
      doc: updatedDoc,
      changed,
      heads: newHeads,
    };
  }

  /**
   * Check if document exists on server
   */
  async exists(docId: string): Promise<boolean> {
    try {
      const response = await this.client.getHeads(docId);
      return response.heads.length > 0;
    } catch (error) {
      if (error instanceof StorageError && error.statusCode === 404) {
        return false;
      }
      throw error;
    }
  }

  /**
   * Delete local tracking for a document
   */
  forget(docId: string): void {
    this.knownHeads.delete(docId);
  }

  /**
   * Get current heads from an Automerge document
   */
  private getHeads<T>(doc: Automerge.Doc<T>): string[] {
    // Automerge.getHeads returns string[] (hex-encoded hashes)
    const heads = Automerge.getHeads(doc);
    return heads as string[];
  }

  /**
   * Get changes since given heads from an Automerge document
   */
  private getChangesSince<T>(doc: Automerge.Doc<T>, heads: string[]): Uint8Array {
    // Automerge.saveSince expects string[] (hex-encoded hashes)
    return Automerge.saveSince(doc, heads as Automerge.Heads);
  }

  /**
   * Convert bytes to hex string
   */
  private toHex(bytes: Uint8Array): string {
    return Array.from(bytes)
      .map((b) => b.toString(16).padStart(2, '0'))
      .join('');
  }

  /**
   * Convert hex string to bytes
   */
  private fromHex(hex: string): Uint8Array {
    const bytes = new Uint8Array(hex.length / 2);
    for (let i = 0; i < hex.length; i += 2) {
      bytes[i / 2] = parseInt(hex.slice(i, i + 2), 16);
    }
    return bytes;
  }
}

/**
 * Convenience function to create a sync helper
 */
export function createSync(client: StorageClient): AutomergeSync {
  return new AutomergeSync(client);
}
