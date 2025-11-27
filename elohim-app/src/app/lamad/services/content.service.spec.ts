import { TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';
import { ContentService, ContentIndexEntry, PathReference, ContentAccessResult } from './content.service';
import { DataLoaderService } from './data-loader.service';
import { AgentService } from './agent.service';
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
    expect(service).toBeTruthy();
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
  });
});
