import { TestBed } from '@angular/core/testing';

import { IndexedDBCacheService } from './indexeddb-cache.service';

describe('IndexedDBCacheService', () => {
  let service: IndexedDBCacheService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [IndexedDBCacheService],
    });

    service = TestBed.inject(IndexedDBCacheService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should have init method', () => {
    expect(service.init).toBeDefined();
    expect(typeof service.init).toBe('function');
  });

  it('should have getContent method', () => {
    expect(service.getContent).toBeDefined();
    expect(typeof service.getContent).toBe('function');
  });

  it('should have setContent method', () => {
    expect(service.setContent).toBeDefined();
    expect(typeof service.setContent).toBe('function');
  });

  it('should have getPath method', () => {
    expect(service.getPath).toBeDefined();
    expect(typeof service.getPath).toBe('function');
  });

  it('should have setPath method', () => {
    expect(service.setPath).toBeDefined();
    expect(typeof service.setPath).toBe('function');
  });

  it('should have clearAll method', () => {
    expect(service.clearAll).toBeDefined();
    expect(typeof service.clearAll).toBe('function');
  });

  it('should have getStats method', () => {
    expect(service.getStats).toBeDefined();
    expect(typeof service.getStats).toBe('function');
  });
});
