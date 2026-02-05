/**
 * SophiaRendererComponent - Content renderer for Sophia assessments.
 *
 * This component implements the InteractiveRenderer interface to integrate
 * with the lamad content system. It renders Sophia moments and emits
 * completion events with Recognition results.
 *
 * Features:
 * - Unified mastery and discovery/reflection mode rendering
 * - Progress tracking for multi-moment assessments
 * - Subscale aggregation for discovery assessments
 * - Recognition-based completion events
 */

import { CommonModule } from '@angular/common';
import {
  Component,
  Input,
  Output,
  EventEmitter,
  ViewChild,
  OnInit,
  OnDestroy,
  OnChanges,
  SimpleChanges,
  ChangeDetectionStrategy,
  ChangeDetectorRef,
  inject,
} from '@angular/core';
import { RouterModule } from '@angular/router';

// @coverage: 92.6% (2026-02-05)

import { Subject } from 'rxjs';

import { ContentNode } from '../../../models/content-node.model';
import {
  ContentRenderer,
  InteractiveRenderer,
  RendererCompletionEvent,
} from '../../interfaces/content-format-plugin.interface';

import {
  getPsycheAPI,
  type PsycheAPI,
  type AggregatedReflection,
  type ReflectionRecognition,
  type UserInputMap,
} from './sophia-element-loader';
import { SophiaWrapperComponent } from './sophia-wrapper.component';

import type { Moment, Recognition } from './sophia-moment.model';

// ─────────────────────────────────────────────────────────────────────────────
// Mode Configuration
// ─────────────────────────────────────────────────────────────────────────────

type AssessmentMode = 'mastery' | 'discovery' | 'reflection';

interface ModeConfig {
  submitButtonText: string;
  nextButtonText: string;
  showFeedback: boolean;
  showCorrectness: boolean;
  trackSubscales: boolean;
}

// Discovery and reflection modes use the same configuration (no correct answers)
const DISCOVERY_CONFIG: ModeConfig = {
  submitButtonText: 'Continue',
  nextButtonText: 'Continue',
  showFeedback: false,
  showCorrectness: false,
  trackSubscales: true,
};

const MODE_PRESETS: Record<AssessmentMode, ModeConfig> = {
  mastery: {
    submitButtonText: 'Check Answer',
    nextButtonText: 'Next Question',
    showFeedback: true,
    showCorrectness: true,
    trackSubscales: false,
  },
  discovery: DISCOVERY_CONFIG,
  reflection: DISCOVERY_CONFIG,
};

