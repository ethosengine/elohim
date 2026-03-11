/**
 * Standards Service Tests
 *
 * Tests for standards-compliant metadata generation:
 * - W3C DIDs
 * - ActivityPub types
 * - Open Graph metadata
 * - JSON-LD linked data
 */

import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import {
  generateDid,
  inferActivityPubType,
  getGitTimestamps,
  generateOpenGraphMetadata,
  generateLinkedData,
  generateStandardsFields,
  enrichWithStandards,
  validateStandardsFields,
  generateCoverageReport
} from './standards.service';
import { ContentNode } from '../models/content-node.model';

describe('Standards Service', () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'standards-service-test-'));
  });

  afterEach(() => {
    if (fs.existsSync(tempDir)) {
      fs.rmSync(tempDir, { recursive: true, force: true });
    }
  });

  describe('generateDid', () => {
    it('should generate DID from simple path', () => {
      const did = generateDid('content/epics/value-scanner.md');

      expect(did).toBe('did:web:elohim.host:content:content:epics:value-scanner');
    });

    it('should remove .md extension', () => {
      const did = generateDid('test.md');

      expect(did).toBe('did:web:elohim.host:content:test');
    });

    it('should remove .feature extension', () => {
      const did = generateDid('scenarios/test.feature');

      expect(did).toBe('did:web:elohim.host:content:scenarios:test');
    });

    it('should convert slashes to colons', () => {
      const did = generateDid('path/to/content.md');

      expect(did).toBe('did:web:elohim.host:content:path:to:content');
    });

    it('should convert underscores to hyphens', () => {
      const did = generateDid('value_scanner/test_file.md');

      expect(did).toBe('did:web:elohim.host:content:value-scanner:test-file');
    });

    it('should lowercase the path', () => {
      const did = generateDid('Content/EPIC/Value_Scanner.md');

      expect(did).toBe('did:web:elohim.host:content:content:epic:value-scanner');
    });

    it('should collapse multiple dashes', () => {
      const did = generateDid('test--multiple---dashes.md');

      expect(did).toBe('did:web:elohim.host:content:test-multiple-dashes');
    });

    it('should remove leading and trailing separators', () => {
      const did = generateDid('---test--.md');

      expect(did).toBe('did:web:elohim.host:content:test');
    });

    it('should support custom node type', () => {
      const did = generateDid('test.md', 'role');

      expect(did).toBe('did:web:elohim.host:role:test');
    });
  });

  describe('inferActivityPubType', () => {
    it('should map epic to Article', () => {
      expect(inferActivityPubType('epic')).toBe('Article');
    });

    it('should map video to Video', () => {
      expect(inferActivityPubType('video')).toBe('Video');
    });

    it('should map scenario to Note', () => {
      expect(inferActivityPubType('scenario')).toBe('Note');
    });

    it('should map organization to Organization', () => {
      expect(inferActivityPubType('organization')).toBe('Organization');
    });

    it('should default to Page for unknown types', () => {
      expect(inferActivityPubType('unknown-type')).toBe('Page');
    });

    it('should handle all defined content types', () => {
      const types = [
        'epic', 'feature', 'scenario', 'video', 'book', 'book-chapter',
        'bible-verse', 'course-module', 'simulation', 'assessment',
        'concept', 'organization', 'podcast', 'article', 'source',
        'role', 'reference', 'example'
      ];

      types.forEach(type => {
        const result = inferActivityPubType(type);
        expect(result).toBeDefined();
        expect(typeof result).toBe('string');
      });
    });
  });

  describe('getGitTimestamps', () => {
    it('should fall back to file system timestamps when not in git', () => {
      const testFile = path.join(tempDir, 'test.txt');
      fs.writeFileSync(testFile, 'content', 'utf-8');

      const timestamps = getGitTimestamps(testFile);

      expect(timestamps.created).toBeDefined();
      expect(timestamps.modified).toBeDefined();
      expect(new Date(timestamps.created)).toBeInstanceOf(Date);
    });

    it('should fall back to current time for non-existent files', () => {
      const timestamps = getGitTimestamps('/nonexistent/file.txt');

      expect(timestamps.created).toBeDefined();
      expect(timestamps.modified).toBeDefined();
      expect(new Date(timestamps.created)).toBeInstanceOf(Date);
    });

    // TODO(test-generator): [LOW] Add git integration tests
    // Context: getGitTimestamps uses execSync to call git commands
    // Story: Testing git-dependent functionality requires git repo setup
    // Suggested approach:
    //   1. Create a temporary git repo in tests
    //   2. Add test files with commits
    //   3. Verify timestamp extraction
    //   4. Test edge cases (uncommitted files, renamed files)
  });

  describe('generateOpenGraphMetadata', () => {
    it('should generate basic Open Graph metadata', () => {
      const og = generateOpenGraphMetadata(
        'Test Title',
        'Test description',
        'content-123',
        'epic',
        {},
        { created: '2024-01-01T00:00:00Z', modified: '2024-01-02T00:00:00Z' }
      );

      expect(og.ogTitle).toBe('Test Title');
      expect(og.ogDescription).toBe('Test description');
      expect(og.ogType).toBe('article');
      expect(og.ogUrl).toBe('https://elohim-protocol.org/content/content-123');
      expect(og.ogSiteName).toBe('Elohim Protocol - Lamad Learning Platform');
    });

    it('should truncate long descriptions', () => {
      const longDescription = 'a'.repeat(300);
      const og = generateOpenGraphMetadata(
        'Title',
        longDescription,
        'content-1',
        'epic'
      );

      expect(og.ogDescription.length).toBeLessThanOrEqual(200);
    });

    it('should use title as description if description is empty', () => {
      const og = generateOpenGraphMetadata('Title', '', 'content-1', 'epic');

      expect(og.ogDescription).toBe('Title');
    });

    it('should set type to article for article-like content', () => {
      const articleTypes = ['epic', 'feature', 'scenario', 'concept', 'course-module', 'article'];

      articleTypes.forEach(type => {
        const og = generateOpenGraphMetadata('Title', 'Desc', 'id', type);
        expect(og.ogType).toBe('article');
      });
    });

    it('should set type to website for other content', () => {
      const og = generateOpenGraphMetadata('Title', 'Desc', 'id', 'video');

      expect(og.ogType).toBe('website');
    });

    it('should add article timestamps for article types', () => {
      const og = generateOpenGraphMetadata(
        'Title',
        'Desc',
        'id',
        'epic',
        {},
        { created: '2024-01-01T00:00:00Z', modified: '2024-01-02T00:00:00Z' }
      );

      expect(og.articlePublishedTime).toBe('2024-01-01T00:00:00Z');
      expect(og.articleModifiedTime).toBe('2024-01-02T00:00:00Z');
    });

    it('should add epic as article section', () => {
      const og = generateOpenGraphMetadata(
        'Title',
        'Desc',
        'id',
        'scenario',
        { epic: 'value-scanner' }
      );

      expect(og.articleSection).toBe('value-scanner');
    });

    it('should generate default image URL', () => {
      const og = generateOpenGraphMetadata('Title', 'Desc', 'id', 'video');

      expect(og.ogImage).toBe('https://elohim-protocol.org/assets/images/og-defaults/video.jpg');
      expect(og.ogImageAlt).toBe('Title - Elohim Protocol');
    });
  });

  describe('generateLinkedData', () => {
    it('should generate basic JSON-LD structure', () => {
      const ld = generateLinkedData(
        'content-123',
        'did:web:elohim.host:content:test',
        'epic',
        'Test Title',
        'Test description',
        { created: '2024-01-01T00:00:00Z', modified: '2024-01-02T00:00:00Z' }
      );

      expect(ld['@context']).toBe('https://schema.org/');
      expect(ld['@type']).toBe('Article');
      expect(ld['@id']).toBe('https://elohim-protocol.org/content/content-123');
      expect(ld.identifier).toBe('did:web:elohim.host:content:test');
      expect(ld.name).toBe('Test Title');
      expect(ld.description).toBe('Test description');
      expect(ld.dateCreated).toBe('2024-01-01T00:00:00Z');
      expect(ld.dateModified).toBe('2024-01-02T00:00:00Z');
    });

    it('should include publisher information', () => {
      const ld = generateLinkedData(
        'content-1',
        'did:test',
        'epic',
        'Title',
        'Description',
        { created: '', modified: '' }
      );

      expect(ld.publisher).toEqual({
        '@type': 'Organization',
        '@id': 'https://elohim-protocol.org',
        name: 'Elohim Protocol'
      });
    });

    it('should add author when provided', () => {
      const ld = generateLinkedData(
        'content-1',
        'did:test',
        'epic',
        'Title',
        'Description',
        { created: '', modified: '' },
        { author: 'Jane Doe' }
      );

      expect(ld.author).toEqual({
        '@type': 'Person',
        name: 'Jane Doe'
      });
    });

    it('should add epic as isPartOf', () => {
      const ld = generateLinkedData(
        'content-1',
        'did:test',
        'scenario',
        'Title',
        'Description',
        { created: '', modified: '' },
        { epic: 'value-scanner' }
      );

      expect(ld.isPartOf).toEqual({
        '@type': 'CreativeWorkSeries',
        name: 'value-scanner'
      });
    });

    it('should use title as description if description is empty', () => {
      const ld = generateLinkedData(
        'content-1',
        'did:test',
        'epic',
        'Title',
        '',
        { created: '', modified: '' }
      );

      expect(ld.description).toBe('Title');
    });

    it('should map content types to Schema.org types', () => {
      const mappings = [
        ['video', 'VideoObject'],
        ['book', 'Book'],
        ['book-chapter', 'Chapter'],
        ['assessment', 'Quiz'],
        ['organization', 'Organization'],
        ['podcast', 'PodcastEpisode']
      ];

      mappings.forEach(([contentType, schemaType]) => {
        const ld = generateLinkedData(
          'id',
          'did:test',
          contentType,
          'Title',
          'Desc',
          { created: '', modified: '' }
        );
        expect(ld['@type']).toBe(schemaType);
      });
    });

    it('should default to CreativeWork for unknown types', () => {
      const ld = generateLinkedData(
        'id',
        'did:test',
        'unknown-type',
        'Title',
        'Desc',
        { created: '', modified: '' }
      );

      expect(ld['@type']).toBe('CreativeWork');
    });
  });

  describe('generateStandardsFields', () => {
    it('should generate all standards fields for a node', () => {
      const node: ContentNode = {
        id: 'epic-value-scanner',
        contentType: 'epic',
        title: 'Value Scanner',
        description: 'Care economy epic',
        content: 'Full content...',
        contentFormat: 'markdown',
        tags: ['epic', 'care-economy'],
        relatedNodeIds: [],
        metadata: { epic: 'value-scanner' },
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-02T00:00:00Z'
      };

      const fields = generateStandardsFields(node, 'content/epics/value-scanner.md');

      expect(fields.did).toContain('did:web:elohim.host');
      expect(fields.activityPubType).toBe('Article');
      expect(fields.openGraphMetadata.ogTitle).toBe('Value Scanner');
      expect(fields.linkedData['@type']).toBe('Article');
      expect(fields.linkedData.name).toBe('Value Scanner');
    });

    it('should use node.id as fallback path', () => {
      const node: ContentNode = {
        id: 'test-node',
        contentType: 'concept',
        title: 'Test',
        description: 'Test',
        content: 'Content',
        contentFormat: 'markdown',
        tags: [],
        relatedNodeIds: [],
        metadata: {},
        createdAt: '',
        updatedAt: ''
      };

      const fields = generateStandardsFields(node);

      expect(fields.did).toContain('test-node');
    });

    it('should use node.sourcePath if no path provided', () => {
      const node: ContentNode = {
        id: 'test-node',
        contentType: 'concept',
        title: 'Test',
        description: 'Test',
        content: 'Content',
        contentFormat: 'markdown',
        tags: [],
        sourcePath: 'original/path.md',
        relatedNodeIds: [],
        metadata: {},
        createdAt: '',
        updatedAt: ''
      };

      const fields = generateStandardsFields(node);

      expect(fields.did).toContain('original:path');
    });
  });

  describe('enrichWithStandards', () => {
    it('should enrich node with all standards fields', () => {
      const node: ContentNode = {
        id: 'test-1',
        contentType: 'scenario',
        title: 'Test Scenario',
        description: 'A test scenario',
        content: 'Content',
        contentFormat: 'gherkin',
        tags: ['test'],
        relatedNodeIds: [],
        metadata: {},
        createdAt: '',
        updatedAt: ''
      };

      const enriched = enrichWithStandards(node);

      expect(enriched.id).toBe('test-1');
      expect(enriched.title).toBe('Test Scenario');
      expect(enriched.did).toBeDefined();
      expect(enriched.activityPubType).toBe('Note');
      expect(enriched.openGraphMetadata).toBeDefined();
      expect(enriched.linkedData).toBeDefined();
    });

    it('should preserve original node properties', () => {
      const node: ContentNode = {
        id: 'test-1',
        contentType: 'epic',
        title: 'Epic',
        description: 'Description',
        content: 'Content',
        contentFormat: 'markdown',
        tags: ['tag1', 'tag2'],
        relatedNodeIds: ['node-1'],
        metadata: { custom: 'value' },
        createdAt: '2024-01-01T00:00:00Z',
        updatedAt: '2024-01-02T00:00:00Z'
      };

      const enriched = enrichWithStandards(node);

      expect(enriched.tags).toEqual(['tag1', 'tag2']);
      expect(enriched.relatedNodeIds).toEqual(['node-1']);
      expect(enriched.metadata.custom).toBe('value');
    });
  });

  describe('validateStandardsFields', () => {
    it('should validate complete standards fields', () => {
      const node = {
        id: 'test-1',
        did: 'did:web:elohim.host:content:test',
        activityPubType: 'Article',
        linkedData: {
          '@context': 'https://schema.org/',
          '@type': 'Article'
        },
        openGraphMetadata: {
          ogTitle: 'Title',
          ogDescription: 'Description',
          ogUrl: 'https://example.com'
        }
      };

      const results = validateStandardsFields(node);

      expect(results).toHaveLength(4);
      expect(results.every(r => r.valid)).toBe(true);
    });

    it('should detect missing DID', () => {
      const node = {
        activityPubType: 'Article',
        linkedData: { '@context': 'https://schema.org/', '@type': 'Article' },
        openGraphMetadata: { ogTitle: 'T', ogDescription: 'D', ogUrl: 'U' }
      };

      const results = validateStandardsFields(node);
      const didResult = results.find(r => r.field === 'did');

      expect(didResult?.valid).toBe(false);
      expect(didResult?.error).toBe('Missing DID');
    });

    it('should detect invalid DID format', () => {
      const node = {
        did: 'invalid-did',
        activityPubType: 'Article',
        linkedData: { '@context': 'https://schema.org/', '@type': 'Article' },
        openGraphMetadata: { ogTitle: 'T', ogDescription: 'D', ogUrl: 'U' }
      };

      const results = validateStandardsFields(node);
      const didResult = results.find(r => r.field === 'did');

      expect(didResult?.valid).toBe(false);
      expect(didResult?.error).toContain('Invalid DID format');
    });

    it('should detect missing activityPubType', () => {
      const node = {
        did: 'did:web:test',
        linkedData: { '@context': 'https://schema.org/', '@type': 'Article' },
        openGraphMetadata: { ogTitle: 'T', ogDescription: 'D', ogUrl: 'U' }
      };

      const results = validateStandardsFields(node);
      const apResult = results.find(r => r.field === 'activityPubType');

      expect(apResult?.valid).toBe(false);
      expect(apResult?.error).toBe('Missing activityPubType');
    });

    it('should detect missing JSON-LD @context', () => {
      const node = {
        did: 'did:web:test',
        activityPubType: 'Article',
        linkedData: { '@type': 'Article' },
        openGraphMetadata: { ogTitle: 'T', ogDescription: 'D', ogUrl: 'U' }
      };

      const results = validateStandardsFields(node);
      const ldResult = results.find(r => r.field === 'linkedData');

      expect(ldResult?.valid).toBe(false);
      expect(ldResult?.error).toContain('@context');
    });

    it('should detect missing JSON-LD @type', () => {
      const node = {
        did: 'did:web:test',
        activityPubType: 'Article',
        linkedData: { '@context': 'https://schema.org/' },
        openGraphMetadata: { ogTitle: 'T', ogDescription: 'D', ogUrl: 'U' }
      };

      const results = validateStandardsFields(node);
      const ldResult = results.find(r => r.field === 'linkedData');

      expect(ldResult?.valid).toBe(false);
      expect(ldResult?.error).toContain('@type');
    });

    it('should detect missing Open Graph fields', () => {
      const node = {
        did: 'did:web:test',
        activityPubType: 'Article',
        linkedData: { '@context': 'https://schema.org/', '@type': 'Article' },
        openGraphMetadata: { ogTitle: 'Title' }
      };

      const results = validateStandardsFields(node);
      const ogResult = results.find(r => r.field === 'openGraphMetadata');

      expect(ogResult?.valid).toBe(false);
      expect(ogResult?.error).toContain('ogDescription');
    });
  });

  describe('generateCoverageReport', () => {
    it('should generate coverage report for nodes', () => {
      const nodes = [
        {
          id: 'node-1',
          did: 'did:web:test',
          activityPubType: 'Article',
          linkedData: { '@context': 'https://schema.org/', '@type': 'Article' },
          openGraphMetadata: { ogTitle: 'T', ogDescription: 'D', ogUrl: 'U' }
        },
        {
          id: 'node-2',
          did: 'did:web:test2',
          activityPubType: 'Note'
          // missing linkedData and openGraphMetadata
        }
      ];

      const report = generateCoverageReport(nodes);

      expect(report.total).toBe(2);
      expect(report.coverage.did.count).toBe(2);
      expect(report.coverage.did.percentage).toBe(100);
      expect(report.coverage.activityPubType.count).toBe(2);
      expect(report.coverage.linkedData.count).toBe(1);
      expect(report.coverage.linkedData.percentage).toBe(50);
      expect(report.coverage.openGraphMetadata.count).toBe(1);
    });

    it('should check if all targets are met', () => {
      const completeNodes = [
        {
          id: 'node-1',
          did: 'did:web:test',
          activityPubType: 'Article',
          linkedData: { '@context': 'https://schema.org/', '@type': 'Article' },
          openGraphMetadata: { ogTitle: 'T', ogDescription: 'D', ogUrl: 'U' }
        }
      ];

      const report = generateCoverageReport(completeNodes);

      expect(report.allTargetsMet).toBe(true);
    });

    it('should detect when targets are not met', () => {
      const incompleteNodes = [
        {
          id: 'node-1',
          did: 'did:web:test',
          activityPubType: 'Article'
          // missing linkedData and openGraphMetadata (need 80% coverage)
        }
      ];

      const report = generateCoverageReport(incompleteNodes);

      expect(report.allTargetsMet).toBe(false);
    });

    it('should collect errors', () => {
      const invalidNodes = [
        {
          id: 'node-1',
          did: 'invalid-did'
        }
      ];

      const report = generateCoverageReport(invalidNodes);

      expect(report.errors.length).toBeGreaterThan(0);
      expect(report.errors.some(e => e.includes('Invalid DID'))).toBe(true);
    });

    it('should handle empty node list', () => {
      const report = generateCoverageReport([]);

      expect(report.total).toBe(0);
      expect(report.coverage.did.percentage).toBe(0);
      expect(report.errors).toEqual([]);
    });
  });
});
