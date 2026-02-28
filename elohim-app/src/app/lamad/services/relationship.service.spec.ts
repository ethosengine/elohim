import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { RelationshipService } from './relationship.service';
import { StorageApiService } from '@app/elohim/services/storage-api.service';
import { of } from 'rxjs';
import { RelationshipView } from '@app/elohim/adapters/storage-types.adapter';
import { vi } from 'vitest';

describe('RelationshipService', () => {
  let service: RelationshipService;
  let mockStorageApi: any;

  beforeEach(() => {
    mockStorageApi = {
    getRelationships: vi.fn(),
    createRelationship: vi.fn(),
  };
    mockStorageApi.getRelationships.mockReturnValue(of([]));
    mockStorageApi.createRelationship.mockReturnValue(
      of({
        id: 'rel-123',
        sourceId: 'source-1',
        targetId: 'target-1',
        relationshipType: 'CONTAINS',
        confidence: 1.0,
      } as unknown as RelationshipView)
    );

    TestBed.configureTestingModule({
      providers: [
        RelationshipService,
        { provide: StorageApiService, useValue: mockStorageApi },
        provideHttpClient(),
        provideHttpClientTesting(),
      ],
    });
    service = TestBed.inject(RelationshipService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getRelationshipsForContent', () => {
    it('should return observable of relationships', () => new Promise<void>(done => {
      service.getRelationshipsForContent('content-1').subscribe((relationships) => {
        expect(Array.isArray(relationships)).toBe(true);
        done();
      });
    }));

    it('should call storage api with correct content ID', () => new Promise<void>(done => {
      service.getRelationshipsForContent('content-abc').subscribe(() => {
        expect(mockStorageApi.getRelationships).toHaveBeenCalledWith({ sourceId: 'content-abc' });
        done();
      });
    }));
  });

  describe('getBidirectionalRelationships', () => {
    it('should return observable of relationships', () => new Promise<void>(done => {
      service.getBidirectionalRelationships('content-1').subscribe((relationships) => {
        expect(Array.isArray(relationships)).toBe(true);
        done();
      });
    }));

    it('should call storage api for both directions', () => new Promise<void>(done => {
      service.getBidirectionalRelationships('content-1').subscribe(() => {
        expect(mockStorageApi.getRelationships).toHaveBeenCalledWith({ sourceId: 'content-1' });
        expect(mockStorageApi.getRelationships).toHaveBeenCalledWith({ targetId: 'content-1' });
        done();
      });
    }));

    it('should deduplicate relationships by ID', () => new Promise<void>(done => {
      const mockRel: RelationshipView = {
        id: 'rel-123',
        sourceId: 'source-1',
        targetId: 'target-1',
        relationshipType: 'CONTAINS',
        confidence: 1.0,
      } as unknown as RelationshipView;

      mockStorageApi.getRelationships.mockReturnValue(of([mockRel]));

      service.getBidirectionalRelationships('content-1').subscribe((relationships) => {
        expect(relationships.length).toBe(1);
        done();
      });
    }));
  });

  describe('getRelationshipsByType', () => {
    it('should return observable of relationships', () => new Promise<void>(done => {
      service.getRelationshipsByType('content-1', 'CONTAINS').subscribe((relationships) => {
        expect(Array.isArray(relationships)).toBe(true);
        done();
      });
    }));

    it('should call storage api with relationship type filter', () => new Promise<void>(done => {
      service.getRelationshipsByType('content-1', 'PREREQUISITE').subscribe(() => {
        expect(mockStorageApi.getRelationships).toHaveBeenCalledWith({
          sourceId: 'content-1',
          relationshipType: 'PREREQUISITE',
        });
        done();
      });
    }));
  });

  describe('getHighConfidenceRelationships', () => {
    it('should return observable of relationships', () => new Promise<void>(done => {
      service.getHighConfidenceRelationships('content-1').subscribe((relationships) => {
        expect(Array.isArray(relationships)).toBe(true);
        done();
      });
    }));

    it('should use default minimum confidence of 0.8', () => new Promise<void>(done => {
      service.getHighConfidenceRelationships('content-1').subscribe(() => {
        expect(mockStorageApi.getRelationships).toHaveBeenCalledWith({
          sourceId: 'content-1',
          minConfidence: 0.8,
        });
        done();
      });
    }));

    it('should use custom minimum confidence', () => new Promise<void>(done => {
      service.getHighConfidenceRelationships('content-1', 0.9).subscribe(() => {
        expect(mockStorageApi.getRelationships).toHaveBeenCalledWith({
          sourceId: 'content-1',
          minConfidence: 0.9,
        });
        done();
      });
    }));
  });

  describe('createRelationship', () => {
    it('should return observable of created relationship', () => new Promise<void>(done => {
      const input = {
        sourceId: 'source-1',
        targetId: 'target-1',
        relationshipType: 'CONTAINS',
      };

      service.createRelationship(input).subscribe((relationship) => {
        expect(relationship).toBeDefined();
        expect(relationship.id).toBe('rel-123');
        done();
      });
    }));

    it('should call storage api with input', () => new Promise<void>(done => {
      const input = {
        sourceId: 'source-1',
        targetId: 'target-1',
        relationshipType: 'CONTAINS',
      };

      service.createRelationship(input).subscribe(() => {
        expect(mockStorageApi.createRelationship).toHaveBeenCalledWith(input);
        done();
      });
    }));
  });

  describe('createBidirectionalRelationship', () => {
    it('should return observable of created relationship', () => new Promise<void>(done => {
      service.createBidirectionalRelationship('source-1', 'target-1', 'CONTAINS', 'BELONGS_TO').subscribe(
        (relationship) => {
          expect(relationship).toBeDefined();
          done();
        }
      );
    }));

    it('should call storage api with inverse relationship flag', () => new Promise<void>(done => {
      service.createBidirectionalRelationship('source-1', 'target-1', 'CONTAINS', 'BELONGS_TO').subscribe(
        () => {
          expect(mockStorageApi.createRelationship).toHaveBeenCalledWith(
            expect.objectContaining({
              sourceId: 'source-1',
              targetId: 'target-1',
              relationshipType: 'CONTAINS',
              createInverse: true,
              inverseType: 'BELONGS_TO',
            })
          );
          done();
        }
      );
    }));

    it('should use default confidence of 1.0', () => new Promise<void>(done => {
      service.createBidirectionalRelationship('source-1', 'target-1', 'CONTAINS', 'BELONGS_TO').subscribe(
        () => {
          expect(mockStorageApi.createRelationship).toHaveBeenCalledWith(
            expect.objectContaining({
              confidence: 1.0,
            })
          );
          done();
        }
      );
    }));
  });

  describe('getRelationshipGraph', () => {
    it('should return observable of relationship graph', () => new Promise<void>(done => {
      service.getRelationshipGraph('root-1').subscribe((graph) => {
        expect(graph instanceof Map).toBe(true);
        done();
      });
    }));

    it('should have root content ID in graph', () => new Promise<void>(done => {
      service.getRelationshipGraph('root-1').subscribe((graph) => {
        expect(graph.has('root-1')).toBe(true);
        done();
      });
    }));
  });

  describe('Service Creation', () => {
    it('should be an injectable service', () => {
      const service2 = TestBed.inject(RelationshipService);
      expect(service).toEqual(service2);
    });

    it('should have defined prototype methods', () => {
      const proto = Object.getPrototypeOf(service);
      expect(proto).toBeDefined();
    });
  });
});
