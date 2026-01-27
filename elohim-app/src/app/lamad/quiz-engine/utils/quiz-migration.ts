/**
 * Quiz Migration Utilities
 *
 * Converts legacy quiz-json format to Perseus format for the new quiz engine.
 *
 * Legacy Format (quiz-json):
 * - questions[].type: 'multiple-choice' | 'true-false' | 'short-answer'
 * - questions[].options: string[]
 * - questions[].correctAnswer: number (index) | boolean | string
 *
 * Perseus Format:
 * - question.widgets.radio: for multiple choice
 * - question.widgets.input-number: for numeric answers
 * - question.widgets.expression: for math expressions
 */

import type { PerseusItem } from '../../content-io/plugins/sophia/sophia-moment.model';
import type { MasteryLevel } from '../../models/content-mastery.model';

// Radio widget options type for quiz migration
interface RadioWidgetOptions {
  choices: {
    content: string;
    correct?: boolean;
    isNoneOfTheAbove?: boolean;
  }[];
  hasNoneOfTheAbove?: boolean;
  multipleSelect?: boolean;
  deselectEnabled?: boolean;
  randomize?: boolean;
}

// ─────────────────────────────────────────────────────────────────────────────
// Legacy Format Types
// ─────────────────────────────────────────────────────────────────────────────

export interface LegacyQuizQuestion {
  id: string;
  type: 'multiple-choice' | 'true-false' | 'short-answer' | 'connection';
  question: string;
  text?: string; // Alternative to question
  options?: string[];
  correctAnswer?: number | string | boolean;
  rubric?: string;
  explanation?: string;
}

export interface LegacyQuizContent {
  passingScore: number;
  allowRetake: boolean;
  showCorrectAnswers: boolean;
  questions: LegacyQuizQuestion[];
}

