import { ComponentFixture, TestBed } from '@angular/core/testing';

import { LearningSuccessComponent } from './learning-success.component';

describe('LearningSuccessComponent', () => {
  let component: LearningSuccessComponent;
  let fixture: ComponentFixture<LearningSuccessComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [LearningSuccessComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(LearningSuccessComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
