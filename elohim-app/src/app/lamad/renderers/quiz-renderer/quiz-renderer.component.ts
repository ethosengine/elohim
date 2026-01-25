import { CommonModule } from '@angular/common';
import { Component, Input, Output, EventEmitter, OnChanges } from '@angular/core';
import { FormsModule } from '@angular/forms';

import { ContentNode } from '../../models/content-node.model';
import { InteractiveRenderer, RendererCompletionEvent } from '../renderer-registry.service';

interface QuizQuestion {
  id: string;
  type: 'multiple-choice' | 'true-false' | 'short-answer' | 'connection';
  question: string;
  options?: string[]; // For multiple-choice
  correctAnswer?: number | string | boolean;
  rubric?: string; // For manually graded
  explanation?: string; // Shown after answer
}

interface QuizContent {
  passingScore: number; // 0-100
  allowRetake: boolean;
  showCorrectAnswers: boolean;
  questions: QuizQuestion[];
}

@Component({
  selector: 'app-quiz-renderer',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './quiz-renderer.component.html',
  styleUrls: ['./quiz-renderer.component.css'],
})
export class QuizRendererComponent implements OnChanges, InteractiveRenderer {
  @Input() node!: ContentNode;

  /** Standard completion event for affinity tracking integration */
  @Output() complete = new EventEmitter<RendererCompletionEvent>();

  /** Legacy event for backwards compatibility */
  @Output() quizComplete = new EventEmitter<{ passed: boolean; score: number }>();

  quiz: QuizContent | null = null;
  answers = new Map<string, any>();
  submitted = false;
  score = 0;
  passed = false;

  ngOnChanges(): void {
    // Reset state when node changes
    this.quiz = null;
    this.answers.clear();
    this.submitted = false;
    this.score = 0;
    this.passed = false;

    if (!this.node) return;

    // Handle quiz content - could be quiz-json format or assessment type
    if (this.node.contentFormat === 'quiz-json' || this.node.contentType === 'assessment') {
      let content = this.node.content;

      // Parse if it's a string
      if (typeof content === 'string') {
        try {
          content = JSON.parse(content);
        } catch (e) {
          console.error('[QuizRenderer] Failed to parse quiz content:', e);
          return;
        }
      }

      // Validate quiz structure
      const quizData = content as QuizContent;
      if (quizData && Array.isArray(quizData.questions)) {
        this.quiz = quizData;
      } else {
        console.warn('[QuizRenderer] Invalid quiz structure - missing questions array:', content);
      }
    }
  }

  selectAnswer(questionId: string, answer: any): void {
    if (this.submitted) return;
    this.answers.set(questionId, answer);
  }

  submitQuiz(): void {
    if (!this.quiz) return;

    let correct = 0;
    let gradeable = 0;

    for (const q of this.quiz.questions) {
      if (q.type === 'multiple-choice' && q.correctAnswer !== undefined) {
        gradeable++;
        if (this.answers.get(q.id) === q.correctAnswer) {
          correct++;
        }
      }
    }

    this.score = gradeable > 0 ? Math.round((correct / gradeable) * 100) : 0;
    this.passed = this.score >= this.quiz.passingScore;
    this.submitted = true;

    // Emit standardized completion event for affinity tracking
    this.complete.emit({
      type: 'quiz',
      passed: this.passed,
      score: this.score,
      details: {
        correct,
        total: gradeable,
        passingScore: this.quiz.passingScore,
      },
    });

    // Also emit legacy event for backwards compatibility
    this.quizComplete.emit({ passed: this.passed, score: this.score });
  }

  retake(): void {
    this.answers.clear();
    this.submitted = false;
    this.score = 0;
    this.passed = false;
  }
}
