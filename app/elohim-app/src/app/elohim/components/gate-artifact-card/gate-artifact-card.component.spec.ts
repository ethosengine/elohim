import { ComponentFixture, TestBed } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { of } from 'rxjs';
import { vi } from 'vitest';

import type { GateEvaluationView, TrustContextView } from '@elohim/storage-client';

import { GateArtifactCardComponent } from './gate-artifact-card.component';

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

describe('GateArtifactCardComponent', () => {
  let component: GateArtifactCardComponent;
  let fixture: ComponentFixture<GateArtifactCardComponent>;
  let httpMock: { post: ReturnType<typeof vi.fn> };

  beforeEach(async () => {
    httpMock = { post: vi.fn().mockReturnValue(of({})) };

    await TestBed.configureTestingModule({
      imports: [GateArtifactCardComponent],
      providers: [{ provide: HttpClient, useValue: httpMock }],
    }).compileComponents();

    fixture = TestBed.createComponent(GateArtifactCardComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  // --- DRAFT state ---

  it('should render textarea in draft state', () => {
    const textarea = fixture.nativeElement.querySelector(
      '[data-testid="artifact-textarea"]',
    );
    expect(textarea).toBeTruthy();
    expect(textarea.tagName).toBe('TEXTAREA');
  });

  it('should render submit button in draft state', () => {
    const btn = fixture.nativeElement.querySelector(
      '[data-testid="artifact-submit"]',
    );
    expect(btn).toBeTruthy();
    expect(btn.textContent.trim()).toContain('Submit');
  });

  it('should disable submit when textarea is empty', () => {
    const btn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="artifact-submit"]',
    );
    expect(btn.disabled).toBe(true);
  });

  // --- EVALUATING state ---

  it('should show shimmer class in evaluating state', () => {
    component.localText.set('test text');
    component.onSubmit();
    fixture.detectChanges();

    const card = fixture.nativeElement.querySelector('.artifact-card');
    expect(card.classList.contains('evaluating')).toBe(true);
  });

  it('should show preview text in evaluating state', () => {
    component.localText.set('my artifact');
    component.onSubmit();
    fixture.detectChanges();

    const preview = fixture.nativeElement.querySelector(
      '[data-testid="artifact-preview"]',
    );
    expect(preview).toBeTruthy();
    expect(preview.textContent.trim()).toBe('my artifact');
  });

  // --- AFFIRM state ---

  it('should show reach badge in affirm state', () => {
    component.localText.set('text');
    component.onSubmit();
    component.interaction.handleGateEvaluation(makeEvaluation());
    fixture.detectChanges();

    const badge = fixture.nativeElement.querySelector(
      '[data-testid="reach-badge"]',
    );
    expect(badge).toBeTruthy();
    expect(badge.textContent).toContain('Community');
  });

  it('should show affirm button in affirm state', () => {
    component.localText.set('text');
    component.onSubmit();
    component.interaction.handleGateEvaluation(makeEvaluation());
    fixture.detectChanges();

    const btn = fixture.nativeElement.querySelector(
      '[data-testid="artifact-affirm"]',
    );
    expect(btn).toBeTruthy();
    expect(btn.textContent.trim()).toContain('Affirm');
  });

  // --- DIALOGUE state ---

  it('should show dialogue prompt in dialogue state', () => {
    component.localText.set('text');
    component.onSubmit();
    component.interaction.handleGateEvaluation(
      makeEvaluation({ pausePrompt: 'Consider your words', confirmToken: 'tok' }),
    );
    fixture.detectChanges();

    const prompt = fixture.nativeElement.querySelector(
      '[data-testid="dialogue-prompt"]',
    );
    expect(prompt).toBeTruthy();
    expect(prompt.textContent.trim()).toBe('Consider your words');
  });

  it('should make textarea editable in dialogue state', () => {
    component.localText.set('text');
    component.onSubmit();
    component.interaction.handleGateEvaluation(
      makeEvaluation({ pausePrompt: 'Think again', confirmToken: 'tok' }),
    );
    fixture.detectChanges();

    const textarea = fixture.nativeElement.querySelector(
      '[data-testid="artifact-textarea"]',
    );
    expect(textarea).toBeTruthy();
    expect(textarea.tagName).toBe('TEXTAREA');
  });

  // --- SETTLED state ---

  it('should show settlement info in settled state', () => {
    component.localText.set('text');
    component.onSubmit();
    component.interaction.handleGateEvaluation(
      makeEvaluation({
        settlementBoundary: 'constitutional-limit',
        appealPath: '/appeal/1',
      }),
    );
    fixture.detectChanges();

    const info = fixture.nativeElement.querySelector(
      '[data-testid="settlement-info"]',
    );
    expect(info).toBeTruthy();
    expect(info.textContent).toContain('constitutional-limit');
  });

  it('should show settlement link in settled state', () => {
    component.localText.set('text');
    component.onSubmit();
    component.interaction.handleGateEvaluation(
      makeEvaluation({
        settlementBoundary: 'constitutional-limit',
        appealPath: '/appeal/1',
      }),
    );
    fixture.detectChanges();

    const link: HTMLAnchorElement = fixture.nativeElement.querySelector(
      '[data-testid="settlement-link"]',
    );
    expect(link).toBeTruthy();
    expect(link.getAttribute('href')).toBe('/appeal/1');
  });

  // --- Event emission ---

  it('should emit posted event on affirm click', () => {
    const postedSpy = vi.fn();
    component.posted.subscribe(postedSpy);

    component.localText.set('my post');
    component.onSubmit();
    component.interaction.handleGateEvaluation(makeEvaluation());
    fixture.detectChanges();

    const affirmBtn: HTMLButtonElement = fixture.nativeElement.querySelector(
      '[data-testid="artifact-affirm"]',
    );
    affirmBtn.click();
    fixture.detectChanges();

    expect(postedSpy).toHaveBeenCalledWith({ reachTier: 'community' });
  });

  it('should emit settled event on settlement', () => {
    const settledSpy = vi.fn();
    component.settled.subscribe(settledSpy);

    component.localText.set('text');
    component.onSubmit();
    fixture.detectChanges();

    component.interaction.handleGateEvaluation(
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

  // --- POSTED state ---

  it('should show preview and reach badge in posted state', () => {
    // Get to posted state: submit → handleGateEvaluation (affirm) → affirm()
    component.localText.set('Posted text');
    component.interaction.submit('Posted text', 'comment', { contentId: 'c-1' });
    component.interaction.handleGateEvaluation(
      makeEvaluation({
        trustContext: makeTrustContext({ compositeTrust: 0.75 }),
      }),
    );
    fixture.detectChanges();

    // Click affirm
    httpMock.post.mockReturnValue(of({}));
    const affirmBtn = fixture.nativeElement.querySelector(
      '[data-testid="artifact-affirm"]',
    );
    affirmBtn.click();
    fixture.detectChanges();

    // Verify posted state
    const preview = fixture.nativeElement.querySelector(
      '[data-testid="artifact-preview"]',
    );
    expect(preview?.textContent).toContain('Posted text');
    const badge = fixture.nativeElement.querySelector(
      '[data-testid="reach-badge"]',
    );
    expect(badge?.textContent).toContain('Network');
  });

  // --- gateApiCall input ---

  it('should call submitWithApi when gateApiCall is provided', () => {
    const apiCall = vi.fn().mockReturnValue(of({}));
    const submitWithApiSpy = vi.spyOn(component.interaction, 'submitWithApi');
    component.gateApiCall = apiCall;
    component.mutationType = 'comment';
    component.contextMetadata = { contentId: 'c-1' };

    component.localText.set('api text');
    component.onSubmit();

    expect(submitWithApiSpy).toHaveBeenCalledWith(
      'api text',
      'comment',
      { contentId: 'c-1' },
      apiCall,
    );
  });

  it('should call regular submit when no gateApiCall is provided', () => {
    const submitSpy = vi.spyOn(component.interaction, 'submit');
    component.gateApiCall = undefined;
    component.mutationType = 'comment';
    component.contextMetadata = { contentId: 'c-2' };

    component.localText.set('manual text');
    component.onSubmit();

    expect(submitSpy).toHaveBeenCalledWith(
      'manual text',
      'comment',
      { contentId: 'c-2' },
    );
  });

  it('should not submit when text is empty', () => {
    const submitSpy = vi.spyOn(component.interaction, 'submit');
    const submitWithApiSpy = vi.spyOn(component.interaction, 'submitWithApi');

    component.localText.set('   ');
    component.onSubmit();

    expect(submitSpy).not.toHaveBeenCalled();
    expect(submitWithApiSpy).not.toHaveBeenCalled();
  });

  // --- Wrapper class ---

  it('should have gate-comment wrapper class', () => {
    const card = fixture.nativeElement.querySelector('.gate-comment');
    expect(card).toBeTruthy();
  });
});
