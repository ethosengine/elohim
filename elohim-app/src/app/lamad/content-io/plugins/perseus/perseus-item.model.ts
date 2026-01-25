/**
 * Perseus Item Model - Question types and structures for Khan Academy Perseus integration.
 *
 * Based on the Perseus item JSON format used by Khan Academy for their
 * exercise system. Each item represents a single question with widgets
 * for interactive elements (radio buttons, graphs, expressions, etc.).
 *
 * @see https://github.com/Khan/perseus
 */

// ─────────────────────────────────────────────────────────────────────────────
// Core Perseus Item Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Perseus Item - Standard format for all quiz questions.
 *
 * This is the top-level structure that contains the question content,
 * widgets, hints, and metadata for assessment.
 */
export interface PerseusItem {
  /** Unique question identifier */
  id: string;

  /** Perseus item version (for format compatibility) */
  version: PerseusItemVersion;

  /** The question content with Perseus widgets */
  question: PerseusItemData;

  /** Answer area configuration (optional) */
  answerArea?: PerseusAnswerArea;

  /** Hints for progressive disclosure */
  hints?: PerseusHint[];

  /** Assessment metadata for lamad integration */
  metadata: PerseusItemMetadata;

  /**
   * Discovery mode flag for self-assessment quizzes.
   * When true, answers are not graded as correct/incorrect.
   * Instead, each answer contributes to subscale scores
   * for personalization and path recommendation.
   */
  discoveryMode?: boolean;
}

/**
 * Version information for Perseus item format compatibility.
 */
export interface PerseusItemVersion {
  /** Major version - breaking changes */
  major: number;
  /** Minor version - backward-compatible changes */
  minor: number;
}

/**
 * The main question content including rich text and widgets.
 */
export interface PerseusItemData {
  /** Rich content with widget placeholders (e.g., "What is $2 + 2$? [[☃ radio 1]]") */
  content: string;

  /** Widget definitions referenced in content by placeholder */
  widgets: Record<string, PerseusWidget>;

  /** Images referenced in content */
  images: Record<string, PerseusImage>;
}

/**
 * Answer area configuration for multi-part questions.
 */
export interface PerseusAnswerArea {
  /** Type of answer area */
  type?: 'multiple' | 'table';

  /** Options for the answer area */
  options?: PerseusAnswerAreaOptions;

  /** Calculator visibility */
  calculator?: boolean;

  /** Periodic table visibility */
  periodicTable?: boolean;

  /** Z table visibility (statistics) */
  zTable?: boolean;

  /** T table visibility (statistics) */
  tTable?: boolean;

  /** Chi squared table visibility */
  chi2Table?: boolean;
}

export interface PerseusAnswerAreaOptions {
  /** Number of inputs for table type */
  numRows?: number;
  numColumns?: number;
}

/**
 * Hint for progressive disclosure during quiz taking.
 */
export interface PerseusHint {
  /** Hint content (supports markdown and widgets) */
  content: string;

  /** Widgets used in the hint */
  widgets: Record<string, PerseusWidget>;

  /** Images used in the hint */
  images: Record<string, PerseusImage>;

  /** Whether this hint replaces previous hints */
  replace?: boolean;
}

// ─────────────────────────────────────────────────────────────────────────────
// Widget Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * All available Perseus widget types.
 *
 * Widgets are interactive elements that can be embedded in question content.
 */
export type PerseusWidgetType =
  | 'radio' // Multiple choice (single select)
  | 'numeric-input' // Number entry with validation
  | 'expression' // Mathematical expression input
  | 'input-number' // Simple number input
  | 'interactive-graph' // Graphing and geometry
  | 'image' // Image with optional interaction
  | 'transformer' // Geometric transformations
  | 'number-line' // Number line interactions
  | 'sorter' // Ordering/sorting items
  | 'categorizer' // Categorize items into groups
  | 'matcher' // Match items between columns
  | 'orderer' // Put items in order
  | 'graded-group' // Group of graded questions
  | 'graded-group-set' // Set of graded groups
  | 'iframe' // Custom interactive via iframe
  | 'definition' // Term definition tooltip
  | 'dropdown' // Dropdown selection
  | 'explanation' // Expandable explanation
  | 'passage' // Reading passage
  | 'passage-ref' // Reference to passage
  | 'phet-simulation' // PhET interactive simulation
  | 'plotter' // Bar chart/histogram plotter
  | 'table' // Data table
  | 'grapher' // Function graphing
  | 'measurer' // Measurement tool
  | 'matrix' // Matrix input
  | 'cs-program' // Computer science code
  | 'video' // Embedded video
  | 'label-image'; // Image with labels

