import { Component, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';
import { DocumentGraphService } from '../../services/document-graph.service';
import { EpicNode, FeatureNode, ScenarioNode } from '../../models';

@Component({
  selector: 'app-docs-home',
  standalone: true,
  imports: [CommonModule, RouterLink],
  templateUrl: './docs-home.component.html',
  styleUrls: ['./docs-home.component.css']
})
export class DocsHomeComponent implements OnInit {
  epics: EpicNode[] = [];
  features: FeatureNode[] = [];
  scenarios: ScenarioNode[] = [];
  stats: any = {};

  constructor(private documentGraphService: DocumentGraphService) {}

  ngOnInit(): void {
    const graph = this.documentGraphService.getGraph();

    if (graph) {
      this.epics = Array.from(graph.nodesByType.epics.values());
      this.features = Array.from(graph.nodesByType.features.values());
      this.scenarios = Array.from(graph.nodesByType.scenarios.values());
      this.stats = graph.metadata.stats;
    }
  }

  getEpicCategory(epic: EpicNode): string {
    return epic.category || 'general';
  }

  getFeatureCategory(feature: FeatureNode): string {
    return feature.category || 'general';
  }

  getCategoryIcon(category: string): string {
    const icons: Record<string, string> = {
      observer: '👁️',
      'value-scanner': '🔍',
      'autonomous-entity': '🤖',
      social: '🌐',
      deployment: '🚀',
      general: '📄'
    };
    return icons[category] || '📄';
  }
}
