/**
 * Blob Fallback Service - Phase 1: Resilient URL Cascading
 *
 * Implements fallback URL strategy for blob delivery:
 * - Try primary URL first
 * - If fails, try secondary URL
 * - If fails, try tertiary URL
 * - If all fail, return appropriate error
 *
 * Also tracks URL health for future requests.
 */

import { Injectable } from '@angular/core';
import { Observable, throwError } from 'rxjs';
import { retry, timeout, catchError } from 'rxjs/operators';
import { HttpClient, HttpErrorResponse } from '@angular/common/http';

/**
 * Result of blob fetch attempt
 */
export interface BlobFetchResult {
  /** The blob data */
  blob: Blob;

  /** Which URL succeeded (0 = primary, 1 = secondary, 2 = tertiary, etc.) */
  urlIndex: number;

  /** The URL that succeeded */
  successUrl: string;

  /** Total time in milliseconds for fetch */
  durationMs: number;

  /** Number of retries before success */
  retryCount: number;
}

/**
 * Health status of a URL
 */
export interface UrlHealth {
  url: string;
  successCount: number;
  failureCount: number;
  lastAccessTime?: Date;
  lastErrorMessage?: string;
  isHealthy: boolean; // failureCount < successCount
}

@Injectable({
  providedIn: 'root',
})
export class BlobFallbackService {
  /** Track URL health for prioritization */
  private urlHealthMap = new Map<string, UrlHealth>();

  constructor(private http: HttpClient) {}

  /**
   * Fetch blob with fallback URLs.
   * Tries URLs in order until one succeeds.
   *
   * @param fallbackUrls Array of URLs to try in order
   * @param timeoutMs Timeout per URL (default 30s)
   * @param maxRetries Retries per URL (default 2)
   * @returns Observable with blob and metadata
   */
  fetchWithFallback(
    fallbackUrls: string[],
    timeoutMs: number = 30000,
    maxRetries: number = 2,
  ): Observable<BlobFetchResult> {
    if (!fallbackUrls || fallbackUrls.length === 0) {
      return throwError(() => new Error('No fallback URLs provided'));
    }

    // Sort by health (healthier URLs first)
    const sortedUrls = this.sortUrlsByHealth(fallbackUrls);

    return this.fetchUrlCascade(sortedUrls, 0, timeoutMs, maxRetries, {
      startTime: performance.now(),
      retryCount: 0,
    });
  }

  /**
   * Recursively try URLs until one succeeds.
   *
   * @param urls Sorted URLs to try
   * @param currentIndex Current URL index
   * @param timeoutMs Timeout per URL
   * @param maxRetries Max retries per URL
   * @param context Context object with startTime and retryCount
   * @returns Observable with blob result
   */
  private fetchUrlCascade(
    urls: string[],
    currentIndex: number,
    timeoutMs: number,
    maxRetries: number,
    context: { startTime: number; retryCount: number },
  ): Observable<BlobFetchResult> {
    if (currentIndex >= urls.length) {
      return throwError(
        () =>
          new Error(
            `All fallback URLs exhausted. Tried ${urls.length} URLs, all failed.`,
          ),
      );
    }

    const url = urls[currentIndex];

    return this.http
      .get(url, {
        responseType: 'blob',
      })
      .pipe(
        timeout(timeoutMs),
        retry({
          count: maxRetries,
          delay: (error, retryCount) => {
            context.retryCount++;
            console.warn(
              `Retry ${retryCount} for ${url}: ${error.message}`,
            );
            // Exponential backoff: 100ms, 200ms, 400ms
            return new Promise((resolve) =>
              setTimeout(resolve, 100 * Math.pow(2, retryCount - 1)),
            );
          },
        }),
        catchError((error: HttpErrorResponse) => {
          // Record failure
          this.recordUrlFailure(url, error.message);

          console.warn(
            `URL failed: ${url} (${error.status}). Trying next fallback...`,
            error,
          );

          // Try next URL
          return this.fetchUrlCascade(
            urls,
            currentIndex + 1,
            timeoutMs,
            maxRetries,
            context,
          );
        }),
        // Success - record URL health and return
        catchError((error) => {
          // If we get here, all URLs exhausted
          return throwError(() => error);
        }),
      )
      .pipe(
        catchError((error) => {
          return this.fetchUrlCascade(
            urls,
            currentIndex + 1,
            timeoutMs,
            maxRetries,
            context,
          );
        }),
      );

    // This is the success path
    return new Observable((subscriber) => {
      this.fetchUrl(url, timeoutMs, maxRetries)
        .subscribe(
          (blob) => {
            const durationMs = performance.now() - context.startTime;

            // Record success
            this.recordUrlSuccess(url);

            subscriber.next({
              blob,
              urlIndex: currentIndex,
              successUrl: url,
              durationMs,
              retryCount: context.retryCount,
            });
            subscriber.complete();
          },
          (error) => {
            // Try next URL on failure
            this.recordUrlFailure(url, error.message);
            this.fetchUrlCascade(
              urls,
              currentIndex + 1,
              timeoutMs,
              maxRetries,
              context,
            ).subscribe(
              (result) => subscriber.next(result),
              (err) => subscriber.error(err),
              () => subscriber.complete(),
            );
          },
        );
    });
  }

