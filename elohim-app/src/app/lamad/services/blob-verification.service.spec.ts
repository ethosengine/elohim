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
            .subscribe((results) => {
              expect(results).toHaveLength(2);
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
            .subscribe((results) => {
              expect(results).toHaveLength(2);
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
});
