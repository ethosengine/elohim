import { ComponentFixture, TestBed } from '@angular/core/testing';
import { LamadLayoutComponent } from './lamad-layout.component';
import { provideRouter } from '@angular/router';
import { provideHttpClient } from '@angular/common/http';
import { ELOHIM_CLIENT, GOVERNANCE, CONTENT_ATTESTATION } from '@elohim/service';
import { LAMAD_STORAGE_CLIENT } from '../../interfaces/storage.interface';
import { DataLoaderService } from '../../services/data-loader.service';
import { RendererInitializerService } from '../../renderers/renderer-initializer.service';
import { of } from 'rxjs';
import { vi } from 'vitest';

describe('LamadLayoutComponent', () => {
  let component: LamadLayoutComponent;
  let fixture: ComponentFixture<LamadLayoutComponent>;

  const mockElohimClient = {
    get: vi.fn().mockReturnValue(Promise.resolve(null)),
    query: vi.fn().mockReturnValue(Promise.resolve([])),
    supportsOffline: vi.fn().mockReturnValue(false),
    backpressure: vi.fn().mockReturnValue(Promise.resolve(0)),
  };

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [LamadLayoutComponent],
      providers: [
        provideRouter([]),
        provideHttpClient(),
        { provide: ELOHIM_CLIENT, useValue: mockElohimClient },
        { provide: GOVERNANCE, useValue: {} },
        { provide: CONTENT_ATTESTATION, useValue: {} },
        {
          provide: LAMAD_STORAGE_CLIENT,
          useValue: {
            getBlobUrl: (h: string) => `https://test/blob/${h}`,
            getStorageBaseUrl: () => 'https://test',
          },
        },
        {
          provide: DataLoaderService,
          useValue: {
            getContentIndex: vi.fn().mockReturnValue(of({ nodes: [] })),
            getContent: vi.fn(),
          },
        },
        { provide: RendererInitializerService, useValue: {} },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(LamadLayoutComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