  /**
   * Fetch a single URL with retries.
   *
   * @param url URL to fetch
   * @param timeoutMs Timeout in milliseconds
   * @param maxRetries Max retries
   * @returns Observable with Blob
   */
  private fetchUrl(
    url: string,
    timeoutMs: number,
    maxRetries: number,
  ): Observable<Blob> {
    return this.http
      .get(url, { responseType: 'blob' })
      .pipe(
        timeout(timeoutMs),
        retry({
          count: maxRetries,
          delay: (error, retryCount) => {
            const delay = 100 * Math.pow(2, retryCount - 1);
            console.warn(
              `Retry ${retryCount}/${maxRetries} for ${url} in ${delay}ms`,
            );
            return new Promise((resolve) => setTimeout(resolve, delay));
          },
        }),
      );
  }

  /**
   * Get health information for a URL.
   *
   * @param url The URL to check health for
   * @returns UrlHealth status
   */
  getUrlHealth(url: string): UrlHealth {
    return (
      this.urlHealthMap.get(url) || {
        url,
        successCount: 0,
        failureCount: 0,
        isHealthy: true,
      }
    );
  }

  /**
   * Get health info for multiple URLs.
   *
   * @param urls URLs to check
   * @returns Array of UrlHealth statuses
   */
  getUrlsHealth(urls: string[]): UrlHealth[] {
    return urls.map((url) => this.getUrlHealth(url));
  }

  /**
   * Clear URL health history (useful for testing or forced refresh).
   */
  clearUrlHealth(): void {
    this.urlHealthMap.clear();
  }

  /**
   * Record successful fetch of URL.
   *
   * @param url The URL that succeeded
   */
  private recordUrlSuccess(url: string): void {
    const health = this.urlHealthMap.get(url) || {
      url,
      successCount: 0,
      failureCount: 0,
    };

    health.successCount++;
    health.lastAccessTime = new Date();
    health.isHealthy = health.failureCount < health.successCount;

    this.urlHealthMap.set(url, health);
  }

  /**
   * Record failed fetch of URL.
   *
   * @param url The URL that failed
   * @param errorMessage Error message
   */
  private recordUrlFailure(url: string, errorMessage: string): void {
    const health = this.urlHealthMap.get(url) || {
      url,
      successCount: 0,
      failureCount: 0,
    };

    health.failureCount++;
    health.lastAccessTime = new Date();
    health.lastErrorMessage = errorMessage;
    health.isHealthy = health.failureCount < health.successCount;

    this.urlHealthMap.set(url, health);
  }

  /**
   * Sort URLs by health (healthier URLs first).
   *
   * @param urls URLs to sort
   * @returns Sorted array (healthiest first)
   */
  private sortUrlsByHealth(urls: string[]): string[] {
    return [...urls].sort((a, b) => {
      const healthA = this.getUrlHealth(a);
      const healthB = this.getUrlHealth(b);

      // Prefer healthy URLs
      if (healthA.isHealthy && !healthB.isHealthy) return -1;
      if (!healthA.isHealthy && healthB.isHealthy) return 1;

      // Among healthy (or both unhealthy), prefer higher success rate
      const rateA = healthA.successCount / Math.max(1, healthA.successCount + healthA.failureCount);
      const rateB = healthB.successCount / Math.max(1, healthB.successCount + healthB.failureCount);

      return rateB - rateA;
    });
  }

  /**
   * Test all fallback URLs and report health.
   * Useful for diagnostics.
   *
   * @param fallbackUrls URLs to test
   * @returns Promise resolving to health report
   */
  async testFallbackUrls(fallbackUrls: string[]): Promise<UrlHealth[]> {
    const tests = fallbackUrls.map((url) =>
      this.http
        .head(url, {
          responseType: 'blob',
        })
        .toPromise()
        .then(
          () => {
            this.recordUrlSuccess(url);
            return this.getUrlHealth(url);
          },
          (error) => {
            this.recordUrlFailure(url, error.message);
            return this.getUrlHealth(url);
          },
        ),
    );

    return Promise.all(tests);
  }
}
