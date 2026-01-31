import { Injectable } from '@angular/core';

// @coverage: 47.0% (2026-01-31)

import { BehaviorSubject, Observable } from 'rxjs';

import {
  SourceChainEntry,
  EntryLink,
  LamadEntryType,
  LamadLinkType,
  EntryQuery,
  LinkQuery,
  ChainMetadata,
  ChainMigrationPackage,
} from '../models/source-chain.model';

/**
 * LocalSourceChainService - Agent-centric source chain simulation using localStorage.
 *
 * Philosophy:
 * - Model data as Holochain would, even in localStorage
 * - Entries are immutable once created (append-only)
 * - Links are explicit relationships, not embedded IDs
 * - When Holochain is ready, swap this service for HolochainSourceChainService
 *
 * Storage Keys:
 * - lamad-chain-{agentId}-entries: All entries for this agent
 * - lamad-chain-{agentId}-links: All links created by this agent
 * - lamad-chain-{agentId}-metadata: Chain metadata
 *
 * Holochain Migration:
 * - prepareMigration() packages all chain data
 * - Data model is already Holochain-compatible
 * - Just change persistence layer, not data structures
 */
@Injectable({ providedIn: 'root' })
export class LocalSourceChainService {
  private readonly ENTRIES_SUFFIX = '-entries';
  private readonly LINKS_SUFFIX = '-links';
  private readonly METADATA_SUFFIX = '-metadata';
  private readonly CHAIN_PREFIX = 'lamad-chain-';

  private agentId: string | null = null;
  private readonly entriesSubject = new BehaviorSubject<SourceChainEntry[]>([]);
  private readonly linksSubject = new BehaviorSubject<EntryLink[]>([]);

  public readonly entries$: Observable<SourceChainEntry[]> = this.entriesSubject.asObservable();
  public readonly links$: Observable<EntryLink[]> = this.linksSubject.asObservable();

  constructor() {
    // Agent ID will be set by SessionHumanService
  }

  // =========================================================================
  // INITIALIZATION
  // =========================================================================

  /**
   * Initialize the chain for an agent.
   * Called by SessionHumanService when session is created/restored.
   */
  initializeForAgent(agentId: string): void {
    this.agentId = agentId;
    this.loadChain();
  }

  /**
   * Get the current agent ID.
   */
  getAgentId(): string {
    if (!this.agentId) {
      throw new Error(
        '[LocalSourceChainService] Agent not initialized. Call initializeForAgent first.'
      );
    }
    return this.agentId;
  }

  /**
   * Check if chain is initialized for an agent.
   */
  isInitialized(): boolean {
    return this.agentId !== null;
  }

  // =========================================================================
  // ENTRY OPERATIONS
  // =========================================================================

  /**
   * Create a new entry on the source chain.
   * Entries are immutable once created.
   *
   * @param entryType - The type of entry (e.g., 'mastery-record')
   * @param content - The entry content (structure depends on type)
   * @returns The created entry with generated hash
   */
  createEntry<T>(entryType: LamadEntryType, content: T): SourceChainEntry<T> {
    const agentId = this.getAgentId();
    const entries = this.entriesSubject.value;
    const prevEntry = entries.length > 0 ? entries[entries.length - 1] : null;

    const entry: SourceChainEntry<T> = {
      entryHash: this.generateEntryHash(),
      authorAgent: agentId,
      entryType,
      content,
      timestamp: new Date().toISOString(),
      prevEntryHash: prevEntry?.entryHash,
      sequence: entries.length,
    };

    // Append to chain
    const updatedEntries = [...entries, entry];
    this.entriesSubject.next(updatedEntries);
    this.saveEntries(updatedEntries);
    this.updateMetadata();

    return entry;
  }

  /**
   * Get a single entry by hash.
   */
  getEntry<T>(entryHash: string): SourceChainEntry<T> | null {
    const entry = this.entriesSubject.value.find(e => e.entryHash === entryHash);
    return (entry as SourceChainEntry<T>) ?? null;
  }

