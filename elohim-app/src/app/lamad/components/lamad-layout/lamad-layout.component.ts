import { Component, OnInit } from '@angular/core';
import { RouterOutlet, RouterLink, RouterLinkActive, Router, NavigationEnd } from '@angular/router';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { filter } from 'rxjs/operators';
import { DocumentGraphService } from '../../services/document-graph.service';
import { LearningPathService } from '../../services/learning-path.service';
import { ThemeToggleComponent } from '../../../components/theme-toggle/theme-toggle.component';

@Component({
  selector: 'app-lamad-layout',
  standalone: true,
  imports: [CommonModule, RouterOutlet, RouterLink, RouterLinkActive, FormsModule, ThemeToggleComponent],
  templateUrl: './lamad-layout.component.html',
  styleUrls: ['./lamad-layout.component.css']
})
export class LamadLayoutComponent implements OnInit {
  searchQuery = '';
  isGraphBuilding = true;
  isHomePage = false;

  constructor(
    private readonly documentGraphService: DocumentGraphService,
    private readonly learningPathService: LearningPathService,
    private readonly router: Router
  ) {}

  ngOnInit(): void {
    // Check initial route
    this.checkIfHomePage(this.router.url);

    // Listen for route changes
    this.router.events.pipe(
      filter(event => event instanceof NavigationEnd)
    ).subscribe((event: any) => {
      this.checkIfHomePage(event.urlAfterRedirects || event.url);
    });

    // Build the content graph for Lamad learning platform
    this.documentGraphService.buildGraph().subscribe({
      next: graph => {
        console.log('Lamad content graph built successfully', graph.metadata);
        
        // Initialize Learning Path with Epics
        const epics = this.documentGraphService.getNodesByType('epic');
        this.learningPathService.setPath(epics);

        this.isGraphBuilding = false;
      },
      error: err => {
        console.error('Failed to build content graph:', err);
        this.isGraphBuilding = false;
      }
    });
  }

  private checkIfHomePage(url: string): void {
    // Check if url matches /lamad exactly or with query params, but not sub-routes like /lamad/map
    const path = url.split('?')[0];
    this.isHomePage = path === '/lamad' || path === '/lamad/';
  }

  onSearch(): void {
    if (this.searchQuery.trim()) {
      this.router.navigate(['/lamad/search'], {
        queryParams: { q: this.searchQuery }
      });
    }
  }
}
