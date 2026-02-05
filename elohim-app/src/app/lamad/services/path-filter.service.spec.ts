import { TestBed } from '@angular/core/testing';
import { PathFilterService } from './path-filter.service';
import { PathIndexEntry } from '../models/learning-path.model';

describe('PathFilterService', () => {
  let service: PathFilterService;

  const mockPaths: PathIndexEntry[] = [
    {
      id: 'path-1',
      title: 'Introduction to Governance',
      description: 'Learn the basics of governance in the Elohim Protocol',
      difficulty: 'beginner',
      estimatedDuration: '2 hours',
      stepCount: 8,
      tags: ['governance', 'introduction', 'featured'],
      category: 'governance',
    },
    {
      id: 'path-2',
      title: 'Advanced Sovereignty',
      description: 'Deep dive into personal sovereignty concepts',
      difficulty: 'advanced',
      estimatedDuration: '5 hours',
      stepCount: 15,
      tags: ['sovereignty', 'advanced'],
      category: 'sovereignty',
    },
    {
      id: 'path-3',
      title: 'Getting Started',
      description: 'Quick overview for new users',
      difficulty: 'beginner',
      estimatedDuration: '30 minutes',
      stepCount: 4,
      tags: ['getting-started', 'overview', 'beginner-friendly'],
      category: 'introduction',
    },
    {
      id: 'path-4',
      title: 'Intermediate Concepts',
      description: 'Build on foundational knowledge',
      difficulty: 'intermediate',
      estimatedDuration: '3 hours',
      stepCount: 12,
      tags: ['intermediate', 'concepts'],
      category: 'general',
    },
    {
      id: 'path-5',
      title: 'Community Building',
      description: 'Learn how to build and nurture communities',
      difficulty: 'intermediate',
      estimatedDuration: '4 hours',
      stepCount: 10,
      tags: ['community', 'social'],
      category: 'community',
    },
    {
      id: 'path-6',
      title: 'Technical Deep Dive',
      description: 'For developers and technical learners. This is a comprehensive guide.',
      difficulty: 'advanced',
      estimatedDuration: '10 hours',
      stepCount: 25,
      tags: ['technical', 'developer'],
      category: 'technical',
    },
    {
      id: 'path-7',
      title: 'Quick Tutorial',
      description: 'Short introduction',
      difficulty: 'beginner',
      estimatedDuration: '15 minutes',
      stepCount: 3,
      tags: [],
      category: 'tutorial',
    },
  ];

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [PathFilterService],
    });

    service = TestBed.inject(PathFilterService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getFeaturedPaths', () => {
    it('should return all paths if count <= limit', () => {
      const featured = service.getFeaturedPaths(mockPaths, 10);
      expect(featured.length).toBe(mockPaths.length);
    });

    it('should limit results to specified count', () => {
      const featured = service.getFeaturedPaths(mockPaths, 3);
      expect(featured.length).toBe(3);
    });

    it('should prioritize beginner paths', () => {
      const featured = service.getFeaturedPaths(mockPaths, 6);
      const beginnerCount = featured.filter(p => p.difficulty === 'beginner').length;
      expect(beginnerCount).toBeGreaterThan(0);
    });

    it('should prioritize paths with featured tags', () => {
      const featured = service.getFeaturedPaths(mockPaths, 6);
      const hasFeaturedTag = featured.some(p => p.tags?.includes('featured'));
      expect(hasFeaturedTag).toBe(true);
    });

    it('should prioritize introduction and getting-started tags', () => {
      const featured = service.getFeaturedPaths(mockPaths, 6);
      const hasIntroTags = featured.some(
        p =>
          p.tags?.includes('introduction') ||
          p.tags?.includes('getting-started') ||
          p.tags?.includes('overview')
      );
      expect(hasIntroTags).toBe(true);
    });

    it('should prioritize shorter paths for approachability', () => {
      const featured = service.getFeaturedPaths(mockPaths, 6);
      // Should include some short paths (< 5 steps)
      const hasShortPath = featured.some(p => p.stepCount <= 5);
      expect(hasShortPath).toBe(true);
    });

    it('should prioritize paths with good descriptions', () => {
      const featured = service.getFeaturedPaths(mockPaths, 6);
      const allHaveDescriptions = featured.every(
        p => p.description && p.description.length > 0
      );
      expect(allHaveDescriptions).toBe(true);
    });

    it('should handle empty path list', () => {
      const featured = service.getFeaturedPaths([]);
      expect(featured.length).toBe(0);
    });

    it('should use default limit of 6', () => {
      const featured = service.getFeaturedPaths(mockPaths);
      expect(featured.length).toBe(6);
    });

    it('should score paths by multiple criteria', () => {
      // Path with multiple "featured" attributes should rank higher
      const featured = service.getFeaturedPaths(mockPaths, 1);
      // Should likely pick a beginner path with featured tags
      expect(['beginner', 'intermediate']).toContain(featured[0].difficulty);
    });
  });

  describe('filterByTags', () => {
    it('should return all paths when no tags specified', () => {
      const filtered = service.filterByTags(mockPaths, []);
      expect(filtered.length).toBe(mockPaths.length);
    });

    it('should filter by single tag', () => {
      const filtered = service.filterByTags(mockPaths, ['governance']);
      expect(filtered.length).toBe(1);
      expect(filtered[0].id).toBe('path-1');
    });

    it('should filter by multiple tags (any match)', () => {
      const filtered = service.filterByTags(mockPaths, ['governance', 'sovereignty']);
      expect(filtered.length).toBe(2);
      expect(filtered.map(p => p.id)).toContain('path-1');
      expect(filtered.map(p => p.id)).toContain('path-2');
    });

    it('should be case-insensitive', () => {
      const filtered = service.filterByTags(mockPaths, ['GOVERNANCE', 'Sovereignty']);
      expect(filtered.length).toBe(2);
    });

    it('should handle paths without tags', () => {
      const filtered = service.filterByTags(mockPaths, ['nonexistent']);
      expect(filtered.length).toBe(0);
    });

    it('should handle empty tags array on paths', () => {
      const pathsWithEmptyTags = [
        { ...mockPaths[0], tags: [] },
        { ...mockPaths[1], tags: [] },
      ];
      const filtered = service.filterByTags(pathsWithEmptyTags, ['governance']);
      expect(filtered.length).toBe(0);
    });
  });

  describe('filterByDifficulty', () => {
    it('should return all paths when no difficulty specified', () => {
      const filtered = service.filterByDifficulty(mockPaths, []);
      expect(filtered.length).toBe(mockPaths.length);
    });

    it('should filter by single difficulty', () => {
      const filtered = service.filterByDifficulty(mockPaths, ['beginner']);
      expect(filtered.length).toBe(3);
      expect(filtered.every(p => p.difficulty === 'beginner')).toBe(true);
    });

    it('should filter by multiple difficulties', () => {
      const filtered = service.filterByDifficulty(mockPaths, ['beginner', 'advanced']);
      expect(filtered.length).toBe(5); // 3 beginner + 2 advanced
      expect(filtered.every(p => ['beginner', 'advanced'].includes(p.difficulty))).toBe(true);
    });

    it('should handle intermediate difficulty', () => {
      const filtered = service.filterByDifficulty(mockPaths, ['intermediate']);
      expect(filtered.length).toBe(2);
      expect(filtered.every(p => p.difficulty === 'intermediate')).toBe(true);
    });
  });

  describe('filterByCategory', () => {
    it('should filter by category', () => {
      const filtered = service.filterByCategory(mockPaths, 'governance');
      expect(filtered.length).toBe(1);
      expect(filtered[0].id).toBe('path-1');
    });

    it('should return empty array for non-existent category', () => {
      const filtered = service.filterByCategory(mockPaths, 'nonexistent');
      expect(filtered.length).toBe(0);
    });

    it('should be case-sensitive for category', () => {
      const filtered = service.filterByCategory(mockPaths, 'Governance');
      expect(filtered.length).toBe(0); // Case mismatch
    });

    it('should handle multiple paths in same category', () => {
      // Add another governance path
      const pathsWithDuplicates = [
        ...mockPaths,
        { ...mockPaths[0], id: 'path-8', category: 'governance' },
      ];
      const filtered = service.filterByCategory(pathsWithDuplicates, 'governance');
      expect(filtered.length).toBe(2);
    });
  });

  describe('searchPaths', () => {
    it('should return all paths for empty query', () => {
      const results = service.searchPaths(mockPaths, '');
      expect(results.length).toBe(mockPaths.length);
    });

    it('should return all paths for whitespace query', () => {
      const results = service.searchPaths(mockPaths, '   ');
      expect(results.length).toBe(mockPaths.length);
    });

    it('should search in title', () => {
      const results = service.searchPaths(mockPaths, 'governance');
      expect(results.length).toBe(1);
      expect(results[0].id).toBe('path-1');
    });

    it('should search in description', () => {
      const results = service.searchPaths(mockPaths, 'sovereignty');
      expect(results.length).toBeGreaterThan(0);
      expect(results.some(p => p.id === 'path-2')).toBe(true);
    });

    it('should search in tags', () => {
      const results = service.searchPaths(mockPaths, 'community');
      expect(results.some(p => p.id === 'path-5')).toBe(true);
    });

    it('should be case-insensitive', () => {
      const results = service.searchPaths(mockPaths, 'GOVERNANCE');
      expect(results.length).toBe(1);
    });

    it('should match partial words', () => {
      const results = service.searchPaths(mockPaths, 'gov');
      expect(results.some(p => p.id === 'path-1')).toBe(true);
    });

    it('should search across title, description, and tags', () => {
      const results = service.searchPaths(mockPaths, 'beginner');
      // Should find paths with "beginner" in difficulty, tags, or description
      expect(results.length).toBeGreaterThan(0);
    });

    it('should handle paths with no tags', () => {
      const results = service.searchPaths(mockPaths, 'tutorial');
      expect(results.some(p => p.id === 'path-7')).toBe(true);
    });

    it('should return empty array for no matches', () => {
      const results = service.searchPaths(mockPaths, 'zzz-nonexistent-xyz');
      expect(results.length).toBe(0);
    });
  });

  describe('combined filtering', () => {
    it('should chain multiple filters', () => {
      let filtered = service.filterByDifficulty(mockPaths, ['beginner']);
      filtered = service.filterByTags(filtered, ['featured', 'introduction']);
      expect(filtered.length).toBeGreaterThan(0);
    });

    it('should narrow results progressively', () => {
      const step1 = service.filterByDifficulty(mockPaths, ['beginner', 'intermediate']);
      const step2 = service.searchPaths(step1, 'community');
      const step3 = service.filterByTags(step2, ['social']);

      expect(step3.length).toBeLessThanOrEqual(step2.length);
      expect(step2.length).toBeLessThanOrEqual(step1.length);
    });
  });

  describe('scoring algorithm details', () => {
    it('should give higher score to beginner than advanced', () => {
      const beginnerPath: PathIndexEntry = {
        id: 'beginner',
        title: 'Beginner Path',
        description: 'A path for beginners',
        difficulty: 'beginner',
        estimatedDuration: '1 hour',
        stepCount: 5,
        tags: [],
      };
      const advancedPath: PathIndexEntry = {
        id: 'advanced',
        title: 'Advanced Path',
        description: 'A path for experts',
        difficulty: 'advanced',
        estimatedDuration: '5 hours',
        stepCount: 20,
        tags: [],
      };

      const featured = service.getFeaturedPaths([beginnerPath, advancedPath], 1);
      expect(featured[0].id).toBe('beginner');
    });

    it('should boost paths with featured tag', () => {
      const regularPath: PathIndexEntry = {
        id: 'regular',
        title: 'Regular Path',
        description: 'A regular path',
        difficulty: 'intermediate',
        estimatedDuration: '2 hours',
        stepCount: 8,
        tags: [],
      };
      const featuredPath: PathIndexEntry = {
        id: 'featured',
        title: 'Featured Path',
        description: 'A featured path',
        difficulty: 'intermediate',
        estimatedDuration: '2 hours',
        stepCount: 8,
        tags: ['featured'],
      };

      const featured = service.getFeaturedPaths([regularPath, featuredPath], 1);
      expect(featured[0].id).toBe('featured');
    });

    it('should prefer shorter paths (< 5 steps)', () => {
      const shortPath: PathIndexEntry = {
        id: 'short',
        title: 'Short Path',
        description: 'A short path for quick learning',
        difficulty: 'beginner',
        estimatedDuration: '30 min',
        stepCount: 3,
        tags: [],
      };
      const longPath: PathIndexEntry = {
        id: 'long',
        title: 'Long Path',
        description: 'A comprehensive path',
        difficulty: 'beginner',
        estimatedDuration: '5 hours',
        stepCount: 20,
        tags: [],
      };

      const featured = service.getFeaturedPaths([longPath, shortPath], 1);
      expect(featured[0].id).toBe('short');
    });

    it('should boost paths with good descriptions', () => {
      const goodDesc: PathIndexEntry = {
        id: 'good',
        title: 'Path with Description',
        description:
          'This is a comprehensive description that provides valuable context for learners',
        difficulty: 'beginner',
        estimatedDuration: '1 hour',
        stepCount: 5,
        tags: [],
      };
      const poorDesc: PathIndexEntry = {
        id: 'poor',
        title: 'Path without Description',
        description: 'Short',
        difficulty: 'beginner',
        estimatedDuration: '1 hour',
        stepCount: 5,
        tags: [],
      };

      const featured = service.getFeaturedPaths([poorDesc, goodDesc], 1);
      expect(featured[0].id).toBe('good');
    });
  });

  describe('edge cases', () => {
    it('should handle undefined tags gracefully', () => {
      const pathWithEmptyTags: PathIndexEntry = {
        id: 'no-tags',
        title: 'No Tags',
        description: 'Path without tags',
        difficulty: 'beginner',
        estimatedDuration: '1 hour',
        stepCount: 5,
        tags: [],
      };

      expect(() => service.filterByTags([pathWithEmptyTags], ['test'])).not.toThrow();
      expect(() => service.searchPaths([pathWithEmptyTags], 'test')).not.toThrow();
      expect(() => service.getFeaturedPaths([pathWithEmptyTags], 1)).not.toThrow();
    });

    it('should handle empty description', () => {
      const noDesc: PathIndexEntry = {
        id: 'no-desc',
        title: 'Basic Path',
        description: '',
        difficulty: 'beginner',
        estimatedDuration: '1 hour',
        stepCount: 5,
        tags: [],
      };

      // Search for text that would only match if description field is checked
      const results = service.searchPaths([noDesc], 'nonexistent');
      expect(results.length).toBe(0);
    });

    it('should handle zero step count', () => {
      const emptyPath: PathIndexEntry = {
        id: 'empty',
        title: 'Empty Path',
        description: 'Path with no steps',
        difficulty: 'beginner',
        estimatedDuration: '0 hours',
        stepCount: 0,
        tags: [],
      };

      expect(() => service.getFeaturedPaths([emptyPath], 1)).not.toThrow();
    });

    it('should handle very large path list', () => {
      const largePaths = Array.from({ length: 1000 }, (_, i) => ({
        id: `path-${i}`,
        title: `Path ${i}`,
        description: `Description ${i}`,
        difficulty: (['beginner', 'intermediate', 'advanced'] as const)[i % 3],
        estimatedDuration: `${i} hours`,
        stepCount: i,
        tags: [`tag-${i % 10}`],
      }));

      const featured = service.getFeaturedPaths(largePaths, 10);
      expect(featured.length).toBe(10);
    });
  });
});
