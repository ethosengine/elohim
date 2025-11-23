import { Component, OnInit, OnDestroy } from '@angular/core';
import { RouterOutlet, RouterLink, RouterLinkActive, Router, NavigationEnd } from '@angular/router';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Subject } from 'rxjs';
import { filter, takeUntil } from 'rxjs/operators';
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
export class LamadLayoutComponent implements OnInit, OnDestroy {
  searchQuery = '';
  isGraphBuilding = true;
  isHomePage = false;
  
  private destroy$ = new Subject<void>();

  constructor(
    private readonly documentGraphService: DocumentGraphService,
    private readonly learningPathService: LearningPathService,
    private readonly router: Router
  ) {}

  ngOnInit(): void {
    // Build the content graph for Lamad learning platform
    this.documentGraphService.buildGraph().subscribe({
      next: graph => {
        console.log('Lamad content graph built successfully', graph.metadata);
        
        // Initialize learning path with epics
        const epics = this.documentGraphService.getNodesByType('epic');
        this.learningPathService.setPath(epics);
        
        this.isGraphBuilding = false;
      },
      error: err => {
        console.error('Failed to build content graph:', err);
        this.isGraphBuilding = false;
      }
    });
    
    // Track route for UI state
    this.router.events.pipe(
      filter(event => event instanceof NavigationEnd),
      takeUntil(this.destroy$)
    ).subscribe(() => {
      this.checkIfHomePage();
    });
    
    this.checkIfHomePage();
  }
  
  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  onSearch(): void {
    if (this.searchQuery.trim()) {
      this.router.navigate(['/lamad/search'], {
        queryParams: { q: this.searchQuery }
      });
    }
  }

  navigateToAbout(): void {
    // Navigate to the About content in the knowledge graph
    // The ID is generated from the filename: lamad-about.md -> lamad-about
    this.router.navigate(['/lamad/content', 'lamad-about']);
  }

  private checkIfHomePage(): void {
    this.isHomePage = this.router.url === '/lamad' || this.router.url === '/lamad/';
  }
}
