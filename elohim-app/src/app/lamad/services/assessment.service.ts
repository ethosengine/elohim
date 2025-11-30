import { Injectable } from '@angular/core';
import { Observable, of, BehaviorSubject } from 'rxjs';
import { map } from 'rxjs/operators';
import {
  DataLoaderService,
  AssessmentIndex,
  AssessmentIndexEntry
} from './data-loader.service';
import { ContentNode } from '../models/content-node.model';
import { SessionHumanService } from './session-human.service';

/**
 * Assessment result stored in localStorage (MVP) or source chain (Holochain).
 */
export interface AssessmentResult {
  /** Assessment ID */
  assessmentId: string;

  /** Agent who completed the assessment */
  agentId: string;

  /** When completed */
  completedAt: string;

  /** Time spent in milliseconds */
  timeSpentMs: number;

  /** Responses by question ID */
  responses: Record<string, QuestionResponse>;

  /** Computed scores by subscale */
  scores: Record<string, number>;

  /** Overall score (if applicable) */
  overallScore?: number;

  /** Interpretation/outcome (e.g., "Secure" for attachment) */
  interpretation?: string;

  /** Attestation granted on completion */
  attestationGranted?: string;
}

export interface QuestionResponse {
  questionId: string;
  questionType: string;
  value: number | string | string[];
  answeredAt: string;
}

/**
 * In-progress assessment session.
 */
export interface AssessmentSession {
  assessmentId: string;
  agentId: string;
  startedAt: string;
  currentQuestionIndex: number;
  responses: Record<string, QuestionResponse>;
  timeSpentMs: number;
}

/**
 * AssessmentService - Manages psychometric assessments and self-knowledge instruments.
 *
 * Responsibilities:
 * - Load assessment instruments from data loader
 * - Manage assessment sessions (in-progress state)
 * - Score completed assessments
 * - Store results (localStorage for MVP, source chain for Holochain)
 * - Track attestations granted from assessments
 *
 * Privacy note: Assessment results are private by default.
 * Only aggregated, anonymized data can be shared (with explicit consent).
 */
@Injectable({ providedIn: 'root' })
export class AssessmentService {
  private readonly STORAGE_PREFIX = 'lamad-assessment-';

  // Active session (one at a time)
  private readonly activeSession$ = new BehaviorSubject<AssessmentSession | null>(null);

  constructor(
    private readonly dataLoader: DataLoaderService,
    private readonly sessionUser: SessionHumanService
  ) {}

  // =========================================================================
  // Assessment Discovery
  // =========================================================================

  /**
   * Get all available assessments.
   */
  getAssessmentIndex(): Observable<AssessmentIndex> {
    return this.dataLoader.getAssessmentIndex();
  }

  /**
   * Get assessments filtered by domain.
   */
  getAssessmentsByDomain(domain: string): Observable<AssessmentIndexEntry[]> {
    return this.dataLoader.getAssessmentsByDomain(domain);
  }

  /**
   * Load a full assessment instrument.
   */
  getAssessment(assessmentId: string): Observable<ContentNode | null> {
    return this.dataLoader.getAssessment(assessmentId);
  }

  /**
   * Check if user has prerequisite attestation for gated assessment.
   */
  canAccessAssessment(assessmentId: string): Observable<boolean> {
    return this.getAssessment(assessmentId).pipe(
      map(assessment => {
        if (!assessment) return false;

        const metadata = assessment.metadata as any;
        const prerequisite = metadata?.prerequisiteAttestation;

        if (!prerequisite) return true; // No prerequisite

        // Check if user has the prerequisite attestation
        const userAttestations = this.getUserAttestations();
        return userAttestations.includes(prerequisite);
      })
    );
  }

  // =========================================================================
  // Assessment Sessions
  // =========================================================================

  /**
   * Start a new assessment session.
   */
  startAssessment(assessmentId: string): Observable<AssessmentSession> {
    const agentId = this.sessionUser.getSessionId() || 'anonymous';

    const session: AssessmentSession = {
      assessmentId,
      agentId,
      startedAt: new Date().toISOString(),
      currentQuestionIndex: 0,
      responses: {},
      timeSpentMs: 0
    };

    this.activeSession$.next(session);
    this.saveSessionToStorage(session);

    return of(session);
  }

