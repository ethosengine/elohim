/**
 * RecoveryInterviewComponent - Interviewer UI for recovery attestations.
 *
 * For Elohim network members who help verify identity recovery requests.
 * This component allows them to:
 * - View pending recovery requests
 * - Conduct interviews with generated questions
 * - Submit attestation decisions with confidence levels
 */

import { CommonModule } from '@angular/common';
import { Component, inject, OnInit, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';

import {
  type PendingRecoveryRequest,
  type InterviewQuestion,
  type AttestationDecision,
  getRecoveryStatusDisplay,
  getQuestionTypeDisplay,
  maskIdentity,
} from '../../models/recovery.model';
import { RecoveryCoordinatorService } from '../../services/recovery-coordinator.service';

type ViewMode = 'queue' | 'interview' | 'attestation';

@Component({
  selector: 'app-recovery-interview',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './recovery-interview.component.html',
  styleUrls: ['./recovery-interview.component.css'],
})
export class RecoveryInterviewComponent implements OnInit {
  private readonly recoveryService = inject(RecoveryCoordinatorService);

  // ===========================================================================
  // State Delegation
  // ===========================================================================

  readonly pendingRequests = this.recoveryService.pendingRequests;
  readonly conductingInterview = this.recoveryService.conductingInterview;
  readonly isLoading = this.recoveryService.isLoading;
  readonly error = this.recoveryService.error;

  // ===========================================================================
  // Component State
  // ===========================================================================

  readonly viewMode = signal<ViewMode>('queue');
  readonly currentQuestionIndex = signal(0);
  readonly questions = signal<InterviewQuestion[]>([]);
  readonly answers = signal<Map<string, string>>(new Map());

  // Attestation form
  decision: AttestationDecision = 'abstain';
  confidence = 50;
  notes = '';

  // ===========================================================================
  // Template Helpers
  // ===========================================================================

  readonly getRecoveryStatusDisplay = getRecoveryStatusDisplay;
  readonly getQuestionTypeDisplay = getQuestionTypeDisplay;
  readonly maskIdentity = maskIdentity;

  // ===========================================================================
  // Lifecycle
  // ===========================================================================

  ngOnInit(): void {
    this.loadPendingRequests();
  }

  // ===========================================================================
  // Actions
  // ===========================================================================

  async loadPendingRequests(): Promise<void> {
    await this.recoveryService.loadPendingRequests();
  }

  async startInterview(requestId: string): Promise<void> {
    const success = await this.recoveryService.startInterview(requestId);
    if (success) {
      // Load questions
      const questions = await this.recoveryService.generateQuestions(requestId);
      this.questions.set(questions);
      this.answers.set(new Map());
      this.currentQuestionIndex.set(0);
      this.viewMode.set('interview');
    }
  }

  async submitAnswer(): Promise<void> {
    const interview = this.conductingInterview();
    const questionsList = this.questions();
    const currentIndex = this.currentQuestionIndex();

    if (!interview || currentIndex >= questionsList.length) return;

    const currentQuestion = questionsList[currentIndex];
    const answer = this.answers().get(currentQuestion.id) ?? '';

    if (!answer.trim()) return;

    await this.recoveryService.submitResponse(currentQuestion.id, answer);

    // Move to next question or attestation
    if (currentIndex < questionsList.length - 1) {
      this.currentQuestionIndex.update(i => i + 1);
    } else {
      this.viewMode.set('attestation');
    }
  }

  setAnswer(questionId: string, answer: string): void {
    this.answers.update(map => {
      const newMap = new Map(map);
      newMap.set(questionId, answer);
      return newMap;
    });
  }

  previousQuestion(): void {
    if (this.currentQuestionIndex() > 0) {
      this.currentQuestionIndex.update(i => i - 1);
    }
  }

  async submitAttestation(): Promise<void> {
    const success = await this.recoveryService.submitAttestation(
      this.decision,
      this.confidence,
      this.notes.trim() || undefined
    );

    if (success) {
      this.resetInterviewState();
      this.viewMode.set('queue');
    }
  }

  abandonInterview(): void {
    this.recoveryService.abandonInterview();
    this.resetInterviewState();
    this.viewMode.set('queue');
  }

  clearError(): void {
    this.recoveryService.clearError();
  }

  // ===========================================================================
  // Helpers
  // ===========================================================================

  private resetInterviewState(): void {
    this.questions.set([]);
    this.answers.set(new Map());
    this.currentQuestionIndex.set(0);
    this.decision = 'abstain';
    this.confidence = 50;
    this.notes = '';
  }

  getCurrentQuestion(): InterviewQuestion | null {
    const list = this.questions();
    const index = this.currentQuestionIndex();
    return list[index] ?? null;
  }

  getCurrentAnswer(): string {
    const question = this.getCurrentQuestion();
    if (!question) return '';
    return this.answers().get(question.id) ?? '';
  }

  getProgressPercentage(): number {
    const total = this.questions().length;
    if (total === 0) return 0;
    return Math.round((this.currentQuestionIndex() / total) * 100);
  }

  trackByRequestId(_index: number, request: PendingRecoveryRequest): string {
    return request.requestId;
  }

  trackByQuestionId(_index: number, question: InterviewQuestion): string {
    return question.id;
  }

  getConfidenceLabel(): string {
    if (this.confidence >= 90) return 'Very High';
    if (this.confidence >= 70) return 'High';
    if (this.confidence >= 50) return 'Moderate';
    if (this.confidence >= 30) return 'Low';
    return 'Very Low';
  }

  getDecisionClass(): string {
    switch (this.decision) {
      case 'affirm':
        return 'decision-affirm';
      case 'deny':
        return 'decision-deny';
      default:
        return 'decision-abstain';
    }
  }
}