/**
 * Base widget structure. Each widget type has specific options.
 */
export interface PerseusWidget {
  /** Widget type identifier */
  type: PerseusWidgetType;

  /** Widget-specific options */
  options: PerseusWidgetOptions;

  /** Whether this widget is graded */
  graded?: boolean;

  /** Widget alignment in content */
  alignment?: 'default' | 'block' | 'inline' | 'full-width';

  /** Whether widget is static (non-interactive) */
  static?: boolean;

  /** Widget version */
  version?: PerseusItemVersion;
}

/**
 * Union type for all widget option types.
 */
export type PerseusWidgetOptions =
  | RadioWidgetOptions
  | NumericInputWidgetOptions
  | ExpressionWidgetOptions
  | InteractiveGraphWidgetOptions
  | SorterWidgetOptions
  | MatcherWidgetOptions
  | DropdownWidgetOptions
  | ImageWidgetOptions
  | Record<string, unknown>; // Fallback for less common widgets

// ─────────────────────────────────────────────────────────────────────────────
// Widget-Specific Options
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Radio (multiple choice) widget options.
 */
export interface RadioWidgetOptions {
  /** Available choices */
  choices: RadioChoice[];

  /** Randomize choice order on display */
  randomize?: boolean;

  /** Allow multiple selections */
  multipleSelect?: boolean;

  /** Display choices inline */
  displayCount?: number;

  /** Don't randomize "None of the above" */
  noneOfTheAbove?: boolean;

  /** Has "None of the above" option */
  hasNoneOfTheAbove?: boolean;

  /** Choices per row in multi-column layout */
  countChoices?: boolean;
}

export interface RadioChoice {
  /** Choice content (supports markdown/math) */
  content: string;

  /** Whether this is the correct answer (for graded quizzes) */
  correct?: boolean;

  /** Clue shown after answering */
  clue?: string;

  /** Whether this is "None of the above" */
  isNoneOfTheAbove?: boolean;

  /** Widget references in choice content */
  widgets?: Record<string, PerseusWidget>;

  /**
   * Subscale contributions for discovery assessments.
   * Maps subscale names to contribution values (typically 0-1).
   * Used when discoveryMode is true at the item level.
   *
   * @example
   * { "governance": 1, "care": 0, "economic": 0 }
   */
  subscaleContributions?: Record<string, number>;
}

/**
 * Numeric input widget options.
 */
export interface NumericInputWidgetOptions {
  /** Accepted answers */
  answers: NumericAnswer[];

  /** Label text for the input */
  labelText?: string;

  /** Size of input field */
  size?: 'normal' | 'small';

  /** Whether to use a coefficient for the answer */
  coefficient?: boolean;

  /** Whether answer is a vector */
  answerType?:
    | 'number'
    | 'decimal'
    | 'integer'
    | 'rational'
    | 'improper'
    | 'mixed'
    | 'percent'
    | 'pi';

  /** Right-hand side content */
  rightAlign?: boolean;
}

export interface NumericAnswer {
  /** The correct value */
  value: number;

  /** Answer status */
  status: 'correct' | 'wrong';

  /** Error message for this answer */
  message?: string;

  /** Whether this is the simplest form */
  simplify?: 'required' | 'optional' | 'enforced';

  /** Whether to use strict equivalence */
  strict?: boolean;

  /** Maximum allowed error (for decimals) */
  maxError?: number;
}

/**
 * Expression (math input) widget options.
 */
export interface ExpressionWidgetOptions {
  /** Answer forms that are correct */
  answerForms: ExpressionAnswerForm[];