  /**
   * Get all entries of a specific type.
   */
  getEntriesByType<T>(entryType: LamadEntryType): SourceChainEntry<T>[] {
    return this.entriesSubject.value.filter(
      e => e.entryType === entryType
    ) as SourceChainEntry<T>[];
  }

  /**
   * Get the latest entry of a specific type.
   * Useful for getting current state (e.g., latest mastery for content).
   */
  getLatestEntryByType<T>(entryType: LamadEntryType): SourceChainEntry<T> | null {
    const entries = this.getEntriesByType<T>(entryType);
    return entries.length > 0 ? entries[entries.length - 1] : null;
  }

  /**
   * Query entries with filters.
   */
  queryEntries<T>(query: EntryQuery): SourceChainEntry<T>[] {
    let results = this.entriesSubject.value as SourceChainEntry<T>[];

    if (query.entryType) {
      results = results.filter(e => e.entryType === query.entryType);
    }

    if (query.authorAgent) {
      results = results.filter(e => e.authorAgent === query.authorAgent);
    }

    if (query.after) {
      results = results.filter(e => e.timestamp > query.after!);
    }

    if (query.before) {
      results = results.filter(e => e.timestamp < query.before!);
    }

    // Sort
    if (query.order === 'desc') {
      results = [...results].reverse();
    }

    // Pagination
    if (query.offset) {
      results = results.slice(query.offset);
    }

    if (query.limit) {
      results = results.slice(0, query.limit);
    }

    return results;
  }

  /**
   * Get the head (latest) entry in the chain.
   */
  getHeadEntry(): SourceChainEntry | null {
    const entries = this.entriesSubject.value;
    return entries.length > 0 ? entries[entries.length - 1] : null;
  }

  /**
   * Get total entry count.
   */
  getEntryCount(): number {
    return this.entriesSubject.value.length;
  }

  // =========================================================================
  // LINK OPERATIONS
  // =========================================================================

  /**
   * Create a link between two entries.
   * Links are how we express relationships in agent-centric systems.
   *
   * @param baseHash - Entry hash of the source/base entry
   * @param targetHash - Entry hash of the target entry
   * @param linkType - Type of relationship
   * @param tag - Optional tag for filtering
   * @returns The created link
   */
  createLink(
    baseHash: string,
    targetHash: string,
    linkType: LamadLinkType,
    tag?: string
  ): EntryLink {
    const agentId = this.getAgentId();

    const link: EntryLink = {
      linkHash: this.generateLinkHash(),
      baseHash,
      targetHash,
      linkType,
      tag,
      timestamp: new Date().toISOString(),
      authorAgent: agentId,
    };

    const links = [...this.linksSubject.value, link];
    this.linksSubject.next(links);
    this.saveLinks(links);
    this.updateMetadata();

    return link;
  }

  /**
   * Get links from a base entry.
   */
  getLinksFromBase(baseHash: string, linkType?: LamadLinkType): EntryLink[] {
    let links = this.linksSubject.value.filter(l => l.baseHash === baseHash && !l.deleted);

    if (linkType) {
      links = links.filter(l => l.linkType === linkType);
    }

    return links;
  }

  /**
   * Get links to a target entry.
   */
  getLinksToTarget(targetHash: string, linkType?: LamadLinkType): EntryLink[] {
    let links = this.linksSubject.value.filter(l => l.targetHash === targetHash && !l.deleted);

    if (linkType) {
      links = links.filter(l => l.linkType === linkType);
    }

    return links;
  }

  /**
   * Query links with filters.
   */
  queryLinks(query: LinkQuery): EntryLink[] {
    let results = this.linksSubject.value;

    if (query.baseHash) {
      results = results.filter(l => l.baseHash === query.baseHash);
    }

    if (query.targetHash) {
      results = results.filter(l => l.targetHash === query.targetHash);
    }

    if (query.linkType) {
      results = results.filter(l => l.linkType === query.linkType);
    }

    if (query.authorAgent) {
      results = results.filter(l => l.authorAgent === query.authorAgent);
    }

    if (query.tag) {
      results = results.filter(l => l.tag === query.tag);
    }

    if (!query.includeDeleted) {
      results = results.filter(l => !l.deleted);
    }

    return results;
  }

