/**
 * Storage Distribution Component
 *
 * Shows breakdown of what types of content are stored where:
 * - By content type (videos, documents, learning materials, etc.)
 * - By reach level (private to commons)
 * - By node (which nodes store what)
 *
 * Helps users understand their digital footprint distribution.
 */

import { Component, Input, Output, EventEmitter, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Subject, takeUntil } from 'rxjs';

import { ShefaComputeService } from '../../services/shefa-compute.service';
import {
  StorageContentDistribution,
  ContentTypeStorage,
  ReachLevelStorage,
  NodeStorageBreakdown,
} from '../../models/shefa-dashboard.model';

@Component({
  selector: 'app-storage-distribution',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './storage-distribution.component.html',
  styleUrls: ['./storage-distribution.component.scss'],
})
export class StorageDistributionComponent implements OnInit, OnDestroy {
  /** Operator ID to load storage data for */
  @Input() operatorId!: string;

  /** Display mode: 'summary' for compact, 'full' for all breakdowns */
  @Input() displayMode: 'summary' | 'full' = 'full';

  /** Which breakdown view to show initially */
  @Input() initialView: 'type' | 'reach' | 'node' = 'type';

  /** Emits when a content type is selected */
  @Output() contentTypeSelected = new EventEmitter<ContentTypeStorage>();

  /** Emits when a node is selected */
  @Output() nodeSelected = new EventEmitter<NodeStorageBreakdown>();

  // State
  distribution: StorageContentDistribution | null = null;
  isLoading = true;
  error: string | null = null;
  activeView: 'type' | 'reach' | 'node' = 'type';

  private destroy$ = new Subject<void>();

  constructor(private shefaCompute: ShefaComputeService) {}

  ngOnInit(): void {
    this.activeView = this.initialView;

    if (!this.operatorId) {
      this.error = 'No operator ID provided';
      this.isLoading = false;
      return;
    }

    this.loadDistribution();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  /**
   * Load storage distribution data
   */
  private loadDistribution(): void {
    this.shefaCompute
      .getStorageContentDistribution(this.operatorId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: (dist) => {
          this.distribution = dist;
          this.isLoading = false;
        },
        error: (err) => {
          console.error('[StorageDistribution] Failed to load:', err);
          this.error = 'Failed to load storage distribution';
          this.isLoading = false;
        },
      });
  }

  /**
   * Switch active view
   */
  setActiveView(view: 'type' | 'reach' | 'node'): void {
    this.activeView = view;
  }

  /**
   * Select a content type for details
   */
  selectContentType(ct: ContentTypeStorage): void {
    this.contentTypeSelected.emit(ct);
  }

  /**
   * Select a node for details
   */
  selectNode(node: NodeStorageBreakdown): void {
    this.nodeSelected.emit(node);
  }

  /**
   * Format GB to human readable
   */
  formatSize(gb: number): string {
    if (gb >= 1000) {
      return `${(gb / 1000).toFixed(1)} TB`;
    }
    if (gb < 1) {
      return `${(gb * 1024).toFixed(0)} MB`;
    }
    return `${gb.toFixed(1)} GB`;
  }

  /**
   * Get progress bar width
   */
  getProgressWidth(value: number, max: number): string {
    if (max <= 0) return '0%';
    return `${Math.min(100, Math.max(0, (value / max) * 100))}%`;
  }

  /**
   * Get color class for replication status
   */
  getReplicationClass(status: string): string {
    switch (status) {
      case 'met':
        return 'replication-met';
      case 'under':
        return 'replication-under';
      case 'over':
        return 'replication-over';
      default:
        return '';
    }
  }

  /**
   * Get color class for node status
   */
  getNodeStatusClass(status: string): string {
    return `node-${status}`;
  }

  /**
   * Get icon for content type
   */
  getContentTypeIcon(type: string): string {
    const icons: Record<string, string> = {
      video: '🎬',
      audio: '🎵',
      image: '🖼️',
      document: '📄',
      application: '💻',
      learning: '📚',
      other: '📦',
    };
    return icons[type] || '📦';
  }

  /**
   * Get icon for reach level
   */
  getReachIcon(level: number): string {
    const icons = ['🔒', '🏠', '👪', '🤝', '🏘️', '🌍', '🌐', '🌏'];
    return icons[level] || '📍';
  }

  /**
   * Get largest content type for summary
   */
  get largestContentType(): ContentTypeStorage | null {
    if (!this.distribution?.byContentType.length) return null;
    return [...this.distribution.byContentType].sort((a, b) => b.sizeGB - a.sizeGB)[0];
  }
}
