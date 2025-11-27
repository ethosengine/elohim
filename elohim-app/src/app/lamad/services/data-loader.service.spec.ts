import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { DataLoaderService } from './data-loader.service';

describe('DataLoaderService', () => {
  let service: DataLoaderService;
  let httpMock: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [DataLoaderService]
    });
    service = TestBed.inject(DataLoaderService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getContentIndex', () => {
    it('should fetch content index', (done) => {
      const mockIndex = {
        nodes: [
          { id: 'node-1', title: 'Test Node', contentType: 'epic' }
        ]
      };

      service.getContentIndex().subscribe(index => {
        expect(index).toBeTruthy();
        expect(index.nodes.length).toBe(1);
        done();
      });

      const req = httpMock.expectOne(req => req.url.includes('content/index.json'));
      expect(req.request.method).toBe('GET');
      req.flush(mockIndex);
    });

    it('should cache content index', (done) => {
      const mockIndex = { nodes: [] };

      // First call
      service.getContentIndex().subscribe(() => {
        // Second call should use cache
        service.getContentIndex().subscribe(() => {
          done();
        });
      });

      const req = httpMock.expectOne(req => req.url.includes('content/index.json'));
      req.flush(mockIndex);
    });
  });

  describe('getContent', () => {
    it('should fetch content by ID', (done) => {
      const mockContent = {
        id: 'test-node',
        title: 'Test Content',
        content: '# Test',
        contentType: 'epic'
      };

      service.getContent('test-node').subscribe(content => {
        expect(content).toBeTruthy();
        expect(content.id).toBe('test-node');
        done();
      });

      const req = httpMock.expectOne(req => req.url.includes('test-node'));
      expect(req.request.method).toBe('GET');
      req.flush(mockContent);
    });

    it('should handle content not found', (done) => {
      service.getContent('nonexistent').subscribe({
        error: err => {
          expect(err).toBeTruthy();
          done();
        }
      });

      const req = httpMock.expectOne(req => req.url.includes('nonexistent'));
      req.flush('Not Found', { status: 404, statusText: 'Not Found' });
    });
  });

  describe('getPaths', () => {
    it('should fetch learning paths', (done) => {
      const mockPaths = [
        { id: 'path-1', title: 'Test Path', steps: [] }
      ];

      service.getPaths().subscribe(paths => {
        expect(paths).toBeTruthy();
        expect(paths.length).toBe(1);
        done();
      });

      const req = httpMock.expectOne(req => req.url.includes('paths'));
      expect(req.request.method).toBe('GET');
      req.flush(mockPaths);
    });
  });

  describe('getAgents', () => {
    it('should fetch agents', (done) => {
      const mockAgents = [
        { id: 'agent-1', name: 'Test Agent', type: 'human' }
      ];

      service.getAgents().subscribe(agents => {
        expect(agents).toBeTruthy();
        expect(agents.length).toBe(1);
        done();
      });

      const req = httpMock.expectOne(req => req.url.includes('agents'));
      expect(req.request.method).toBe('GET');
      req.flush(mockAgents);
    });
  });

  describe('getAttestations', () => {
    it('should fetch attestations', (done) => {
      const mockAttestations = [
        { id: 'att-1', agentId: 'agent-1', type: 'completion' }
      ];

      service.getAttestations().subscribe(attestations => {
        expect(attestations).toBeTruthy();
        expect(attestations.length).toBe(1);
        done();
      });

      const req = httpMock.expectOne(req => req.url.includes('attestations'));
      expect(req.request.method).toBe('GET');
      req.flush(mockAttestations);
    });
  });
});