  /**
   * Delete a link (soft delete).
   * In Holochain, deletions are visible actions, not removal.
   */
  deleteLink(linkHash: string): boolean {
    const links = this.linksSubject.value;
    const link = links.find(l => l.linkHash === linkHash);

    if (!link) {
      return false;
    }

    link.deleted = true;
    link.deletedAt = new Date().toISOString();
    this.linksSubject.next([...links]);
    this.saveLinks(links);

    return true;
  }

  /**
   * Get total link count (excluding deleted).
   */
  getLinkCount(): number {
    return this.linksSubject.value.filter(l => !l.deleted).length;
  }

  // =========================================================================
  // CONVENIENCE METHODS
  // =========================================================================

  /**
   * Get the entry that a link points to.
   */
  getLinkedEntry<T>(link: EntryLink): SourceChainEntry<T> | null {
    return this.getEntry<T>(link.targetHash);
  }

  /**
   * Get all entries linked from a base entry.
   */
  getLinkedEntries<T>(baseHash: string, linkType?: LamadLinkType): SourceChainEntry<T>[] {
    const links = this.getLinksFromBase(baseHash, linkType);
    return links
      .map(link => this.getEntry<T>(link.targetHash))
      .filter((entry): entry is SourceChainEntry<T> => entry !== null);
  }

  /**
   * Find entries by content property.
   * Useful for looking up entries by contentId, presenceId, etc.
   */
  findEntriesByContentProperty<T>(
    entryType: LamadEntryType,
    propertyName: keyof T,
    propertyValue: unknown
  ): SourceChainEntry<T>[] {
    return this.getEntriesByType<T>(entryType).filter(
      entry => entry.content[propertyName] === propertyValue
    );
  }

  /**
   * Get the latest entry matching a content property.
   * Useful for getting current mastery for a contentId, etc.
   */
  getLatestEntryByContentProperty<T>(
    entryType: LamadEntryType,
    propertyName: keyof T,
    propertyValue: unknown
  ): SourceChainEntry<T> | null {
    const entries = this.findEntriesByContentProperty<T>(entryType, propertyName, propertyValue);
    return entries.length > 0 ? entries[entries.length - 1] : null;
  }

  // =========================================================================
  // CHAIN METADATA
  // =========================================================================

  /**
   * Get chain metadata.
   */
  getMetadata(): ChainMetadata | null {
    if (!this.agentId) return null;

    const key = this.getMetadataKey();
    try {
      const stored = localStorage.getItem(key);
      if (stored) {
        return JSON.parse(stored);
      }
    } catch {
      // Ignore
    }

    return null;
  }

  /**
   * Update chain metadata.
   */
  private updateMetadata(): void {
    if (!this.agentId) return;

    const entries = this.entriesSubject.value;
    const links = this.linksSubject.value.filter(l => !l.deleted);
    const head = entries.length > 0 ? entries[entries.length - 1] : null;

    const existingMetadata = this.getMetadata();

    const metadata: ChainMetadata = {
      agentId: this.agentId,
      headHash: head?.entryHash ?? '',
      entryCount: entries.length,
      linkCount: links.length,
      createdAt: existingMetadata?.createdAt ?? new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };

    const key = this.getMetadataKey();
    try {
      localStorage.setItem(key, JSON.stringify(metadata));
    } catch {
      // localStorage write failure is non-critical
    }
  }

  // =========================================================================
  // MIGRATION
  // =========================================================================

  /**
   * Prepare migration package for Holochain.
   */
  prepareMigration(): ChainMigrationPackage | null {
    if (!this.agentId) return null;

    const metadata = this.getMetadata();
    if (!metadata) return null;

    return {
      sourceAgentId: this.agentId,
      entries: this.entriesSubject.value,
      links: this.linksSubject.value,
      metadata,
      preparedAt: new Date().toISOString(),
      status: 'pending',
    };
  }

