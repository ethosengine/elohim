import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute, Router } from '@angular/router';
import { of } from 'rxjs';

import { ContentEditorPageComponent } from './content-editor-page.component';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { ContentEditorService } from '../../content-io/services/content-editor.service';
import { ContentFormatRegistryService } from '../../content-io/services/content-format-registry.service';

describe('ContentEditorPageComponent', () => {
  let component: ContentEditorPageComponent;
  let fixture: ComponentFixture<ContentEditorPageComponent>;
  let mockDataLoader: jasmine.SpyObj<DataLoaderService>;
  let mockEditorService: jasmine.SpyObj<ContentEditorService>;
  let mockRegistry: jasmine.SpyObj<ContentFormatRegistryService>;
  let mockRouter: jasmine.SpyObj<Router>;

  beforeEach(async () => {
    mockDataLoader = jasmine.createSpyObj('DataLoaderService', ['getContent']);
    mockDataLoader.getContent.and.returnValue(of(null as any));

    mockEditorService = jasmine.createSpyObj('ContentEditorService', [
      'canEdit',
      'createNewDraft',
      'saveContent',
    ]);
    mockEditorService.canEdit.and.returnValue(false);

    mockRegistry = jasmine.createSpyObj('ContentFormatRegistryService', [
      'getEditorComponent',
      'getEditorConfig',
    ]);
    mockRegistry.getEditorComponent.and.returnValue(null);
    mockRegistry.getEditorConfig.and.returnValue({
      editorMode: 'visual',
      supportsLivePreview: false,
    });

    mockRouter = jasmine.createSpyObj('Router', ['navigate']);

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
