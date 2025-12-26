/**
 * Quiz Engine Components
 *
 * UI components for quiz interactions:
 * - Inline quiz for post-content attestation
 * - Mastery gate for section boundaries
 * - Pre-assessment for skip-ahead
 */

export { InlineQuizComponent } from './inline-quiz/inline-quiz.component';
export type { InlineQuizCompletionEvent } from './inline-quiz/inline-quiz.component';

export { MasteryGateComponent } from './mastery-gate/mastery-gate.component';
export type { MasteryQuizRequestEvent } from './mastery-gate/mastery-gate.component';

export { PreAssessmentComponent } from './pre-assessment/pre-assessment.component';
export type {
  PreAssessmentDecision,
  SkipSelectionEvent
} from './pre-assessment/pre-assessment.component';