  /** Button sets to show */
  buttonSets?: (
    | 'basic'
    | 'basic+div'
    | 'trig'
    | 'prealgebra'
    | 'logarithms'
    | 'basic relations'
    | 'advanced relations'
  )[];

  /** Available functions */
  functions?: string[];

  /** Multiplication sign style */
  times?: boolean;

  /** Number of visible lines */
  visibleLabel?: string;
}

export interface ExpressionAnswerForm {
  /** The expression */
  value: string;

  /** Form type */
  form?: boolean;

  /** Simplification level */
  simplify?: boolean;

  /** Whether this is considered */
  considered?: 'correct' | 'wrong' | 'ungraded';

  /** Key for identification */
  key?: string;
}

/**
 * Interactive graph widget options.
 */
export interface InteractiveGraphWidgetOptions {
  /** Graph type */
  graph: {
    type:
      | 'linear'
      | 'quadratic'
      | 'sinusoid'
      | 'circle'
      | 'point'
      | 'polygon'
      | 'segment'
      | 'ray'
      | 'linear-system'
      | 'angle'
      | 'none';
    numPoints?: number;
    coords?: [number, number][];
    startCoords?: [number, number][];
  };

  /** Axis range */
  range: [[number, number], [number, number]];

  /** Grid step */
  step: [number, number];

  /** Snap step */
  snapStep?: [number, number];

  /** Background image */
  backgroundImage?: PerseusImage;

  /** Graph markings */
  markings?: 'graph' | 'grid' | 'none';

  /** Show protractor */
  showProtractor?: boolean;

  /** Show ruler */
  showRuler?: boolean;

  /** Show tooltips */
  showTooltips?: boolean;

  /** Correct answer */
  correct?: {
    type: string;
    coords?: [number, number][];
    numPoints?: number;
  };
}

/**
 * Sorter widget options.
 */
export interface SorterWidgetOptions {
  /** Items to sort in correct order */
  correct: string[];

  /** Layout direction */
  layout?: 'horizontal' | 'vertical';

  /** Padding between items */
  padding?: boolean;
}

/**
 * Matcher widget options.
 */
export interface MatcherWidgetOptions {
  /** Left column items */
  left: string[];

  /** Right column items */
  right: string[];

  /** Labels for columns */
  labels?: [string, string];

  /** Order sensitivity */
  orderMatters?: boolean;

  /** Padding */
  padding?: boolean;
}

/**
 * Dropdown widget options.
 */
export interface DropdownWidgetOptions {
  /** Placeholder text */
  placeholder?: string;

  /** Available choices */
  choices: DropdownChoice[];
}

export interface DropdownChoice {
  /** Choice content */
  content: string;

  /** Whether correct */
  correct?: boolean;
}

/**
 * Image widget options.
 */
export interface ImageWidgetOptions {
  /** Image URL */
  backgroundImage: PerseusImage;

  /** Labels on image */
  labels?: ImageLabel[];

  /** Alt text */
  alt?: string;

  /** Caption */
  caption?: string;

  /** Title */
  title?: string;
}

export interface ImageLabel {
  /** Label content */
  content: string;

  /** Label position */
  coordinates: [number, number];

  /** Alignment */
  alignment?: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Supporting Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Image reference in Perseus content.
 */
export interface PerseusImage {
  /** Image URL */
  url: string;

  /** Image width */
  width?: number;

  /** Image height */
  height?: number;

  /** Alt text for accessibility */
  alt?: string;

  /** Image title/caption */
  title?: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Lamad Integration Metadata
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Metadata for lamad system integration.
 *
 * Extends Perseus items with information needed for mastery tracking,
 * path integration, and content graph relationships.
 */
export interface PerseusItemMetadata {
  /** Content node ID this question assesses */
  assessesContentId: string;

  /** Bloom's taxonomy level this question targets */
  bloomsLevel: BloomsLevel;

  /** Question difficulty */
  difficulty: QuestionDifficulty;

  /** Estimated time to answer in seconds */
  estimatedTimeSeconds: number;

  /** Tags for filtering and categorization */
  tags: string[];

  /** Question type for analytics */
  questionType: QuestionType;

