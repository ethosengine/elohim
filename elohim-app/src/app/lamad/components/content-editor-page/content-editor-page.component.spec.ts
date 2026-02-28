import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute, Router } from '@angular/router';
import { of } from 'rxjs';

import { ContentEditorPageComponent } from './content-editor-page.component';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { ContentEditorService } from '../../content-io/services/content-editor.service';
import { ContentFormatRegistryService } from '../../content-io/services/content-format-registry.service';
import { vi } from 'vitest';

describe('ContentEditorPageComponent', () => {
  let component: ContentEditorPageComponent;
  let fixture: ComponentFixture<ContentEditorPageComponent>;
  let mockDataLoader: any;
  let mockEditorService: any;
  let mockRegistry: any;
  let mockRouter: any;

  beforeEach(async () => {
    mockDataLoader = {
    getContent: vi.fn(),
  };
    mockDataLoader.getContent.mockReturnValue(of(null as any));

    mockEditorService = {
    canEdit: vi.fn(),
    createNewDraft: vi.fn(),
    saveContent: vi.fn(),
  };
    mockEditorService.canEdit.mockReturnValue(false);

    mockRegistry = {
    getEditorComponent: vi.fn(),
    getEditorConfig: vi.fn(),
  };
    mockRegistry.getEditorComponent.mockReturnValue(null);
    mockRegistry.getEditorConfig.mockReturnValue({
      editorMode: 'visual',
      supportsLivePreview: false,
    });

    mockRouter = {
    navigate: vi.fn(),
  };

    await TestBed.configureTestingModule({
      imports: [ContentEditorPageComponent],
      providers: [
        {
          provide: ActivatedRoute,
          useValue: {
            params: of({ resourceId: 'test-resource' }),
          },
        },
        { provide: Router, useValue: mockRouter },
        { provide: DataLoaderService, useValue: mockDataLoader },
        { provide: ContentEditorService, useValue: mockEditorService },
        { provide: ContentFormatRegistryService, useValue: mockRegistry },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(ContentEditorPageComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
