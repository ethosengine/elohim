/**
 * Assessment Tools
 *
 * Tools for creating quizzes and assessments from content.
 */

import * as fs from 'fs/promises';
import * as path from 'path';
import { glob } from 'glob';
import {
  assessmentSchema,
  questionSchema,
  conceptSchema,
  type Assessment,
  type Question,
  type Concept,
} from '../schemas/index.js';

const ASSESSMENTS_DIR = 'assessments';
const CONCEPTS_DIR = 'content';

interface QuizInput {
  id: string;
  title: string;
  conceptIds: string[];
  questionCount?: number;
}

interface AssessmentInput {
  id: string;
  title: string;
  type: 'diagnostic' | 'formative' | 'summative';
  conceptIds?: string[];
  description?: string;
  passingScore?: number;
  timeLimit?: number;
}

interface QuestionInput {
  id: string;
  type: 'multiple-choice' | 'true-false' | 'short-answer' | 'essay' | 'matching';
  question: string;
  options?: string[];
  correctAnswer?: string | string[];
  explanation?: string;
  conceptId?: string;
  difficulty?: 'easy' | 'medium' | 'hard';
  points?: number;
}

/**
 * Load all concepts from disk
 */
async function loadAllConcepts(dataDir: string): Promise<Map<string, Concept>> {
  const conceptsPath = path.join(dataDir, CONCEPTS_DIR);
  const concepts = new Map<string, Concept>();

  try {
    const files = await glob('**/*.json', { cwd: conceptsPath, nodir: true });

    for (const file of files) {
      try {
        const fullPath = path.join(conceptsPath, file);
        const content = await fs.readFile(fullPath, 'utf-8');
        const data = JSON.parse(content);
        const parsed = conceptSchema.safeParse(data);
        if (parsed.success) {
          concepts.set(parsed.data.id, parsed.data);
        }
      } catch {
        // Skip invalid files
      }
    }
  } catch {
    // Directory doesn't exist yet
  }

  return concepts;
}

/**
 * Load an assessment from disk
 */
async function loadAssessment(dataDir: string, assessmentId: string): Promise<Assessment> {
  const assessmentPath = path.join(dataDir, ASSESSMENTS_DIR, `${assessmentId}.json`);

  try {
    const content = await fs.readFile(assessmentPath, 'utf-8');
    return JSON.parse(content);
  } catch {
    throw new Error(`Assessment not found: ${assessmentId}`);
  }
}

/**
 * Save an assessment to disk
 */
async function saveAssessment(dataDir: string, assessment: Assessment): Promise<string> {
  const relativePath = path.join(ASSESSMENTS_DIR, `${assessment.id}.json`);
  const fullPath = path.join(dataDir, relativePath);

  await fs.mkdir(path.dirname(fullPath), { recursive: true });
  await fs.writeFile(fullPath, JSON.stringify(assessment, null, 2), 'utf-8');

  return relativePath;
}

/**
 * Create a quiz from concepts
 *
 * This creates a scaffold with placeholder questions that can be refined
 * by the LLM in subsequent calls.
 */
export async function createQuiz(
  dataDir: string,
  input: QuizInput
): Promise<{ success: boolean; path: string; assessment: Assessment }> {
  const concepts = await loadAllConcepts(dataDir);

  // Gather concepts for the quiz
  const quizConcepts: Concept[] = [];
  for (const id of input.conceptIds) {
    const concept = concepts.get(id);
    if (concept) {
      quizConcepts.push(concept);
    }
  }

  if (quizConcepts.length === 0) {
    throw new Error('No valid concepts found for quiz');
  }

  // Generate placeholder questions
  const questionCount = input.questionCount || Math.min(quizConcepts.length * 2, 10);
  const questions: Question[] = [];

  for (let i = 0; i < questionCount && i < quizConcepts.length; i++) {
    const concept = quizConcepts[i % quizConcepts.length];

    // Create a placeholder question
    questions.push({
      id: `q-${input.id}-${i + 1}`,
      type: 'multiple-choice',
      question: `Question about: ${concept.title}`,
      options: ['Option A', 'Option B', 'Option C', 'Option D'],
      correctAnswer: 'Option A',
      explanation: `This relates to the concept "${concept.title}"`,
      conceptId: concept.id,
      difficulty: 'medium',
      points: 1,
    });
  }

  const assessment: Assessment = {
    id: input.id,
    title: input.title,
    type: 'quiz',
    questions,
    conceptIds: input.conceptIds,
    passingScore: 70,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };

  // Validate
  const result = assessmentSchema.safeParse(assessment);
  if (!result.success) {
    throw new Error(`Invalid assessment: ${result.error.errors.map(e => e.message).join(', ')}`);
  }

  const relativePath = await saveAssessment(dataDir, assessment);

  return { success: true, path: relativePath, assessment };
}

