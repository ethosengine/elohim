import { ComponentFixture, TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { MasteryGateComponent } from './mastery-gate.component';
import { PathAdaptationService } from '../../services/path-adaptation.service';
import { AttemptCooldownService } from '../../services/attempt-cooldown.service';

describe('MasteryGateComponent', () => {
  let component: MasteryGateComponent;
  let fixture: ComponentFixture<MasteryGateComponent>;
  let mockAdaptationService: jasmine.SpyObj<PathAdaptationService>;
  let mockCooldownService: jasmine.SpyObj<AttemptCooldownService>;

  beforeEach(async () => {
    mockAdaptationService = jasmine.createSpyObj('PathAdaptationService', ['getGateStatus$']);
    mockAdaptationService.getGateStatus$.and.returnValue(
      of({
        sectionId: 'test-section',
        locked: true,
        mastered: false,
        bestScore: 0,
        remainingAttempts: 2,
        quizAvailable: true,
      })
    );

    mockCooldownService = jasmine.createSpyObj('AttemptCooldownService', ['getCooldownStatus$']);
    mockCooldownService.getCooldownStatus$.and.returnValue(
      of({
        inCooldown: false,
        remainingMs: 0,
        remainingFormatted: '',
        cooldownEndsAt: null,
        attemptsUsed: 0,
        attemptsRemaining: 2,
        resetsAt: new Date().toISOString(),
        timeUntilResetMs: 0,
      })
    );

    await TestBed.configureTestingModule({
      imports: [MasteryGateComponent],
      providers: [
        { provide: PathAdaptationService, useValue: mockAdaptationService },
        { provide: AttemptCooldownService, useValue: mockCooldownService },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(MasteryGateComponent);
    component = fixture.componentInstance;

    // Set required inputs
    component.pathId = 'test-path';
    component.sectionId = 'test-section';
    component.sectionTitle = 'Test Section';
    component.humanId = 'test-human';

    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
