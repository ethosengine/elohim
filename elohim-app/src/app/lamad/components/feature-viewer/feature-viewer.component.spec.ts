import { ComponentFixture, TestBed } from '@angular/core/testing';
import { FeatureViewerComponent } from './feature-viewer.component';
import { ActivatedRoute } from '@angular/router';
import { provideHttpClient } from '@angular/common/http';
import { of } from 'rxjs';

describe('FeatureViewerComponent', () => {
  let component: FeatureViewerComponent;
  let fixture: ComponentFixture<FeatureViewerComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [FeatureViewerComponent],
      providers: [
        provideHttpClient(),
        {
          provide: ActivatedRoute,
          useValue: {
            params: of({ id: 'test-feature-1' })
          }
        }
      ]
    }).compileComponents();

    fixture = TestBed.createComponent(FeatureViewerComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