  /**
   * Clear chain after successful migration.
   */
  clearAfterMigration(): void {
    if (!this.agentId) return;

    const entriesKey = this.getEntriesKey();
    const linksKey = this.getLinksKey();
    const metadataKey = this.getMetadataKey();

    localStorage.removeItem(entriesKey);
    localStorage.removeItem(linksKey);
    localStorage.removeItem(metadataKey);

    this.entriesSubject.next([]);
    this.linksSubject.next([]);
    this.agentId = null;
  }

  // =========================================================================
  // STORAGE HELPERS
  // =========================================================================

  private getEntriesKey(): string {
    return `${this.CHAIN_PREFIX}${this.agentId}${this.ENTRIES_SUFFIX}`;
  }

  private getLinksKey(): string {
    return `${this.CHAIN_PREFIX}${this.agentId}${this.LINKS_SUFFIX}`;
  }

  private getMetadataKey(): string {
    return `${this.CHAIN_PREFIX}${this.agentId}${this.METADATA_SUFFIX}`;
  }

  private loadChain(): void {
    if (!this.agentId) return;

    // Load entries
    const entriesKey = this.getEntriesKey();
    try {
      const stored = localStorage.getItem(entriesKey);
      if (stored) {
        this.entriesSubject.next(JSON.parse(stored));
      } else {
        this.entriesSubject.next([]);
      }
    } catch {
      this.entriesSubject.next([]);
    }

    // Load links
    const linksKey = this.getLinksKey();
    try {
      const stored = localStorage.getItem(linksKey);
      if (stored) {
        this.linksSubject.next(JSON.parse(stored));
      } else {
        this.linksSubject.next([]);
      }
    } catch {
      this.linksSubject.next([]);
    }

    // Ensure metadata exists
    if (!this.getMetadata()) {
      this.updateMetadata();
    }
  }

  private saveEntries(entries: SourceChainEntry[]): void {
    const key = this.getEntriesKey();
    try {
      localStorage.setItem(key, JSON.stringify(entries));
    } catch {
      // localStorage write failure is non-critical
    }
  }

  private saveLinks(links: EntryLink[]): void {
    const key = this.getLinksKey();
    try {
      localStorage.setItem(key, JSON.stringify(links));
    } catch {
      // localStorage write failure is non-critical
    }
  }

  /**
   * Generate a unique entry hash.
   * In Holochain, this would be computed from content.
   * In localStorage, we use UUID-style generation.
   */
  private generateEntryHash(): string {
    const timestamp = Date.now().toString(36);
    const randomBytes = crypto.getRandomValues(new Uint8Array(6));
    const random = Array.from(randomBytes)
      .map(b => b.toString(36))
      .join('')
      .substring(0, 8);
    return `entry-${timestamp}-${random}`;
  }

  /**
   * Generate a unique link hash.
   */
  private generateLinkHash(): string {
    const timestamp = Date.now().toString(36);
    const randomBytes = crypto.getRandomValues(new Uint8Array(6));
    const random = Array.from(randomBytes)
      .map(b => b.toString(36))
      .join('')
      .substring(0, 8);
    return `link-${timestamp}-${random}`;
  }

  // =========================================================================
  // DEBUG / TESTING
  // =========================================================================

  /**
   * Reset the chain (for testing).
   */
  resetChain(): void {
    if (!this.agentId) return;

    this.clearAfterMigration();
    this.agentId = null;
  }

  /**
   * Get all raw data (for debugging).
   */
  getRawData(): {
    entries: SourceChainEntry[];
    links: EntryLink[];
    metadata: ChainMetadata | null;
  } {
    return {
      entries: this.entriesSubject.value,
      links: this.linksSubject.value,
      metadata: this.getMetadata(),
    };
  }
}