  /**
   * Get the current active session.
   */
  getActiveSession(): Observable<AssessmentSession | null> {
    return this.activeSession$.asObservable();
  }

  /**
   * Resume an incomplete assessment.
   */
  resumeAssessment(assessmentId: string): Observable<AssessmentSession | null> {
    const agentId = this.sessionUser.getSessionId() || 'anonymous';
    const session = this.loadSessionFromStorage(agentId, assessmentId);

    if (session) {
      this.activeSession$.next(session);
    }

    return of(session);
  }

  /**
   * Record a response to a question.
   */
  recordResponse(
    questionId: string,
    questionType: string,
    value: number | string | string[]
  ): void {
    const session = this.activeSession$.value;
    if (!session) {
      console.error('[AssessmentService] No active session');
      return;
    }

    session.responses[questionId] = {
      questionId,
      questionType,
      value,
      answeredAt: new Date().toISOString()
    };

    session.currentQuestionIndex++;
    this.activeSession$.next(session);
    this.saveSessionToStorage(session);
  }

  /**
   * Update time spent on assessment.
   */
  updateTimeSpent(additionalMs: number): void {
    const session = this.activeSession$.value;
    if (session) {
      session.timeSpentMs += additionalMs;
      this.activeSession$.next(session);
    }
  }

  /**
   * Abandon current assessment session.
   */
  abandonAssessment(): void {
    const session = this.activeSession$.value;
    if (session) {
      this.clearSessionFromStorage(session.agentId, session.assessmentId);
    }
    this.activeSession$.next(null);
  }

  // =========================================================================
  // Scoring & Completion
  // =========================================================================

  /**
   * Complete the assessment and compute scores.
   */
  completeAssessment(): Observable<AssessmentResult | null> {
    const session = this.activeSession$.value;
    if (!session) {
      return of(null);
    }

    return this.getAssessment(session.assessmentId).pipe(
      map(assessment => {
        if (!assessment) return null;

        const content = assessment.content as any;
        const scores = this.computeScores(session.responses, content);
        const interpretation = this.computeInterpretation(scores, content);

        const result: AssessmentResult = {
          assessmentId: session.assessmentId,
          agentId: session.agentId,
          completedAt: new Date().toISOString(),
          timeSpentMs: session.timeSpentMs,
          responses: session.responses,
          scores,
          interpretation,
          attestationGranted: (assessment.metadata as any)?.attestationId
        };

        // Save result
        this.saveResultToStorage(result);

        // Clear session
        this.clearSessionFromStorage(session.agentId, session.assessmentId);
        this.activeSession$.next(null);

        // Grant attestation if applicable
        if (result.attestationGranted) {
          this.grantAttestation(result.attestationGranted);
        }

        return result;
      })
    );
  }

  /**
   * Compute scores from responses based on assessment structure.
   */
  private computeScores(
    responses: Record<string, QuestionResponse>,
    content: any
  ): Record<string, number> {
    const scores: Record<string, number> = {};

    // Simple scoring: aggregate by subscale
    const sections = content.sections || [];
    const questions = content.questions || [];

    const allQuestions = [
      ...questions,
      ...sections.flatMap((s: any) => s.questions || [])
    ];

    for (const question of allQuestions) {
      const response = responses[question.id];
      if (!response) continue;

      const subscales = question.subscales || [];
      const value = typeof response.value === 'number' ? response.value : 0;
      const actualValue = question.reverseScored ? (8 - value) : value; // Assume 7-point scale

      for (const subscale of subscales) {
        scores[subscale] = (scores[subscale] || 0) + actualValue;
      }
    }

    return scores;
  }

  /**
   * Compute interpretation based on scores and assessment configuration.
   */
  private computeInterpretation(
    scores: Record<string, number>,
    content: any
  ): string | undefined {
    const interpretation = content.interpretation;
    if (!interpretation) return undefined;

    if (interpretation.method === 'quadrant') {
      // E.g., attachment style
      const dimensions = interpretation.dimensions as string[];
      const outcomes = interpretation.outcomes as Array<{
        name: string;
        [key: string]: string;
      }>;

      if (dimensions.length === 2) {
        const [dim1, dim2] = dimensions;
        const score1 = scores[dim1] || 0;
        const score2 = scores[dim2] || 0;

        // Simple threshold (midpoint of possible range)
        const threshold = 20; // Adjust based on actual scale

        const level1 = score1 > threshold ? 'high' : 'low';
        const level2 = score2 > threshold ? 'high' : 'low';

        const match = outcomes.find(o =>
          o[dim1] === level1 && o[dim2] === level2
        );

        return match?.name;
      }
    }

    if (interpretation.method === 'ranking') {
      // Return top subscale
      const sorted = Object.entries(scores).sort(([, a], [, b]) => b - a);
      return sorted[0]?.[0];
    }

    return undefined;
  }

