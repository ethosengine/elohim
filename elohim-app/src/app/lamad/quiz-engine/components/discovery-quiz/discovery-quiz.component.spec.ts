import { ComponentFixture, TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { DiscoveryQuizComponent } from './discovery-quiz.component';
import { DiscoveryAttestationService } from '../../services/discovery-attestation.service';
import { QuestionPoolService } from '../../services/question-pool.service';

describe('DiscoveryQuizComponent', () => {
  let component: DiscoveryQuizComponent;
  let fixture: ComponentFixture<DiscoveryQuizComponent>;
  let mockQuestionPool: jasmine.SpyObj<QuestionPoolService>;
  let mockDiscoveryService: jasmine.SpyObj<DiscoveryAttestationService>;

  beforeEach(async () => {
    mockQuestionPool = jasmine.createSpyObj('QuestionPoolService', ['getPoolForContent']);
    mockQuestionPool.getPoolForContent.and.returnValue(
      of({
        id: 'test-pool',
        contentId: 'test-content',
        name: 'Test Pool',
        questions: [],
        totalQuestions: 0,
        difficulty: 'medium',
        metadata: {},
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        version: 1,
      } as any)
    );

    mockDiscoveryService = jasmine.createSpyObj('DiscoveryAttestationService', [
      'recordAttestation',
    ]);

    await TestBed.configureTestingModule({
      imports: [DiscoveryQuizComponent],
      providers: [
        { provide: QuestionPoolService, useValue: mockQuestionPool },
        { provide: DiscoveryAttestationService, useValue: mockDiscoveryService },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(DiscoveryQuizComponent);
    component = fixture.componentInstance;

    // Set required inputs
    component.quizId = 'test-quiz';
    component.humanId = 'test-human';
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
