import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { DocumentGraphService } from './document-graph.service';

describe('DocumentGraphService', () => {
  let service: DocumentGraphService;
  let httpMock: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [
        DocumentGraphService,
        provideHttpClient(),
        provideHttpClientTesting()
      ]
    });
    service = TestBed.inject(DocumentGraphService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should return null graph initially', () => {
    expect(service.getGraph()).toBeNull();
  });

  it('should get nodes by type', () => {
    const result = service.getNodesByType('epic');
    expect(result).toEqual([]);
  });

  it('should return empty array for related nodes when graph is null', () => {
    const result = service.getRelatedNodes('some-id');
    expect(result).toEqual([]);
  });

  it('should return empty array for search when graph is null', () => {
    const result = service.searchNodes('test');
    expect(result).toEqual([]);
  });

  it('should return undefined for getNode when graph is null', () => {
    const result = service.getNode('test-id');
    expect(result).toBeUndefined();
  });
});
