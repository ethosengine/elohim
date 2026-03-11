#!/usr/bin/env npx tsx
/**
 * Quiz Migration Script
 *
 * Converts existing quiz-json files to Perseus format for the new quiz engine.
 *
 * Usage:
 *   npx tsx scripts/migrate-quizzes-to-perseus.ts
 *   npx tsx scripts/migrate-quizzes-to-perseus.ts --dry-run
 *   npx tsx scripts/migrate-quizzes-to-perseus.ts --source /path/to/quizzes
 */

import * as fs from 'fs';
import * as path from 'path';

// ─────────────────────────────────────────────────────────────────────────────
// Types (inline to avoid import issues with tsx)
// ─────────────────────────────────────────────────────────────────────────────

interface LegacyQuizQuestion {
  id: string;
  type: 'multiple-choice' | 'true-false' | 'short-answer' | 'connection';
  question: string;
  text?: string;
  options?: string[];
  correctAnswer?: number | string | boolean;
  rubric?: string;
  explanation?: string;
}

interface LegacyQuizContent {
  passingScore: number;
  allowRetake: boolean;
  showCorrectAnswers: boolean;
  questions: LegacyQuizQuestion[];
}

interface LegacyContentNode {
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

interface PerseusItem {
  id: string;
  question: {
    content: string;
    images: Record<string, unknown>;
    widgets: Record<string, unknown>;
  };
  answerArea: {
    type: string;
    options: {
      content: string;
      images: Record<string, unknown>;
      widgets: Record<string, unknown>;
    };
    calculator: boolean;
  };
  hints: Array<{
    content: string;
    images: Record<string, unknown>;
    widgets: Record<string, unknown>;
  }>;
  metadata?: Record<string, unknown>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Migration Logic
// ─────────────────────────────────────────────────────────────────────────────

function migrateMultipleChoice(
  question: LegacyQuizQuestion,
  sourceNode: LegacyContentNode
): PerseusItem {
  const options = question.options || [];
  const correctIndex = typeof question.correctAnswer === 'number'
    ? question.correctAnswer
    : 0;

  const widgetId = 'radio 1';
  const questionText = question.question || question.text || '';

  return {
    id: question.id,
    question: {
      content: `${questionText}\n\n[[☃ ${widgetId}]]`,
      images: {},
      widgets: {
        [widgetId]: {
          type: 'radio',
          options: {
            choices: options.map((opt, idx) => ({
              content: opt,
              correct: idx === correctIndex
            })),
            randomize: true,
            multipleSelect: false,
            displayCount: null,
            hasNoneOfTheAbove: false,
            deselectEnabled: false
          },
          graded: true,
          version: { major: 0, minor: 0 }
        }
      }
    },
    answerArea: {
      type: 'multiple',
      options: { content: '', images: {}, widgets: {} },
      calculator: false
    },
    hints: question.explanation ? [{
      content: question.explanation,
      images: {},
      widgets: {}
    }] : [],
    metadata: {
      sourceContentId: sourceNode.id,
      assessesContentId: sourceNode.relatedNodeIds?.[0],
      bloomsLevel: 'remember',
      difficulty: (sourceNode.metadata?.difficulty as string) ?? 'medium',
      tags: sourceNode.tags,
      createdAt: new Date().toISOString()
    }
  };
}

function migrateTrueFalse(
  question: LegacyQuizQuestion,
  sourceNode: LegacyContentNode
): PerseusItem {
  const correctAnswer = question.correctAnswer === true;
  const widgetId = 'radio 1';
  const questionText = question.question || question.text || '';

  return {
    id: question.id,
    question: {
      content: `${questionText}\n\n[[☃ ${widgetId}]]`,
      images: {},
      widgets: {
        [widgetId]: {
          type: 'radio',
          options: {
            choices: [
              { content: 'True', correct: correctAnswer },
              { content: 'False', correct: !correctAnswer }
            ],
            randomize: false,
            multipleSelect: false,
            displayCount: null,
            hasNoneOfTheAbove: false,
            deselectEnabled: false
          },
          graded: true,
          version: { major: 0, minor: 0 }
        }
      }
    },
    answerArea: {
      type: 'multiple',
      options: { content: '', images: {}, widgets: {} },
      calculator: false
    },
    hints: question.explanation ? [{
      content: question.explanation,
      images: {},
      widgets: {}
    }] : [],
    metadata: {
      sourceContentId: sourceNode.id,
      assessesContentId: sourceNode.relatedNodeIds?.[0],
      bloomsLevel: 'remember',
      difficulty: 'easy',
      tags: sourceNode.tags,
      createdAt: new Date().toISOString()
    }
  };
}

function migrateQuestion(
  question: LegacyQuizQuestion,
  sourceNode: LegacyContentNode
): PerseusItem {
  switch (question.type) {
    case 'multiple-choice':
      return migrateMultipleChoice(question, sourceNode);
    case 'true-false':
      return migrateTrueFalse(question, sourceNode);
    default:
      // Fallback
      return {
        id: question.id,
        question: {
          content: `**${question.question || question.text}**\n\n*Unsupported type: ${question.type}*`,
          images: {},
          widgets: {}
        },
        answerArea: {
          type: 'multiple',
          options: { content: '', images: {}, widgets: {} },
          calculator: false
        },
        hints: [],
        metadata: {
          sourceContentId: sourceNode.id,
          migrationWarning: `Unsupported question type: ${question.type}`
        }
      };
  }
}

function migrateQuizNode(node: LegacyContentNode): PerseusItem[] {
  let content = node.content;

  if (typeof content === 'string') {
    try {
      content = JSON.parse(content);
    } catch (e) {
      console.error(`Failed to parse ${node.id}:`, e);
      return [];
    }
  }

  const quiz = content as LegacyQuizContent;
  if (!quiz.questions || !Array.isArray(quiz.questions)) {
    console.warn(`No questions in ${node.id}`);
    return [];
  }

  return quiz.questions.map(q => migrateQuestion(q, node));
}

// ─────────────────────────────────────────────────────────────────────────────
// Main Script
// ─────────────────────────────────────────────────────────────────────────────

function findQuizFiles(dir: string): string[] {
  const files: string[] = [];

  if (!fs.existsSync(dir)) {
    return files;
  }

  const entries = fs.readdirSync(dir, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);

    if (entry.isDirectory()) {
      files.push(...findQuizFiles(fullPath));
    } else if (entry.name.endsWith('.json')) {
      // Check if it's a quiz file
      try {
        const content = JSON.parse(fs.readFileSync(fullPath, 'utf-8'));
        if (content.contentFormat === 'quiz-json') {
          files.push(fullPath);
        }
      } catch {
        // Ignore non-JSON or malformed files
      }
    }
  }

