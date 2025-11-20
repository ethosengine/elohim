import {
  UserAffinity,
  AffinityStats,
  CategoryAffinityStats,
  TypeAffinityStats,
  AffinityChangeEvent
} from './user-affinity.model';

describe('UserAffinity Model', () => {
  describe('UserAffinity interface', () => {
    it('should create valid user affinity object', () => {
      const userAffinity: UserAffinity = {
        userId: 'user-1',
        affinity: {
          'node-1': 0.5,
          'node-2': 0.8
        },
        lastUpdated: new Date('2025-01-01')
      };

      expect(userAffinity.userId).toBe('user-1');
      expect(userAffinity.affinity['node-1']).toBe(0.5);
      expect(userAffinity.affinity['node-2']).toBe(0.8);
      expect(userAffinity.lastUpdated).toEqual(new Date('2025-01-01'));
    });

    it('should accept empty affinity map', () => {
      const userAffinity: UserAffinity = {
        userId: 'user-1',
        affinity: {},
        lastUpdated: new Date()
      };

      expect(Object.keys(userAffinity.affinity)).toHaveLength(0);
    });

    it('should accept affinity values between 0 and 1', () => {
      const userAffinity: UserAffinity = {
        userId: 'user-1',
        affinity: {
          'node-1': 0.0,
          'node-2': 0.5,
          'node-3': 1.0
        },
        lastUpdated: new Date()
      };

      expect(userAffinity.affinity['node-1']).toBe(0.0);
      expect(userAffinity.affinity['node-2']).toBe(0.5);
      expect(userAffinity.affinity['node-3']).toBe(1.0);
    });
  });

  describe('AffinityStats interface', () => {
    it('should create valid affinity stats object', () => {
      const stats: AffinityStats = {
        totalNodes: 100,
        engagedNodes: 50,
        averageAffinity: 0.4,
        distribution: {
          unseen: 50,
          low: 20,
          medium: 20,
          high: 10
        },
        byCategory: new Map(),
        byType: new Map()
      };

      expect(stats.totalNodes).toBe(100);
      expect(stats.engagedNodes).toBe(50);
      expect(stats.averageAffinity).toBe(0.4);
      expect(stats.distribution.unseen).toBe(50);
      expect(stats.distribution.low).toBe(20);
      expect(stats.distribution.medium).toBe(20);
      expect(stats.distribution.high).toBe(10);
    });

    it('should support category and type maps', () => {
      const categoryMap = new Map<string, CategoryAffinityStats>();
      categoryMap.set('category-1', {
        category: 'category-1',
        nodeCount: 10,
        averageAffinity: 0.5,
        engagedCount: 5
      });

      const typeMap = new Map<string, TypeAffinityStats>();
      typeMap.set('type-1', {
        type: 'type-1',
        nodeCount: 20,
        averageAffinity: 0.6,
        engagedCount: 10
      });

      const stats: AffinityStats = {
        totalNodes: 100,
        engagedNodes: 50,
        averageAffinity: 0.4,
        distribution: {
          unseen: 50,
          low: 20,
          medium: 20,
          high: 10
        },
        byCategory: categoryMap,
        byType: typeMap
      };

      expect(stats.byCategory.get('category-1')?.category).toBe('category-1');
      expect(stats.byType.get('type-1')?.type).toBe('type-1');
    });
  });

  describe('CategoryAffinityStats interface', () => {
    it('should create valid category stats', () => {
      const categoryStats: CategoryAffinityStats = {
        category: 'learning',
        nodeCount: 50,
        averageAffinity: 0.6,
        engagedCount: 30
      };

      expect(categoryStats.category).toBe('learning');
      expect(categoryStats.nodeCount).toBe(50);
      expect(categoryStats.averageAffinity).toBe(0.6);
      expect(categoryStats.engagedCount).toBe(30);
    });
  });

  describe('TypeAffinityStats interface', () => {
    it('should create valid type stats', () => {
      const typeStats: TypeAffinityStats = {
        type: 'documentation',
        nodeCount: 25,
        averageAffinity: 0.7,
        engagedCount: 15
      };

      expect(typeStats.type).toBe('documentation');
      expect(typeStats.nodeCount).toBe(25);
      expect(typeStats.averageAffinity).toBe(0.7);
      expect(typeStats.engagedCount).toBe(15);
    });
  });

  describe('AffinityChangeEvent interface', () => {
    it('should create valid affinity change event', () => {
      const event: AffinityChangeEvent = {
        nodeId: 'node-1',
        oldValue: 0.5,
        newValue: 0.8,
        timestamp: new Date('2025-01-01')
      };

      expect(event.nodeId).toBe('node-1');
      expect(event.oldValue).toBe(0.5);
      expect(event.newValue).toBe(0.8);
      expect(event.timestamp).toEqual(new Date('2025-01-01'));
    });

    it('should handle zero to non-zero change', () => {
      const event: AffinityChangeEvent = {
        nodeId: 'node-1',
        oldValue: 0.0,
        newValue: 0.3,
        timestamp: new Date()
      };

      expect(event.oldValue).toBe(0.0);
      expect(event.newValue).toBe(0.3);
    });

    it('should handle affinity decrease', () => {
      const event: AffinityChangeEvent = {
        nodeId: 'node-1',
        oldValue: 0.8,
        newValue: 0.4,
        timestamp: new Date()
      };

      expect(event.oldValue).toBe(0.8);
      expect(event.newValue).toBe(0.4);
    });
  });
});
