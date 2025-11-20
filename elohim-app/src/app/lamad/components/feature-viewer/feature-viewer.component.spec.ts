import { ComponentFixture, TestBed } from '@angular/core/testing';
import { FeatureViewerComponent } from './feature-viewer.component';

describe('FeatureViewerComponent', () => {
  let component: FeatureViewerComponent;
  let fixture: ComponentFixture<FeatureViewerComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [FeatureViewerComponent]
    }).compileComponents();

    fixture = TestBed.createComponent(FeatureViewerComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
