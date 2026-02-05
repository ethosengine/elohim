/**
 * Tests for PathMetadata model types
 *
 * These are primarily type definitions, so tests focus on validating
 * type compatibility and interface contracts.
 */

import {
  PathMetadata,
  PathParserOptions,
  ContentDomain,
  EpicCategory,
  ContentCategory,
  ResourceType
} from './path-metadata.model';

describe('PathMetadata Model', () => {
  describe('PathMetadata interface', () => {
    it('should accept valid PathMetadata with all required fields', () => {
      const metadata: PathMetadata = {
        fullPath: '/projects/elohim/data/content/elohim-protocol/governance/epic.md',
        relativePath: 'data/content/elohim-protocol/governance/epic.md',
        domain: 'elohim-protocol',
        epic: 'governance',
        contentCategory: 'epic',
        baseName: 'epic',
        extension: '.md',
        isArchetypeDefinition: false,
        isEpicNarrative: true,
        isScenario: false,
        isResource: false,
        suggestedId: 'epic-governance'
      };

      expect(metadata.fullPath).toBe('/projects/elohim/data/content/elohim-protocol/governance/epic.md');
      expect(metadata.domain).toBe('elohim-protocol');
      expect(metadata.isEpicNarrative).toBe(true);
    });

    it('should accept PathMetadata with optional userType', () => {
      const metadata: PathMetadata = {
        fullPath: '/test/path',
        relativePath: 'test/path',
        domain: 'elohim-protocol',
        epic: 'governance',
        userType: 'policy_maker',
        contentCategory: 'scenario',
        baseName: 'funding',
        extension: '.feature',
        isArchetypeDefinition: false,
        isEpicNarrative: false,
        isScenario: true,
        isResource: false,
        suggestedId: 'scenario-governance-policy-maker-funding'
      };

      expect(metadata.userType).toBe('policy_maker');
      expect(metadata.isScenario).toBe(true);
    });

    it('should accept PathMetadata with resourceType', () => {
      const metadata: PathMetadata = {
        fullPath: '/test/path',
        relativePath: 'test/path',
        domain: 'elohim-protocol',
        epic: 'governance',
        contentCategory: 'resource',
        resourceType: 'book',
        baseName: 'climate_justice',
        extension: '.md',
        isArchetypeDefinition: false,
        isEpicNarrative: false,
        isScenario: false,
        isResource: true,
        suggestedId: 'resource-governance-book-climate-justice'
      };

      expect(metadata.resourceType).toBe('book');
      expect(metadata.isResource).toBe(true);
    });
  });

  describe('PathParserOptions interface', () => {
    it('should accept minimal options with only contentRoot', () => {
      const options: PathParserOptions = {
        contentRoot: '/projects/elohim/data/content'
      };

      expect(options.contentRoot).toBe('/projects/elohim/data/content');
      expect(options.normalizeIds).toBeUndefined();
      expect(options.idPrefix).toBeUndefined();
    });

    it('should accept full options with all fields', () => {
      const options: PathParserOptions = {
        contentRoot: '/projects/elohim/data/content',
        normalizeIds: true,
        idPrefix: 'source-'
      };

      expect(options.normalizeIds).toBe(true);
      expect(options.idPrefix).toBe('source-');
    });

    it('should accept options with normalizeIds false', () => {
      const options: PathParserOptions = {
        contentRoot: '/test/content',
        normalizeIds: false
      };

      expect(options.normalizeIds).toBe(false);
    });
  });

  describe('ContentDomain type', () => {
    it('should accept valid domain values', () => {
      const domains: ContentDomain[] = [
        'elohim-protocol',
        'fct',
        'ethosengine'
      ];

      expect(domains).toHaveLength(3);
      expect(domains).toContain('elohim-protocol');
      expect(domains).toContain('fct');
      expect(domains).toContain('ethosengine');
    });

    it('should allow domain to be assigned to string variable', () => {
      const domain: ContentDomain = 'elohim-protocol';
      const stringDomain: string = domain;

      expect(stringDomain).toBe('elohim-protocol');
    });
  });

  describe('EpicCategory type', () => {
    it('should accept all valid epic categories', () => {
      const epics: EpicCategory[] = [
        'governance',
        'autonomous_entity',
        'public_observer',
        'social_medium',
        'value_scanner',
        'economic_coordination',
        'lamad',
        'other'
      ];

      expect(epics).toHaveLength(8);
      expect(epics).toContain('governance');
      expect(epics).toContain('lamad');
      expect(epics).toContain('other');
    });
  });

  describe('ContentCategory type', () => {
    it('should accept all valid content categories', () => {
      const categories: ContentCategory[] = [
        'epic',
        'archetype',
        'scenario',
        'resource',
        'concept',
        'documentation',
        'other'
      ];

      expect(categories).toHaveLength(7);
      expect(categories).toContain('epic');
      expect(categories).toContain('scenario');
      expect(categories).toContain('archetype');
    });
  });

  describe('ResourceType type', () => {
    it('should accept all valid resource types', () => {
      const types: ResourceType[] = [
        'book',
        'video',
        'audio',
        'organization',
        'article',
        'document',
        'tool'
      ];

      expect(types).toHaveLength(7);
      expect(types).toContain('book');
      expect(types).toContain('video');
      expect(types).toContain('organization');
    });
  });

  describe('Type compatibility', () => {
    it('should allow generic strings for domain and epic', () => {
      // PathMetadata allows both strict types and generic strings
      const metadata: PathMetadata = {
        fullPath: '/test/path',
        relativePath: 'test/path',
        domain: 'custom-domain', // Generic string
        epic: 'custom-epic',     // Generic string
        contentCategory: 'other',
        baseName: 'test',
        extension: '.md',
        isArchetypeDefinition: false,
        isEpicNarrative: false,
        isScenario: false,
        isResource: false,
        suggestedId: 'test-id'
      };

      expect(metadata.domain).toBe('custom-domain');
      expect(metadata.epic).toBe('custom-epic');
    });
  });

  describe('Boolean flags', () => {
    it('should track all boolean flags correctly', () => {
      const scenarioMetadata: PathMetadata = {
        fullPath: '/test',
        relativePath: 'test',
        domain: 'elohim-protocol',
        epic: 'governance',
        contentCategory: 'scenario',
        baseName: 'test',
        extension: '.feature',
        isArchetypeDefinition: false,
        isEpicNarrative: false,
        isScenario: true,
        isResource: false,
        suggestedId: 'test-id'
      };

      expect(scenarioMetadata.isScenario).toBe(true);
      expect(scenarioMetadata.isArchetypeDefinition).toBe(false);
      expect(scenarioMetadata.isEpicNarrative).toBe(false);
      expect(scenarioMetadata.isResource).toBe(false);
    });

    it('should handle archetype definition flags', () => {
      const archetypeMetadata: PathMetadata = {
        fullPath: '/test',
        relativePath: 'test',
        domain: 'elohim-protocol',
        epic: 'governance',
        userType: 'policy_maker',
        contentCategory: 'archetype',
        baseName: 'README',
        extension: '.md',
        isArchetypeDefinition: true,
        isEpicNarrative: false,
        isScenario: false,
        isResource: false,
        suggestedId: 'archetype-governance-policy-maker'
      };

      expect(archetypeMetadata.isArchetypeDefinition).toBe(true);
      expect(archetypeMetadata.userType).toBe('policy_maker');
    });
  });
});
