import { Component, OnInit } from '@angular/core';
import { RouterOutlet, RouterLink, RouterLinkActive, Router } from '@angular/router';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { DocumentGraphService } from '../../services/document-graph.service';

@Component({
  selector: 'app-docs-layout',
  standalone: true,
  imports: [CommonModule, RouterOutlet, RouterLink, RouterLinkActive, FormsModule],
  templateUrl: './docs-layout.component.html',
  styleUrls: ['./docs-layout.component.css']
})
export class DocsLayoutComponent implements OnInit {
  searchQuery = '';
  isGraphBuilding = true;

  constructor(
    private readonly documentGraphService: DocumentGraphService,
    private readonly router: Router
  ) {}

  ngOnInit(): void {
    // Build the documentation graph on initialization
    this.documentGraphService.buildGraph().subscribe({
      next: graph => {
        console.log('Graph built successfully', graph.metadata);
        this.isGraphBuilding = false;
      },
      error: err => {
        console.error('Failed to build graph:', err);
        this.isGraphBuilding = false;
      }
    });
  }

  onSearch(): void {
    if (this.searchQuery.trim()) {
      this.router.navigate(['/docs/search'], {
        queryParams: { q: this.searchQuery }
      });
    }
  }
}