  /** Source document this was derived from */
  sourceDoc?: string;

  /** Creation timestamp */
  createdAt?: string;

  /** Last update timestamp */
  updatedAt?: string;

  /** Author information */
  author?: string;

  // ─────────────────────────────────────────────────────────────────────────
  // Migration fields (used during quiz migration from legacy formats)
  // ─────────────────────────────────────────────────────────────────────────

  /** Original content node ID (for migration tracking) */
  sourceContentId?: string;

  /** Warning message if migration had issues */
  migrationWarning?: string;
}

/**
 * Bloom's taxonomy levels for question targeting.
 */
export type BloomsLevel =
  | 'remember' // Recall facts and basic concepts
  | 'understand' // Explain ideas or concepts
  | 'apply' // Use information in new situations
  | 'analyze' // Draw connections among ideas
  | 'evaluate' // Justify a decision or course of action
  | 'create'; // Produce new or original work

/**
 * Question difficulty levels.
 */
export type QuestionDifficulty = 'easy' | 'medium' | 'hard';

/**
 * Question types for path integration.
 */
export type QuestionType =
  | 'core' // Essential understanding check
  | 'applied' // Practical application
  | 'synthesis'; // Combining multiple concepts

// ─────────────────────────────────────────────────────────────────────────────
// Score Result Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Result from scoring a Perseus item.
 */
export interface PerseusScoreResult {
  /** Whether the answer is correct */
  correct: boolean;

  /** Score from 0 to 1 (for partial credit) */
  score: number;

  /** User's answer (widget-specific format) */
  guess: unknown;

  /** Empty answer indicator */
  empty: boolean;

  /** Feedback message */
  message?: string;

  /** Per-widget scoring breakdown */
  widgetScores?: Record<string, WidgetScore>;
}

/**
 * Score for an individual widget.
 */
export interface WidgetScore {
  /** Widget type */
  type: PerseusWidgetType;

  /** Whether correct */
  correct: boolean;

  /** Score value */
  score: number;

  /** Empty indicator */
  empty: boolean;

  /** Error message if any */
  message?: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Factory Functions
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Create a new Perseus item with default structure.
 */
export function createPerseusItem(
  id: string,
  metadata: Partial<PerseusItemMetadata> = {}
): PerseusItem {
  return {
    id,
    version: { major: 1, minor: 0 },
    question: {
      content: '',
      widgets: {},
      images: {},
    },
    hints: [],
    metadata: {
      assessesContentId: metadata.assessesContentId ?? '',
      bloomsLevel: metadata.bloomsLevel ?? 'understand',
      difficulty: metadata.difficulty ?? 'medium',
      estimatedTimeSeconds: metadata.estimatedTimeSeconds ?? 60,
      tags: metadata.tags ?? [],
      questionType: metadata.questionType ?? 'core',
      ...metadata,
    },
  };
}

/**
 * Create a multiple choice question.
 */
export function createRadioQuestion(
  id: string,
  questionText: string,
  choices: { text: string; correct?: boolean }[],
  metadata: Partial<PerseusItemMetadata> = {}
): PerseusItem {
  const item = createPerseusItem(id, metadata);

  item.question.content = `${questionText}\n\n[[☃ radio 1]]`;
  item.question.widgets = {
    'radio 1': {
      type: 'radio',
      options: {
        choices: choices.map(c => ({
          content: c.text,
          correct: c.correct ?? false,
        })),
        randomize: true,
        multipleSelect: false,
      },
    },
  };

  return item;
}

/**
 * Create a numeric input question.
 */
export function createNumericQuestion(
  id: string,
  questionText: string,
  correctAnswer: number,
  metadata: Partial<PerseusItemMetadata> = {}
): PerseusItem {
  const item = createPerseusItem(id, metadata);

  item.question.content = `${questionText}\n\n[[☃ numeric-input 1]]`;
  item.question.widgets = {
    'numeric-input 1': {
      type: 'numeric-input',
      options: {
        answers: [
          {
            value: correctAnswer,
            status: 'correct',
            simplify: 'optional',
          },
        ],
        size: 'normal',
      },
    },
  };

  return item;
}
