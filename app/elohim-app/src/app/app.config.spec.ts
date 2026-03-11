import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { provideRouter } from '@angular/router';

import { appConfig } from './app.config';
import { GOVERNANCE } from './elohim/interfaces/governance.interface';
import { CONTENT_ATTESTATION } from './elohim/interfaces/content-attestation.interface';
import { DATA_LOADER } from './elohim/interfaces/data-loader.interface';
import { BLOB_FETCHER } from './elohim/interfaces/blob-fetcher.interface';
import { STORAGE_API } from './elohim/interfaces/storage-api.interface';
import { STORAGE_WRITER } from './elohim/interfaces/storage-writer.interface';
import { CUSTODIAN_SELECTOR } from './elohim/interfaces/custodian.interface';

describe('App Config', () => {
  it('should have application config defined', () => {
    expect(appConfig).toBeDefined();
  });

  it('should have providers array', () => {
    expect(appConfig.providers).toBeDefined();
    expect(Array.isArray(appConfig.providers)).toBe(true);
  });

  it('should have at least 3 providers', () => {
    expect(appConfig.providers.length).toBeGreaterThanOrEqual(3);
  });
});

/**
 * DI smoke tests — verify that all InjectionTokens used by core services
 * are resolvable with the real appConfig providers. Catches missing
 * token-to-service bindings that unit tests miss (since they mock tokens).
 */
describe('App DI Resolution', () => {
  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [
        ...appConfig.providers,
        // Swap real HTTP with testing backend so no network calls fire
        provideHttpClient(),
        provideHttpClientTesting(),
      ],
    });
  });

  it('should resolve GOVERNANCE token', () => {
    const service = TestBed.inject(GOVERNANCE);
    expect(service).toBeTruthy();
  });

  it('should resolve CONTENT_ATTESTATION token', () => {
    const service = TestBed.inject(CONTENT_ATTESTATION);
    expect(service).toBeTruthy();
  });

  it('should resolve BLOB_FETCHER token', () => {
    const service = TestBed.inject(BLOB_FETCHER);
    expect(service).toBeTruthy();
  });

  it('should resolve STORAGE_API token', () => {
    const service = TestBed.inject(STORAGE_API);
    expect(service).toBeTruthy();
  });

  it('should resolve STORAGE_WRITER token', () => {
    const service = TestBed.inject(STORAGE_WRITER);
    expect(service).toBeTruthy();
  });

  it('should resolve CUSTODIAN_SELECTOR token', () => {
    const service = TestBed.inject(CUSTODIAN_SELECTOR);
    expect(service).toBeTruthy();
  });
});
