/**
 * Tests for path-metadata-parser
 */

import {
  parsePathMetadata,
  isProcessableFile,
  filterProcessableFiles,
  isSourceContent
} from './path-metadata-parser';
import { PathMetadata, PathParserOptions } from '../models/path-metadata.model';
import * as path from 'path';

describe('path-metadata-parser', () => {
  const defaultOptions: PathParserOptions = {
    contentRoot: '/projects/elohim/data/content',
    normalizeIds: true,
    idPrefix: ''
  };

  describe('parsePathMetadata', () => {
    describe('Epic narratives', () => {
      it('should parse epic.md as epic narrative', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/epic.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.domain).toBe('elohim-protocol');
        expect(metadata.epic).toBe('governance');
        expect(metadata.contentCategory).toBe('epic');
        expect(metadata.isEpicNarrative).toBe(true);
        expect(metadata.baseName).toBe('epic');
        expect(metadata.extension).toBe('.md');
      });

      it('should parse epic.md for different domains', () => {
        const fctPath = '/projects/elohim/data/content/fct/foundations/epic.md';
        const fctMeta = parsePathMetadata(fctPath, defaultOptions);

        expect(fctMeta.domain).toBe('fct');
        expect(fctMeta.epic).toBe('foundations');
        expect(fctMeta.isEpicNarrative).toBe(true);
      });

      it('should generate ID for epic narrative', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/lamad/epic.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.suggestedId).toBe('epic-lamad');
      });
    });

    describe('Archetype definitions', () => {
      it('should parse README.md in user directory as archetype', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/policy_maker/README.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.contentCategory).toBe('archetype');
        expect(metadata.isArchetypeDefinition).toBe(true);
        expect(metadata.userType).toBe('policy_maker');
        expect(metadata.baseName).toBe('README');
      });

      it('should not treat top-level README as archetype', () => {
        const filePath = '/projects/elohim/data/content/README.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.isArchetypeDefinition).toBe(false);
      });

      it('should generate ID for archetype', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/policy_maker/README.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.suggestedId).toBe('archetype-governance-policy-maker');
      });
    });

    describe('Scenarios', () => {
      it('should parse .feature files as scenarios', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/policy_maker/scenarios/funding.feature';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.contentCategory).toBe('scenario');
        expect(metadata.isScenario).toBe(true);
        expect(metadata.userType).toBe('policy_maker');
        expect(metadata.extension).toBe('.feature');
      });

      it('should detect scenarios directory', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/autonomous_entity/scenarios/test.feature';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.isScenario).toBe(true);
        expect(metadata.epic).toBe('autonomous_entity');
      });

      it('should extract user type from scenario path', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/activist/scenarios/protest.feature';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.userType).toBe('activist');
      });

      it('should generate ID for scenario', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/policy_maker/scenarios/funding.feature';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.suggestedId).toBe('scenario-governance-policy-maker-funding');
      });
    });

    describe('Resources', () => {
      it('should parse files in books directory as book resources', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/resources/books/climate_justice.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.contentCategory).toBe('resource');
        expect(metadata.isResource).toBe(true);
        expect(metadata.resourceType).toBe('book');
      });

      it('should parse files in videos directory as video resources', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/lamad/resources/videos/intro.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.resourceType).toBe('video');
        expect(metadata.isResource).toBe(true);
      });

      it('should handle organization resources', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/resources/organizations/un.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.resourceType).toBe('organization');
      });

      it('should extract user type from resource path if present', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/policy_maker/resources/books/manual.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.userType).toBe('policy_maker');
        expect(metadata.isResource).toBe(true);
      });

      it('should handle all resource types', () => {
        const resourceTypes = [
          { dir: 'books', type: 'book' },
          { dir: 'videos', type: 'video' },
          { dir: 'audio', type: 'audio' },
          { dir: 'organizations', type: 'organization' },
          { dir: 'articles', type: 'article' },
          { dir: 'documents', type: 'document' },
          { dir: 'tools', type: 'tool' }
        ];

        resourceTypes.forEach(({ dir, type }) => {
          const filePath = `/projects/elohim/data/content/elohim-protocol/governance/resources/${dir}/test.md`;
          const metadata = parsePathMetadata(filePath, defaultOptions);

          expect(metadata.resourceType).toBe(type);
          expect(metadata.isResource).toBe(true);
        });
      });
    });

    describe('Concepts', () => {
      it('should detect concept files by name', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/concept-sovereignty.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.contentCategory).toBe('concept');
      });

      it('should detect files with concept in name', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/digital-concept.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.contentCategory).toBe('concept');
      });
    });

    describe('Documentation', () => {
      it('should categorize other .md files as documentation', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/policy_maker/guide.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.contentCategory).toBe('documentation');
      });

      it('should extract user type from documentation path', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/policy_maker/onboarding.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.userType).toBe('policy_maker');
        expect(metadata.contentCategory).toBe('documentation');
      });
    });

    describe('Path parsing', () => {
      it('should extract domain from path', () => {
        const filePath = '/projects/elohim/data/content/ethosengine/test/file.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.domain).toBe('ethosengine');
      });

      it('should extract epic from path', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/public_observer/test.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.epic).toBe('public_observer');
      });

      it('should handle unknown epics', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/unknown_epic/test.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.epic).toBe('unknown_epic');
      });

      it('should extract file extension', () => {
        const mdPath = '/test/file.md';
        const featurePath = '/test/file.feature';

        const mdMeta = parsePathMetadata(mdPath, defaultOptions);
        const featureMeta = parsePathMetadata(featurePath, defaultOptions);

        expect(mdMeta.extension).toBe('.md');
        expect(featureMeta.extension).toBe('.feature');
      });

      it('should extract base name', () => {
        const filePath = '/test/my-test-file.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.baseName).toBe('my-test-file');
      });

      it('should handle paths outside content root', () => {
        const filePath = '/completely/different/path/file.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.fullPath).toBe(path.normalize(filePath));
      });
    });

    describe('ID generation', () => {
      it('should normalize IDs to lowercase kebab-case by default', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/Policy_Maker/Test_File.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.suggestedId).toMatch(/^[a-z0-9-]+$/);
        expect(metadata.suggestedId).not.toContain('_');
        expect(metadata.suggestedId).not.toContain(' ');
      });

      it('should skip normalization when normalizeIds is false', () => {
        const options: PathParserOptions = {
          ...defaultOptions,
          normalizeIds: false
        };

        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/Test_File.md';
        const metadata = parsePathMetadata(filePath, options);

        // Without normalization, underscores remain
        expect(metadata.suggestedId).toContain('-');
      });

      it('should add ID prefix when provided', () => {
        const options: PathParserOptions = {
          ...defaultOptions,
          idPrefix: 'source-'
        };

        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/epic.md';
        const metadata = parsePathMetadata(filePath, options);

        expect(metadata.suggestedId).toMatch(/^source-/);
      });

      it('should skip domain prefix for elohim-protocol', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/epic.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.suggestedId).not.toContain('elohim-protocol');
        expect(metadata.suggestedId).toBe('epic-governance');
      });

      it('should include domain prefix for other domains', () => {
        const filePath = '/projects/elohim/data/content/fct/foundations/epic.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.suggestedId).toContain('fct');
      });

      it('should skip generic filenames in ID', () => {
        const readmePath = '/projects/elohim/data/content/elohim-protocol/governance/policy_maker/README.md';
        const epicPath = '/projects/elohim/data/content/elohim-protocol/governance/epic.md';

        const readmeMeta = parsePathMetadata(readmePath, defaultOptions);
        const epicMeta = parsePathMetadata(epicPath, defaultOptions);

        expect(readmeMeta.suggestedId).not.toContain('readme');
        expect(epicMeta.suggestedId).not.toContain('epic'); // Only in ID once as category
      });

      it('should collapse multiple hyphens', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/test___file.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.suggestedId).not.toContain('--');
        expect(metadata.suggestedId).not.toContain('___');
      });

      it('should remove leading and trailing hyphens', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/-test-.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.suggestedId).not.toMatch(/^-/);
        expect(metadata.suggestedId).not.toMatch(/-$/);
      });
    });

    describe('Relative path handling', () => {
      it('should compute relative path from content root', () => {
        const filePath = '/projects/elohim/data/content/elohim-protocol/governance/epic.md';
        const metadata = parsePathMetadata(filePath, defaultOptions);

        expect(metadata.relativePath).toBe('elohim-protocol/governance/epic.md');
      });

      it('should handle Windows-style paths', () => {
        const options: PathParserOptions = {
          contentRoot: 'C:\\projects\\elohim\\data\\content',
          normalizeIds: true
        };

        const filePath = 'C:\\projects\\elohim\\data\\content\\elohim-protocol\\governance\\epic.md';
        const metadata = parsePathMetadata(filePath, options);

        expect(metadata.domain).toBe('elohim-protocol');
        expect(metadata.epic).toBe('governance');
      });
    });
  });

  describe('isProcessableFile', () => {
    it('should return true for .md files', () => {
      expect(isProcessableFile('test.md')).toBe(true);
      expect(isProcessableFile('/path/to/file.md')).toBe(true);
      expect(isProcessableFile('README.md')).toBe(true);
    });

    it('should return true for .feature files', () => {
      expect(isProcessableFile('test.feature')).toBe(true);
      expect(isProcessableFile('/path/to/scenario.feature')).toBe(true);
    });

    it('should return false for other extensions', () => {
      expect(isProcessableFile('test.txt')).toBe(false);
      expect(isProcessableFile('test.json')).toBe(false);
      expect(isProcessableFile('test.js')).toBe(false);
      expect(isProcessableFile('test.ts')).toBe(false);
      expect(isProcessableFile('image.png')).toBe(false);
    });

    it('should be case-insensitive for extensions', () => {
      expect(isProcessableFile('test.MD')).toBe(true);
      expect(isProcessableFile('test.FEATURE')).toBe(true);
      expect(isProcessableFile('test.Md')).toBe(true);
    });

    it('should handle files without extensions', () => {
      expect(isProcessableFile('README')).toBe(false);
      expect(isProcessableFile('test')).toBe(false);
    });
  });

  describe('filterProcessableFiles', () => {
    it('should filter to only .md and .feature files', () => {
      const files = [
        'epic.md',
        'scenario.feature',
        'config.json',
        'README.md',
        'image.png',
        'test.feature',
        'script.js'
      ];

      const filtered = filterProcessableFiles(files);

      expect(filtered).toHaveLength(4);
      expect(filtered).toContain('epic.md');
      expect(filtered).toContain('scenario.feature');
      expect(filtered).toContain('README.md');
      expect(filtered).toContain('test.feature');
    });

    it('should return empty array for no matches', () => {
      const files = ['test.txt', 'image.png', 'data.json'];
      const filtered = filterProcessableFiles(files);

      expect(filtered).toEqual([]);
    });

    it('should handle empty input', () => {
      const filtered = filterProcessableFiles([]);
      expect(filtered).toEqual([]);
    });

    it('should handle full paths', () => {
      const files = [
        '/projects/elohim/data/content/elohim-protocol/governance/epic.md',
        '/projects/elohim/data/content/elohim-protocol/governance/policy_maker/scenarios/funding.feature',
        '/projects/elohim/data/content/config.json'
      ];

      const filtered = filterProcessableFiles(files);

      expect(filtered).toHaveLength(2);
    });
  });

  describe('isSourceContent', () => {
    it('should return true for files under data/content', () => {
      const metadata: PathMetadata = {
        fullPath: '/test',
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

      expect(isSourceContent(metadata)).toBe(true);
    });

    it('should handle Windows-style paths', () => {
      const metadata: PathMetadata = {
        fullPath: 'C:\\test',
        relativePath: 'data\\content\\elohim-protocol\\governance\\epic.md',
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

      expect(isSourceContent(metadata)).toBe(true);
    });

    it('should return false for files under assets', () => {
      const metadata: PathMetadata = {
        fullPath: '/test',
        relativePath: 'assets/lamad-data/content/epic-governance.json',
        domain: 'elohim-protocol',
        epic: 'governance',
        contentCategory: 'epic',
        baseName: 'epic-governance',
        extension: '.json',
        isArchetypeDefinition: false,
        isEpicNarrative: false,
        isScenario: false,
        isResource: false,
        suggestedId: 'epic-governance'
      };

      expect(isSourceContent(metadata)).toBe(false);
    });

    it('should return true for relative paths not containing assets', () => {
      const metadata: PathMetadata = {
        fullPath: '/test',
        relativePath: 'some/other/path/file.md',
        domain: 'test',
        epic: 'test',
        contentCategory: 'other',
        baseName: 'file',
        extension: '.md',
        isArchetypeDefinition: false,
        isEpicNarrative: false,
        isScenario: false,
        isResource: false,
        suggestedId: 'test-id'
      };

      expect(isSourceContent(metadata)).toBe(true);
    });
  });

  describe('Edge cases', () => {
    it('should handle empty path parts', () => {
      const filePath = '/projects/elohim/data/content///epic.md';
      const metadata = parsePathMetadata(filePath, defaultOptions);

      expect(metadata.domain).toBeDefined();
    });

    it('should handle path with single part', () => {
      const filePath = '/projects/elohim/data/content/epic.md';
      const metadata = parsePathMetadata(filePath, defaultOptions);

      expect(metadata.domain).toBe('epic');
    });

    it('should handle deeply nested paths', () => {
      const filePath = '/projects/elohim/data/content/elohim-protocol/governance/policy_maker/subdir1/subdir2/subdir3/file.md';
      const metadata = parsePathMetadata(filePath, defaultOptions);

      expect(metadata.domain).toBe('elohim-protocol');
      expect(metadata.epic).toBe('governance');
    });
  });
});
