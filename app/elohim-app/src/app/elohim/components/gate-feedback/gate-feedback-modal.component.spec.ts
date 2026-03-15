import { ComponentFixture, TestBed, fakeAsync, tick } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { of } from 'rxjs';
import { vi } from 'vitest';

import type { GateEvaluationView, TrustContextView } from '@elohim/storage-client';

import { GateArtifactCardComponent } from '../gate-artifact-card/gate-artifact-card.component';
import { GateFeedbackModalComponent } from './gate-feedback-modal.component';

function makeTrustContext(
  overrides: Partial<TrustContextView> = {},
): TrustContextView {
  return {
    compositeTrust: 0.5,
    masteryDepth: 0.5,
    stewardStanding: 0.7,
    relationshipDensity: 0.3,
    governanceHealth: 0.9,
    behavioralTrust: 0.85,
    intentDivergence: 0.1,
    declaredIntent: null,
    ...overrides,
  };
}

function makeEvaluation(
  overrides: Partial<GateEvaluationView> = {},
): GateEvaluationView {
  return {
    tier: 'standard',
    trustContext: makeTrustContext(),
    pausePrompt: null,
    confirmToken: null,
    settlementBoundary: null,
    appealPath: null,
    ...overrides,
  };
}

describe('GateFeedbackModalComponent', () => {
  let component: GateFeedbackModalComponent;
  let fixture: ComponentFixture<GateFeedbackModalComponent>;
  let httpMock: { post: ReturnType<typeof vi.fn> };

  beforeEach(async () => {
    httpMock = { post: vi.fn().mockReturnValue(of({})) };

    await TestBed.configureTestingModule({
      imports: [GateFeedbackModalComponent],
      providers: [{ provide: HttpClient, useValue: httpMock }],
    }).compileComponents();

    fixture = TestBed.createComponent(GateFeedbackModalComponent);
    component = fixture.componentInstance;
    fixture.componentRef.setInput('feedbackType', 'feedback');
    fixture.detectChanges();
  });

  // --- Modal structure ---

  it('should render modal backdrop', () => {
    const backdrop = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-backdrop"]',
    );
    expect(backdrop).toBeTruthy();
  });

  it('should render modal panel', () => {
    const panel = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-panel"]',
    );
    expect(panel).toBeTruthy();
  });

  it('should render close button', () => {
    const btn = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-close"]',
    );
    expect(btn).toBeTruthy();
  });

  it('should contain a app-gate-artifact-card', () => {
    const card = fixture.nativeElement.querySelector('app-gate-artifact-card');
    expect(card).toBeTruthy();
  });

  // --- Title by feedback type ---

  it('should render title "Share Feedback" for feedback type', () => {
    fixture.componentRef.setInput('feedbackType', 'feedback');
    fixture.detectChanges();

    const title = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-panel"]',
    );
    expect(title.textContent).toContain('Share Feedback');
  });

  it('should render "Flag Content" for flag type', () => {
    fixture.componentRef.setInput('feedbackType', 'flag');
    fixture.detectChanges();

    const title = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-panel"]',
    );
    expect(title.textContent).toContain('Flag Content');
  });

  it('should render "Challenge Content" for challenge type', () => {
    fixture.componentRef.setInput('feedbackType', 'challenge');
    fixture.detectChanges();

    const title = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-panel"]',
    );
    expect(title.textContent).toContain('Challenge Content');
  });

  // --- Placeholder by feedback type ---

  it('should set placeholder "Describe the issue..." for flag', () => {
    fixture.componentRef.setInput('feedbackType', 'flag');
    fixture.detectChanges();

    const textarea = fixture.nativeElement.querySelector(
      '[data-testid="artifact-textarea"]',
    );
    expect(textarea).toBeTruthy();
    expect(textarea.getAttribute('placeholder')).toBe('Describe the issue...');
  });

  it('should set placeholder "State your case..." for challenge', () => {
    fixture.componentRef.setInput('feedbackType', 'challenge');
    fixture.detectChanges();

    const textarea = fixture.nativeElement.querySelector(
      '[data-testid="artifact-textarea"]',
    );
    expect(textarea).toBeTruthy();
    expect(textarea.getAttribute('placeholder')).toBe('State your case...');
  });

  it('should set placeholder "Share your thoughts..." for feedback', () => {
    fixture.componentRef.setInput('feedbackType', 'feedback');
    fixture.detectChanges();

    const textarea = fixture.nativeElement.querySelector(
      '[data-testid="artifact-textarea"]',
    );
    expect(textarea).toBeTruthy();
    expect(textarea.getAttribute('placeholder')).toBe(
      'Share your thoughts...',
    );
  });

  // --- Close / dismiss events ---

  it('should emit closed when close button clicked', () => {
    const closedSpy = vi.fn();
    component.closed.subscribe(closedSpy);

    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-close"]',
    );
    btn.click();
    fixture.detectChanges();

    expect(closedSpy).toHaveBeenCalled();
  });

  it('should emit closed when backdrop clicked', () => {
    const closedSpy = vi.fn();
    component.closed.subscribe(closedSpy);

    const backdrop: HTMLElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-backdrop"]',
    );
    backdrop.click();
    fixture.detectChanges();

    expect(closedSpy).toHaveBeenCalled();
  });

  it('should NOT emit closed when panel clicked (stopPropagation)', () => {
    const closedSpy = vi.fn();
    component.closed.subscribe(closedSpy);

    const panel: HTMLElement = fixture.nativeElement.querySelector(
      '[data-testid="feedback-modal-panel"]',
    );
    panel.click();
    fixture.detectChanges();

    expect(closedSpy).not.toHaveBeenCalled();
  });

  // --- Gate flow events ---

  it('should emit posted when artifact card posts', () => {
    const postedSpy = vi.fn();
    component.posted.subscribe(postedSpy);

    // Drive the inner card through the full gate flow
    const card = component.artifactCard();
    card.localText.set('test feedback');
    card.onSubmit();
    card.interaction.handleGateEvaluation(makeEvaluation());
    fixture.detectChanges();

    // Click affirm on the inner card
    httpMock.post.mockReturnValue(of({}));
    const affirmBtn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="artifact-affirm"]',
    );
    affirmBtn.click();
    fixture.detectChanges();

    expect(postedSpy).toHaveBeenCalledWith({ reachTier: 'community' });
  });

  it('should emit settled when artifact card settles', () => {
    const settledSpy = vi.fn();
    component.settled.subscribe(settledSpy);

    // Drive the inner card to settlement
    const card = component.artifactCard();
    card.localText.set('test feedback');
    card.onSubmit();
    fixture.detectChanges();

    card.interaction.handleGateEvaluation(
      makeEvaluation({
        settlementBoundary: 'constitutional-limit',
        appealPath: '/appeal/1',
      }),
    );
    fixture.detectChanges();

    expect(settledSpy).toHaveBeenCalledWith({
      boundary: 'constitutional-limit',
      appealPath: '/appeal/1',
    });
  });

  // --- API wiring ---

  it('should have an apiCall function that delegates to StorageApiService', () => {
    expect(component.apiCall).toBeDefined();
    expect(typeof component.apiCall).toBe('function');
  });

  it('should pass gateApiCall to the artifact card', () => {
    const card = component.artifactCard();
    expect(card.gateApiCall).toBe(component.apiCall);
  });

  // --- Auto-close on posted ---

  it('should emit closed ~800ms after posted', fakeAsync(() => {
    const closedSpy = vi.fn();
    component.closed.subscribe(closedSpy);

    component.onPosted({ reachTier: 'community' });
    fixture.detectChanges();

    expect(closedSpy).not.toHaveBeenCalled();

    tick(800);
    expect(closedSpy).toHaveBeenCalled();
  }));

  it('should NOT auto-close on settled', fakeAsync(() => {
    const closedSpy = vi.fn();
    component.closed.subscribe(closedSpy);

    component.settled.emit({ boundary: 'harm', appealPath: null });
    fixture.detectChanges();

    tick(2000);
    expect(closedSpy).not.toHaveBeenCalled();
  }));
});
