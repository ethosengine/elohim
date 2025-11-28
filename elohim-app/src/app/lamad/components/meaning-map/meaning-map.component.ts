import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterModule } from '@angular/router';
import { Subject, combineLatest } from 'rxjs';
import { takeUntil, map } from 'rxjs/operators';
import { DataLoaderService } from '../../services/data-loader.service';
import { AffinityTrackingService } from '../../services/affinity-tracking.service';
import { ContentNode } from '../../models/content-node.model';
import { CategoryAffinityStats } from '../../models/human-affinity.model';

interface CategorySection {
  name: string;
  displayName: string;
  icon: string;
  nodes: ContentNodeWithAffinity[];
  stats: CategoryAffinityStats | null;
  expanded: boolean;
}

interface ContentNodeWithAffinity extends ContentNode {
  affinity: number;
  affinityLevel: 'unseen' | 'low' | 'medium' | 'high';
}

@Component({
  selector: 'app-meaning-map',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './meaning-map.component.html',
  styleUrls: ['./meaning-map.component.css'],
})
export class MeaningMapComponent implements OnInit, OnDestroy {
  categories: CategorySection[] = [];
  isLoading = true;
  overallStats = {
    totalNodes: 0,
    averageAffinity: 0,
    engagedCount: 0,
  };

  private readonly destroy$ = new Subject<void>();

  constructor(
    private readonly dataLoader: DataLoaderService,
    private readonly affinityService: AffinityTrackingService,
    private readonly router: Router
  ) {}

  ngOnInit(): void {
    combineLatest([
      this.dataLoader.getContentIndex().pipe(
        map(index => index.nodes || [])
      ),
      this.affinityService.affinity$,
    ])
      .pipe(takeUntil(this.destroy$))
      .subscribe(([nodes, affinity]) => {
        if (nodes.length > 0) {
          this.buildMeaningMap(nodes);
          this.isLoading = false;
        }
      });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  /**
   * Build the hierarchical Meaning Map from content nodes
   */
  private buildMeaningMap(contentNodes: ContentNode[]): void {
    // Get affinity stats
    const stats = this.affinityService.getStats(contentNodes);
    this.overallStats = {
      totalNodes: stats.totalNodes,
      averageAffinity: stats.averageAffinity,
      engagedCount: stats.engagedNodes,
    };

    // Group nodes by category
    const categoryMap = new Map<string, ContentNode[]>();
    contentNodes.forEach((node) => {
      const category = node.metadata?.['category'] ?? 'uncategorized';
      if (!categoryMap.has(category)) {
        categoryMap.set(category, []);
      }
      categoryMap.get(category)!.push(node);
    });

    // Build category sections
    this.categories = Array.from(categoryMap.entries()).map(
      ([categoryName, nodes]) => {
        const nodesWithAffinity = nodes.map((node) =>
          this.enrichNodeWithAffinity(node)
        );

        // Sort by affinity (lower affinity first to encourage exploration)
        nodesWithAffinity.sort((a, b) => a.affinity - b.affinity);

        return {
          name: categoryName,
          displayName: this.getCategoryDisplayName(categoryName),
          icon: this.getCategoryIcon(categoryName),
          nodes: nodesWithAffinity,
          stats: stats.byCategory.get(categoryName) ?? null,
          expanded: true, // Default to expanded
        };
      }
    );

    // Sort categories by average affinity (lowest first)
    this.categories.sort((a, b) => {
      const aAvg = a.stats?.averageAffinity ?? 0;
      const bAvg = b.stats?.averageAffinity ?? 0;
      return aAvg - bAvg;
    });
  }

  /**
   * Enrich a content node with affinity data
   */
  private enrichNodeWithAffinity(node: ContentNode): ContentNodeWithAffinity {
    const affinity = this.affinityService.getAffinity(node.id);
    return {
      ...node,
      affinity,
      affinityLevel: this.getAffinityLevel(affinity),
    };
  }

  /**
   * Get affinity level category
   */
  getAffinityLevel(
    affinity: number
  ): 'unseen' | 'low' | 'medium' | 'high' {
    if (affinity === 0) return 'unseen';
    if (affinity <= 0.33) return 'low';
    if (affinity <= 0.66) return 'medium';
    return 'high';
  }

  /**
   * Get user-friendly category display name
   */
  private getCategoryDisplayName(category: string): string {
    const displayNames: Record<string, string> = {
      core: 'Core Platform',
      'value-scanner': 'Value Scanner',
      deployment: 'Deployment & CI/CD',
      uncategorized: 'Other',
    };
    return displayNames[category] || category;
  }

  /**
   * Get icon for category
   */
  private getCategoryIcon(category: string): string {
    const icons: Record<string, string> = {
      core: '🏛️',
      'value-scanner': '💎',
      deployment: '🚀',
      uncategorized: '📄',
    };
    return icons[category] || '📦';
  }

  /**
   * Toggle category expansion
   */
  toggleCategory(category: CategorySection): void {
    category.expanded = !category.expanded;
  }

  /**
   * Navigate to content viewer
   */
  viewContent(node: ContentNodeWithAffinity): void {
    this.router.navigate(['/lamad/content', node.id]);
  }

  /**
   * Get affinity color class
   */
  getAffinityColorClass(level: 'unseen' | 'low' | 'medium' | 'high'): string {
    const classes: Record<string, string> = {
      unseen: 'affinity-unseen',
      low: 'affinity-low',
      medium: 'affinity-medium',
      high: 'affinity-high',
    };
    return classes[level];
  }

  /**
   * Get affinity percentage for progress bar
   */
  getAffinityPercentage(affinity: number): number {
    return Math.round(affinity * 100);
  }

  /**
   * Get content type display name
   */
  getContentTypeDisplay(contentType: string): string {
    const displays: Record<string, string> = {
      epic: 'Epic',
      feature: 'Feature',
      scenario: 'Scenario',
    };
    return displays[contentType] || contentType;
  }

  /**
   * Get content type icon
   */
  getContentTypeIcon(contentType: string): string {
    const icons: Record<string, string> = {
      epic: '📖',
      feature: '⚙️',
      scenario: '✓',
    };
    return icons[contentType] || '•';
  }
}