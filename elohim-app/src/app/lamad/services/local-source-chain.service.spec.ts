import { TestBed } from '@angular/core/testing';
import { LocalSourceChainService } from './local-source-chain.service';
import {
  SourceChainEntry,
  EntryLink,
  LamadEntryType,
  LamadLinkType,
  MasteryRecordContent,
  HumanProfileContent,
} from '../models';

describe('LocalSourceChainService', () => {
  let service: LocalSourceChainService;
  let localStorageMock: { [key: string]: string };
  let mockStorage: Storage;

  const TEST_AGENT_ID = 'test-agent-123';

  beforeEach(() => {
    // Mock localStorage
    localStorageMock = {};

    // Create a complete Storage mock
    mockStorage = {
      getItem: (key: string) => localStorageMock[key] || null,
      setItem: (key: string, value: string) => { localStorageMock[key] = value; },
      removeItem: (key: string) => { delete localStorageMock[key]; },
      key: (index: number) => Object.keys(localStorageMock)[index] || null,
      get length() { return Object.keys(localStorageMock).length; },
      clear: () => { localStorageMock = {}; }
    };

    // Replace global localStorage with our mock
    spyOnProperty(window, 'localStorage', 'get').and.returnValue(mockStorage);

    TestBed.configureTestingModule({});
    service = TestBed.inject(LocalSourceChainService);
  });

  afterEach(() => {
    localStorageMock = {};
    if (service.isInitialized()) {
      service.resetChain();
    }
  });

  describe('Initialization', () => {
    it('should create the service', () => {
      expect(service).toBeTruthy();
    });

    it('should not be initialized before initializeForAgent is called', () => {
      expect(service.isInitialized()).toBe(false);
    });

    it('should initialize for an agent', () => {
      service.initializeForAgent(TEST_AGENT_ID);
      expect(service.isInitialized()).toBe(true);
      expect(service.getAgentId()).toBe(TEST_AGENT_ID);
    });

    it('should throw error when getAgentId called before initialization', () => {
      expect(() => service.getAgentId()).toThrowError(
        '[LocalSourceChainService] Agent not initialized. Call initializeForAgent first.'
      );
    });

    it('should load existing chain data on initialization', () => {
      // Pre-populate localStorage with existing entries
      const existingEntries: SourceChainEntry[] = [
        {
          entryHash: 'entry-existing-1',
          authorAgent: TEST_AGENT_ID,
          entryType: 'human-profile',
          content: { displayName: 'Test User', isAnonymous: true, accessLevel: 'visitor' },
          timestamp: '2025-01-01T00:00:00.000Z',
          sequence: 0,
        }
      ];
      localStorageMock[`lamad-chain-${TEST_AGENT_ID}-entries`] = JSON.stringify(existingEntries);

      service.initializeForAgent(TEST_AGENT_ID);

      expect(service.getEntryCount()).toBe(1);
      const entry = service.getEntry('entry-existing-1');
      expect(entry).toBeTruthy();
      expect(entry?.entryType).toBe('human-profile');
    });
  });

  describe('Entry Operations', () => {
    beforeEach(() => {
      service.initializeForAgent(TEST_AGENT_ID);
    });

    it('should create an entry with generated hash', () => {
      const content: MasteryRecordContent = {
        contentId: 'content-1',
        level: 'seen',
        levelAchievedAt: '2025-01-01T00:00:00.000Z',
        freshness: 1.0,
        lastEngagementAt: '2025-01-01T00:00:00.000Z',
        lastEngagementType: 'view',
      };

      const entry = service.createEntry<MasteryRecordContent>('mastery-record', content);

      expect(entry.entryHash).toMatch(/^entry-/);
      expect(entry.authorAgent).toBe(TEST_AGENT_ID);
      expect(entry.entryType).toBe('mastery-record');
      expect(entry.content.contentId).toBe('content-1');
      expect(entry.sequence).toBe(0);
    });

    it('should maintain chain ordering with prevEntryHash', () => {
      const entry1 = service.createEntry('human-profile', { displayName: 'User 1' });
      const entry2 = service.createEntry('human-profile', { displayName: 'User 2' });
      const entry3 = service.createEntry('human-profile', { displayName: 'User 3' });

      expect(entry1.prevEntryHash).toBeUndefined();
      expect(entry2.prevEntryHash).toBe(entry1.entryHash);
      expect(entry3.prevEntryHash).toBe(entry2.entryHash);
      expect(entry3.sequence).toBe(2);
    });

    it('should persist entries to localStorage', () => {
      service.createEntry('human-profile', { displayName: 'Test' });

      const stored = localStorageMock[`lamad-chain-${TEST_AGENT_ID}-entries`];
      expect(stored).toBeTruthy();

      const parsed = JSON.parse(stored);
      expect(parsed.length).toBe(1);
      expect(parsed[0].content.displayName).toBe('Test');
    });

    it('should get entry by hash', () => {
      const created = service.createEntry('mastery-record', { contentId: 'test' });
      const retrieved = service.getEntry(created.entryHash);

      expect(retrieved).toBeTruthy();
      expect(retrieved?.entryHash).toBe(created.entryHash);
    });

    it('should return null for non-existent entry', () => {
      const entry = service.getEntry('non-existent-hash');
      expect(entry).toBeNull();
    });

    it('should get entries by type', () => {
      service.createEntry('human-profile', { displayName: 'User 1' });
      service.createEntry('mastery-record', { contentId: 'content-1' });
      service.createEntry('human-profile', { displayName: 'User 2' });
      service.createEntry('mastery-record', { contentId: 'content-2' });

      const profiles = service.getEntriesByType('human-profile');
      const masteries = service.getEntriesByType('mastery-record');

      expect(profiles.length).toBe(2);
      expect(masteries.length).toBe(2);
    });

    it('should get latest entry by type', () => {
      service.createEntry('mastery-record', { contentId: 'content-1', level: 'seen' });
      service.createEntry('mastery-record', { contentId: 'content-1', level: 'understand' });
      service.createEntry('mastery-record', { contentId: 'content-1', level: 'apply' });

      const latest = service.getLatestEntryByType<{ level: string }>('mastery-record');
      expect(latest?.content.level).toBe('apply');
    });

    it('should return null if no entries of type exist', () => {
      const latest = service.getLatestEntryByType('recognition-event');
      expect(latest).toBeNull();
    });

    it('should get head entry', () => {
      service.createEntry('human-profile', { displayName: 'First' });
      const second = service.createEntry('human-profile', { displayName: 'Second' });

      const head = service.getHeadEntry();
      expect(head?.entryHash).toBe(second.entryHash);
    });

    it('should return null head entry for empty chain', () => {
      const head = service.getHeadEntry();
      expect(head).toBeNull();
    });

    it('should get entry count', () => {
      expect(service.getEntryCount()).toBe(0);

      service.createEntry('human-profile', {});
      service.createEntry('mastery-record', {});
      service.createEntry('affinity-mark', {});

      expect(service.getEntryCount()).toBe(3);
    });
  });

  describe('Entry Queries', () => {
    beforeEach(() => {
      service.initializeForAgent(TEST_AGENT_ID);

      // Create test entries with specific timestamps
      service.createEntry('human-profile', { displayName: 'User 1' });
      service.createEntry('mastery-record', { contentId: 'c1' });
      service.createEntry('mastery-record', { contentId: 'c2' });
      service.createEntry('affinity-mark', { contentId: 'c1', value: 0.5 });
    });

    it('should query entries by type', () => {
      const results = service.queryEntries({ entryType: 'mastery-record' });
      expect(results.length).toBe(2);
    });

    it('should query entries by author', () => {
      const results = service.queryEntries({ authorAgent: TEST_AGENT_ID });
      expect(results.length).toBe(4);

      const noResults = service.queryEntries({ authorAgent: 'other-agent' });
      expect(noResults.length).toBe(0);
    });

    it('should query entries with limit', () => {
      const results = service.queryEntries({ limit: 2 });
      expect(results.length).toBe(2);
    });

    it('should query entries with offset', () => {
      const results = service.queryEntries({ offset: 2 });
      expect(results.length).toBe(2);
    });

    it('should query entries with offset and limit', () => {
      const results = service.queryEntries({ offset: 1, limit: 2 });
      expect(results.length).toBe(2);
    });

    it('should query entries in descending order', () => {
      const results = service.queryEntries<{ displayName?: string; contentId?: string }>({
        order: 'desc'
      });
      expect(results[0].entryType).toBe('affinity-mark');
      expect(results[3].entryType).toBe('human-profile');
    });
  });

  describe('Link Operations', () => {
    beforeEach(() => {
      service.initializeForAgent(TEST_AGENT_ID);
    });

    it('should create a link between entries', () => {
      const entry1 = service.createEntry('human-profile', {});
      const entry2 = service.createEntry('mastery-record', { contentId: 'c1' });

      const link = service.createLink(
        entry1.entryHash,
        entry2.entryHash,
        'mastery-for-content'
      );

      expect(link.linkHash).toMatch(/^link-/);
      expect(link.baseHash).toBe(entry1.entryHash);
      expect(link.targetHash).toBe(entry2.entryHash);
      expect(link.linkType).toBe('mastery-for-content');
      expect(link.authorAgent).toBe(TEST_AGENT_ID);
    });

    it('should create link with optional tag', () => {
      const entry1 = service.createEntry('human-profile', {});
      const entry2 = service.createEntry('mastery-record', {});

      const link = service.createLink(
        entry1.entryHash,
        entry2.entryHash,
        'mastery-for-content',
        'custom-tag'
      );

      expect(link.tag).toBe('custom-tag');
    });

    it('should persist links to localStorage', () => {
      const entry1 = service.createEntry('human-profile', {});
      const entry2 = service.createEntry('mastery-record', {});
      service.createLink(entry1.entryHash, entry2.entryHash, 'mastery-for-content');

      const stored = localStorageMock[`lamad-chain-${TEST_AGENT_ID}-links`];
      expect(stored).toBeTruthy();

      const parsed = JSON.parse(stored);
      expect(parsed.length).toBe(1);
    });

    it('should get links from base', () => {
      const baseEntry = service.createEntry('human-profile', {});
      const target1 = service.createEntry('mastery-record', { contentId: 'c1' });
      const target2 = service.createEntry('mastery-record', { contentId: 'c2' });

      service.createLink(baseEntry.entryHash, target1.entryHash, 'mastery-for-content');
      service.createLink(baseEntry.entryHash, target2.entryHash, 'mastery-for-content');

      const links = service.getLinksFromBase(baseEntry.entryHash);
      expect(links.length).toBe(2);
    });

    it('should filter links from base by type', () => {
      const baseEntry = service.createEntry('human-profile', {});
      const target1 = service.createEntry('mastery-record', {});
      const target2 = service.createEntry('affinity-mark', {});

      service.createLink(baseEntry.entryHash, target1.entryHash, 'mastery-for-content');
      service.createLink(baseEntry.entryHash, target2.entryHash, 'affinity-for-content');

      const masteryLinks = service.getLinksFromBase(baseEntry.entryHash, 'mastery-for-content');
      expect(masteryLinks.length).toBe(1);
      expect(masteryLinks[0].linkType).toBe('mastery-for-content');
    });

    it('should get links to target', () => {
      const target = service.createEntry('mastery-record', {});
      const base1 = service.createEntry('human-profile', {});
      const base2 = service.createEntry('human-profile', {});

      service.createLink(base1.entryHash, target.entryHash, 'mastery-for-content');
      service.createLink(base2.entryHash, target.entryHash, 'mastery-for-content');

      const links = service.getLinksToTarget(target.entryHash);
      expect(links.length).toBe(2);
    });

    it('should delete link (soft delete)', () => {
      const entry1 = service.createEntry('human-profile', {});
      const entry2 = service.createEntry('mastery-record', {});
      const link = service.createLink(entry1.entryHash, entry2.entryHash, 'mastery-for-content');

      const result = service.deleteLink(link.linkHash);
      expect(result).toBe(true);

      // Should not appear in regular queries
      const links = service.getLinksFromBase(entry1.entryHash);
      expect(links.length).toBe(0);
    });

    it('should return false when deleting non-existent link', () => {
      const result = service.deleteLink('non-existent-hash');
      expect(result).toBe(false);
    });

    it('should get link count excluding deleted', () => {
      const entry1 = service.createEntry('human-profile', {});
      const entry2 = service.createEntry('mastery-record', {});
      const link1 = service.createLink(entry1.entryHash, entry2.entryHash, 'mastery-for-content');
      service.createLink(entry1.entryHash, entry2.entryHash, 'affinity-for-content');

      expect(service.getLinkCount()).toBe(2);

      service.deleteLink(link1.linkHash);
      expect(service.getLinkCount()).toBe(1);
    });
  });

  describe('Link Queries', () => {
    beforeEach(() => {
      service.initializeForAgent(TEST_AGENT_ID);

      const e1 = service.createEntry('human-profile', {});
      const e2 = service.createEntry('mastery-record', {});
      const e3 = service.createEntry('affinity-mark', {});

      service.createLink(e1.entryHash, e2.entryHash, 'mastery-for-content', 'tag-1');
      service.createLink(e1.entryHash, e3.entryHash, 'affinity-for-content', 'tag-2');
      service.createLink(e2.entryHash, e3.entryHash, 'correction-of');
    });

    it('should query links by base hash', () => {
      const entries = service.getEntriesByType('human-profile');
      const results = service.queryLinks({ baseHash: entries[0].entryHash });
      expect(results.length).toBe(2);
    });

    it('should query links by target hash', () => {
      const entries = service.getEntriesByType('affinity-mark');
      const results = service.queryLinks({ targetHash: entries[0].entryHash });
      expect(results.length).toBe(2);
    });

    it('should query links by type', () => {
      const results = service.queryLinks({ linkType: 'mastery-for-content' });
      expect(results.length).toBe(1);
    });

    it('should query links by tag', () => {
      const results = service.queryLinks({ tag: 'tag-1' });
      expect(results.length).toBe(1);
    });

    it('should include deleted links when requested', () => {
      const links = service.queryLinks({});
      const linkToDelete = links[0];
      service.deleteLink(linkToDelete.linkHash);

      const withoutDeleted = service.queryLinks({});
      expect(withoutDeleted.length).toBe(2);

      const withDeleted = service.queryLinks({ includeDeleted: true });
      expect(withDeleted.length).toBe(3);
    });
  });

  describe('Convenience Methods', () => {
    beforeEach(() => {
      service.initializeForAgent(TEST_AGENT_ID);
    });

    it('should get linked entry from link', () => {
      const base = service.createEntry('human-profile', {});
      const target = service.createEntry<MasteryRecordContent>('mastery-record', {
        contentId: 'c1',
        level: 'seen',
        levelAchievedAt: '',
        freshness: 1,
        lastEngagementAt: '',
        lastEngagementType: 'view'
      });
      const link = service.createLink(base.entryHash, target.entryHash, 'mastery-for-content');

      const linkedEntry = service.getLinkedEntry<MasteryRecordContent>(link);
      expect(linkedEntry?.content.contentId).toBe('c1');
    });

    it('should get all linked entries from base', () => {
      const base = service.createEntry('human-profile', {});
      service.createEntry('mastery-record', { contentId: 'c1' });
      service.createEntry('mastery-record', { contentId: 'c2' });

      const masteries = service.getEntriesByType('mastery-record');
      service.createLink(base.entryHash, masteries[0].entryHash, 'mastery-for-content');
      service.createLink(base.entryHash, masteries[1].entryHash, 'mastery-for-content');

      const linkedEntries = service.getLinkedEntries<{ contentId: string }>(
        base.entryHash,
        'mastery-for-content'
      );
      expect(linkedEntries.length).toBe(2);
    });

    it('should find entries by content property', () => {
      service.createEntry<MasteryRecordContent>('mastery-record', {
        contentId: 'content-abc',
        level: 'seen',
        levelAchievedAt: '',
        freshness: 1,
        lastEngagementAt: '',
        lastEngagementType: 'view'
      });
      service.createEntry<MasteryRecordContent>('mastery-record', {
        contentId: 'content-abc',
        level: 'understand',
        levelAchievedAt: '',
        freshness: 1,
        lastEngagementAt: '',
        lastEngagementType: 'quiz'
      });
      service.createEntry<MasteryRecordContent>('mastery-record', {
        contentId: 'content-xyz',
        level: 'seen',
        levelAchievedAt: '',
        freshness: 1,
        lastEngagementAt: '',
        lastEngagementType: 'view'
      });

      const entries = service.findEntriesByContentProperty<MasteryRecordContent>(
        'mastery-record',
        'contentId',
        'content-abc'
      );
      expect(entries.length).toBe(2);
    });

    it('should get latest entry by content property', () => {
      service.createEntry<MasteryRecordContent>('mastery-record', {
        contentId: 'content-abc',
        level: 'seen',
        levelAchievedAt: '',
        freshness: 1,
        lastEngagementAt: '',
        lastEngagementType: 'view'
      });
      service.createEntry<MasteryRecordContent>('mastery-record', {
        contentId: 'content-abc',
        level: 'apply',
        levelAchievedAt: '',
        freshness: 1,
        lastEngagementAt: '',
        lastEngagementType: 'quiz'
      });

      const latest = service.getLatestEntryByContentProperty<MasteryRecordContent>(
        'mastery-record',
        'contentId',
        'content-abc'
      );
      expect(latest?.content.level).toBe('apply');
    });
  });

  describe('Chain Metadata', () => {
    beforeEach(() => {
      service.initializeForAgent(TEST_AGENT_ID);
    });

    it('should create metadata on initialization', () => {
      const metadata = service.getMetadata();
      expect(metadata).toBeTruthy();
      expect(metadata?.agentId).toBe(TEST_AGENT_ID);
      expect(metadata?.entryCount).toBe(0);
      expect(metadata?.linkCount).toBe(0);
    });

    it('should update metadata when entries are added', () => {
      service.createEntry('human-profile', {});
      service.createEntry('mastery-record', {});

      const metadata = service.getMetadata();
      expect(metadata?.entryCount).toBe(2);
    });

    it('should update metadata when links are added', () => {
      const e1 = service.createEntry('human-profile', {});
      const e2 = service.createEntry('mastery-record', {});
      service.createLink(e1.entryHash, e2.entryHash, 'mastery-for-content');

      const metadata = service.getMetadata();
      expect(metadata?.linkCount).toBe(1);
    });

    it('should track head hash', () => {
      const e1 = service.createEntry('human-profile', {});
      expect(service.getMetadata()?.headHash).toBe(e1.entryHash);

      const e2 = service.createEntry('mastery-record', {});
      expect(service.getMetadata()?.headHash).toBe(e2.entryHash);
    });

    it('should preserve createdAt across updates', (done) => {
      service.createEntry('human-profile', {});
      const initialMetadata = service.getMetadata();
      const createdAt = initialMetadata?.createdAt;

      // Wait a small amount to ensure timestamps differ
      setTimeout(() => {
        service.createEntry('mastery-record', {});
        const updatedMetadata = service.getMetadata();

        expect(updatedMetadata?.createdAt).toBe(createdAt);
        // updatedAt should be >= createdAt (may be equal if fast enough)
        expect(new Date(updatedMetadata?.updatedAt ?? '').getTime())
          .toBeGreaterThanOrEqual(new Date(createdAt ?? '').getTime());
        done();
      }, 5);
    });
  });

  describe('Migration', () => {
    beforeEach(() => {
      service.initializeForAgent(TEST_AGENT_ID);
    });

    it('should prepare migration package', () => {
      const e1 = service.createEntry('human-profile', { displayName: 'Test' });
      const e2 = service.createEntry('mastery-record', { contentId: 'c1' });
      service.createLink(e1.entryHash, e2.entryHash, 'mastery-for-content');

      const migration = service.prepareMigration();

      expect(migration).toBeTruthy();
      expect(migration?.sourceAgentId).toBe(TEST_AGENT_ID);
      expect(migration?.entries.length).toBe(2);
      expect(migration?.links.length).toBe(1);
      expect(migration?.status).toBe('pending');
      expect(migration?.preparedAt).toBeTruthy();
    });

    it('should return null if not initialized', () => {
      service.resetChain();
      const anotherService = new LocalSourceChainService();
      const migration = anotherService.prepareMigration();
      expect(migration).toBeNull();
    });

    it('should clear chain after migration', () => {
      service.createEntry('human-profile', {});
      service.clearAfterMigration();

      expect(service.isInitialized()).toBe(false);
      expect(localStorageMock[`lamad-chain-${TEST_AGENT_ID}-entries`]).toBeUndefined();
      expect(localStorageMock[`lamad-chain-${TEST_AGENT_ID}-links`]).toBeUndefined();
      expect(localStorageMock[`lamad-chain-${TEST_AGENT_ID}-metadata`]).toBeUndefined();
    });
  });

  describe('Debug/Testing Methods', () => {
    beforeEach(() => {
      service.initializeForAgent(TEST_AGENT_ID);
    });

    it('should reset chain', () => {
      service.createEntry('human-profile', {});
      service.resetChain();

      expect(service.isInitialized()).toBe(false);
    });

    it('should get raw data', () => {
      const e1 = service.createEntry('human-profile', {});
      const e2 = service.createEntry('mastery-record', {});
      service.createLink(e1.entryHash, e2.entryHash, 'mastery-for-content');

      const raw = service.getRawData();

      expect(raw.entries.length).toBe(2);
      expect(raw.links.length).toBe(1);
      expect(raw.metadata).toBeTruthy();
    });
  });

  describe('Observables', () => {
    beforeEach(() => {
      service.initializeForAgent(TEST_AGENT_ID);
    });

    it('should emit entries via entries$ observable', (done) => {
      service.entries$.subscribe(entries => {
        if (entries.length > 0) {
          expect(entries[0].entryType).toBe('human-profile');
          done();
        }
      });

      service.createEntry('human-profile', { displayName: 'Test' });
    });

    it('should emit links via links$ observable', (done) => {
      const e1 = service.createEntry('human-profile', {});
      const e2 = service.createEntry('mastery-record', {});

      service.links$.subscribe(links => {
        if (links.length > 0) {
          expect(links[0].linkType).toBe('mastery-for-content');
          done();
        }
      });

      service.createLink(e1.entryHash, e2.entryHash, 'mastery-for-content');
    });
  });

  describe('Error Handling', () => {
    it('should handle corrupted localStorage entries gracefully', () => {
      localStorageMock[`lamad-chain-${TEST_AGENT_ID}-entries`] = 'invalid json{{{';

      // Should not throw, should initialize with empty arrays
      service.initializeForAgent(TEST_AGENT_ID);
      expect(service.getEntryCount()).toBe(0);
    });

    it('should handle corrupted localStorage links gracefully', () => {
      localStorageMock[`lamad-chain-${TEST_AGENT_ID}-links`] = 'not valid json';

      service.initializeForAgent(TEST_AGENT_ID);
      expect(service.getLinkCount()).toBe(0);
    });
  });
});
