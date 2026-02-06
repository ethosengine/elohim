import { ComponentFixture, TestBed } from '@angular/core/testing';
import { QuizRendererComponent } from './quiz-renderer.component';

describe('QuizRendererComponent', () => {
  let component: QuizRendererComponent;
  let fixture: ComponentFixture<QuizRendererComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [QuizRendererComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(QuizRendererComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
