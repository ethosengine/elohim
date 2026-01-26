/**
 * Custodian View Component
 *
 * Shows bidirectional custodian relationships:
 * - Who I'm helping (storing their content)
 * - Who's helping me (storing my content)
 * - Mutual aid balance indicator
 * - Community strength visualization
 *
 * This helps users understand the reciprocal nature of the
 * family-community protection network.
 */

import { CommonModule } from '@angular/common';
import { Component, Input, Output, EventEmitter, OnInit, OnDestroy } from '@angular/core';

import { Subject, takeUntil } from 'rxjs';

import {
  BidirectionalCustodianView,
  CustodianRelationship,
} from '../../models/shefa-dashboard.model';
import { ShefaComputeService } from '../../services/shefa-compute.service';

@Component({
  selector: 'app-custodian-view',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './custodian-view.component.html',
  styleUrls: ['./custodian-view.component.scss'],
})
export class CustodianViewComponent implements OnInit, OnDestroy {
  /** Operator ID to load custodian data for */
  @Input() operatorId!: string;

  /** Display mode: 'summary' for compact, 'full' for detailed view */
  @Input() displayMode: 'summary' | 'full' = 'full';

  /** Emits when a custodian is selected */
  @Output() custodianSelected = new EventEmitter<CustodianRelationship>();

  // State
  view: BidirectionalCustodianView | null = null;
  isLoading = true;
  error: string | null = null;
  activeTab: 'helping' | 'beingHelped' = 'helping';

  private readonly destroy$ = new Subject<void>();

  constructor(private readonly shefaCompute: ShefaComputeService) {}

  ngOnInit(): void {
    if (!this.operatorId) {
      this.error = 'No operator ID provided';
      this.isLoading = false;
      return;
    }

    this.loadCustodianView();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  /**
   * Load bidirectional custodian view
   */
  private loadCustodianView(): void {
    this.shefaCompute
      .getBidirectionalCustodianView(this.operatorId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: view => {
          this.view = view;
          this.isLoading = false;
        },
        error: err => {
          console.error('[CustodianView] Failed to load:', err);
          this.error = 'Failed to load custodian relationships';
          this.isLoading = false;
        },
      });
  }

  /**
   * Switch between helping/beingHelped tabs
   */
  setActiveTab(tab: 'helping' | 'beingHelped'): void {
    this.activeTab = tab;
  }

  /**
   * Get list based on active tab
   */
  get activeList(): CustodianRelationship[] {
    if (!this.view) return [];
    return this.activeTab === 'helping' ? this.view.helping : this.view.beingHelpedBy;
  }

  /**
   * Select a custodian to view details
   */
  selectCustodian(relationship: CustodianRelationship): void {
    this.custodianSelected.emit(relationship);
  }

  /**
   * Get color class for balance status
   */
  getBalanceClass(): string {
    if (!this.view) return '';
    switch (this.view.mutualAidBalance.status) {
      case 'giving-more':
        return 'balance-giving';
      case 'receiving-more':
        return 'balance-receiving';
      default:
        return 'balance-balanced';
    }
  }

  /**
   * Get icon for balance status
   */
  getBalanceIcon(): string {
    if (!this.view) return '⚖️';
    switch (this.view.mutualAidBalance.status) {
      case 'giving-more':
        return '💝';
      case 'receiving-more':
        return '🙏';
      default:
        return '⚖️';
    }
  }

  /**
   * Get strength indicator class
   */
  getStrengthClass(): string {
    if (!this.view) return '';
    return `strength-${this.view.communityStrength}`;
  }

  /**
   * Get relationship type icon
   */
  getRelationshipIcon(type: string): string {
    const icons: Record<string, string> = {
      family: '👨‍👩‍👧‍👦',
      friend: '🤝',
      community: '🏘️',
      professional: '💼',
    };
    return icons[type] || '👤';
  }

  /**
   * Get status badge class
   */
  getStatusClass(status: string): string {
    return `status-${status}`;
  }

  /**
   * Format GB to human readable
   */
  formatGB(gb: number): string {
    if (gb >= 1000) {
      return `${(gb / 1000).toFixed(1)} TB`;
    }
    return `${gb.toFixed(1)} GB`;
  }

  /**
   * Get progress bar width for reliability
   */
  getReliabilityWidth(reliability: number): string {
    return `${Math.min(100, Math.max(0, reliability))}%`;
  }
}
