import { ComponentFixture, TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { InlineQuizComponent } from './inline-quiz.component';
import { StreakTrackerService } from '../../services/streak-tracker.service';
import { QuizSoundService } from '../../services/quiz-sound.service';
import { QuestionPoolService } from '../../services/question-pool.service';
import { GovernanceSignalService } from '@app/elohim/services/governance-signal.service';

describe('InlineQuizComponent', () => {
  let component: InlineQuizComponent;
  let fixture: ComponentFixture<InlineQuizComponent>;
  let mockStreakTracker: jasmine.SpyObj<StreakTrackerService>;
  let mockSoundService: jasmine.SpyObj<QuizSoundService>;
  let mockPoolService: jasmine.SpyObj<QuestionPoolService>;
  let mockGovernanceSignal: jasmine.SpyObj<GovernanceSignalService>;

  beforeEach(async () => {
    mockStreakTracker = jasmine.createSpyObj('StreakTrackerService', [
      'startTracking',
      'recordAnswer',
      'onAchieved',
      'offAchieved',
    ]);
    mockStreakTracker.startTracking.and.returnValue({
      contentId: 'test-content',
      humanId: 'test-human',
      currentStreak: 0,
      totalAttempted: 0,
      totalCorrect: 0,
      targetStreak: 3,
      maxQuestions: 10,
      achieved: false,
      recentAnswers: [],
      bestStreak: 0,
      startedAt: new Date().toISOString(),
    });

    mockSoundService = jasmine.createSpyObj('QuizSoundService', [
      'playCorrectAnswerFeedback',
      'playIncorrectAnswerFeedback',
      'playStreakAchieved',
    ]);

    mockPoolService = jasmine.createSpyObj('QuestionPoolService', ['getPoolForContent']);
    mockPoolService.getPoolForContent.and.returnValue(
      of({
        contentId: 'test-content',
        questions: [],
        metadata: {
          minPracticeQuestions: 3,
          minMasteryQuestions: 5,
          bloomsDistribution: {
            remember: 0,
            understand: 0,
            apply: 0,
            analyze: 0,
            evaluate: 0,
            create: 0,
          },
          difficultyDistribution: {
            easy: 0,
            medium: 0,
            hard: 0,
          },
          isComplete: false,
          tags: [],
          sourceDocs: [],
        },
        updatedAt: new Date().toISOString(),
        createdAt: new Date().toISOString(),
        version: 1,
      })
    );

    mockGovernanceSignal = jasmine.createSpyObj('GovernanceSignalService', ['recordLearningSignal']);
    mockGovernanceSignal.recordLearningSignal.and.returnValue(of(true));

    await TestBed.configureTestingModule({
      imports: [InlineQuizComponent],
      providers: [
        { provide: StreakTrackerService, useValue: mockStreakTracker },
        { provide: QuizSoundService, useValue: mockSoundService },
        { provide: QuestionPoolService, useValue: mockPoolService },
        { provide: GovernanceSignalService, useValue: mockGovernanceSignal },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(InlineQuizComponent);
    component = fixture.componentInstance;

    // Set required inputs
    component.contentId = 'test-content';
    component.humanId = 'test-human';

    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
