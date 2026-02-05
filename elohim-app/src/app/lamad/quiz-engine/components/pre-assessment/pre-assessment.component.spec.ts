import { ComponentFixture, TestBed } from '@angular/core/testing';
import { PreAssessmentComponent } from './pre-assessment.component';

describe('PreAssessmentComponent', () => {
  let component: PreAssessmentComponent;
  let fixture: ComponentFixture<PreAssessmentComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [PreAssessmentComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(PreAssessmentComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
