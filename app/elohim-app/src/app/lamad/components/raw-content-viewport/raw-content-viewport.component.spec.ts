import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { provideRouter } from '@angular/router';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute, convertToParamMap } from '@angular/router';
import { BehaviorSubject, of } from 'rxjs';
import { describe, it, expect, beforeEach } from 'vitest';

import { RawContentViewportComponent } from './raw-content-viewport.component';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';

describe('RawContentViewportComponent', () => {
  let fixture: ComponentFixture<RawContentViewportComponent>;
  let dataLoader: { getContent: (id: string) => unknown };

  beforeEach(async () => {
    dataLoader = {
      getContent: (id: string) =>
        of({
          id,
          title: 'Test Content',
          description: 'A test content node',
          contentType: 'concept',
          contentFormat: 'markdown',
          content: '# Hello',
          tags: [],
          relatedNodeIds: [],
        }),
    };

    await TestBed.configureTestingModule({
      imports: [RawContentViewportComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        provideRouter([]),
        {
          provide: ActivatedRoute,
          useValue: {
            paramMap: new BehaviorSubject(convertToParamMap({ resourceId: 'cn-1' })),
          },
        },
        { provide: DataLoaderService, useValue: dataLoader },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(RawContentViewportComponent);
    fixture.detectChanges();
  });

  it('renders the protocol signal badge in the viewport', () => {
    const badge = fixture.nativeElement.querySelector('app-protocol-signal-badge');
    expect(badge).not.toBeNull();
  });

  it('exposes an exit affordance', () => {
    const exit = fixture.nativeElement.querySelector('[data-testid="raw-viewport-exit"]');
    expect(exit).not.toBeNull();
  });

  it('hosts a renderer host container for the content', () => {
    const host = fixture.nativeElement.querySelector('[data-testid="raw-viewport-renderer-host"]');
    expect(host).not.toBeNull();
  });
});
