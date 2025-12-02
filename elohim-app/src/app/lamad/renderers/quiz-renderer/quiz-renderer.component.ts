import { Component, Input, Output, EventEmitter, OnChanges } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { ContentNode } from '../../models/content-node.model';
import { InteractiveRenderer, RendererCompletionEvent } from '../renderer-registry.service';

interface QuizQuestion {
  id: string;
  type: 'multiple-choice' | 'true-false' | 'short-answer' | 'connection';
  question: string;
  options?: string[];         // For multiple-choice
  correctAnswer?: number | string | boolean;
  rubric?: string;            // For manually graded
  explanation?: string;       // Shown after answer
}

interface QuizContent {
  passingScore: number;       // 0-100
  allowRetake: boolean;
  showCorrectAnswers: boolean;
  questions: QuizQuestion[];
}

@Component({
  selector: 'app-quiz-renderer',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './quiz-renderer.component.html',
  styleUrls: ['./quiz-renderer.component.css']
})
export class QuizRendererComponent implements OnChanges, InteractiveRenderer {
  @Input() node!: ContentNode;

  /** Standard completion event for affinity tracking integration */
  @Output() complete = new EventEmitter<RendererCompletionEvent>();

  /** Legacy event for backwards compatibility */
  @Output() quizComplete = new EventEmitter<{ passed: boolean; score: number }>();

  quiz: QuizContent | null = null;
  answers: Map<string, any> = new Map();
  submitted = false;
  score = 0;
  passed = false;

  ngOnChanges(): void {
    if (this.node && this.node.contentFormat === 'quiz-json') {
      this.quiz = this.node.content as QuizContent;
      this.answers.clear();
      this.submitted = false;
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
        passingScore: this.quiz.passingScore
      }
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
