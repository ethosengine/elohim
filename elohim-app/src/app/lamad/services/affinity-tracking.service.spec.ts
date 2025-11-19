import { TestBed } from '@angular/core/testing';
import { AffinityTrackingService } from './affinity-tracking.service';
import { DocumentNode, NodeType } from '../models/document-node.model';

describe('AffinityTrackingService', () => {
  let service: AffinityTrackingService;
  let localStorageMock: { [key: string]: string } = {};

  beforeEach(() => {
    // Mock localStorage
    localStorageMock = {};
    spyOn(localStorage, 'getItem').and.callFake((key: string) => {
      return localStorageMock[key] || null;
    });
    spyOn(localStorage, 'setItem').and.callFake((key: string, value: string) => {
      localStorageMock[key] = value;
    });
    spyOn(localStorage, 'clear').and.callFake(() => {
      localStorageMock = {};
    });

    TestBed.configureTestingModule({});
    service = TestBed.inject(AffinityTrackingService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should return 0.0 affinity for unknown node', () => {
    expect(service.getAffinity('unknown-node')).toBe(0.0);
  });

  it('should set and retrieve affinity', () => {
    service.setAffinity('node-1', 0.5);
    expect(service.getAffinity('node-1')).toBe(0.5);
  });

  it('should clamp affinity values between 0.0 and 1.0', () => {
    service.setAffinity('node-low', -0.5);
    expect(service.getAffinity('node-low')).toBe(0.0);

    service.setAffinity('node-high', 1.5);
    expect(service.getAffinity('node-high')).toBe(1.0);
  });

  it('should emit changes when affinity is updated', (done) => {
    service.changes$.subscribe(change => {
      if (change) {
        expect(change.nodeId).toBe('node-1');
        expect(change.oldValue).toBe(0);
        expect(change.newValue).toBe(0.5);
        done();
      }
    });
    service.setAffinity('node-1', 0.5);
  });

  it('should not emit changes if value is identical', () => {
    service.setAffinity('node-1', 0.5);
    let changeEmitCount = 0;
    service.changes$.subscribe(() => changeEmitCount++);
    
    // Initial subscription emits null, so we ignore that or handle it
    // But strictly speaking, behavior subject emits last value.
    // Let's rely on spy logic if possible, or just reset spy.
    
    const subscription = service.changes$.subscribe(); 
    // We need a way to count NEW emissions.
    
    // Easier approach:
    service.setAffinity('node-1', 0.5); // Same value
    // Since we don't have an easy way to spy on the Subject directly without mocking it,
    // we can check if localStorage.setItem was called.
    // The service implementation returns early: if (oldValue === clampedValue) return;
    
    (localStorage.setItem as jasmine.Spy).calls.reset();
    service.setAffinity('node-1', 0.5);
    expect(localStorage.setItem).not.toHaveBeenCalled();
  });

  it('should increment affinity', () => {
    service.setAffinity('node-1', 0.2);
    service.incrementAffinity('node-1', 0.3);
    expect(service.getAffinity('node-1')).toBeCloseTo(0.5);

    service.incrementAffinity('node-1', -0.1);
    expect(service.getAffinity('node-1')).toBeCloseTo(0.4);
  });

  it('should track view and auto-increment if low', () => {
    // Default 0
    service.trackView('node-new');
    expect(service.getAffinity('node-new')).toBe(0.2); // AUTO_INCREMENT_DELTA

    // Already high
    service.setAffinity('node-known', 0.5);
    service.trackView('node-known');
    expect(service.getAffinity('node-known')).toBe(0.5); // No change
  });

  it('should persist data to localStorage', () => {
    service.setAffinity('node-1', 0.5);
    expect(localStorage.setItem).toHaveBeenCalled();
    expect(localStorageMock['elohim-user-affinity']).toContain('node-1');
    expect(localStorageMock['elohim-user-affinity']).toContain('0.5');
  });

  it('should load data from localStorage on init', () => {
    const storedData = {
      userId: 'demo-user',
      affinity: { 'stored-node': 0.8 },
      lastUpdated: new Date().toISOString()
    };
    localStorageMock['elohim-user-affinity'] = JSON.stringify(storedData);

    const newService = new AffinityTrackingService();
    expect(newService.getAffinity('stored-node')).toBe(0.8);
  });

  it('should handle localStorage errors gracefully', () => {
    (localStorage.getItem as jasmine.Spy).and.callFake(() => {
      throw new Error('Storage error');
    });
    const newService = TestBed.inject(AffinityTrackingService);
    expect(newService.getAffinity('any')).toBe(0);
  });

  it('should reset affinity data', () => {
    service.setAffinity('node-1', 0.5);
    service.reset();
    expect(service.getAffinity('node-1')).toBe(0.0);
    expect(localStorage.setItem).toHaveBeenCalled();
  });

  describe('getStats', () => {
    it('should calculate statistics correctly', () => {
      const nodes: DocumentNode[] = [
        {
          id: 'n1',
          title: 'Node 1',
          type: NodeType.EPIC,
          description: '',
          tags: [],
          sourcePath: '',
          relatedNodeIds: [],
          content: '',
          metadata: { category: 'cat1' }
        },
        {
          id: 'n2',
          title: 'Node 2',
          type: NodeType.EPIC,
          description: '',
          tags: [],
          sourcePath: '',
          relatedNodeIds: [],
          content: '',
          metadata: { category: 'cat1' }
        },
        {
          id: 'n3',
          title: 'Node 3',
          type: NodeType.FEATURE,
          description: '',
          tags: [],
          sourcePath: '',
          relatedNodeIds: [],
          content: '',
          metadata: { category: 'cat2' }
        }
      ];

      service.setAffinity('n1', 0.8); // High
      service.setAffinity('n2', 0.5); // Medium
      service.setAffinity('n3', 0.0); // Unseen

      const stats = service.getStats(nodes);

      expect(stats.totalNodes).toBe(3);
      expect(stats.engagedNodes).toBe(2);
      expect(stats.averageAffinity).toBeCloseTo((0.8 + 0.5 + 0) / 3);

      expect(stats.distribution.high).toBe(1);
      expect(stats.distribution.medium).toBe(1);
      expect(stats.distribution.low).toBe(0);
      expect(stats.distribution.unseen).toBe(1);

      expect(stats.byCategory.get('cat1')?.engagedCount).toBe(2);
      expect(stats.byCategory.get('cat2')?.engagedCount).toBe(0);
    });
  });
});
