import { TestBed } from '@angular/core/testing';
import { AffinityTrackingService } from './affinity-tracking.service';
import { SessionUserService } from './session-user.service';
import { ContentNode } from '../models/content-node.model';

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

    TestBed.configureTestingModule({
      providers: [
        AffinityTrackingService,
        { provide: SessionUserService, useValue: null }
      ]
    });
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
    const subscription = service.changes$.subscribe(change => {
      if (change && change.nodeId === 'node-1') {
        expect(change.nodeId).toBe('node-1');
        expect(change.newValue).toBe(0.5);
        subscription.unsubscribe();
        done();
      }
    });
    service.setAffinity('node-1', 0.5);
  });

  it('should not emit changes if value is identical', () => {
    service.setAffinity('node-1', 0.5);
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
    service.trackView('node-new');
    expect(service.getAffinity('node-new')).toBe(0.2);

    service.setAffinity('node-known', 0.5);
    service.trackView('node-known');
    expect(service.getAffinity('node-known')).toBe(0.5);
  });

  it('should persist data to localStorage', () => {
    service.setAffinity('node-1', 0.5);
    expect(localStorage.setItem).toHaveBeenCalled();
    expect(localStorageMock['elohim-user-affinity']).toContain('node-1');
    expect(localStorageMock['elohim-user-affinity']).toContain('0.5');
  });

  it('should reset affinity data', () => {
    service.setAffinity('node-1', 0.5);
    service.reset();
    expect(service.getAffinity('node-1')).toBe(0.0);
    expect(localStorage.setItem).toHaveBeenCalled();
  });

  describe('getStats', () => {
    it('should calculate statistics correctly', () => {
      const nodes: ContentNode[] = [
        {
          id: 'n1',
          title: 'Node 1',
          contentType: 'epic',
          contentFormat: 'markdown',
          description: '',
          tags: [],
          relatedNodeIds: [],
          content: '',
          metadata: { category: 'cat1' }
        },
        {
          id: 'n2',
          title: 'Node 2',
          contentType: 'epic',
          contentFormat: 'markdown',
          description: '',
          tags: [],
          relatedNodeIds: [],
          content: '',
          metadata: { category: 'cat1' }
        },
        {
          id: 'n3',
          title: 'Node 3',
          contentType: 'feature',
          contentFormat: 'gherkin',
          description: '',
          tags: [],
          relatedNodeIds: [],
          content: '',
          metadata: { category: 'cat2' }
        }
      ];

      service.setAffinity('n1', 0.8);
      service.setAffinity('n2', 0.5);
      service.setAffinity('n3', 0.0);

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

    it('should handle empty node list', () => {
      const stats = service.getStats([]);

      expect(stats.totalNodes).toBe(0);
      expect(stats.engagedNodes).toBe(0);
      expect(stats.averageAffinity).toBe(0);
    });

    it('should calculate byType statistics', () => {
      const nodes: ContentNode[] = [
        {
          id: 'n1',
          title: 'Node 1',
          contentType: 'epic',
          contentFormat: 'markdown',
          description: '',
          tags: [],
          relatedNodeIds: [],
          content: '',
          metadata: {}
        },
        {
          id: 'n2',
          title: 'Node 2',
          contentType: 'feature',
          contentFormat: 'gherkin',
          description: '',
          tags: [],
          relatedNodeIds: [],
          content: '',
          metadata: {}
        }
      ];

      service.setAffinity('n1', 0.7);
      service.setAffinity('n2', 0.3);

      const stats = service.getStats(nodes);

      expect(stats.byType.get('epic')).toBeDefined();
      expect(stats.byType.get('feature')).toBeDefined();
      expect(stats.byType.get('epic')?.nodeCount).toBe(1);
      expect(stats.byType.get('feature')?.nodeCount).toBe(1);
    });
  });
});
