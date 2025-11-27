import { TestBed } from '@angular/core/testing';
import { BehaviorSubject } from 'rxjs';
import { AffinityTrackingService } from './affinity-tracking.service';
import { SessionUserService } from './session-user.service';
import { ContentNode } from '../models/content-node.model';

describe('AffinityTrackingService', () => {
  let service: AffinityTrackingService;
  let sessionUserServiceSpy: jasmine.SpyObj<SessionUserService>;
  let localStorageMock: { [key: string]: string };
  let mockStorage: Storage;
  let sessionSubject: BehaviorSubject<any>;

  beforeEach(() => {
    localStorageMock = {};

    mockStorage = {
      getItem: (key: string) => localStorageMock[key] || null,
      setItem: (key: string, value: string) => { localStorageMock[key] = value; },
      removeItem: (key: string) => { delete localStorageMock[key]; },
      key: (index: number) => Object.keys(localStorageMock)[index] || null,
      get length() { return Object.keys(localStorageMock).length; },
      clear: () => { localStorageMock = {}; }
    };

    spyOnProperty(window, 'localStorage', 'get').and.returnValue(mockStorage);

    sessionUserServiceSpy = jasmine.createSpyObj('SessionUserService', [
      'getSessionId',
      'getAffinityStorageKey',
      'recordContentView',
      'recordAffinityChange'
    ]);

    sessionSubject = new BehaviorSubject<any>(null);
    Object.defineProperty(sessionUserServiceSpy, 'session$', {
      get: () => sessionSubject.asObservable()
    });

    sessionUserServiceSpy.getSessionId.and.returnValue('test-session');
    sessionUserServiceSpy.getAffinityStorageKey.and.returnValue('affinity-test-session');

    TestBed.configureTestingModule({
      providers: [
        AffinityTrackingService,
        { provide: SessionUserService, useValue: sessionUserServiceSpy }
      ]
    });

    service = TestBed.inject(AffinityTrackingService);
  });

  afterEach(() => {
    localStorageMock = {};
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getAffinity', () => {
    it('should return 0 for unknown node', () => {
      expect(service.getAffinity('unknown-node')).toBe(0);
    });

    it('should return affinity value for known node', () => {
      service.setAffinity('node-1', 0.5);
      expect(service.getAffinity('node-1')).toBe(0.5);
    });
  });

  describe('setAffinity', () => {
    it('should set affinity value', () => {
      service.setAffinity('node-1', 0.7);
      expect(service.getAffinity('node-1')).toBe(0.7);
    });

    it('should clamp values above 1.0', () => {
      service.setAffinity('node-1', 1.5);
      expect(service.getAffinity('node-1')).toBe(1.0);
    });

    it('should clamp values below 0.0', () => {
      service.setAffinity('node-1', -0.5);
      expect(service.getAffinity('node-1')).toBe(0.0);
    });

    it('should not emit change if value is the same', () => {
      service.setAffinity('node-1', 0.5);

      let changeCount = 0;
      service.changes$.subscribe(change => {
        if (change) changeCount++;
      });

      service.setAffinity('node-1', 0.5);
      // Second set with same value should not trigger additional change
      expect(service.getAffinity('node-1')).toBe(0.5);
    });

    it('should emit change event when value changes', (done) => {
      service.changes$.subscribe(change => {
        if (change && change.nodeId === 'node-2') {
          expect(change.oldValue).toBe(0);
          expect(change.newValue).toBe(0.8);
          done();
        }
      });

      service.setAffinity('node-2', 0.8);
    });

    it('should record affinity change in session service', () => {
      service.setAffinity('node-1', 0.6);
      expect(sessionUserServiceSpy.recordAffinityChange).toHaveBeenCalledWith('node-1', 0.6);
    });
  });

  describe('incrementAffinity', () => {
    it('should increment affinity by delta', () => {
      service.setAffinity('node-1', 0.3);
      service.incrementAffinity('node-1', 0.2);
      expect(service.getAffinity('node-1')).toBeCloseTo(0.5);
    });

    it('should decrement affinity with negative delta', () => {
      service.setAffinity('node-1', 0.5);
      service.incrementAffinity('node-1', -0.2);
      expect(service.getAffinity('node-1')).toBeCloseTo(0.3);
    });

    it('should clamp result to valid range', () => {
      service.setAffinity('node-1', 0.9);
      service.incrementAffinity('node-1', 0.5);
      expect(service.getAffinity('node-1')).toBe(1.0);
    });
  });

  describe('trackView', () => {
    it('should auto-increment affinity on first view', () => {
      service.trackView('new-node');
      expect(service.getAffinity('new-node')).toBe(0.2); // AUTO_INCREMENT_DELTA
    });

    it('should not increment if affinity is already high', () => {
      service.setAffinity('known-node', 0.5);
      service.trackView('known-node');
      expect(service.getAffinity('known-node')).toBe(0.5);
    });

    it('should record view in session service', () => {
      service.trackView('node-1');
      expect(sessionUserServiceSpy.recordContentView).toHaveBeenCalledWith('node-1');
    });
  });

  describe('getStats', () => {
    const mockNodes: ContentNode[] = [
      {
        id: 'n1',
        title: 'Node 1',
        contentType: 'epic',
        contentFormat: 'markdown',
        description: 'Test',
        tags: [],
        sourcePath: '',
        relatedNodeIds: [],
        content: '',
        metadata: { category: 'cat1' }
      },
      {
        id: 'n2',
        title: 'Node 2',
        contentType: 'feature',
        contentFormat: 'markdown',
        description: 'Test',
        tags: [],
        sourcePath: '',
        relatedNodeIds: [],
        content: '',
        metadata: { category: 'cat1' }
      },
      {
        id: 'n3',
        title: 'Node 3',
        contentType: 'scenario',
        contentFormat: 'gherkin',
        description: 'Test',
        tags: [],
        sourcePath: '',
        relatedNodeIds: [],
        content: '',
        metadata: { category: 'cat2' }
      }
    ];

    it('should calculate total nodes correctly', () => {
      const stats = service.getStats(mockNodes);
      expect(stats.totalNodes).toBe(3);
    });

    it('should calculate engaged nodes correctly', () => {
      service.setAffinity('n1', 0.5);
      service.setAffinity('n2', 0.3);
      const stats = service.getStats(mockNodes);
      expect(stats.engagedNodes).toBe(2);
    });

    it('should calculate average affinity correctly', () => {
      service.setAffinity('n1', 0.6);
      service.setAffinity('n2', 0.3);
      // n3 is 0
      const stats = service.getStats(mockNodes);
      expect(stats.averageAffinity).toBeCloseTo(0.3); // (0.6 + 0.3 + 0) / 3
    });

    it('should calculate distribution correctly', () => {
      service.setAffinity('n1', 0.8);  // high
      service.setAffinity('n2', 0.5);  // medium
      // n3 is 0 - unseen
      const stats = service.getStats(mockNodes);
      expect(stats.distribution.high).toBe(1);
      expect(stats.distribution.medium).toBe(1);
      expect(stats.distribution.unseen).toBe(1);
      expect(stats.distribution.low).toBe(0);
    });

    it('should calculate category stats correctly', () => {
      service.setAffinity('n1', 0.6);
      service.setAffinity('n2', 0.4);
      const stats = service.getStats(mockNodes);

      const cat1Stats = stats.byCategory.get('cat1');
      expect(cat1Stats).toBeTruthy();
      expect(cat1Stats!.nodeCount).toBe(2);
      expect(cat1Stats!.engagedCount).toBe(2);
    });

    it('should calculate type stats correctly', () => {
      service.setAffinity('n1', 0.5);
      const stats = service.getStats(mockNodes);

      const epicStats = stats.byType.get('epic');
      expect(epicStats).toBeTruthy();
      expect(epicStats!.nodeCount).toBe(1);
      expect(epicStats!.engagedCount).toBe(1);
    });

    it('should handle empty nodes array', () => {
      const stats = service.getStats([]);
      expect(stats.totalNodes).toBe(0);
      expect(stats.averageAffinity).toBe(0);
    });
  });

  describe('reset', () => {
    it('should reset all affinity data', () => {
      service.setAffinity('node-1', 0.5);
      service.setAffinity('node-2', 0.7);
      service.reset();
      expect(service.getAffinity('node-1')).toBe(0);
      expect(service.getAffinity('node-2')).toBe(0);
    });
  });

  describe('affinity$ observable', () => {
    it('should emit current affinity state', (done) => {
      service.affinity$.subscribe(affinity => {
        expect(affinity).toBeTruthy();
        expect(affinity.userId).toBeDefined();
        done();
      });
    });
  });

  describe('localStorage integration', () => {
    it('should persist affinity to localStorage', () => {
      service.setAffinity('node-1', 0.5);
      const stored = localStorageMock['affinity-test-session'];
      expect(stored).toBeTruthy();
      const parsed = JSON.parse(stored);
      expect(parsed.affinity['node-1']).toBe(0.5);
    });

    it('should load existing affinity from localStorage on session change', () => {
      const existingData = {
        userId: 'test-user',
        affinity: { 'existing-node': 0.9 },
        lastUpdated: new Date().toISOString()
      };
      localStorageMock['affinity-test-session'] = JSON.stringify(existingData);

      // Trigger session change to reload from storage
      sessionSubject.next({ id: 'new-session' });

      expect(service.getAffinity('existing-node')).toBe(0.9);
    });
  });
});
