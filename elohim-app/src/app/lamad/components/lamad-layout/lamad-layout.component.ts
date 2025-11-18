import { Component, OnInit } from '@angular/core';
import { RouterOutlet, RouterLink, RouterLinkActive, Router } from '@angular/router';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { DocumentGraphService } from '../../services/document-graph.service';

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

  constructor(
    private readonly documentGraphService: DocumentGraphService,
    private readonly router: Router
  ) {}

  ngOnInit(): void {
    // Build the content graph for Lamad learning platform
    this.documentGraphService.buildGraph().subscribe({
      next: graph => {
        console.log('Lamad content graph built successfully', graph.metadata);
        this.isGraphBuilding = false;
      },
      error: err => {
        console.error('Failed to build content graph:', err);
        this.isGraphBuilding = false;
      }
    });
  }

  onSearch(): void {
    if (this.searchQuery.trim()) {
      this.router.navigate(['/lamad/search'], {
        queryParams: { q: this.searchQuery }
      });
    }
  }
}
