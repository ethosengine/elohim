import { ComponentFixture, TestBed } from '@angular/core/testing';
import { EpicViewerComponent } from './epic-viewer.component';

describe('EpicViewerComponent', () => {
  let component: EpicViewerComponent;
  let fixture: ComponentFixture<EpicViewerComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [EpicViewerComponent]
    }).compileComponents();

    fixture = TestBed.createComponent(EpicViewerComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
