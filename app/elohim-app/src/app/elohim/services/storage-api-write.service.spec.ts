import { describe, it, expect, beforeEach, vi } from 'vitest';
import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';

import { StorageApiService } from './storage-api.service';

describe('StorageApiService write methods', () => {
  let service: StorageApiService;
  let httpTesting: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [StorageApiService, provideHttpClient(), provideHttpClientTesting()],
    });
    service = TestBed.inject(StorageApiService);
    httpTesting = TestBed.inject(HttpTestingController);
  });

  it('createContent posts to /db/content', () => {
    const input = {
      id: 'test',
      title: 'Test',
      schemaVersion: 1,
      tags: [],
    };
    const mockResponse = { id: 'test', title: 'Test', tags: [] };

    service.createContent(input as any).subscribe(result => {
      expect(result.id).toBe('test');
    });

    const req = httpTesting.expectOne(r => r.url.includes('/db/content') && r.method === 'POST');
    expect(req.request.body).toEqual(input);
    req.flush(mockResponse);
  });

  it('updateContent patches to /db/content/{id}', () => {
    const patch = { metadata: { status: 'done' } };
    const mockResponse = { id: 'story-123', tags: [], metadata: { status: 'done' } };

    service.updateContent('story-123', patch).subscribe(result => {
      expect(result.id).toBe('story-123');
    });

    const req = httpTesting.expectOne(
      r => r.url.includes('/db/content/story-123') && r.method === 'PATCH',
    );
    expect(req.request.body).toEqual(patch);
    req.flush(mockResponse);
  });
});