/**
 * Create a more comprehensive assessment
 */
export async function createAssessment(
  dataDir: string,
  input: AssessmentInput
): Promise<{ success: boolean; path: string; assessment: Assessment }> {
  const assessment: Assessment = {
    id: input.id,
    title: input.title,
    description: input.description,
    type: input.type,
    questions: [],
    conceptIds: input.conceptIds || [],
    passingScore: input.passingScore,
    timeLimit: input.timeLimit,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };

  // Validate
  const result = assessmentSchema.safeParse(assessment);
  if (!result.success) {
    throw new Error(`Invalid assessment: ${result.error.errors.map(e => e.message).join(', ')}`);
  }

  const relativePath = await saveAssessment(dataDir, assessment);

  return { success: true, path: relativePath, assessment };
}

/**
 * Update an existing assessment
 */
export async function updateAssessment(
  dataDir: string,
  id: string,
  updates: Partial<AssessmentInput> & { questions?: QuestionInput[] }
): Promise<{ success: boolean; assessment: Assessment }> {
  const assessment = await loadAssessment(dataDir, id);

  // Apply updates
  if (updates.title !== undefined) assessment.title = updates.title;
  if (updates.description !== undefined) assessment.description = updates.description;
  if (updates.type !== undefined) assessment.type = updates.type;
  if (updates.conceptIds !== undefined) assessment.conceptIds = updates.conceptIds;
  if (updates.passingScore !== undefined) assessment.passingScore = updates.passingScore;
  if (updates.timeLimit !== undefined) assessment.timeLimit = updates.timeLimit;

  // Update questions if provided
  if (updates.questions) {
    const validatedQuestions: Question[] = [];
    for (const q of updates.questions) {
      const result = questionSchema.safeParse(q);
      if (result.success) {
        validatedQuestions.push(result.data);
      } else {
        throw new Error(`Invalid question ${q.id}: ${result.error.errors.map(e => e.message).join(', ')}`);
      }
    }
    assessment.questions = validatedQuestions;
  }

  assessment.updatedAt = new Date().toISOString();

  // Validate full assessment
  const result = assessmentSchema.safeParse(assessment);
  if (!result.success) {
    throw new Error(`Invalid assessment: ${result.error.errors.map(e => e.message).join(', ')}`);
  }

  await saveAssessment(dataDir, assessment);

  return { success: true, assessment };
}

/**
 * Add a question to an assessment
 */
export async function addQuestion(
  dataDir: string,
  assessmentId: string,
  question: QuestionInput
): Promise<{ success: boolean; assessment: Assessment }> {
  const assessment = await loadAssessment(dataDir, assessmentId);

  // Validate question
  const result = questionSchema.safeParse(question);
  if (!result.success) {
    throw new Error(`Invalid question: ${result.error.errors.map(e => e.message).join(', ')}`);
  }

  assessment.questions.push(result.data);
  assessment.updatedAt = new Date().toISOString();

  await saveAssessment(dataDir, assessment);

  return { success: true, assessment };
}

/**
 * Remove a question from an assessment
 */
export async function removeQuestion(
  dataDir: string,
  assessmentId: string,
  questionId: string
): Promise<{ success: boolean; assessment: Assessment }> {
  const assessment = await loadAssessment(dataDir, assessmentId);

  assessment.questions = assessment.questions.filter(q => q.id !== questionId);
  assessment.updatedAt = new Date().toISOString();

  await saveAssessment(dataDir, assessment);

  return { success: true, assessment };
}
