import { TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';
import { ContentService, ContentIndexEntry, PathReference, ContentAccessResult } from './content.service';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { AgentService } from '@app/elohim/services/agent.service';
import { ContentNode, ContentType } from '../models/content-node.model';

describe('ContentService', () => {
  let service: ContentService;
  let dataLoaderSpy: jasmine.SpyObj<DataLoaderService>;
  let agentServiceSpy: jasmine.SpyObj<AgentService>;

  const mockContent: ContentNode = {
    id: 'test-content',
    title: 'Test Content',
    description: 'A test content node',
    contentType: 'concept',
    contentFormat: 'markdown',
    content: '# Test Content\n\nThis is test content.',
    tags: ['test', 'example'],
    relatedNodeIds: ['related-1', 'related-2'],
    metadata: {}
  };

  const mockRelatedContent: ContentNode = {
    id: 'related-1',
    title: 'Related Content',
    description: 'Related content node',
    contentType: 'concept',
    contentFormat: 'markdown',
    content: '# Related',
    tags: ['related'],
    relatedNodeIds: [],
    metadata: {}
  };

  const mockContentIndex = {
    nodes: [
      {
        id: 'content-1',
        title: 'Introduction to TypeScript',
        description: 'Learn TypeScript basics',
        contentType: 'concept' as ContentType,
        tags: ['typescript', 'programming']
      },
      {
        id: 'content-2',
        title: 'Angular Tutorial',
        description: 'Build apps with Angular',
        contentType: 'video' as ContentType,
        tags: ['angular', 'framework']
      },
      {
        id: 'content-3',
        title: 'Testing Guide',
        description: 'Write tests for Angular',
        contentType: 'book-chapter' as ContentType,
        tags: ['testing', 'angular']
      }
    ]
  };

  beforeEach(() => {
    const dataLoaderSpyObj = jasmine.createSpyObj('DataLoaderService', [
      'getContent',
      'getContentIndex'
    ]);
    const agentServiceSpyObj = jasmine.createSpyObj('AgentService', [
      'getAttestations'
    ]);

    TestBed.configureTestingModule({
      providers: [
        ContentService,
        { provide: DataLoaderService, useValue: dataLoaderSpyObj },
        { provide: AgentService, useValue: agentServiceSpyObj }
      ]
    });

    service = TestBed.inject(ContentService);
    dataLoaderSpy = TestBed.inject(DataLoaderService) as jasmine.SpyObj<DataLoaderService>;
    agentServiceSpy = TestBed.inject(AgentService) as jasmine.SpyObj<AgentService>;

    // Default spy return values
    dataLoaderSpy.getContent.and.returnValue(of(mockContent));
    dataLoaderSpy.getContentIndex.and.returnValue(of(mockContentIndex));
    agentServiceSpy.getAttestations.and.returnValue([]);
  });

  it('should be created', () => {
    expect(service).toBeInstanceOf(ContentService);
  });

  describe('getContent', () => {
    it('should get content by ID', (done) => {
      service.getContent('test-content').subscribe(content => {
        expect(content).toEqual(mockContent);
        expect(dataLoaderSpy.getContent).toHaveBeenCalledWith('test-content');
        done();
      });
    });

    it('should handle content load error', (done) => {
      dataLoaderSpy.getContent.and.returnValue(throwError(() => new Error('Not found')));

      service.getContent('missing').subscribe({
        error: err => {
          expect(err.message).toBe('Not found');
          done();
        }
      });
    });
  });

  describe('getRelatedResourceIds', () => {
    it('should get related resource IDs', (done) => {
      service.getRelatedResourceIds('test-content').subscribe(ids => {
        expect(ids).toEqual(['related-1', 'related-2']);
        done();
      });
    });

    it('should return empty array if no related nodes', (done) => {
      const contentNoRelated: ContentNode = {
        ...mockContent,
        relatedNodeIds: []
      };
      dataLoaderSpy.getContent.and.returnValue(of(contentNoRelated));

      service.getRelatedResourceIds('test-content').subscribe(ids => {
        expect(ids).toEqual([]);
        done();
      });
    });

    it('should handle undefined relatedNodeIds', (done) => {
      const contentNoProperty: any = {
        ...mockContent
      };
      delete contentNoProperty.relatedNodeIds;
      dataLoaderSpy.getContent.and.returnValue(of(contentNoProperty));

      service.getRelatedResourceIds('test-content').subscribe(ids => {
        expect(ids).toEqual([]);
        done();
      });
    });
  });

  describe('getRelatedResource', () => {
    it('should get related resource by index', (done) => {
      dataLoaderSpy.getContent.and.returnValues(
        of(mockContent),
        of(mockRelatedContent)
      );

      service.getRelatedResource('test-content', 0).subscribe(content => {
        expect(content).toEqual(mockRelatedContent);
        expect(dataLoaderSpy.getContent).toHaveBeenCalledWith('related-1');
        done();
      });
    });

    it('should return null for invalid index (negative)', (done) => {
      service.getRelatedResource('test-content', -1).subscribe(content => {
        expect(content).toBeNull();
        done();
      });
    });

    it('should return null for invalid index (too large)', (done) => {
      service.getRelatedResource('test-content', 5).subscribe(content => {
        expect(content).toBeNull();
        done();
      });
    });

    it('should return null if related content fails to load', (done) => {
      dataLoaderSpy.getContent.and.returnValues(
        of(mockContent),
        throwError(() => new Error('Load failed'))
      );

      service.getRelatedResource('test-content', 0).subscribe(content => {
        expect(content).toBeNull();
        done();
      });
    });
  });

  describe('searchContent', () => {
    it('should search content by title', (done) => {
      service.searchContent('TypeScript').subscribe(results => {
        expect(results.length).toBe(1);
        expect(results[0].id).toBe('content-1');
        done();
      });
    });

    it('should search content by description', (done) => {
      service.searchContent('Build apps').subscribe(results => {
        expect(results.length).toBe(1);
        expect(results[0].id).toBe('content-2');
        done();
      });
    });

    it('should search content by tags', (done) => {
      service.searchContent('angular').subscribe(results => {
        expect(results.length).toBe(2); // content-2 and content-3
        expect(results[0].id).toBe('content-2');
        expect(results[1].id).toBe('content-3');
        done();
      });
    });

    it('should be case-insensitive', (done) => {
      service.searchContent('TYPESCRIPT').subscribe(results => {
        expect(results.length).toBe(1);
        expect(results[0].id).toBe('content-1');
        done();
      });
    });

    it('should return all content for empty query', (done) => {
      service.searchContent('').subscribe(results => {
        expect(results.length).toBe(3);
        done();
      });
    });

    it('should return all content for whitespace query', (done) => {
      service.searchContent('   ').subscribe(results => {
        expect(results.length).toBe(3);
        done();
      });
    });

    it('should return empty array for no matches', (done) => {
      service.searchContent('nonexistent').subscribe(results => {
        expect(results.length).toBe(0);
        done();
      });
    });

    it('should trim search query', (done) => {
      service.searchContent('  TypeScript  ').subscribe(results => {
        expect(results.length).toBe(1);
        expect(results[0].id).toBe('content-1');
        done();
      });
    });
  });

  describe('getContentByType', () => {
    it('should filter content by type', (done) => {
      service.getContentByType('concept').subscribe(results => {
        expect(results.length).toBe(1);
        expect(results[0].id).toBe('content-1');
        done();
      });
    });

    it('should return empty array for no matches', (done) => {
      service.getContentByType('assessment').subscribe(results => {
        expect(results.length).toBe(0);
        done();
      });
    });

    it('should handle multiple matches', (done) => {
      const indexWithDuplicates = {
        nodes: [
          ...mockContentIndex.nodes,
          {
            id: 'content-4',
            title: 'Another Video',
            description: 'More videos',
            contentType: 'video' as ContentType,
            tags: ['example']
          }
        ]
      };
      dataLoaderSpy.getContentIndex.and.returnValue(of(indexWithDuplicates));

      service.getContentByType('video').subscribe(results => {
        expect(results.length).toBe(2);
        expect(results[0].id).toBe('content-2');
        expect(results[1].id).toBe('content-4');
        done();
      });
    });

    it('should handle empty content index', (done) => {
      const emptyIndex = { nodes: [] };
      dataLoaderSpy.getContentIndex.and.returnValue(of(emptyIndex));

      service.getContentByType('concept').subscribe(results => {
        expect(results.length).toBe(0);
        done();
      });
    });
  });

  describe('additional edge cases', () => {
    it('should handle missing description in search', (done) => {
      const indexWithNoDescription = {
        nodes: [
          {
            id: 'content-1',
            title: 'Test',
            description: undefined as any,
            contentType: 'concept' as ContentType,
            tags: []
          }
        ]
      };
      dataLoaderSpy.getContentIndex.and.returnValue(of(indexWithNoDescription));

      service.searchContent('Test').subscribe(results => {
        expect(results.length).toBe(1);
        done();
      });
    });

    it('should handle missing tags in search', (done) => {
      const indexWithNoTags = {
        nodes: [
          {
            id: 'content-1',
            title: 'Test',
            description: 'A test',
            contentType: 'concept' as ContentType,
            tags: undefined as any
          }
        ]
      };
      dataLoaderSpy.getContentIndex.and.returnValue(of(indexWithNoTags));

      service.searchContent('Test').subscribe(results => {
        expect(results.length).toBe(1);
        done();
      });
    });
  });

  describe('getContentByTag', () => {
    it('should filter content by tag', (done) => {
      service.getContentByTag('angular').subscribe(results => {
        expect(results.length).toBe(2);
        expect(results[0].id).toBe('content-2');
        expect(results[1].id).toBe('content-3');
        done();
      });
    });

    it('should be case-insensitive', (done) => {
      service.getContentByTag('TYPESCRIPT').subscribe(results => {
        expect(results.length).toBe(1);
        expect(results[0].id).toBe('content-1');
        done();
      });
    });

    it('should return empty array for non-existent tag', (done) => {
      service.getContentByTag('nonexistent').subscribe(results => {
        expect(results.length).toBe(0);
        done();
      });
    });

    it('should handle empty content index', (done) => {
      dataLoaderSpy.getContentIndex.and.returnValue(of({ nodes: [] }));
      service.getContentByTag('test').subscribe(results => {
        expect(results.length).toBe(0);
        done();
      });
    });
  });

  describe('getAllTags', () => {
    it('should return all unique tags sorted alphabetically', (done) => {
      service.getAllTags().subscribe(tags => {
        expect(tags).toEqual(['angular', 'framework', 'programming', 'testing', 'typescript']);
        done();
      });
    });

    it('should handle empty content index', (done) => {
      dataLoaderSpy.getContentIndex.and.returnValue(of({ nodes: [] }));
      service.getAllTags().subscribe(tags => {
        expect(tags).toEqual([]);
        done();
      });
    });

    it('should handle nodes without tags', (done) => {
      const indexWithNoTags = {
        nodes: [
          {
            id: 'content-1',
            title: 'Test',
            description: 'Test',
            contentType: 'concept' as ContentType,
            tags: undefined as any
          }
        ]
      };
      dataLoaderSpy.getContentIndex.and.returnValue(of(indexWithNoTags));
      service.getAllTags().subscribe(tags => {
        expect(tags).toEqual([]);
        done();
      });
    });
  });

  describe('getAllContentTypes', () => {
    it('should return all unique content types', (done) => {
      service.getAllContentTypes().subscribe(types => {
        expect(types.length).toBe(3);
        expect(types).toContain('concept');
        expect(types).toContain('video');
        expect(types).toContain('book-chapter');
        done();
      });
    });

    it('should handle empty content index', (done) => {
      dataLoaderSpy.getContentIndex.and.returnValue(of({ nodes: [] }));
      service.getAllContentTypes().subscribe(types => {
        expect(types).toEqual([]);
        done();
      });
    });

    it('should skip nodes without contentType', (done) => {
      const indexMixed = {
        nodes: [
          {
            id: 'content-1',
            title: 'Test',
            description: 'Test',
            contentType: 'concept' as ContentType,
            tags: []
          },
          {
            id: 'content-2',
            title: 'No Type',
            description: 'No type',
            contentType: undefined as any,
            tags: []
          }
        ]
      };
      dataLoaderSpy.getContentIndex.and.returnValue(of(indexMixed));
      service.getAllContentTypes().subscribe(types => {
        expect(types).toEqual(['concept']);
        done();
      });
    });
  });

  describe('getContentWithAccessCheck', () => {
    beforeEach(() => {
      agentServiceSpy.getCurrentAgentId = jasmine.createSpy().and.returnValue('agent-1');
    });

    it('should allow access to commons content', (done) => {
      const commonsContent: ContentNode = {
        ...mockContent,
        reach: 'commons'
      };
      dataLoaderSpy.getContent.and.returnValue(of(commonsContent));

      service.getContentWithAccessCheck('test-content').subscribe(result => {
        expect(result.canAccess).toBeTrue();
        expect(result.content).toEqual(commonsContent);
        done();
      });
    });

    it('should allow access to content when agent is author', (done) => {
      const privateContent: ContentNode = {
        ...mockContent,
        reach: 'private',
        authorId: 'agent-1'
      };
      dataLoaderSpy.getContent.and.returnValue(of(privateContent));

      service.getContentWithAccessCheck('test-content').subscribe(result => {
        expect(result.canAccess).toBeTrue();
        expect(result.content).toEqual(privateContent);
        done();
      });
    });

    it('should deny access to private content for non-author', (done) => {
      const privateContent: ContentNode = {
        ...mockContent,
        reach: 'private',
        authorId: 'agent-2'
      };
      dataLoaderSpy.getContent.and.returnValue(of(privateContent));
      agentServiceSpy.getAttestations.and.returnValue([]);
      // Override getCurrentAgentId to return null (unauthenticated)
      agentServiceSpy.getCurrentAgentId = jasmine.createSpy().and.returnValue(null);

      service.getContentWithAccessCheck('test-content').subscribe(result => {
        expect(result.canAccess).toBeFalse();
        expect(result.reason).toBe('unauthenticated');
        expect(result.requiredReach).toBe('private');
        done();
      });
    });

    it('should allow access for invited agents', (done) => {
      const invitedContent: ContentNode = {
        ...mockContent,
        reach: 'invited',
        invitedAgentIds: ['agent-1', 'agent-3']
      };
      dataLoaderSpy.getContent.and.returnValue(of(invitedContent));
      agentServiceSpy.getAttestations.and.returnValue([]);

      service.getContentWithAccessCheck('test-content').subscribe(result => {
        expect(result.canAccess).toBeTrue();
        expect(result.content).toEqual(invitedContent);
        done();
      });
    });

    it('should handle not-found error', (done) => {
      dataLoaderSpy.getContent.and.returnValue(throwError(() => new Error('Not found')));

      service.getContentWithAccessCheck('missing').subscribe(result => {
        expect(result.canAccess).toBeFalse();
        expect(result.reason).toBe('not-found');
        done();
      });
    });

    it('should default to commons reach for content without reach property', (done) => {
      const contentNoReach: ContentNode = {
        ...mockContent
      };
      delete (contentNoReach as any).reach;
      dataLoaderSpy.getContent.and.returnValue(of(contentNoReach));

      service.getContentWithAccessCheck('test-content').subscribe(result => {
        expect(result.canAccess).toBeTrue();
        done();
      });
    });

    it('should allow access when agent reach is sufficient', (done) => {
      const municipalContent: ContentNode = {
        ...mockContent,
        reach: 'municipal'
      };
      dataLoaderSpy.getContent.and.returnValue(of(municipalContent));
      agentServiceSpy.getAttestations.and.returnValue(['regional-member']);

      service.getContentWithAccessCheck('test-content').subscribe(result => {
        expect(result.canAccess).toBeTrue();
        expect(result.agentReach).toBe('regional');
        done();
      });
    });
  });

  describe('extractYouTubeId', () => {
    it('should extract video ID from watch URL', (done) => {
      const contentWithYouTube: any = {
        ...mockContentIndex.nodes[0],
        url: 'https://www.youtube.com/watch?v=dQw4w9WgXcQ'
      };
      const index = { nodes: [contentWithYouTube] };
      dataLoaderSpy.getContentIndex.and.returnValue(of(index));

      service.getContentPreviewsForCategory('test').subscribe(previews => {
        if (previews.length > 0 && previews[0].thumbnailUrl) {
          expect(previews[0].thumbnailUrl).toContain('dQw4w9WgXcQ');
        }
        done();
      });
    });

    it('should extract video ID from youtu.be URL', (done) => {
      const contentWithYouTube: any = {
        ...mockContentIndex.nodes[0],
        category: 'test',
        url: 'https://youtu.be/dQw4w9WgXcQ'
      };
      const index = { nodes: [contentWithYouTube] };
      dataLoaderSpy.getContentIndex.and.returnValue(of(index));

      service.getContentPreviewsForCategory('test').subscribe(previews => {
        if (previews.length > 0 && previews[0].thumbnailUrl) {
          expect(previews[0].thumbnailUrl).toContain('dQw4w9WgXcQ');
        }
        done();
      });
    });

    it('should extract video ID from embed URL', (done) => {
      const contentWithYouTube: any = {
        ...mockContentIndex.nodes[0],
        category: 'test',
        url: 'https://www.youtube.com/embed/dQw4w9WgXcQ'
      };
      const index = { nodes: [contentWithYouTube] };
      dataLoaderSpy.getContentIndex.and.returnValue(of(index));

      service.getContentPreviewsForCategory('test').subscribe(previews => {
        if (previews.length > 0 && previews[0].thumbnailUrl) {
          expect(previews[0].thumbnailUrl).toContain('dQw4w9WgXcQ');
        }
        done();
      });
    });
  });

  describe('getOpenGraphMetadata', () => {
    it('should return Open Graph metadata', (done) => {
      const ogMetadata = {
        ogTitle: 'Test Title',
        ogDescription: 'Test Description'
      };
      const contentWithOG: ContentNode = {
        ...mockContent,
        openGraphMetadata: ogMetadata as any
      };
      dataLoaderSpy.getContent.and.returnValue(of(contentWithOG));

      service.getOpenGraphMetadata('test-content').subscribe(metadata => {
        expect(metadata).toEqual(ogMetadata);
        done();
      });
    });

    it('should return null if no Open Graph metadata', (done) => {
      dataLoaderSpy.getContent.and.returnValue(of(mockContent));

      service.getOpenGraphMetadata('test-content').subscribe(metadata => {
        expect(metadata).toBeNull();
        done();
      });
    });

    it('should return null on error', (done) => {
      dataLoaderSpy.getContent.and.returnValue(throwError(() => new Error('Not found')));

      service.getOpenGraphMetadata('missing').subscribe(metadata => {
        expect(metadata).toBeNull();
        done();
      });
    });
  });

  describe('getActivityPubObject', () => {
    it('should build ActivityPub object', (done) => {
      const apContent: ContentNode = {
        ...mockContent,
        activityPubType: 'Article',
        did: 'did:web:elohim.host:content:test',
        authorId: 'agent-1',
        createdAt: '2024-01-01',
        updatedAt: '2024-01-02'
      };
      dataLoaderSpy.getContent.and.returnValue(of(apContent));

      service.getActivityPubObject('test-content').subscribe(obj => {
        expect(obj).not.toBeNull();
        if (obj) {
          expect(obj['type']).toBe('Article');
          expect(obj['@context']).toBe('https://www.w3.org/ns/activitystreams');
          expect(obj['id']).toBe('did:web:elohim.host:content:test');
          expect(obj['attributedTo']).toBe('agent-1');
        }
        done();
      });
    });

    it('should return null if no activityPubType', (done) => {
      dataLoaderSpy.getContent.and.returnValue(of(mockContent));

      service.getActivityPubObject('test-content').subscribe(obj => {
        expect(obj).toBeNull();
        done();
      });
    });

    it('should include tags as hashtags', (done) => {
      const apContent: ContentNode = {
        ...mockContent,
        activityPubType: 'Article',
        tags: ['test', 'example']
      };
      dataLoaderSpy.getContent.and.returnValue(of(apContent));

      service.getActivityPubObject('test-content').subscribe(obj => {
        expect(obj).not.toBeNull();
        if (obj && Array.isArray(obj['tag'])) {
          expect(obj['tag'].length).toBe(2);
          expect(obj['tag'][0]['name']).toBe('#test');
        }
        done();
      });
    });

    it('should return null on error', (done) => {
      dataLoaderSpy.getContent.and.returnValue(throwError(() => new Error('Not found')));

      service.getActivityPubObject('missing').subscribe(obj => {
        expect(obj).toBeNull();
        done();
      });
    });
  });

  describe('getJsonLd', () => {
    it('should return JSON-LD metadata', (done) => {
      const linkedData = {
        '@context': 'https://schema.org',
        '@type': 'Article'
      };
      const contentWithLD: ContentNode = {
        ...mockContent,
        linkedData: linkedData as any
      };
      dataLoaderSpy.getContent.and.returnValue(of(contentWithLD));

      service.getJsonLd('test-content').subscribe(metadata => {
        expect(metadata).toEqual(linkedData);
        done();
      });
    });

    it('should return null if no linked data', (done) => {
      dataLoaderSpy.getContent.and.returnValue(of(mockContent));

      service.getJsonLd('test-content').subscribe(metadata => {
        expect(metadata).toBeNull();
        done();
      });
    });

    it('should return null on error', (done) => {
      dataLoaderSpy.getContent.and.returnValue(throwError(() => new Error('Not found')));

      service.getJsonLd('missing').subscribe(metadata => {
        expect(metadata).toBeNull();
        done();
      });
    });
  });

  describe('getDid', () => {
    it('should return DID string', (done) => {
      const contentWithDid: ContentNode = {
        ...mockContent,
        did: 'did:web:elohim.host:content:test'
      };
      dataLoaderSpy.getContent.and.returnValue(of(contentWithDid));

      service.getDid('test-content').subscribe(did => {
        expect(did).toBe('did:web:elohim.host:content:test');
        done();
      });
    });

    it('should return null if no DID', (done) => {
      dataLoaderSpy.getContent.and.returnValue(of(mockContent));

      service.getDid('test-content').subscribe(did => {
        expect(did).toBeNull();
        done();
      });
    });

    it('should return null on error', (done) => {
      dataLoaderSpy.getContent.and.returnValue(throwError(() => new Error('Not found')));

      service.getDid('missing').subscribe(did => {
        expect(did).toBeNull();
        done();
      });
    });
  });

  describe('getStandardsMetadata', () => {
    it('should return all standards metadata', (done) => {
      const fullContent: ContentNode = {
        ...mockContent,
        did: 'did:web:test',
        activityPubType: 'Article',
        openGraphMetadata: { ogTitle: 'Test' } as any,
        linkedData: { '@type': 'Article' } as any
      };
      dataLoaderSpy.getContent.and.returnValue(of(fullContent));

      service.getStandardsMetadata('test-content').subscribe(metadata => {
        expect(metadata.did).toBe('did:web:test');
        expect(metadata.activityPubType).toBe('Article');
        expect(metadata.openGraph).not.toBeNull();
        expect(metadata.jsonLd).not.toBeNull();
        expect(metadata.activityPubObject).not.toBeNull();
        done();
      });
    });

    it('should return null values for missing metadata', (done) => {
      dataLoaderSpy.getContent.and.returnValue(of(mockContent));

      service.getStandardsMetadata('test-content').subscribe(metadata => {
        expect(metadata.did).toBeNull();
        expect(metadata.activityPubType).toBeNull();
        expect(metadata.openGraph).toBeNull();
        expect(metadata.jsonLd).toBeNull();
        expect(metadata.activityPubObject).toBeNull();
        done();
      });
    });

    it('should handle error gracefully', (done) => {
      dataLoaderSpy.getContent.and.returnValue(throwError(() => new Error('Not found')));

      service.getStandardsMetadata('missing').subscribe(metadata => {
        expect(metadata.did).toBeNull();
        expect(metadata.activityPubType).toBeNull();
        expect(metadata.openGraph).toBeNull();
        expect(metadata.jsonLd).toBeNull();
        expect(metadata.activityPubObject).toBeNull();
        done();
      });
    });
  });

  describe('getContentPreviewsForCategory', () => {
    it('should filter content by category', (done) => {
      const categoryIndex = {
        nodes: [
          {
            ...mockContentIndex.nodes[0],
            category: 'governance'
          },
          {
            ...mockContentIndex.nodes[1],
            category: 'education'
          }
        ]
      };
      dataLoaderSpy.getContentIndex.and.returnValue(of(categoryIndex));

      service.getContentPreviewsForCategory('governance').subscribe(previews => {
        expect(previews.length).toBe(1);
        expect(previews[0].category).toBe('governance');
        done();
      });
    });

    it('should handle empty results', (done) => {
      service.getContentPreviewsForCategory('nonexistent').subscribe(previews => {
        expect(previews.length).toBe(0);
        done();
      });
    });
  });

  describe('searchContentWithPreviews', () => {
    it('should return previews instead of index entries', (done) => {
      service.searchContentWithPreviews('TypeScript').subscribe(previews => {
        expect(previews.length).toBe(1);
        expect(previews[0].id).toBe('content-1');
        expect(previews[0].title).toBeDefined();
        done();
      });
    });
  });

  describe('getContentPreviewsByType', () => {
    it('should return previews filtered by type', (done) => {
      service.getContentPreviewsByType('video').subscribe(previews => {
        expect(previews.length).toBe(1);
        expect(previews[0].contentType).toBe('video');
        done();
      });
    });
  });

  describe('getRichMediaForCategory', () => {
    it('should categorize rich media content', (done) => {
      const richMediaIndex = {
        nodes: [
          { ...mockContentIndex.nodes[0], category: 'test', contentType: 'video' as ContentType },
          { ...mockContentIndex.nodes[1], category: 'test', contentType: 'book-chapter' as ContentType },
          { ...mockContentIndex.nodes[2], category: 'test', contentType: 'organization' as ContentType }
        ]
      };
      dataLoaderSpy.getContentIndex.and.returnValue(of(richMediaIndex));

      service.getRichMediaForCategory('test').subscribe(media => {
        expect(media.videos.length).toBe(1);
        expect(media.books.length).toBe(1);
        expect(media.organizations.length).toBe(1);
        expect(media.tools.length).toBe(0);
        done();
      });
    });
  });

  describe('getRelatedContentPreviews', () => {
    it('should return previews for related content', (done) => {
      const relatedIndex = {
        nodes: [
          { ...mockContentIndex.nodes[0], id: 'related-1' },
          { ...mockContentIndex.nodes[1], id: 'related-2' }
        ]
      };
      dataLoaderSpy.getContentIndex.and.returnValue(of(relatedIndex));

      service.getRelatedContentPreviews('test-content').subscribe(previews => {
        expect(previews.length).toBe(2);
        done();
      });
    });

    it('should handle content with no related nodes', (done) => {
      const noRelatedContent: ContentNode = {
        ...mockContent,
        relatedNodeIds: []
      };
      dataLoaderSpy.getContent.and.returnValue(of(noRelatedContent));

      service.getRelatedContentPreviews('test-content').subscribe(previews => {
        expect(previews.length).toBe(0);
        done();
      });
    });

    it('should handle errors gracefully', (done) => {
      dataLoaderSpy.getContent.and.returnValue(throwError(() => new Error('Not found')));

      service.getRelatedContentPreviews('missing').subscribe(previews => {
        expect(previews.length).toBe(0);
        done();
      });
    });
  });
});
