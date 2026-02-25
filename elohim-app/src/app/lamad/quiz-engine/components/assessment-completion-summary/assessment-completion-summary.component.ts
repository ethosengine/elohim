/**
 * Assessment Completion Summary — Common feedback interface for both
 * discovery and mastery assessment completion.
 *
 * Discovery mode: records attestation, shows personalized result ("You're an ISTP!"),
 * hex badge preview, subscale breakdown bars.
 *
 * Mastery mode: shows score/pass-fail, XP/streak preview, dashboard link.
 *
 * Reflection mode: shows generic completion with profile link.
 */

import { CommonModule } from '@angular/common';
import {
  Component,
  ChangeDetectionStrategy,
  input,
  output,
  computed,
  inject,
  OnInit,
} from '@angular/core';

import { MasteryStatsService } from '@app/lamad/services/mastery-stats.service';

import { getInstrument, type InstrumentRegistryEntry } from '../../instruments/instrument-registry';
import {
  type DiscoveryResult,
  type DiscoveryResultSummary,
  getCategoryIcon,
} from '../../models/discovery-assessment.model';
import { DiscoveryAttestationService } from '../../services/discovery-attestation.service';

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export type CompletionMode = 'mastery' | 'discovery' | 'reflection';

export interface SubscaleBar {
  key: string;
  name: string;
  icon: string;
  color: string;
  score: number;
  percent: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Component
// ─────────────────────────────────────────────────────────────────────────────

@Component({
  selector: 'app-assessment-completion-summary',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="completion-summary" [attr.data-mode]="mode()" data-testid="completion-summary">
      <!-- Result Card -->
      <div
        class="result-card"
        [class.passed]="mode() === 'mastery' && passed()"
        [class.failed]="mode() === 'mastery' && passed() === false"
        data-testid="completion-result-card"
      >
        <div class="result-icon" data-testid="completion-icon">{{ icon() }}</div>
        <h4 class="result-title" data-testid="completion-headline">{{ headline() }}</h4>
        <p class="result-description" data-testid="completion-description">
          {{ description() }}
        </p>
      </div>

      <!-- Discovery: Hex Badge Preview -->
      @if (mode() === 'discovery' && badgeDisplay()) {
        <div class="badge-reveal" data-testid="completion-badge-reveal">
          <span class="badge-label">Your new badge</span>
          <div
            class="hex-badge"
            [style.--badge-color]="badgeDisplay()!.color"
            data-testid="completion-hex-badge"
          >
            <span class="hex-badge__icon">{{ badgeDisplay()!.icon }}</span>
            <span class="hex-badge__label">{{ badgeDisplay()!.label }}</span>
          </div>
        </div>
      }

      <!-- Discovery: Subscale Breakdown -->
      @if (mode() === 'discovery' && subscaleBars().length > 0) {
        <div class="subscale-breakdown" data-testid="completion-subscales">
          <h5 class="subscale-heading">Your Profile</h5>
          @for (bar of subscaleBars(); track bar.key) {
            <div class="subscale-row" [attr.data-testid]="'subscale-' + bar.key">
              <span class="subscale-label">
                <span class="subscale-icon">{{ bar.icon }}</span>
                {{ bar.name }}
              </span>
              <div class="subscale-bar">
                <div
                  class="subscale-fill"
                  [style.width.%]="bar.percent"
                  [style.backgroundColor]="bar.color"
                ></div>
              </div>
              <span class="subscale-percent">{{ bar.percent | number: '1.0-0' }}%</span>
            </div>
          }
        </div>
      }

      <!-- Mastery: Score Display -->
      @if (mode() === 'mastery') {
        <div class="score-display" data-testid="completion-score">
          <span class="score-value">{{ score() }}%</span>
          <span class="score-label">Score</span>
          <span class="score-detail">{{ correctCount() }} of {{ totalCount() }} correct</span>
        </div>

        <!-- Mastery: Learner Profile Preview -->
        @if (learnerProfile$ | async; as profile) {
          <div class="mastery-preview" data-testid="completion-mastery-preview">
            <div class="preview-item">
              <span class="preview-icon material-icons" [style.color]="profile.learnerLevel.color">
                {{ profile.learnerLevel.icon }}
              </span>
              <div class="preview-content">
                <strong>{{ profile.learnerLevel.label }}</strong>
                <span class="preview-detail">{{ profile.totalXP | number }} XP</span>
              </div>
            </div>
            <div class="preview-item">
              <span
                class="preview-icon material-icons streak-fire"
                [class.streak-active]="profile.streak.todayActive"
              >
                local_fire_department
              </span>
              <div class="preview-content">
                <strong>{{ profile.streak.currentStreak }} Day Streak</strong>
                <span class="preview-detail">Best: {{ profile.streak.bestStreak }} days</span>
              </div>
            </div>
            <div class="preview-item">
              <span class="preview-icon material-icons">emoji_events</span>
              <div class="preview-content">
                <strong>{{ profile.totalMasteredNodes }} Mastered</strong>
                <span class="preview-detail">{{ profile.nodesAboveGate }} above gate</span>
              </div>
            </div>
          </div>
        }
      }

      <!-- Navigation -->
      <div class="completion-nav" data-testid="completion-nav">
        <button
          class="profile-link"
          (click)="viewProfile.emit()"
          data-testid="completion-profile-link"
        >
          {{ profileLinkLabel() }}
        </button>
        <button class="btn btn-primary" (click)="continue.emit()" data-testid="completion-continue">
          Continue
        </button>
      </div>
    </div>
  `,
  styles: [
    `
      .completion-summary {
        padding: 1.5rem;
        text-align: center;
      }

      /* Result Card */
      .result-card {
        background: rgba(255, 255, 255, 0.95);
        border-radius: 12px;
        padding: 2rem;
        margin-bottom: 1.5rem;
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.08);
        color: #1a1a1a;
      }

      .result-card.passed {
        border: 2px solid #2e7d32;
      }

      .result-card.failed {
        border: 2px solid #f57c00;
      }

      .result-icon {
        font-size: 3rem;
        margin-bottom: 1rem;
      }

      .result-title {
        font-size: 1.25rem;
        font-weight: 600;
        margin: 0 0 0.5rem;
        color: #1a1a2e;
      }

      .result-description {
        font-size: 0.9rem;
        color: #555;
        margin: 0;
        line-height: 1.5;
      }

      /* Hex Badge Reveal */
      .badge-reveal {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 0.5rem;
        margin-bottom: 1.5rem;
      }

      .badge-label {
        font-size: 0.8rem;
        font-weight: 500;
        color: #6c757d;
        text-transform: uppercase;
        letter-spacing: 0.05em;
      }

      .hex-badge {
        display: inline-flex;
        align-items: center;
        gap: 0.375rem;
        padding: 0.75rem 1.5rem;
        background: color-mix(in srgb, var(--badge-color, #8b5cf6) 12%, rgba(255, 255, 255, 0.95));
        border: 2px solid color-mix(in srgb, var(--badge-color, #8b5cf6) 35%, transparent);
        clip-path: polygon(10% 0%, 90% 0%, 100% 50%, 90% 100%, 10% 100%, 0% 50%);
        color: var(--badge-color, #8b5cf6);
        font-size: 1rem;
        font-weight: 700;
        letter-spacing: 0.05em;
      }

      .hex-badge__icon {
        font-size: 1rem;
      }

      /* Subscale Breakdown */
      .subscale-breakdown {
        text-align: left;
        background: rgba(245, 245, 245, 0.95);
        border-radius: 8px;
        padding: 1rem 1.25rem;
        margin-bottom: 1.5rem;
      }

      .subscale-heading {
        font-size: 0.85rem;
        font-weight: 600;
        color: #333;
        margin: 0 0 0.75rem;
      }

      .subscale-row {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        margin-bottom: 0.5rem;
      }

      .subscale-row:last-child {
        margin-bottom: 0;
      }

      .subscale-label {
        display: flex;
        align-items: center;
        gap: 0.25rem;
        min-width: 110px;
        font-size: 0.8rem;
        font-weight: 500;
        color: #333;
        white-space: nowrap;
      }

      .subscale-icon {
        font-size: 0.85rem;
      }

      .subscale-bar {
        flex: 1;
        height: 8px;
        background: #e0e0e0;
        border-radius: 4px;
        overflow: hidden;
      }

      .subscale-fill {
        height: 100%;
        border-radius: 4px;
        transition: width 0.6s ease;
      }

      .subscale-percent {
        min-width: 36px;
        text-align: right;
        font-size: 0.75rem;
        font-weight: 600;
        color: #555;
      }

      /* Score Display */
      .score-display {
        display: flex;
        flex-direction: column;
        align-items: center;
        margin-bottom: 1.5rem;
        padding: 1rem;
        background: rgba(245, 245, 245, 0.95);
        border-radius: 8px;
      }

      .score-value {
        font-size: 2.5rem;
        font-weight: 700;
        color: #1976d2;
      }

      .score-label {
        font-size: 0.8rem;
        font-weight: 500;
        color: #888;
        text-transform: uppercase;
        letter-spacing: 0.05em;
      }

      .score-detail {
        font-size: 0.85rem;
        color: #555;
        margin-top: 0.25rem;
      }

      /* Mastery Preview */
      .mastery-preview {
        background: rgba(245, 245, 245, 0.95);
        border-radius: 8px;
        padding: 1rem;
        margin-bottom: 1.5rem;
        color: #333;
        text-align: left;
      }

      .preview-item {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 0.5rem 0;
      }

      .preview-item:not(:last-child) {
        border-bottom: 1px solid #e0e0e0;
      }

      .preview-icon {
        font-size: 1.25rem;
      }

      .preview-content {
        display: flex;
        flex-direction: column;
        gap: 0.125rem;
      }

      .preview-content strong {
        font-size: 0.9rem;
        color: #1a1a2e;
      }

      .preview-detail {
        font-size: 0.8rem;
        color: #888;
      }

      .streak-fire {
        color: #bdbdbd;
      }

      .streak-active {
        color: #ff6d00 !important;
      }

      /* Navigation */
      .completion-nav {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 0.75rem;
        padding-top: 0.5rem;
      }

      .profile-link {
        background: none;
        border: none;
        color: #1976d2;
        font-size: 0.9rem;
        font-weight: 500;
        cursor: pointer;
        padding: 0.5rem;
      }

      .profile-link:hover {
        text-decoration: underline;
      }

      .btn {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        padding: 0.75rem 2rem;
        border: none;
        border-radius: 8px;
        font-size: 1rem;
        font-weight: 500;
        cursor: pointer;
        transition:
          background-color 0.2s,
          transform 0.1s;
      }

      .btn:not(:disabled):active {
        transform: scale(0.98);
      }

      .btn-primary {
        background: #1976d2;
        color: white;
        min-width: 160px;
      }

      .btn-primary:hover {
        background: #1565c0;
      }

      /* Dark theme */
      :host-context(body[data-theme='dark']) .result-card,
      :host-context(body:not([data-theme])) .result-card {
        background: rgba(30, 30, 35, 0.95);
        color: #f0f0f0;
      }

      :host-context(body[data-theme='dark']) .result-title,
      :host-context(body:not([data-theme])) .result-title {
        color: #f0f0f0;
      }

      :host-context(body[data-theme='dark']) .result-description,
      :host-context(body:not([data-theme])) .result-description {
        color: #aaa;
      }

      :host-context(body[data-theme='dark']) .subscale-breakdown,
      :host-context(body:not([data-theme])) .subscale-breakdown,
      :host-context(body[data-theme='dark']) .score-display,
      :host-context(body:not([data-theme])) .score-display,
      :host-context(body[data-theme='dark']) .mastery-preview,
      :host-context(body:not([data-theme])) .mastery-preview {
        background: rgba(45, 45, 50, 0.95);
        color: #ddd;
      }

      :host-context(body[data-theme='dark']) .subscale-heading,
      :host-context(body:not([data-theme])) .subscale-heading,
      :host-context(body[data-theme='dark']) .subscale-label,
      :host-context(body:not([data-theme])) .subscale-label {
        color: #ddd;
      }

      :host-context(body[data-theme='dark']) .preview-content strong,
      :host-context(body:not([data-theme])) .preview-content strong {
        color: #f0f0f0;
      }

      :host-context(body[data-theme='dark']) .profile-link,
      :host-context(body:not([data-theme])) .profile-link {
        color: #64b5f6;
      }

      :host-context(body[data-theme='dark']) .preview-item:not(:last-child),
      :host-context(body:not([data-theme])) .preview-item:not(:last-child) {
        border-bottom-color: #444;
      }

      :host-context(body[data-theme='dark']) .score-value,
      :host-context(body:not([data-theme])) .score-value {
        color: #64b5f6;
      }

      :host-context(body[data-theme='dark']) .hex-badge,
      :host-context(body:not([data-theme])) .hex-badge {
        background: color-mix(in srgb, var(--badge-color, #8b5cf6) 15%, rgba(30, 30, 35, 0.95));
      }
    `,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AssessmentCompletionSummaryComponent implements OnInit {
  private readonly discoveryService = inject(DiscoveryAttestationService);
  private readonly masteryStats = inject(MasteryStatsService);

  // ─── Inputs ──────────────────────────────────────────────────────────────────

  /** Completion mode: mastery, discovery, or reflection */
  readonly mode = input.required<CompletionMode>();

  /** Assessment title */
  readonly title = input.required<string>();

  /** Content node ID for linking back to the source assessment */
  readonly contentNodeId = input<string>('');

  /** Instrument ID for registry lookup (discovery mode) */
  readonly instrumentId = input<string | null>(null);

  /** Raw subscale totals from aggregated reflection */
  readonly subscaleScores = input<Record<string, number> | null>(null);

  /** Normalized subscale percentages from aggregated reflection */
  readonly normalizedScores = input<Record<string, number> | null>(null);

  /** Primary type result from Psyche API interpretation */
  readonly primaryType = input<DiscoveryResultSummary | null>(null);

  /** Mastery score percentage (0-100) */
  readonly score = input<number | null>(null);

  /** Mastery pass/fail */
  readonly passed = input<boolean | null>(null);

  /** Mastery correct count */
  readonly correctCount = input<number | null>(null);

  /** Total question count */
  readonly totalCount = input<number>(0);

  // ─── Outputs ─────────────────────────────────────────────────────────────────

  /** User clicks Continue */
  readonly continue = output<void>();

  /** User clicks profile/dashboard link */
  readonly viewProfile = output<void>();

  // ─── Observables ─────────────────────────────────────────────────────────────

  readonly learnerProfile$ = this.masteryStats.learnerProfile$;

  // ─── Computed ────────────────────────────────────────────────────────────────

  /** Resolved instrument entry from registry */
  readonly instrument = computed<InstrumentRegistryEntry | undefined>(() => {
    const id = this.instrumentId();
    return id ? getInstrument(id) : undefined;
  });

  /** Celebration icon */
  readonly icon = computed(() => {
    const m = this.mode();
    if (m === 'mastery') return this.passed() ? '🎯' : '📚';
    if (m === 'discovery') {
      const inst = this.instrument();
      const cat = inst?.config.category ?? 'personality';
      return getCategoryIcon(cat);
    }
    return '✨';
  });

  /** Main headline */
  readonly headline = computed(() => {
    const m = this.mode();
    if (m === 'mastery') return this.passed() ? 'Well Done!' : 'Keep Learning';
    if (m === 'discovery') {
      const pt = this.primaryType();
      if (pt) return `You're ${addArticle(pt.name)}!`;
      return 'Assessment Complete';
    }
    return 'Assessment Complete';
  });

  /** Description text */
  readonly description = computed(() => {
    const m = this.mode();
    if (m === 'mastery') {
      return `You got ${this.correctCount() ?? 0} of ${this.totalCount()} correct.`;
    }
    if (m === 'discovery') {
      const pt = this.primaryType();
      return pt
        ? `Your responses reveal ${pt.name} as your primary result.`
        : 'Thank you for completing this reflection.';
    }
    return 'Your responses contribute to your learning profile.';
  });

  /** Badge display for hex badge preview (discovery mode) */
  readonly badgeDisplay = computed(() => {
    if (this.mode() !== 'discovery') return null;
    const recorded = this.recordedResult();
    if (recorded) return this.discoveryService.getBadgeDisplay(recorded);
    return null;
  });

  /** Subscale breakdown bars (discovery mode) */
  readonly subscaleBars = computed<SubscaleBar[]>(() => {
    if (this.mode() !== 'discovery') return [];
    const scores = this.normalizedScores() ?? this.subscaleScores();
    if (!scores) return [];

    const total = Object.values(scores).reduce((sum, v) => sum + v, 0) || 1;
    const inst = this.instrument();
    const subscaleDefs = inst?.subscales ?? [];

    return Object.entries(scores)
      .map(([key, value]) => {
        const def = subscaleDefs.find(s => s.id === key);
        return {
          key,
          name: def?.name ?? key,
          icon: def?.icon ?? '📌',
          color: def?.color ?? '#888',
          score: value,
          percent: (value / total) * 100,
        };
      })
      .sort((a, b) => b.score - a.score);
  });

  /** Profile link label (mode-specific) */
  readonly profileLinkLabel = computed(() => {
    return this.mode() === 'discovery' ? 'View on your profile →' : 'View your Dashboard →';
  });

  // ─── State ───────────────────────────────────────────────────────────────────

  /** The recorded discovery result (set during ngOnInit) */
  private recordedResultValue: DiscoveryResult | null = null;

  private readonly recordedResult = computed(() => this.recordedResultValue);

  // ─── Lifecycle ───────────────────────────────────────────────────────────────

  ngOnInit(): void {
    if (this.mode() === 'discovery') {
      this.recordAttestation();
    }
  }

  // ─── Private ─────────────────────────────────────────────────────────────────

  private recordAttestation(): void {
    const instId = this.instrumentId();
    const pt = this.primaryType();
    const scores = this.subscaleScores();
    if (!instId || !pt || !scores) return;

    this.recordedResultValue = this.discoveryService.recordFromCompletion({
      contentNodeId: this.contentNodeId(),
      assessmentTitle: this.title(),
      instrumentId: instId,
      subscaleScores: scores,
      primaryType: pt,
    });
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Prefix a name with "a" or "an" for the headline.
 * Simple heuristic: use "an" before vowel sounds.
 */
function addArticle(name: string): string {
  const first = name.charAt(0).toLowerCase();
  const article = 'aeiou'.includes(first) ? 'an' : 'a';
  return `${article} ${name}`;
}
