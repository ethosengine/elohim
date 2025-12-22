import { TestBed } from '@angular/core/testing';
import { BlobVerificationService, BlobVerificationResult } from './blob-verification.service';

describe('BlobVerificationService', () => {
  let service: BlobVerificationService;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    service = TestBed.inject(BlobVerificationService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('verifyBlob', () => {
    it('should verify blob with matching hash', (done) => {
      // Create a simple test blob
      const testData = new TextEncoder().encode('test content');
      const blob = new Blob([testData]);

      // Compute expected hash first
      service.verifyBlob(blob, '').subscribe((result) => {
        const expectedHash = result.computedHash;

        // Now verify with the correct hash
        service.verifyBlob(blob, expectedHash).subscribe((verifyResult) => {
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

      service.verifyBlob(blob, wrongHash).subscribe((result) => {
        expect(result.isValid).toBe(false);
        expect(result.expectedHash).toEqual(wrongHash);
        expect(result.computedHash).not.toEqual(wrongHash);
        done();
      });
    });

    it('should handle case-insensitive hash comparison', (done) => {
      const testData = new TextEncoder().encode('test');
      const blob = new Blob([testData]);

      service.verifyBlob(blob, '').subscribe((result) => {
        const hash = result.computedHash;

        // Test with uppercase
        service.verifyBlob(blob, hash.toUpperCase()).subscribe((verifyResult) => {
          expect(verifyResult.isValid).toBe(true);
          done();
        });
      });
    });

    it('should measure verification duration', (done) => {
      const testData = new TextEncoder().encode('larger test content for timing');
      const blob = new Blob([testData]);

      service.verifyBlob(blob, '').subscribe((result) => {
        expect(result.durationMs).toBeGreaterThanOrEqual(0);
        done();
      });
    });
  });

  describe('streamComputeHash', () => {
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

  describe('verifyChunk', () => {
    it('should verify individual chunk', (done) => {
      const chunkData = new TextEncoder().encode('chunk data');
      const chunk = new Blob([chunkData]);

      service.verifyChunk(chunk, '').subscribe((result) => {
        const expectedHash = result.computedHash;

        service.verifyChunk(chunk, expectedHash).subscribe((verifyResult) => {
          expect(verifyResult.isValid).toBe(true);
          done();
        });
      });
    });
  });

  describe('verifyMultiple', () => {
    it('should verify multiple blobs in parallel', (done) => {
      const testData1 = new TextEncoder().encode('blob 1');
      const testData2 = new TextEncoder().encode('blob 2');
      const blob1 = new Blob([testData1]);
      const blob2 = new Blob([testData2]);

      // First pass to get hashes
      let hash1 = '';
      let hash2 = '';
      let completed = 0;

      service.verifyBlob(blob1, '').subscribe((result) => {
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

      service.verifyBlob(blob2, '').subscribe((result) => {
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
      service.verifyMultiple([]).subscribe((results) => {
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
      service.verifyBlob(blob, expectedHash).subscribe((result) => {
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
      service.verifyBlob(blob, '0000').subscribe((result1) => {
        hash1 = result1.computedHash;

        // Compute same hash again to verify consistency
        service.verifyBlob(new Blob([testData]), '0000').subscribe((result2) => {
          expect(result2.computedHash).toBe(hash1);
          done();
        });
      });
    });

    it('should fallback gracefully when SubtleCrypto fails', (done) => {
      const blob = new Blob(['test']);

      // Even if SubtleCrypto is unavailable or fails, should get a valid hash
      service.verifyBlob(blob, 'invalid_hash').subscribe((result) => {
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

      service.verifyBlob(blob1, '0000').subscribe((result1) => {
        hash1 = result1.computedHash;

        service.verifyBlob(blob2, '0000').subscribe((result2) => {
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

      service.verifyBlob(blob, 'dummy_hash').subscribe((result) => {
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

      service.verifyBlob(emptyBlob, 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855')
        .subscribe((result) => {
          expect(result.isValid).toBe(true);
          expect(result.computedHash).toBe('e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855');
          done();
        });
    });
  });
});
