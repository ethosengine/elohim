/**
 * Lamad-local agent interface + injection token.
 *
 * P+inversion disposition: AgentService stays in elohim-app (it has an
 * @app/imagodei dependency — SessionHumanService — that Slice 2.3b owns).
 * This token decouples lamad services from the concrete class import.
 *
 * Registration in lamad's app.config.ts:
 *   { provide: LAMAD_AGENT, useExisting: AgentService }
 *
 * Slice 2.1c: Cross-pillar import cleanup.
 */

import { InjectionToken } from '@angular/core';

import { Observable } from 'rxjs';

import type {
  Agent,
  AgentProgress,
  AgentAttestation,
} from '@elohim/service';
import type { MasteryLevel } from '@elohim/service/angular/models/agent.model';

// ============================================================================
// ILamadAgent — narrow contract for what lamad services actually call
// ============================================================================

export interface ILamadAgent {
  // ---- Identity ----
  getCurrentAgentId(): string;

  // ---- Progress ----
  getAgentProgress(): Observable<AgentProgress[]>;
  getProgressForPath(pathId: string): Observable<AgentProgress | null>;
  completeStep(pathId: string, stepIndex: number, resourceId?: string): Observable<void>;

  // ---- Attestations ----
  getAttestations(): string[];

  // ---- Completion tracking ----
  isContentCompleted(contentId: string, agentId?: string): Observable<boolean>;
  getCompletedContentIds(agentId?: string): Observable<Set<string>>;

  // ---- Mastery ----
  getContentMastery(contentId: string, agentId?: string): Observable<MasteryLevel>;
  getAllContentMastery(agentId?: string): Observable<Map<string, MasteryLevel>>;
  updateContentMastery(
    contentId: string,
    level: MasteryLevel,
    agentId?: string
  ): Observable<void>;

  // ---- Content visibility ----
  markContentSeen(contentId: string): Observable<void>;
  saveAgentProgress(progress: AgentProgress): Observable<void>;
}

/**
 * Injection token for the lamad agent service.
 *
 * No factory — registered in lamad's app.config.ts:
 *   { provide: LAMAD_AGENT, useExisting: AgentService }
 *
 * In tests:
 *   { provide: LAMAD_AGENT, useValue: mockAgent }
 */
export const LAMAD_AGENT = new InjectionToken<ILamadAgent>('LamadAgent');