  return files;
}

async function main() {
  const args = process.argv.slice(2);
  const dryRun = args.includes('--dry-run');
  const sourceIdx = args.indexOf('--source');
  const sourceDir = sourceIdx >= 0 && args[sourceIdx + 1]
    ? args[sourceIdx + 1]
    : path.resolve(__dirname, '../../data/lamad/content');

  console.log('╔══════════════════════════════════════════════════════════════╗');
  console.log('║           Quiz Migration: Legacy → Perseus                   ║');
  console.log('╚══════════════════════════════════════════════════════════════╝\n');

  console.log(`Source directory: ${sourceDir}`);
  console.log(`Mode: ${dryRun ? 'DRY RUN' : 'LIVE'}\n`);

  // Find quiz files
  const quizFiles = findQuizFiles(sourceDir);
  console.log(`Found ${quizFiles.length} quiz-json files\n`);

  if (quizFiles.length === 0) {
    console.log('No quiz files to migrate.');
    return;
  }

  // Output directory for Perseus files
  const outputDir = path.resolve(__dirname, '../../data/lamad/perseus');
  if (!dryRun && !fs.existsSync(outputDir)) {
    fs.mkdirSync(outputDir, { recursive: true });
  }

  let totalQuestions = 0;
  let totalWarnings = 0;

  for (const file of quizFiles) {
    const relativePath = path.relative(sourceDir, file);
    console.log(`\n📄 ${relativePath}`);

    try {
      const node = JSON.parse(fs.readFileSync(file, 'utf-8')) as LegacyContentNode;
      const perseusItems = migrateQuizNode(node);

      console.log(`   ✓ Converted ${perseusItems.length} questions`);
      totalQuestions += perseusItems.length;

      // Check for warnings
      const warnings = perseusItems.filter(item => item.metadata?.migrationWarning);
      if (warnings.length > 0) {
        console.log(`   ⚠ ${warnings.length} warning(s)`);
        totalWarnings += warnings.length;
      }

      // Write output
      if (!dryRun) {
        const outputFile = path.join(outputDir, `${node.id}.perseus.json`);
        fs.writeFileSync(outputFile, JSON.stringify({
          id: node.id,
          title: node.title,
          description: node.description,
          sourceFormat: 'quiz-json',
          migratedAt: new Date().toISOString(),
          questions: perseusItems
        }, null, 2));
        console.log(`   📝 Written to: ${path.relative(process.cwd(), outputFile)}`);
      }
    } catch (e) {
      console.error(`   ✗ Error: ${e}`);
    }
  }

  console.log('\n────────────────────────────────────────────────────────────────');
  console.log(`Total: ${quizFiles.length} files, ${totalQuestions} questions, ${totalWarnings} warnings`);

  if (dryRun) {
    console.log('\n💡 Run without --dry-run to write Perseus files');
  } else {
    console.log(`\n✅ Perseus files written to: ${outputDir}`);
  }
}

main().catch(console.error);
