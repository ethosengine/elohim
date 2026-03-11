import { TestBed } from '@angular/core/testing';
import { QuizSoundService } from './quiz-sound.service';

describe('QuizSoundService', () => {
  let service: QuizSoundService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [QuizSoundService],
    });
    service = TestBed.inject(QuizSoundService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });
});
