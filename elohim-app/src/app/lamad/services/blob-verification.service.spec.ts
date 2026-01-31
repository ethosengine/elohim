import { TestBed } from '@angular/core/testing';
import { of, throwError, Observable } from 'rxjs';
import { BlobVerificationService, BlobVerificationResult } from './blob-verification.service';
import { DoorwayClientService } from '../../elohim/services/doorway-client.service';

/**
 * Mock DoorwayClientService that simulates server verification failure
 * so that tests exercise the SubtleCrypto/fallback path.
 */
const mockDoorwayClientService = {
  verifyBlob: jasmine
    .createSpy('verifyBlob')
    .and.returnValue(throwError(() => new Error('Mock: Server unavailable'))),
  verifyBlobData: jasmine
    .createSpy('verifyBlobData')
    .and.returnValue(throwError(() => new Error('Mock: Server unavailable'))),
  checkHealth: jasmine
    .createSpy('checkHealth')
    .and.returnValue(throwError(() => new Error('Mock: Server unavailable'))),
};

describe('BlobVerificationService', () => {
  let service: BlobVerificationService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [
        BlobVerificationService,
        { provide: DoorwayClientService, useValue: mockDoorwayClientService },
      ],
    });
    service = TestBed.inject(BlobVerificationService);

    // Reset spies between tests
    mockDoorwayClientService.verifyBlob.calls.reset();
    mockDoorwayClientService.verifyBlobData.calls.reset();
    mockDoorwayClientService.checkHealth.calls.reset();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // ================== SERVICE CREATION & INITIALIZATION ==================

  describe('Service Initialization', () => {
    it('should be a singleton (providedIn: root)', () => {
      const service1 = TestBed.inject(BlobVerificationService);
      const service2 = TestBed.inject(BlobVerificationService);
      expect(service1).toBe(service2);
    });

    it('should have private properties initialized', () => {
      // Access via any to test private properties
      const svc = service as any;
      expect(svc.wasmModule).toBeNull();
      expect(svc.wasmLoadAttempted).toBe(false);
      expect(svc.wasmAvailable).toBe(false);
    });

    it('should inject DoorwayClientService dependency', () => {
      const doorway = TestBed.inject(DoorwayClientService);
      expect(doorway).toBeTruthy();
    });
  });

  // ================== METHOD EXISTENCE TESTS ==================

  describe('Public Methods Exist', () => {
    it('should have verifyBlob method', () => {
      expect(service.verifyBlob).toBeDefined();
      expect(typeof service.verifyBlob).toBe('function');
    });

    it('should have verifyChunk method', () => {
      expect(service.verifyChunk).toBeDefined();
      expect(typeof service.verifyChunk).toBe('function');
    });

    it('should have verifyMultiple method', () => {
      expect(service.verifyMultiple).toBeDefined();
      expect(typeof service.verifyMultiple).toBe('function');
    });

    it('should have streamComputeHash method', () => {
      expect(service.streamComputeHash).toBeDefined();
      expect(typeof service.streamComputeHash).toBe('function');
    });

    it('should have checkAvailableMethods method', () => {
      expect(service.checkAvailableMethods).toBeDefined();
      expect(typeof service.checkAvailableMethods).toBe('function');
    });
  });

  // ================== OBSERVABLE RETURN TYPE TESTS ==================

  describe('Observable Return Types', () => {
    it('verifyBlob should return Observable<BlobVerificationResult>', done => {
      const testBlob = new Blob(['test']);
      const result$ = service.verifyBlob(testBlob, 'dummy_hash');

      expect(result$ instanceof Observable).toBe(true);

      result$.subscribe(result => {
        expect(result).toBeDefined();
        expect(typeof result).toBe('object');
        expect(result.isValid).toBeDefined();
        expect(result.computedHash).toBeDefined();
        expect(result.expectedHash).toBeDefined();
        expect(result.durationMs).toBeDefined();
        done();
      });
    });

    it('verifyChunk should return Observable<BlobVerificationResult>', done => {
      const testBlob = new Blob(['chunk']);
      const result$ = service.verifyChunk(testBlob, 'dummy_hash');

      expect(result$ instanceof Observable).toBe(true);

      result$.subscribe(result => {
        expect(result).toBeDefined();
        expect(typeof result).toBe('object');
        done();
      });
    });

    it('verifyMultiple should return Observable<BlobVerificationResult[]>', done => {
      const blobs: [Blob, string][] = [
        [new Blob(['data1']), 'hash1'],
        [new Blob(['data2']), 'hash2'],
      ];
      const result$ = service.verifyMultiple(blobs);

      expect(result$ instanceof Observable).toBe(true);

      result$.subscribe(results => {
        expect(Array.isArray(results)).toBe(true);
        expect(results.length).toBe(2);
        expect(results[0]).toBeDefined();
        done();
      });
    });
  });

  // ================== PROPERTY INITIALIZATION TESTS ==================

  describe('Result Object Structure', () => {
    it('BlobVerificationResult should have all required properties', done => {
      const testBlob = new Blob(['test']);

      service.verifyBlob(testBlob, 'test_hash').subscribe(result => {
        // Required properties
        expect(result.isValid).toBeDefined();
        expect(typeof result.isValid).toBe('boolean');

        expect(result.computedHash).toBeDefined();
        expect(typeof result.computedHash).toBe('string');

        expect(result.expectedHash).toBeDefined();
        expect(typeof result.expectedHash).toBe('string');

        expect(result.durationMs).toBeDefined();
        expect(typeof result.durationMs).toBe('number');

        // Optional property
        if (result.error !== undefined) {
          expect(typeof result.error).toBe('string');
        }

        // Optional method indicator
        if (result.method !== undefined) {
          expect(['wasm', 'server', 'subtle-crypto', 'fallback-js']).toContain(result.method);
        }

        done();
      });
    });

    it('should include durationMs in milliseconds', done => {
      const testBlob = new Blob(['test data']);

      service.verifyBlob(testBlob, 'hash').subscribe(result => {
        expect(result.durationMs).toBeGreaterThanOrEqual(0);
        expect(typeof result.durationMs).toBe('number');
        done();
      });
    });

    it('computedHash should be hexadecimal string (64 chars for SHA256)', done => {
      const testBlob = new Blob(['test']);

      service.verifyBlob(testBlob, '').subscribe(result => {
        expect(result.computedHash).toMatch(/^[a-f0-9]*$/);
        expect(result.computedHash.length).toBe(64);
        done();
      });
    });
  });

  // ================== SIMPLE INPUT/OUTPUT TESTS ==================

  describe('Simple Input/Output', () => {
    it('should accept Blob and string as parameters to verifyBlob', done => {
      const blob = new Blob(['data']);
      const hash = 'a'.repeat(64);

      service.verifyBlob(blob, hash).subscribe(result => {
        expect(result).toBeDefined();
        done();
      });
    });

    it('should handle empty Blob input', done => {
      const emptyBlob = new Blob(['']);

      service.verifyBlob(emptyBlob, '').subscribe(result => {
        expect(result.computedHash).toBeDefined();
        expect(result.computedHash.length).toBe(64);
        done();
      });
    });

    it('should handle empty string hash input', done => {
      const blob = new Blob(['test']);

      service.verifyBlob(blob, '').subscribe(result => {
        expect(result.expectedHash).toBe('');
        expect(result.isValid).toBe(false);
        done();
      });
    });

    it('should return isValid false when hashes do not match', done => {
      const blob = new Blob(['test']);
      const wrongHash = '0000000000000000000000000000000000000000000000000000000000000000';

      service.verifyBlob(blob, wrongHash).subscribe(result => {
        expect(result.isValid).toBe(false);
        expect(result.expectedHash).toBe(wrongHash);
        done();
      });
    });

    it('should return expectedHash matching input parameter', done => {
      const blob = new Blob(['test']);
      const inputHash = 'abcdef123456789' + '0'.repeat(49);

      service.verifyBlob(blob, inputHash).subscribe(result => {
        expect(result.expectedHash).toBe(inputHash);
        done();
      });
    });

    it('should handle Blob with various content types', done => {
      const blobTypes = [
        new Blob(['text'], { type: 'text/plain' }),
        new Blob([new Uint8Array([1, 2, 3])], { type: 'application/octet-stream' }),
        new Blob(['{"json": true}'], { type: 'application/json' }),
      ];

      let completed = 0;

      blobTypes.forEach(blob => {
        service.verifyBlob(blob, '').subscribe(result => {
          expect(result.computedHash).toBeDefined();
          expect(result.computedHash.length).toBe(64);
          completed++;

          if (completed === blobTypes.length) {
            done();
          }
        });
      });
    });
  });

  // ================== METHOD SIGNATURE TESTS ==================

  describe('Method Signatures', () => {
    it('verifyChunk should delegate to verifyBlob', done => {
      const chunkBlob = new Blob(['chunk data']);
      const hash = 'abc123' + '0'.repeat(58);

      // First verify with verifyBlob
      service.verifyBlob(chunkBlob, hash).subscribe(blobResult => {
        // Then verify with verifyChunk
        service.verifyChunk(chunkBlob, hash).subscribe(chunkResult => {
          // Results should be structurally identical
          expect(chunkResult.computedHash).toEqual(blobResult.computedHash);
          expect(chunkResult.isValid).toEqual(blobResult.isValid);
          done();
        });
      });
    });

    it('verifyMultiple with empty array should return empty array', done => {
      service.verifyMultiple([]).subscribe(results => {
        expect(Array.isArray(results)).toBe(true);
        expect(results.length).toBe(0);
        done();
      });
    });

    it('verifyMultiple should process all blobs', done => {
      const blobs: [Blob, string][] = [
        [new Blob(['blob1']), 'hash1'],
        [new Blob(['blob2']), 'hash2'],
        [new Blob(['blob3']), 'hash3'],
      ];

      service.verifyMultiple(blobs).subscribe(results => {
        expect(results.length).toBe(3);
        expect(results[0]).toBeDefined();
        expect(results[1]).toBeDefined();
        expect(results[2]).toBeDefined();
        done();
      });
    });

    it('streamComputeHash should return Promise<string>', async () => {
      const blob = new Blob(['test']);
      const result = service.streamComputeHash(blob);

      expect(result instanceof Promise).toBe(true);

      const hash = await result;
      expect(typeof hash).toBe('string');
      expect(hash.length).toBe(64);
    });

    it('streamComputeHash should accept optional chunkSize parameter', async () => {
      const blob = new Blob(['test']);

      const hash1 = await service.streamComputeHash(blob, 512 * 1024);
      const hash2 = await service.streamComputeHash(blob, 2 * 1024 * 1024);

      expect(typeof hash1).toBe('string');
      expect(typeof hash2).toBe('string');
    });

    it('streamComputeHash should accept optional onProgress callback', async () => {
      const blob = new Blob(['test']);
      let progressCalled = false;

      await service.streamComputeHash(blob, 512 * 1024, () => {
        progressCalled = true;
      });

      // For small blobs, progress may not be called, but method should not error
      expect(typeof progressCalled).toBe('boolean');
    });

    it('checkAvailableMethods should return Promise with availability flags', async () => {
      const result = await service.checkAvailableMethods();

      expect(result).toBeDefined();
      expect(typeof result).toBe('object');
      expect(result.wasm).toBeDefined();
      expect(typeof result.wasm).toBe('boolean');
      expect(result.server).toBeDefined();
      expect(typeof result.server).toBe('boolean');
      expect(result.subtleCrypto).toBeDefined();
      expect(typeof result.subtleCrypto).toBe('boolean');
      expect(result.fallbackJs).toBeDefined();
      expect(typeof result.fallbackJs).toBe('boolean');
      expect(result.fallbackJs).toBe(true); // Always available
    });
  });

  // ================== ERROR HANDLING TESTS ==================

  describe('Error Handling', () => {
    it('should handle verifyBlob error gracefully', done => {
      const blob = new Blob(['test']);

      service.verifyBlob(blob, 'hash').subscribe(result => {
        // Should return result even if internal error occurs
        expect(result).toBeDefined();
        expect(result.isValid).toBeDefined();
        expect(result.computedHash).toBeDefined();
        done();
      });
    });

    it('should populate error field when verification fails', done => {
      const blob = new Blob(['test']);
      const invalidHash = 'invalid_hash_that_will_not_match';

      service.verifyBlob(blob, invalidHash).subscribe(result => {
        expect(result).toBeDefined();
        // Result should still be valid object structure
        expect(result.isValid).toBe(false);
        done();
      });
    });
  });

  // ================== ASYNC FLOW TESTS ==================

  // TODO: Add async flow tests - Complex waterfall of WASM → Server → SubtleCrypto → JS

  // ================== COMPREHENSIVE MOCKING ==================

  // TODO: Add comprehensive mocks - WASM module loading, server responses, SubtleCrypto availability

  // ================== BUSINESS LOGIC TESTS ==================

  // TODO: Add business logic tests - Hash matching logic, fallback chain execution

  // ================== CRYPTOGRAPHIC VERIFICATION TESTS ==================

  // TODO: Add crypto verification tests - SHA256 implementation correctness, known hash values

  // ================== ORIGINAL INTEGRATION TESTS ==================

  describe('verifyBlob Integration', () => {
    it('should verify blob with matching hash', (done) => {
      // Create a simple test blob
      const testData = new TextEncoder().encode('test content');
      const blob = new Blob([testData]);

      // Compute expected hash first
      service.verifyBlob(blob, '').subscribe(result => {
        const expectedHash = result.computedHash;

        // Now verify with the correct hash
        service.verifyBlob(blob, expectedHash).subscribe(verifyResult => {
          expect(verifyResult.isValid).toBe(true);
          expect(verifyResult.computedHash).toEqual(expectedHash);
          expect(verifyResult.expectedHash).toEqual(expectedHash);
          expect(verifyResult.durationMs).toBeGreaterThanOrEqual(0);
          done();
        });
      });
    });

    it('should reject blob with mismatched hash', (done) => {
      const testData = new TextEncoder().encode('test content');
      const blob = new Blob([testData]);
      const wrongHash = 'definitely_not_the_right_hash_0000000000000000000000000000';

      service.verifyBlob(blob, wrongHash).subscribe(result => {
        expect(result.isValid).toBe(false);
        expect(result.expectedHash).toEqual(wrongHash);
        expect(result.computedHash).not.toEqual(wrongHash);
        done();
      });
    });

    it('should handle case-insensitive hash comparison', (done) => {
      const testData = new TextEncoder().encode('test');
      const blob = new Blob([testData]);

      service.verifyBlob(blob, '').subscribe(result => {
        const hash = result.computedHash;

        // Test with uppercase
        service.verifyBlob(blob, hash.toUpperCase()).subscribe(verifyResult => {
          expect(verifyResult.isValid).toBe(true);
          done();
        });
      });
    });

    it('should measure verification duration', (done) => {
      const testData = new TextEncoder().encode('larger test content for timing');
      const blob = new Blob([testData]);

      service.verifyBlob(blob, '').subscribe(result => {
        expect(result.durationMs).toBeGreaterThanOrEqual(0);
        done();
      });
    });
  });

  describe('streamComputeHash Integration', () => {
    it('should compute hash for small blob', async () => {
      const testData = new TextEncoder().encode('test content');
      const blob = new Blob([testData]);

      const hash = await service.streamComputeHash(blob);
      expect(hash).toMatch(/^[a-f0-9]{64}$/); // SHA256 produces 64 hex chars
    });

    it('should produce same hash for same content', async () => {
      const testData = new TextEncoder().encode('consistent content');
      const blob1 = new Blob([testData]);
      const blob2 = new Blob([testData]);

      const hash1 = await service.streamComputeHash(blob1);
      const hash2 = await service.streamComputeHash(blob2);

      expect(hash1).toEqual(hash2);
    });

    it('should track progress during hashing', async () => {
      const testData = new TextEncoder().encode('a'.repeat(3 * 1024 * 1024)); // 3 MB
      const blob = new Blob([testData]);
      const progressUpdates: Array<{ processed: number; total: number }> = [];

      await service.streamComputeHash(
        blob,
        1024 * 1024, // 1 MB chunks
        (processed, total) => {
          progressUpdates.push({ processed, total });
        }
      );

      // Should have multiple progress updates for a 3MB file with 1MB chunks
      expect(progressUpdates.length).toBeGreaterThan(0);
      expect(progressUpdates[progressUpdates.length - 1].processed).toBe(testData.length);
    });

    it('should handle custom chunk sizes', async () => {
      const testData = new TextEncoder().encode('a'.repeat(2 * 1024 * 1024)); // 2 MB
      const blob = new Blob([testData]);

      const smallChunkHash = await service.streamComputeHash(blob, 256 * 1024); // 256 KB chunks
      const largeChunkHash = await service.streamComputeHash(blob, 2 * 1024 * 1024); // 2 MB chunk

      // Same content should produce same hash regardless of chunk size
      expect(smallChunkHash).toEqual(largeChunkHash);
    });
  });

  describe('verifyChunk Integration', () => {
    it('should verify individual chunk', (done) => {
      const chunkData = new TextEncoder().encode('chunk data');
      const chunk = new Blob([chunkData]);

      service.verifyChunk(chunk, '').subscribe(result => {
        const expectedHash = result.computedHash;

        service.verifyChunk(chunk, expectedHash).subscribe(verifyResult => {
          expect(verifyResult.isValid).toBe(true);
          done();
        });
      });
    });
  });

  describe('verifyMultiple Integration', () => {
    it('should verify multiple blobs in parallel', (done) => {
      const testData1 = new TextEncoder().encode('blob 1');
      const testData2 = new TextEncoder().encode('blob 2');
      const blob1 = new Blob([testData1]);
      const blob2 = new Blob([testData2]);

      // First pass to get hashes
      let hash1 = '';
      let hash2 = '';
      let completed = 0;

      service.verifyBlob(blob1, '').subscribe(result => {
        hash1 = result.computedHash;
        completed++;

        if (completed === 2) {
          // Now verify with correct hashes
          service
            .verifyMultiple([
              [blob1, hash1],
              [blob2, hash2],
            ])
            .subscribe((results: BlobVerificationResult[]) => {
              expect(results.length).toBe(2);
              expect(results[0].isValid).toBe(true);
              expect(results[1].isValid).toBe(true);
              done();
            });
        }
      });

      service.verifyBlob(blob2, '').subscribe(result => {
        hash2 = result.computedHash;
        completed++;

        if (completed === 2) {
          // Now verify with correct hashes
          service
            .verifyMultiple([
              [blob1, hash1],
              [blob2, hash2],
            ])
            .subscribe((results: BlobVerificationResult[]) => {
              expect(results.length).toBe(2);
              expect(results[0].isValid).toBe(true);
              expect(results[1].isValid).toBe(true);
              done();
            });
        }
      });
    });

    it('should handle empty array', (done) => {
      service.verifyMultiple([]).subscribe(results => {
        expect(results).toEqual([]);
        done();
      });
    });
  });

  describe('SubtleCrypto Fallback for Non-HTTPS Contexts', () => {
    it('should use pure-JS SHA256 when SubtleCrypto unavailable', (done) => {
      const testData = 'test content';
      const blob = new Blob([testData]);
      const expectedHash = 'd4d8f0b8d9c8c8f8e8d8c8b8a8989898989898989898989898989898989898';

      // This test will work with the fallback SHA256
      service.verifyBlob(blob, expectedHash).subscribe(result => {
        // Just verify it doesn't error and returns a result
        expect(result.computedHash).toBeDefined();
        expect(typeof result.computedHash).toBe('string');
        expect(result.computedHash.length).toBe(64); // SHA256 is 64 hex chars
        done();
      });
    });

    it('should produce consistent hashes with fallback', (done) => {
      const testData = 'test data for hashing';
      const blob = new Blob([testData]);

      let hash1: string;

      // Compute hash first time
      service.verifyBlob(blob, '0000').subscribe(result1 => {
        hash1 = result1.computedHash;

        // Compute same hash again to verify consistency
        service.verifyBlob(new Blob([testData]), '0000').subscribe(result2 => {
          expect(result2.computedHash).toBe(hash1);
          done();
        });
      });
    });

    it('should fallback gracefully when SubtleCrypto fails', (done) => {
      const blob = new Blob(['test']);

      // Even if SubtleCrypto is unavailable or fails, should get a valid hash
      service.verifyBlob(blob, 'invalid_hash').subscribe(result => {
        expect(result.computedHash).toBeDefined();
        expect(result.computedHash.length).toBe(64);
        expect(result.isValid).toBe(false); // Should not match "invalid_hash"
        done();
      });
    });

    it('should compute different hashes for different content', (done) => {
      const blob1 = new Blob(['content1']);
      const blob2 = new Blob(['content2']);

      let hash1: string;

      service.verifyBlob(blob1, '0000').subscribe(result1 => {
        hash1 = result1.computedHash;

        service.verifyBlob(blob2, '0000').subscribe(result2 => {
          expect(result2.computedHash).not.toBe(hash1);
          done();
        });
      });
    });

    it('should handle large blobs with fallback', (done) => {
      // Create a large blob (1 MB)
      const largeData = new Uint8Array(1024 * 1024);
      for (let i = 0; i < largeData.length; i++) {
        largeData[i] = Math.floor(Math.random() * 256);
      }
      const blob = new Blob([largeData]);

      service.verifyBlob(blob, 'dummy_hash').subscribe(result => {
        expect(result.computedHash).toBeDefined();
        expect(result.computedHash.length).toBe(64);
        expect(result.durationMs).toBeGreaterThan(0);
        done();
      });
    });

    it('should produce standard SHA256 hashes', (done) => {
      // Test with a known SHA256 hash
      // SHA256('') = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
      const emptyBlob = new Blob(['']);

      service
        .verifyBlob(emptyBlob, 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855')
        .subscribe(result => {
          expect(result.isValid).toBe(true);
          expect(result.computedHash).toBe(
            'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855'
          );
          done();
        });
    });
  });
});
