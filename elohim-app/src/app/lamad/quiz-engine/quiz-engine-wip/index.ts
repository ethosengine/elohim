/**
 * Quiz Engine Module
 *
 * Khan Academy-style quiz system for lamad learning platform.
 *
 * Features:
 * - Perseus integration for rich question types
 * - Practice quizzes from content hierarchy
 * - Mastery quizzes with attempt limits
 * - Inline quizzes for post-content attestation
 * - Path adaptation (gates, recommendations, skip-ahead)
 *
 * @example
 * ```typescript
 * import { QuestionPoolService, QuizSession, createQuizSession } from '@app/lamad/quiz-engine';
 *
 * // Load questions
 * const pool = await poolService.getPoolForContent('concept-123').toPromise();
 *
 * // Create session
 * const session = createQuizSession('inline', humanId, pool.questions);
 * ```
 */

// Models
export * from './models';

// Services
export * from './services';

// Components
export * from './components';

// Utils
export * from './utils';