export interface LegacyContentNode {
  id: string;
  contentType: string;
  title: string;
  description: string;
  content: LegacyQuizContent | string;
  contentFormat: string;
  tags: string[];
  relatedNodeIds?: string[];
  metadata?: Record<string, unknown>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Migration Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Convert a legacy quiz-json ContentNode to Perseus format.
 */
export function migrateQuizToPerseus(node: LegacyContentNode): PerseusItem[] {
  let content = node.content;

  // Parse if string
  if (typeof content === 'string') {
    try {
      content = JSON.parse(content) as LegacyQuizContent;
    } catch (e) {
      console.error(`Failed to parse quiz content for ${node.id}:`, e);
      return [];
    }
  }

  const quiz = content as LegacyQuizContent;
  if (!quiz.questions || !Array.isArray(quiz.questions)) {
    console.warn(`No questions array in ${node.id}`);
    return [];
  }

  return quiz.questions.map(q => migrateQuestion(q, node));
}

/**
 * Convert a single legacy question to Perseus format.
 */
export function migrateQuestion(
  question: LegacyQuizQuestion,
  sourceNode: LegacyContentNode
): PerseusItem {
  const questionText = question.question ?? question.text ?? '';

  switch (question.type) {
    case 'multiple-choice':
      return migrateMultipleChoice(question, questionText, sourceNode);

    case 'true-false':
      return migrateTrueFalse(question, questionText, sourceNode);

    case 'short-answer':
      return migrateShortAnswer(question, questionText, sourceNode);

    default:
      // Fallback to simple text display
      return createFallbackItem(question, questionText, sourceNode);
  }
}

/**
 * Migrate multiple-choice question to Perseus radio widget.
 */
function migrateMultipleChoice(
  question: LegacyQuizQuestion,
  questionText: string,
  sourceNode: LegacyContentNode
): PerseusItem {
  const options = question.options ?? [];
  const correctIndex = typeof question.correctAnswer === 'number' ? question.correctAnswer : 0;

  const radioOptions: RadioWidgetOptions = {
    choices: options.map((opt, idx) => ({
      content: opt,
      correct: idx === correctIndex,
    })),
    randomize: true,
    multipleSelect: false,
    displayCount: null,
    hasNoneOfTheAbove: false,
    deselectEnabled: false,
  };

  const widgetId = 'radio 1';

  return {
    id: question.id,
    question: {
      content: `${questionText}\n\n[[☃ ${widgetId}]]`,
      images: {},
      widgets: {
        [widgetId]: {
          type: 'radio',
          options: radioOptions,
          graded: true,
          version: { major: 0, minor: 0 },
        },
      },
    },
    answerArea: {
      type: 'multiple',
      options: {
        content: '',
        images: {},
        widgets: {},
      },
      calculator: false,
    },
    hints: question.explanation
      ? [
          {
            content: question.explanation,
            images: {},
            widgets: {},
          },
        ]
      : [],
    metadata: {
      sourceContentId: sourceNode.id,
      assessesContentId: sourceNode.relatedNodeIds?.[0],
      bloomsLevel: 'remember' as MasteryLevel,
      difficulty: (sourceNode.metadata?.difficulty as string) ?? 'medium',
      tags: sourceNode.tags,
      createdAt: new Date().toISOString(),
    },
  };
}

/**
 * Migrate true/false question to Perseus radio widget with 2 options.
 */
function migrateTrueFalse(
  question: LegacyQuizQuestion,
  questionText: string,
  sourceNode: LegacyContentNode
): PerseusItem {
  const correctAnswer = question.correctAnswer === true;

  const radioOptions: RadioWidgetOptions = {
    choices: [
      { content: 'True', correct: correctAnswer },
      { content: 'False', correct: !correctAnswer },
    ],
    randomize: false,
    multipleSelect: false,
    displayCount: null,
    hasNoneOfTheAbove: false,
    deselectEnabled: false,
  };

  const widgetId = 'radio 1';

  return {
    id: question.id,
    question: {
      content: `${questionText}\n\n[[☃ ${widgetId}]]`,
      images: {},
      widgets: {
        [widgetId]: {
          type: 'radio',
          options: radioOptions,
          graded: true,
          version: { major: 0, minor: 0 },
        },
      },
    },
    answerArea: {
      type: 'multiple',
      options: {
        content: '',
        images: {},
        widgets: {},
      },
      calculator: false,
    },
    hints: question.explanation
      ? [
          {
            content: question.explanation,
            images: {},
            widgets: {},
          },
        ]
      : [],
    metadata: {
      sourceContentId: sourceNode.id,
      assessesContentId: sourceNode.relatedNodeIds?.[0],
      bloomsLevel: 'remember' as MasteryLevel,
      difficulty: 'easy',
      tags: sourceNode.tags,
      createdAt: new Date().toISOString(),
    },
  };
}

/**
 * Migrate short-answer question to Perseus input widget.
 */
function migrateShortAnswer(
  question: LegacyQuizQuestion,
  questionText: string,
  sourceNode: LegacyContentNode
): PerseusItem {
  const expectedAnswer = typeof question.correctAnswer === 'string' ? question.correctAnswer : '';

  const widgetId = 'input-number 1';

  // For text answers, we use a simple text input approach
  // Perseus doesn't have a direct text input, so we'll use expression for flexibility
  return {
    id: question.id,
    question: {
      content: `${questionText}\n\n**Your answer:** [[☃ ${widgetId}]]`,
      images: {},
      widgets: {
        [widgetId]: {
          type: 'input-number',
          options: {
            value: 0, // Placeholder - short answer needs manual grading
            simplify: 'required',
            size: 'normal',
            inexact: true,
            maxError: 0.1,
            answerType: 'number',
          },
          graded: false, // Short answer typically needs manual review
          version: { major: 0, minor: 0 },
        },
      },
    },
    answerArea: {
      type: 'multiple',
      options: {
        content: '',
        images: {},
        widgets: {},
      },
      calculator: false,
    },
    hints: question.explanation
      ? [
          {
            content: question.explanation,
            images: {},
            widgets: {},
          },
        ]
      : [],
    metadata: {
      sourceContentId: sourceNode.id,
      assessesContentId: sourceNode.relatedNodeIds?.[0],
      bloomsLevel: 'understand' as MasteryLevel,
      difficulty: 'medium',
      tags: sourceNode.tags,
      rubric: question.rubric,
      expectedAnswer,
      createdAt: new Date().toISOString(),
    },
  };
}

/**
 * Create fallback item for unsupported question types.
 */
function createFallbackItem(
  question: LegacyQuizQuestion,
  questionText: string,
  sourceNode: LegacyContentNode
): PerseusItem {
  return {
    id: question.id,
    question: {
      content: `**Question:** ${questionText}\n\n*This question type (${question.type}) requires manual conversion.*`,
      images: {},
      widgets: {},
    },
    answerArea: {
      type: 'multiple',
      options: {
        content: '',
        images: {},
        widgets: {},
      },
      calculator: false,
    },
    hints: [],
    metadata: {
      sourceContentId: sourceNode.id,
      assessesContentId: sourceNode.relatedNodeIds?.[0],
      bloomsLevel: 'remember' as MasteryLevel,
      difficulty: 'medium',
      tags: sourceNode.tags,
      migrationWarning: `Unsupported question type: ${question.type}`,
      createdAt: new Date().toISOString(),
    },
  };
}

// ─────────────────────────────────────────────────────────────────────────────
// Batch Migration
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Migration result for a single quiz.
 */
export interface QuizMigrationResult {
  sourceId: string;
  sourceTitle: string;
  questionsConverted: number;
  warnings: string[];
  perseusItems: PerseusItem[];
}

/**
 * Migrate multiple quiz nodes to Perseus format.
 */
export function migrateQuizzes(nodes: LegacyContentNode[]): QuizMigrationResult[] {
  return nodes.map(node => {
    const warnings: string[] = [];
    const perseusItems = migrateQuizToPerseus(node);

    // Check for issues
    for (const item of perseusItems) {
      if (item.metadata?.migrationWarning) {
        warnings.push(item.metadata.migrationWarning as string);
      }
    }

    return {
      sourceId: node.id,
      sourceTitle: node.title,
      questionsConverted: perseusItems.length,
      warnings,
      perseusItems,
    };
  });
}

/**
 * Create a Perseus question pool from migrated items.
 */
export function createQuestionPool(
  results: QuizMigrationResult[],
  poolId: string,
  poolTitle: string
): {
  id: string;
  title: string;
  questions: PerseusItem[];
  sourceQuizIds: string[];
  totalQuestions: number;
} {
  const allQuestions: PerseusItem[] = [];
  const sourceIds: string[] = [];

  for (const result of results) {
    allQuestions.push(...result.perseusItems);
    sourceIds.push(result.sourceId);
  }

  return {
    id: poolId,
    title: poolTitle,
    questions: allQuestions,
    sourceQuizIds: sourceIds,
    totalQuestions: allQuestions.length,
  };
}
