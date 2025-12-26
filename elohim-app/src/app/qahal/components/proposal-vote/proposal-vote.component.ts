import { Component, Input, Output, EventEmitter, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Subject, interval, takeUntil } from 'rxjs';
import { GovernanceService, Vote } from '@app/elohim/services/governance.service';
import { ProposalRecord } from '@app/elohim/services/data-loader.service';

/**
 * ProposalVoteComponent - Loomio-style 4-Position Voting
 *
 * Implements constitutional governance voting with:
 * - 4 positions: Agree, Abstain, Disagree, Block
 * - Changeable votes (can update as discussion evolves)
 * - Block requires written justification (accountability)
 * - Real-time pie chart visualization
 * - SLA countdown timer
 * - Quorum progress indicator
 *
 * "Every decision can be challenged. This is constitutional."
 */
@Component({
  selector: 'app-proposal-vote',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './proposal-vote.component.html',
  styleUrls: ['./proposal-vote.component.css'],
})
export class ProposalVoteComponent implements OnInit, OnDestroy {
  @Input() proposal!: ProposalRecord;
  @Input() showDetails = true;
  @Input() compact = false;

  @Output() voteSubmitted = new EventEmitter<Vote>();
  @Output() voteChanged = new EventEmitter<Vote>();

  // Voting positions (Loomio pattern)
  readonly positions: VotePosition[] = [
    {
      id: 'agree',
      label: 'Agree',
      icon: '👍',
      color: '#27ae60',
      description: 'I support this proposal',
    },
    {
      id: 'abstain',
      label: 'Abstain',
      icon: '🤝',
      color: '#3498db',
      description: 'I am neutral or unsure',
    },
    {
      id: 'disagree',
      label: 'Disagree',
      icon: '👎',
      color: '#e67e22',
      description: 'I have concerns but won\'t block',
    },
    {
      id: 'block',
      label: 'Block',
      icon: '🛑',
      color: '#e74c3c',
      description: 'I have a principled objection',
      requiresReasoning: true,
    },
  ];

  // Current user's vote
  currentVote: VotePosition['id'] | null = null;
  reasoning = '';

  // Vote aggregates (simulated for MVP)
  voteResults: VoteResults = {
    total: 0,
    agree: 0,
    abstain: 0,
    disagree: 0,
    block: 0,
  };

  // UI state
  isSubmitting = false;
  showReasoningField = false;
  hasExistingVote = false;

  // SLA countdown
  slaDeadline: Date | null = null;
  timeRemaining = '';
  slaStatus: 'on-track' | 'warning' | 'critical' | 'breached' = 'on-track';

  // Quorum tracking
  quorumRequired = 10; // Simulated
  quorumProgress = 0;
  quorumMet = false;

  private readonly destroy$ = new Subject<void>();

  constructor(private readonly governanceService: GovernanceService) {}

