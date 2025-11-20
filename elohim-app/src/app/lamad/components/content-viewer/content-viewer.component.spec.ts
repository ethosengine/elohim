import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ContentViewerComponent } from './content-viewer.component';
import { ActivatedRoute } from '@angular/router';
import { of } from 'rxjs';

describe('ContentViewerComponent', () => {
  let component: ContentViewerComponent;
  let fixture: ComponentFixture<ContentViewerComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ContentViewerComponent],
      providers: [
        {
          provide: ActivatedRoute,
          useValue: {
            params: of({ id: 'test-content-1' })
          }
        }
      ]
    }).compileComponents();

    fixture = TestBed.createComponent(ContentViewerComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