  // =========================================================================
  // Results History
  // =========================================================================

  /**
   * Get all assessment results for current user.
   */
  getMyResults(): Observable<AssessmentResult[]> {
    const agentId = this.sessionUser.getSessionId() || 'anonymous';
    return of(this.loadAllResultsFromStorage(agentId));
  }

  /**
   * Get result for a specific assessment.
   */
  getResultForAssessment(assessmentId: string): Observable<AssessmentResult | null> {
    const agentId = this.sessionUser.getSessionId() || 'anonymous';
    return of(this.loadResultFromStorage(agentId, assessmentId));
  }

  /**
   * Check if user has completed an assessment.
   */
  hasCompleted(assessmentId: string): Observable<boolean> {
    return this.getResultForAssessment(assessmentId).pipe(
      map(result => result !== null)
    );
  }

  // =========================================================================
  // Attestation Integration
  // =========================================================================

  /**
   * Get attestations earned from assessments.
   */
  private getUserAttestations(): string[] {
    const agentId = this.sessionUser.getSessionId() || 'anonymous';
    const key = `${this.STORAGE_PREFIX}attestations-${agentId}`;
    const data = localStorage.getItem(key);
    return data ? JSON.parse(data) : [];
  }

  /**
   * Grant an attestation from assessment completion.
   */
  private grantAttestation(attestationId: string): void {
    const agentId = this.sessionUser.getSessionId() || 'anonymous';
    const key = `${this.STORAGE_PREFIX}attestations-${agentId}`;
    const attestations = this.getUserAttestations();

    if (!attestations.includes(attestationId)) {
      attestations.push(attestationId);
      localStorage.setItem(key, JSON.stringify(attestations));
    }
  }

  // =========================================================================
  // Storage (localStorage for MVP)
  // =========================================================================

  private saveSessionToStorage(session: AssessmentSession): void {
    const key = `${this.STORAGE_PREFIX}session-${session.agentId}-${session.assessmentId}`;
    try {
      localStorage.setItem(key, JSON.stringify(session));
    } catch (err) {
      console.error('[AssessmentService] Failed to save session', err);
    }
  }

  private loadSessionFromStorage(agentId: string, assessmentId: string): AssessmentSession | null {
    const key = `${this.STORAGE_PREFIX}session-${agentId}-${assessmentId}`;
    const data = localStorage.getItem(key);
    return data ? JSON.parse(data) : null;
  }

  private clearSessionFromStorage(agentId: string, assessmentId: string): void {
    const key = `${this.STORAGE_PREFIX}session-${agentId}-${assessmentId}`;
    localStorage.removeItem(key);
  }

  private saveResultToStorage(result: AssessmentResult): void {
    const key = `${this.STORAGE_PREFIX}result-${result.agentId}-${result.assessmentId}`;
    try {
      localStorage.setItem(key, JSON.stringify(result));
    } catch (err) {
      console.error('[AssessmentService] Failed to save result', err);
    }
  }

  private loadResultFromStorage(agentId: string, assessmentId: string): AssessmentResult | null {
    const key = `${this.STORAGE_PREFIX}result-${agentId}-${assessmentId}`;
    const data = localStorage.getItem(key);
    return data ? JSON.parse(data) : null;
  }

  private loadAllResultsFromStorage(agentId: string): AssessmentResult[] {
    const results: AssessmentResult[] = [];
    const prefix = `${this.STORAGE_PREFIX}result-${agentId}-`;

    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (key?.startsWith(prefix)) {
        const data = localStorage.getItem(key);
        if (data) {
          try {
            results.push(JSON.parse(data));
          } catch {
            // Skip invalid entries
          }
        }
      }
    }

    return results.sort((a, b) =>
      new Date(b.completedAt).getTime() - new Date(a.completedAt).getTime()
    );
  }
}
