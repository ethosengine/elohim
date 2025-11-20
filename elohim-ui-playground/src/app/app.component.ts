import { Component } from '@angular/core';
import { HexagonGridComponent, HexNode } from 'lamad-ui';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [HexagonGridComponent],
  template: `
    <div class="playground-container">
      <h1>Lamad UI Playground</h1>
      
      <section>
        <h2>Hexagon Grid</h2>
        <p>A responsive honeycomb grid with canvas rendering.</p>
        
        <div class="demo-box">
          <lamad-hexagon-grid 
            [nodes]="demoNodes" 
            [itemsPerRow]="12"
            (nodeClick)="onHexClick($event)">
          </lamad-hexagon-grid>
        </div>
      </section>
    </div>
  `,
  styles: [`
    .playground-container {
      padding: 2rem;
      max-width: 1200px;
      margin: 0 auto;
      font-family: 'Inter', sans-serif;
    }
    
    .demo-box {
      background: #f8fafc;
      border: 1px solid #e2e8f0;
      border-radius: 12px;
      padding: 1rem;
      margin-top: 1rem;
      height: 500px;
      overflow: auto;
    }
  `]
})
export class AppComponent {
  demoNodes: HexNode[] = Array.from({ length: 50 }, (_, i) => ({
    id: `node-${i}`,
    title: `Node ${i + 1}`,
    affinity: Math.random(),
    affinityLevel: this.getAffinityLevel(Math.random())
  }));

  private getAffinityLevel(val: number): 'unseen' | 'low' | 'medium' | 'high' {
    if (val < 0.2) return 'unseen';
    if (val < 0.5) return 'low';
    if (val < 0.8) return 'medium';
    return 'high';
  }

  onHexClick(node: HexNode): void {
    console.log('Clicked:', node);
    alert(`Clicked: ${node.title}`);
  }
}