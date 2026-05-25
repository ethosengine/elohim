// @vitest-environment jsdom
import { TestBed } from '@angular/core/testing';

import { LocalSourceChainService } from './local-source-chain.service';

describe('LocalSourceChainService', () => {
  let service: LocalSourceChainService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [LocalSourceChainService],
    });

    service = TestBed.inject(LocalSourceChainService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should not be initialized before initializeForAgent', () => {
    expect(service.isInitialized()).toBe(false);
  });

  it('should initialize for an agent', () => {
    service.initializeForAgent('test-agent-id');
    expect(service.isInitialized()).toBe(true);
    expect(service.getAgentId()).toBe('test-agent-id');
  });

  it('should create and retrieve entries', () => {
    service.initializeForAgent('test-agent');
    const entry = service.createEntry('mastery-record', { contentId: 'c1', level: 'remember' });
    expect(entry.entryHash).toBeTruthy();
    expect(entry.entryType).toBe('mastery-record');

    const retrieved = service.getEntry(entry.entryHash);
    expect(retrieved).toBeTruthy();
    expect(retrieved!.entryHash).toBe(entry.entryHash);
  });

  it('should create and retrieve links', () => {
    service.initializeForAgent('test-agent');
    const e1 = service.createEntry('mastery-record', {});
    const e2 = service.createEntry('mastery-record', {});

    const link = service.createLink(e1.entryHash, e2.entryHash, 'mastery-for-content');
    expect(link.linkHash).toBeTruthy();

    const links = service.getLinksFromBase(e1.entryHash);
    expect(links.length).toBe(1);
    expect(links[0].targetHash).toBe(e2.entryHash);
  });

  it('should reset chain', () => {
    service.initializeForAgent('test-agent');
    service.createEntry('mastery-record', {});
    service.resetChain();
    expect(service.isInitialized()).toBe(false);
  });
});