  ngOnInit(): void {
    this.loadExistingVote();
    this.calculateVoteResults();
    this.initSlaCountdown();
    this.updateQuorum();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  /**
   * Select a voting position.
   */
  selectPosition(position: VotePosition): void {
    this.currentVote = position.id;

    // Show reasoning field if required (Block) or if changing vote
    if (position.requiresReasoning || this.hasExistingVote) {
      this.showReasoningField = true;
    }
  }

  /**
   * Get position object by ID.
   */
  getPosition(id: VotePosition['id']): VotePosition | undefined {
    return this.positions.find(p => p.id === id);
  }

  /**
   * Check if reasoning is valid.
   */
  get reasoningValid(): boolean {
    const selectedPosition = this.getPosition(this.currentVote!);
    if (selectedPosition?.requiresReasoning) {
      return this.reasoning.trim().length >= 20; // Minimum 20 chars for Block
    }
    return true;
  }

  /**
   * Check if can submit vote.
   */
  get canSubmit(): boolean {
    return this.currentVote !== null && this.reasoningValid;
  }

  /**
   * Submit the vote.
   */
  submitVote(): void {
    if (!this.canSubmit || this.isSubmitting) return;

    this.isSubmitting = true;

    const vote: Vote = {
      proposalId: this.proposal.id,
      position: this.currentVote!,
      reasoning: this.reasoning.trim() || undefined,
    };

    this.governanceService.voteOnProposal(vote)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: (success) => {
          this.isSubmitting = false;
          if (success) {
            this.hasExistingVote = true;

            if (this.hasExistingVote) {
              this.voteChanged.emit(vote);
            } else {
              this.voteSubmitted.emit(vote);
            }

            // Update local results (MVP simulation)
            this.voteResults[vote.position]++;
            this.voteResults.total++;
            this.updateQuorum();
          }
        },
        error: () => {
          this.isSubmitting = false;
        },
      });
  }

  /**
   * Get vote percentage for pie chart.
   */
  getVotePercentage(position: VotePosition['id']): number {
    if (this.voteResults.total === 0) return 0;
    return (this.voteResults[position] / this.voteResults.total) * 100;
  }

  /**
   * Get cumulative rotation for pie chart segment.
   */
  getSegmentRotation(index: number): number {
    let rotation = 0;
    for (let i = 0; i < index; i++) {
      rotation += this.getVotePercentage(this.positions[i].id);
    }
    return rotation * 3.6; // Convert percentage to degrees
  }

  /**
   * Get segment style for pie chart.
   */
  getSegmentStyle(position: VotePosition, index: number): string {
    const percentage = this.getVotePercentage(position.id);
    const rotation = this.getSegmentRotation(index);

    return `
      --rotation: ${rotation}deg;
      --percentage: ${percentage};
      --color: ${position.color};
    `;
  }

  /**
   * Format time remaining for SLA.
   */
  private formatTimeRemaining(deadline: Date): string {
    const now = new Date();
    const diff = deadline.getTime() - now.getTime();

    if (diff <= 0) {
      return 'Expired';
    }

    const days = Math.floor(diff / (1000 * 60 * 60 * 24));
    const hours = Math.floor((diff % (1000 * 60 * 60 * 24)) / (1000 * 60 * 60));
    const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));

    if (days > 0) {
      return `${days}d ${hours}h`;
    }
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    return `${minutes}m`;
  }

  /**
   * Load existing vote for current user.
   */
  private loadExistingVote(): void {
    this.governanceService.getMyVote(this.proposal.id)
      .pipe(takeUntil(this.destroy$))
      .subscribe(vote => {
        if (vote) {
          this.currentVote = vote.position;
          this.reasoning = vote.reasoning ?? '';
          this.hasExistingVote = true;
        }
      });
  }

  /**
   * Calculate vote results from proposal data.
   */
  private calculateVoteResults(): void {
    // MVP: Use simulated data or pull from proposal
    // In production, this would query the actual votes
    if (this.proposal.votes) {
      this.voteResults = {
        total: Object.values(this.proposal.votes).reduce((a, b) => a + b, 0),
        agree: this.proposal.votes['agree'] ?? 0,
        abstain: this.proposal.votes['abstain'] ?? 0,
        disagree: this.proposal.votes['disagree'] ?? 0,
        block: this.proposal.votes['block'] ?? 0,
      };
    }
  }

  /**
   * Initialize SLA countdown timer.
   */
  private initSlaCountdown(): void {
    // Parse deadline from proposal
    if (this.proposal.deadline) {
      this.slaDeadline = new Date(this.proposal.deadline);
    } else {
      // Default: 7 days from now
      this.slaDeadline = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000);
    }

    // Update every minute
    interval(60000)
      .pipe(takeUntil(this.destroy$))
      .subscribe(() => this.updateSlaStatus());

    // Initial update
    this.updateSlaStatus();
  }

  /**
   * Update SLA status and countdown.
   */
  private updateSlaStatus(): void {
    if (!this.slaDeadline) return;

    const now = new Date();
    const diff = this.slaDeadline.getTime() - now.getTime();
    const hoursRemaining = diff / (1000 * 60 * 60);

    this.timeRemaining = this.formatTimeRemaining(this.slaDeadline);

    if (diff <= 0) {
      this.slaStatus = 'breached';
    } else if (hoursRemaining <= 24) {
      this.slaStatus = 'critical';
    } else if (hoursRemaining <= 72) {
      this.slaStatus = 'warning';
    } else {
      this.slaStatus = 'on-track';
    }
  }

  /**
   * Update quorum progress.
   */
  private updateQuorum(): void {
    this.quorumProgress = (this.voteResults.total / this.quorumRequired) * 100;
    this.quorumMet = this.voteResults.total >= this.quorumRequired;
  }
}

// ===========================================================================
// Types
// ===========================================================================

interface VotePosition {
  id: 'agree' | 'abstain' | 'disagree' | 'block';
  label: string;
  icon: string;
  color: string;
  description: string;
  requiresReasoning?: boolean;
}

interface VoteResults {
  total: number;
  agree: number;
  abstain: number;
  disagree: number;
  block: number;
}
