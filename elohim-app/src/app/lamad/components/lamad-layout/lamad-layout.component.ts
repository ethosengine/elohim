import { Component, OnInit } from '@angular/core';
import { RouterOutlet, RouterLink, RouterLinkActive, Router } from '@angular/router';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { DocumentGraphService } from '../../services/document-graph.service';
import { AffinityTrackingService } from '../../services/affinity-tracking.service';
import { combineLatest } from 'rxjs';

@Component({
  selector: 'app-lamad-layout',
  standalone: true,
  imports: [CommonModule, RouterOutlet, RouterLink, RouterLinkActive, FormsModule],
  templateUrl: './lamad-layout.component.html',
  styleUrls: ['./lamad-layout.component.css']
})
export class LamadLayoutComponent implements OnInit {
  searchQuery = '';
  isGraphBuilding = true;
  learningPathName = 'Elohim Protocol';
  masteryPercentage = 0;

  constructor(
    private readonly documentGraphService: DocumentGraphService,
    private readonly affinityTrackingService: AffinityTrackingService,
    private readonly router: Router
  ) {}

  ngOnInit(): void {
    // Build the content graph for Lamad learning platform
    this.documentGraphService.buildGraph().subscribe({
      next: graph => {
        console.log('Lamad content graph built successfully', graph.metadata);
        this.isGraphBuilding = false;
        this.updateMasteryPercentage();
      },
      error: err => {
        console.error('Failed to build content graph:', err);
        this.isGraphBuilding = false;
      }
    });

    // Subscribe to affinity changes to update progress in real-time
    combineLatest([
      this.documentGraphService.graph$,
      this.affinityTrackingService.affinity$
    ]).subscribe(() => {
      this.updateMasteryPercentage();
    });
  }

  onSearch(): void {
    if (this.searchQuery.trim()) {
      this.router.navigate(['/lamad/search'], {
        queryParams: { q: this.searchQuery }
      });
    }
  }

  private updateMasteryPercentage(): void {
    const graph = this.documentGraphService.getGraph();
    if (graph && graph.nodes.size > 0) {
      const nodes = Array.from(graph.nodes.values());
      const stats = this.affinityTrackingService.getStats(nodes);
      this.masteryPercentage = Math.round(stats.averageAffinity * 100);
    }
  }
}