@Component({
  selector: 'app-sophia-renderer',
  standalone: true,
  imports: [CommonModule, RouterModule, SophiaWrapperComponent],
  template: `
    <div class="sophia-renderer">
      <!-- Results View -->
      @if (showResults) {
        <div class="results-section">
          <!-- Mastery Mode Results -->
          @if (assessmentMode === 'mastery') {
            <div class="result-card" [class.passed]="masteryPassed" [class.failed]="!masteryPassed">
              <div class="result-icon">{{ masteryPassed ? '🎯' : '📚' }}</div>
              <h4 class="result-title">{{ masteryPassed ? 'Well Done!' : 'Keep Learning' }}</h4>
              <p class="result-description">
                You got {{ demonstratedCount }} of {{ totalMoments }} correct.
              </p>
              <div class="score-display">
                <span class="score-value">{{ masteryScorePercent }}%</span>
                <span class="score-label">Score</span>
              </div>
            </div>

            <!-- Coming Soon placeholder for mastery rewards -->
            <div class="mastery-preview">
              <div class="preview-item">
                <span class="preview-icon">⭐</span>
                <div class="preview-content">
                  <strong>Points</strong>
                  <span class="coming-soon">Coming Soon</span>
                </div>
              </div>
              <div class="preview-item">
                <span class="preview-icon">🏆</span>
                <div class="preview-content">
                  <strong>Attestations</strong>
                  <span class="coming-soon">Coming Soon</span>
                </div>
              </div>
              <div class="preview-item">
                <span class="preview-icon">📈</span>
                <div class="preview-content">
                  <strong>Mastery Level</strong>
                  <span class="coming-soon">Coming Soon</span>
                </div>
              </div>
            </div>
          } @else {
            <!-- Discovery/Reflection Mode Results -->
            <div class="result-card">
              <div class="result-icon">✨</div>
              <h4 class="result-title">Assessment Complete</h4>
              <p class="result-description">Thank you for completing this reflection.</p>
            </div>

            <!-- Coming Soon placeholder -->
            <div class="imagodei-preview">
              <p class="preview-text">
                <strong>Coming Soon:</strong>
                Your responses will be saved to your ImagoDei profile, helping you create meaning
                from your reflections.
              </p>
              <a class="profile-link" routerLink="/identity/profile">View your ImagoDei →</a>
            </div>
          }

          <!-- Continue navigation -->
          <div class="results-actions">
            <button class="btn btn-primary" (click)="completeAndContinue()">Continue</button>
          </div>
        </div>
      } @else {
        <!-- Header (hidden during results) -->
        <header class="quiz-header" *ngIf="showHeader">
          <h3 class="quiz-title">{{ title }}</h3>
          <div class="quiz-progress" *ngIf="totalMoments > 1">
            <span class="progress-text">{{ currentMomentIndex + 1 }} of {{ totalMoments }}</span>
            <div class="progress-bar">
              <div class="progress-fill" [style.width.%]="progressPercentage"></div>
            </div>
          </div>
        </header>

        <!-- Moment Area -->
        <div class="moment-container">
          <app-sophia-question
            #momentComponent
            [moment]="currentMoment"
            [mode]="assessmentMode"
            [initialUserInput]="currentInitialUserInput"
            [reviewMode]="reviewMode"
            (recognized)="handleRecognition($event)"
            (answerChanged)="handleAnswerChange($event)"
            (ready)="handleReady()"
          ></app-sophia-question>
        </div>

        <!-- Feedback (mastery mode only) -->
        <div
          class="feedback-container"
          *ngIf="showFeedback && modeConfig.showCorrectness && lastRecognition?.mastery"
        >
          <div
            class="feedback"
            [class.correct]="lastRecognition?.mastery?.demonstrated"
            [class.incorrect]="lastRecognition && !lastRecognition.mastery?.demonstrated"
          >
            <span class="feedback-icon">
              {{ lastRecognition?.mastery?.demonstrated ? '✓' : '✗' }}
            </span>
            <span class="feedback-text">
              {{ lastRecognition?.mastery?.demonstrated ? 'Correct!' : 'Not quite.' }}
            </span>
            <span class="feedback-message" *ngIf="lastRecognition?.mastery?.message">
              {{ lastRecognition?.mastery?.message }}
            </span>
          </div>
        </div>

        <!-- Controls -->
        <footer class="quiz-controls" *ngIf="!reviewMode">
          <!-- Mastery mode: Check Answer button with feedback -->
          <ng-container *ngIf="modeConfig.showFeedback">
            <button
              class="btn btn-primary"
              [disabled]="!hasAnswer || isSubmitting"
              (click)="submitAnswer()"
            >
              {{ submitButtonText }}
            </button>
            <button class="btn btn-secondary" *ngIf="showNextButton" (click)="nextMoment()">
              {{ isLastMoment ? 'See Results' : 'Next Question' }}
            </button>
          </ng-container>

          <!-- Discovery/Reflection mode: single-click flow with Back link -->
          <ng-container *ngIf="!modeConfig.showFeedback">
            <!-- Back link - subtle styling, shows when not on first question -->
            <a class="nav-link back-link" *ngIf="currentMomentIndex > 0" (click)="previousMoment()">
              &larr; Back
            </a>

            <!-- Continue button - submits and auto-advances -->
            <button
              class="btn btn-primary"
              [disabled]="!hasAnswer || isSubmitting"
              (click)="submitAnswer()"
            >
              {{ isLastMoment ? 'Finish' : 'Continue' }}
            </button>
          </ng-container>
        </footer>
      }
    </div>
  `,
  styles: [
    `
      :host {
        display: block;
      }

      .sophia-renderer {
        padding: 1rem;
      }

      .quiz-header {
        margin-bottom: 1.5rem;
      }

      .quiz-title {
        margin: 0 0 0.5rem;
        font-size: 1.25rem;
        font-weight: 600;
        color: var(--text-primary, #1a1a1a);
      }

      .quiz-progress {
        display: flex;
        align-items: center;
        gap: 1rem;
      }

      .progress-text {
        font-size: 0.875rem;
        color: var(--text-secondary, #666);
      }

      .progress-bar {
        flex: 1;
        height: 6px;
        background: var(--bg-tertiary, #e0e0e0);
        border-radius: 3px;
        overflow: hidden;
      }

      .progress-fill {
        height: 100%;
        background: var(--primary-color, #1976d2);
        border-radius: 3px;
        transition: width 0.3s ease;
      }

      .moment-container {
        margin-bottom: 1.5rem;
      }

      .feedback-container {
        margin-bottom: 1rem;
      }

      .feedback {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.75rem 1rem;
        border-radius: 8px;
        font-size: 0.9375rem;
      }

      .feedback.correct {
        background: var(--success-bg, #e8f5e9);
        color: var(--success-color, #2e7d32);
      }

      .feedback.incorrect {
        background: var(--error-bg, #ffebee);
        color: var(--error-color, #c62828);
      }

      .feedback-icon {
        font-size: 1.25rem;
        font-weight: bold;
      }

      .feedback-message {
        margin-left: auto;
        font-size: 0.875rem;
        opacity: 0.8;
      }

      .quiz-controls {
        display: flex;
        gap: 0.75rem;
        padding-top: 1rem;
        border-top: 1px solid var(--border-color, #e0e0e0);
      }

      .btn {
        padding: 0.625rem 1.25rem;
        border: none;
        border-radius: 6px;
        font-size: 0.9375rem;
        font-weight: 500;
        cursor: pointer;
        transition:
          background-color 0.2s,
          opacity 0.2s;
      }

      .btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
      }

      .btn-primary {
        background: var(--primary-color, #1976d2);
        color: white;
      }

      .btn-primary:hover:not(:disabled) {
        background: var(--primary-dark, #1565c0);
      }

      .btn-secondary {
        background: var(--bg-secondary, #f5f5f5);
        color: var(--text-primary, #1a1a1a);
      }

      .btn-secondary:hover:not(:disabled) {
        background: var(--bg-tertiary, #e0e0e0);
      }

      .nav-link.back-link {
        color: var(--text-secondary, #666);
        cursor: pointer;
        font-size: 0.9375rem;
        text-decoration: none;
        padding: 0.625rem 0;
        margin-right: 0.5rem;
        display: inline-flex;
        align-items: center;
      }

      .nav-link.back-link:hover {
        color: var(--text-primary, #1a1a1a);
        text-decoration: underline;
      }

      /* Results Section - Theme-aware colors */
      .results-section {
        padding: 1.5rem;
        text-align: center;
      }

      .result-card {
        background: rgba(255, 255, 255, 0.95);
        border-radius: 12px;
        padding: 2rem;
        margin-bottom: 1.5rem;
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
        color: #1a1a1a;
      }

      .result-icon {
        font-size: 3rem;
        margin-bottom: 1rem;
      }

      .result-title {
        font-size: 1.5rem;
        font-weight: 600;
        margin: 0 0 0.5rem;
        color: #1a1a1a;
      }

      .result-description {
        font-size: 0.9375rem;
        color: #555;
        margin: 0;
      }

      .imagodei-preview {
        background: rgba(245, 245, 245, 0.95);
        border-radius: 8px;
        padding: 1rem 1.25rem;
        margin-bottom: 1.5rem;
        text-align: left;
        color: #333;
      }

      .preview-text {
        margin: 0 0 0.5rem;
        font-size: 0.875rem;
        color: #555;
        line-height: 1.5;
      }

      .profile-link {
        color: #1976d2;
        font-size: 0.875rem;
        font-weight: 500;
        text-decoration: none;
      }

      .profile-link:hover {
        text-decoration: underline;
      }

      .results-actions {
        padding-top: 1rem;
      }

      /* Mastery Results */
      .result-card.passed {
        border: 2px solid #2e7d32;
      }

      .result-card.failed {
        border: 2px solid #f57c00;
      }

      .score-display {
        display: flex;
        flex-direction: column;
        align-items: center;
        margin-top: 1rem;
        padding-top: 1rem;
        border-top: 1px solid #e0e0e0;
      }

      .score-value {
        font-size: 2.5rem;
        font-weight: 700;
        color: #1976d2;
        line-height: 1;
      }

      .score-label {
        font-size: 0.875rem;
        color: #555;
        margin-top: 0.25rem;
      }

      .mastery-preview {
        background: rgba(245, 245, 245, 0.95);
        border-radius: 8px;
        padding: 1rem;
        margin-bottom: 1.5rem;
        color: #333;
      }

      .preview-item {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 0.5rem 0;
      }

      .preview-item:not(:last-child) {
        border-bottom: 1px solid #e0e0e0;
      }

      .preview-icon {
        font-size: 1.25rem;
      }

      .preview-content {
        display: flex;
        flex-direction: column;
        gap: 0.125rem;
      }

      .preview-content strong {
        font-size: 0.875rem;
        color: #1a1a1a;
      }

      .coming-soon {
        font-size: 0.75rem;
        color: #777;
        font-style: italic;
      }

      /* Dark theme overrides */
      :host-context(body[data-theme='dark']) .result-card,
      :host-context(body:not([data-theme])) .result-card {
        background: rgba(30, 30, 35, 0.95);
        color: #f0f0f0;
      }

      :host-context(body[data-theme='dark']) .result-title,
      :host-context(body:not([data-theme])) .result-title {
        color: #f0f0f0;
      }

      :host-context(body[data-theme='dark']) .result-description,
      :host-context(body:not([data-theme])) .result-description {
        color: #aaa;
      }

      :host-context(body[data-theme='dark']) .imagodei-preview,
      :host-context(body[data-theme='dark']) .mastery-preview,
      :host-context(body:not([data-theme])) .imagodei-preview,
      :host-context(body:not([data-theme])) .mastery-preview {
        background: rgba(45, 45, 50, 0.95);
        color: #ddd;
      }

      :host-context(body[data-theme='dark']) .preview-text,
      :host-context(body:not([data-theme])) .preview-text {
        color: #aaa;
      }

      :host-context(body[data-theme='dark']) .preview-content strong,
      :host-context(body:not([data-theme])) .preview-content strong {
        color: #f0f0f0;
      }

      :host-context(body[data-theme='dark']) .coming-soon,
      :host-context(body:not([data-theme])) .coming-soon {
        color: #888;
      }

      :host-context(body[data-theme='dark']) .score-label,
      :host-context(body:not([data-theme])) .score-label {
        color: #aaa;
      }

      :host-context(body[data-theme='dark']) .score-display,
      :host-context(body:not([data-theme])) .score-display {
        border-top-color: #444;
      }

      :host-context(body[data-theme='dark']) .preview-item:not(:last-child),
      :host-context(body:not([data-theme])) .preview-item:not(:last-child) {
        border-bottom-color: #444;
      }

      :host-context(body[data-theme='dark']) .profile-link,
      :host-context(body:not([data-theme])) .profile-link {
        color: #64b5f6;
      }
    `,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SophiaRendererComponent
  implements ContentRenderer, InteractiveRenderer, OnInit, OnChanges, OnDestroy
{
  private readonly cdr = inject(ChangeDetectorRef);
  private readonly destroy$ = new Subject<void>();

  // ─────────────────────────────────────────────────────────────────────────
  // ContentRenderer Interface
  // ─────────────────────────────────────────────────────────────────────────

  @Input() node!: ContentNode;

  // ─────────────────────────────────────────────────────────────────────────
  // InteractiveRenderer Interface
  // ─────────────────────────────────────────────────────────────────────────

  @Output() complete = new EventEmitter<RendererCompletionEvent>();

  // ─────────────────────────────────────────────────────────────────────────
  // Additional Inputs
  // ─────────────────────────────────────────────────────────────────────────

  @Input() showHeader = true;
  @Input() reviewMode = false;

  // ─────────────────────────────────────────────────────────────────────────
  // View References
  // ─────────────────────────────────────────────────────────────────────────

  @ViewChild('momentComponent') momentComponent!: SophiaWrapperComponent;

  // ─────────────────────────────────────────────────────────────────────────
  // State
  // ─────────────────────────────────────────────────────────────────────────

  moments: Moment[] = [];
  currentMomentIndex = 0;
  hasAnswer = false;
  isSubmitting = false;
  showFeedback = false;
  showNextButton = false;
  showResults = false;
  lastRecognition: Recognition | null = null;

  // Scoring
  recognitions: Recognition[] = [];
  demonstratedCount = 0;

  // Assessment mode
  assessmentMode: AssessmentMode = 'mastery';
  modeConfig: ModeConfig = MODE_PRESETS.mastery;

  // Psyche API for discovery/reflection aggregation (loaded lazily)
  private psycheAPI: PsycheAPI | null = null;

  // Reflection recognitions for psyche-core aggregation
  private readonly reflectionRecognitions: ReflectionRecognition[] = [];

  // Answer persistence for navigation (momentId → userInput)
  private readonly answersMap = new Map<string, UserInputMap>();

  // Aggregated reflection data (populated via Psyche API)
  aggregatedReflection: AggregatedReflection | null = null;

  // ─────────────────────────────────────────────────────────────────────────
  // Computed Properties
  // ─────────────────────────────────────────────────────────────────────────

  get title(): string {
    return this.node?.title ?? 'Assessment';
  }

  get currentMoment(): Moment | null {
    return this.moments[this.currentMomentIndex] ?? null;
  }

  get totalMoments(): number {
    return this.moments.length;
  }

  get progressPercentage(): number {
    if (this.totalMoments === 0) return 0;
    return ((this.currentMomentIndex + 1) / this.totalMoments) * 100;
  }

  get isLastMoment(): boolean {
    return this.currentMomentIndex >= this.totalMoments - 1;
  }

  get submitButtonText(): string {
    if (this.isSubmitting) return 'Processing...';
    if (this.showFeedback && this.modeConfig.showFeedback) return 'Submitted';
    return this.modeConfig.submitButtonText;
  }

  /** Get stored user input for current moment (for answer restoration on navigation) */
  get currentInitialUserInput(): UserInputMap | null {
    if (!this.currentMoment) return null;
    return this.answersMap.get(this.currentMoment.id) ?? null;
  }

  /** Mastery score as percentage (0-100) */
  get masteryScorePercent(): number {
    if (this.totalMoments === 0) return 0;
    return Math.round((this.demonstratedCount / this.totalMoments) * 100);
  }

  /** Whether mastery assessment was passed (70% threshold) */
  get masteryPassed(): boolean {
    return this.masteryScorePercent >= 70;
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Lifecycle
  // ─────────────────────────────────────────────────────────────────────────

  ngOnInit(): void {
    // Initialize Psyche API for discovery/reflection aggregation
    this.psycheAPI = getPsycheAPI();
  }

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['node'] && this.node) {
      this.loadMoments();
      this.cdr.markForCheck();
    }
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Event Handlers
  // ─────────────────────────────────────────────────────────────────────────

  handleReady(): void {
    this.cdr.markForCheck();
  }

  handleAnswerChange(hasAnswer: boolean): void {
    // Ignore spurious false events when we have a stored answer for this moment
    // (protects against timing edge cases during answer restoration)
    if (!hasAnswer && this.currentMoment && this.answersMap.has(this.currentMoment.id)) {
      return;
    }
    this.hasAnswer = hasAnswer;
    this.cdr.markForCheck();
  }

  handleRecognition(recognition: Recognition): void {
    this.lastRecognition = recognition;
    this.cdr.markForCheck();
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Actions
  // ─────────────────────────────────────────────────────────────────────────

  submitAnswer(): void {
    if (!this.momentComponent || this.isSubmitting || this.showFeedback) {
      return;
    }

    this.isSubmitting = true;
    this.cdr.markForCheck();

    // Get recognition from the Sophia element
    const recognition = this.momentComponent.getRecognition();

    if (recognition) {
      this.processRecognition(recognition);
    }

    this.isSubmitting = false;
    this.cdr.markForCheck();
  }

  private processRecognition(recognition: Recognition): void {
    this.lastRecognition = recognition;

    // Store user input for answer persistence during navigation
    if (this.currentMoment) {
      this.answersMap.set(this.currentMoment.id, recognition.userInput);
    }

    // Update or append recognition (for cases where user changes answer on previous question)
    this.updateRecognitionsList(recognition);

    const isDiscoveryOrReflection =
      this.assessmentMode === 'discovery' || this.assessmentMode === 'reflection';

    if (isDiscoveryOrReflection) {
      this.handleDiscoverySubmission(recognition);
    } else {
      this.handleMasterySubmission(recognition);
    }
  }

  private updateRecognitionsList(recognition: Recognition): void {
    const existingIndex = this.recognitions.findIndex(r => r.momentId === recognition.momentId);
    if (existingIndex >= 0) {
      this.recognitions[existingIndex] = recognition;
    } else {
      this.recognitions.push(recognition);
    }
  }

  private handleDiscoverySubmission(recognition: Recognition): void {
    // Use Psyche API for aggregation
    this.aggregateViaAPI(recognition);
    // Auto-advance: submit AND move to next in one action (no double-click)
    this.isSubmitting = false;
    if (this.isLastMoment) {
      this.finishAssessment();
    } else {
      this.currentMomentIndex++;
      this.resetMomentState();
      // Note: sophia-question now emits onAnswerChange(true) when initialUserInput
      // restores a valid answer, so we don't need to manually set hasAnswer here
    }
    this.cdr.markForCheck();
  }

  private handleMasterySubmission(recognition: Recognition): void {
    // Track mastery
    if (recognition.mastery?.demonstrated) {
      this.demonstratedCount++;
    }
    this.showFeedback = true;
    this.showNextButton = true;
  }

  nextMoment(): void {
    if (this.isLastMoment) {
      this.finishAssessment();
    } else {
      this.currentMomentIndex++;
      this.resetMomentState();
    }
    this.cdr.markForCheck();
  }

  previousMoment(): void {
    if (this.currentMomentIndex > 0) {
      // Save current answer before navigating back (so it persists if user returns)
      if (this.momentComponent && this.currentMoment && this.hasAnswer) {
        const recognition = this.momentComponent.getRecognition();
        if (recognition) {
          this.answersMap.set(this.currentMoment.id, recognition.userInput);
        }
      }

      this.currentMomentIndex--;
      this.hasAnswer = true; // Previous answer will be restored via initialUserInput
      this.showFeedback = false;
      this.showNextButton = false;
      this.lastRecognition =
        this.recognitions.find(r => r.momentId === this.currentMoment?.id) ?? null;
    }
    this.cdr.markForCheck();
  }

  // ─────────────────────────────────────────────────────────────────────────
  // Private Methods
  // ─────────────────────────────────────────────────────────────────────────

  private loadMoments(): void {
    if (!this.node?.content) {
      return;
    }

    const content = this.parseContent();
    if (content === null) {
      return;
    }

    this.moments = this.convertToMoments(content);

    this.initializeAssessmentMode();
  }

  private parseContent(): unknown | null {
    let content = this.node?.content;

    // Parse JSON string if needed
    if (typeof content === 'string') {
      try {
        content = JSON.parse(content);
      } catch {
        return null;
      }
    }

    return content;
  }

  private convertToMoments(content: unknown): Moment[] {
    if (Array.isArray(content)) {
      return content.map(item => this.toMoment(item));
    }
    if (typeof content === 'object' && content !== null) {
      return [this.toMoment(content)];
    }
    return [];
  }

  private initializeAssessmentMode(): void {
    if (this.moments.length === 0) return;

    const firstMoment = this.moments[0];
    this.assessmentMode = this.detectModeFromPurpose(firstMoment.purpose);
    this.modeConfig = MODE_PRESETS[this.assessmentMode];

    // Ensure Psyche API is available for discovery/reflection modes
    if (this.assessmentMode !== 'mastery' && !this.psycheAPI) {
      this.psycheAPI = getPsycheAPI();
    }
  }

  private detectModeFromPurpose(purpose: string): 'mastery' | 'discovery' | 'reflection' {
    if (purpose === 'mastery') return 'mastery';
    if (purpose === 'discovery') return 'discovery';
    return 'reflection';
  }

  /**
   * Convert various content formats to a Sophia Moment.
   */
  private toMoment(item: unknown): Moment {
    const obj = item as Record<string, unknown>;

    // Already a Moment format
    if (obj['purpose'] && obj['content']) {
      return item as Moment;
    }

    // Perseus item format (has 'question' property)
    if (obj['question']) {
      const purpose =
        obj['discoveryMode'] || obj['purpose'] === 'discovery' || obj['purpose'] === 'reflection'
          ? 'reflection'
          : 'mastery';

      return {
        id:
          typeof obj['id'] === 'string' || typeof obj['id'] === 'number'
            ? String(obj['id'])
            : this.generateMomentId(),
        purpose,
        content: obj['question'] as Moment['content'],
        hints: obj['hints'] as Moment['hints'],
        subscaleContributions: obj['subscaleContributions'] as Moment['subscaleContributions'],
        metadata: obj['metadata'] as Moment['metadata'],
      };
    }

    // Default: treat as raw content (requires explicit cast through unknown)
    return {
      id: this.generateMomentId(),
      purpose: 'mastery',
      content: obj as unknown as Moment['content'],
    };
  }

  /**
   * Generate a unique moment ID.
   * Uses crypto.randomUUID for secure random IDs when available.
   */
  private generateMomentId(): string {
    // Use crypto.randomUUID if available (modern browsers)
    if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
      return `moment-${crypto.randomUUID().substring(0, 9)}`;
    }
    // Fallback: use crypto.getRandomValues for better randomness than Math.random
    const array = new Uint32Array(1);
    crypto.getRandomValues(array);
    return `moment-${array[0].toString(36).substring(0, 9)}`;
  }

  /**
   * Aggregate recognition data using the Psyche API.
   * This delegates to sophia's built-in aggregation functions.
   */
  private aggregateViaAPI(recognition: Recognition): void {
    // Convert to ReflectionRecognition format for the Psyche API
    const reflectionRecognition: ReflectionRecognition = {
      momentId: recognition.momentId,
      purpose: 'reflection',
      userInput: recognition.userInput,
      reflection: recognition.reflection ?? {
        subscaleContributions: recognition.resonance?.subscaleContributions ?? {},
      },
      timestamp: recognition.timestamp ?? Date.now(),
    };

    this.reflectionRecognitions.push(reflectionRecognition);

    // Ensure Psyche API is available
    this.psycheAPI ??= getPsycheAPI();

    // Use Psyche API for aggregation if available
    if (this.psycheAPI && this.reflectionRecognitions.length > 0) {
      this.aggregatedReflection = this.psycheAPI.aggregateReflections(this.reflectionRecognitions, {
        normalization: 'sum',
      });
    } else {
      // Fallback: create minimal aggregation manually
      this.aggregateFallback(recognition);
    }
  }

  /**
   * Fallback aggregation when Psyche API is not available.
   * Used only as a safety net - the Psyche API should always be available
   * after sophia-plugin is loaded.
   */
  private aggregateFallback(recognition: Recognition): void {
    const contributions =
      recognition.reflection?.subscaleContributions ?? recognition.resonance?.subscaleContributions;

    if (!contributions) return;

    // Initialize aggregated data if needed
    this.aggregatedReflection ??= {
      subscaleTotals: {},
      subscaleCounts: {},
      normalizedScores: {},
      momentCount: 0,
      momentIds: [],
      aggregatedAt: 0,
    };

    for (const [subscale, value] of Object.entries(contributions)) {
      this.aggregatedReflection.subscaleTotals[subscale] =
        (this.aggregatedReflection.subscaleTotals[subscale] ?? 0) + value;
      this.aggregatedReflection.subscaleCounts[subscale] =
        (this.aggregatedReflection.subscaleCounts[subscale] ?? 0) + 1;
    }

    this.aggregatedReflection.momentIds.push(recognition.momentId);
    this.aggregatedReflection.momentCount++;

    // Normalize scores
    const total =
      Object.values(this.aggregatedReflection.subscaleTotals).reduce((sum, v) => sum + v, 0) ?? 1;

    for (const [subscale, value] of Object.entries(this.aggregatedReflection.subscaleTotals)) {
      this.aggregatedReflection.normalizedScores[subscale] = value / total;
    }
  }

  private resetMomentState(): void {
    this.hasAnswer = false;
    this.showFeedback = false;
    this.showNextButton = false;
    this.lastRecognition = null;
  }

  private finishAssessment(): void {
    // Show results view for all assessment modes
    this.showResults = true;
    this.cdr.markForCheck();
  }

  /**
   * Called when user clicks "Continue" on results view.
   * Emits completion event and triggers navigation to next step.
   */
  completeAndContinue(): void {
    const isDiscoveryOrReflection =
      this.assessmentMode === 'discovery' || this.assessmentMode === 'reflection';
    if (isDiscoveryOrReflection) {
      this.emitReflectionCompletion();
    } else {
      this.emitMasteryCompletion();
    }

    // Trigger navigation to next step after completion event is processed
    // Find and click the path-navigator's "Next" button
    setTimeout(() => {
      const nextButton = document.querySelector('.btn-next:not([disabled])') as HTMLButtonElement;
      if (nextButton) {
        nextButton.click();
      }
    }, 100);
  }

  private emitMasteryCompletion(): void {
    const score = this.demonstratedCount / Math.max(this.totalMoments, 1);
    const passed = score >= 0.7;

    const event: RendererCompletionEvent = {
      type: 'quiz',
      passed,
      score: Math.round(score * 100),
      details: {
        correct: this.demonstratedCount,
        total: this.totalMoments,
        recognitions: this.recognitions,
        moments: this.moments.map((m, i) => ({
          id: m.id,
          correct: this.recognitions[i]?.mastery?.demonstrated ?? false,
          score: this.recognitions[i]?.mastery?.score ?? 0,
        })),
      },
    };

    this.complete.emit(event);
  }

  private emitReflectionCompletion(): void {
    // Guard against null/undefined aggregatedReflection
    if (!this.aggregatedReflection?.subscaleTotals) {
      // Emit basic completion event without subscale details
      const event: RendererCompletionEvent = {
        type: 'quiz',
        passed: true,
        score: 100,
        details: {
          assessmentMode: this.assessmentMode,
          recognitions: this.recognitions,
          total: this.totalMoments,
          correct: this.totalMoments,
        },
      };
      this.complete.emit(event);
      return;
    }

    // Calculate primary subscale (highest scoring)
    let primarySubscale = '';
    const totals = this.aggregatedReflection.subscaleTotals;

    // Use manual calculation to avoid Psyche API edge cases
    if (totals && typeof totals === 'object') {
      let maxScore = 0;
      for (const [subscale, score] of Object.entries(totals)) {
        if (typeof score === 'number' && score > maxScore) {
          maxScore = score;
          primarySubscale = subscale;
        }
      }
    }

    const event: RendererCompletionEvent = {
      type: 'quiz',
      passed: true, // Discovery/reflection always "passes"
      score: 100,
      details: {
        assessmentMode: this.assessmentMode,
        subscaleTotals: { ...this.aggregatedReflection.subscaleTotals },
        normalizedScores: { ...this.aggregatedReflection.normalizedScores },
        primarySubscale,
        aggregatedReflection: { ...this.aggregatedReflection },
        recognitions: this.recognitions,
        total: this.totalMoments,
        correct: this.totalMoments,
      },
    };

    this.complete.emit(event);
  }
}
