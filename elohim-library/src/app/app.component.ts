import { Component } from '@angular/core';
import {
  HexagonGridComponent,
  HexNode,
  ObserverDiagramComponent,
  ValueScannerDiagramComponent,
  GovernanceDiagramComponent
} from 'lamad-ui';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [
    HexagonGridComponent,
    ObserverDiagramComponent,
    ValueScannerDiagramComponent,
    GovernanceDiagramComponent
  ],
  template: `
    <div class="playground-container">
      <header class="header">
        <h1>Lamad UI Playground</h1>
        <p class="subtitle">Animated Infographic Components for Elohim Protocol</p>
      </header>

      <nav class="nav">
        <a href="#observer">Observer</a>
        <a href="#value-scanner">Value Scanner</a>
        <a href="#governance">Governance</a>
        <a href="#hexagon">Hexagon Grid</a>
      </nav>

      <section id="observer" class="section">
        <div class="section-header">
          <h2>👁️ Observer Protocol</h2>
          <p>From Surveillance to Witness: Data flows through observation, value extraction, and cryptographic destruction.</p>
        </div>

        <div class="demo-box light">
          <lamad-observer-diagram></lamad-observer-diagram>
        </div>
      </section>

      <section id="value-scanner" class="section">
        <div class="section-header">
          <h2>❤️ Value Scanner</h2>
          <p>Multi-dimensional value recognition: Economic, Social, and Emotional care captured in a single bundle.</p>
        </div>

        <div class="demo-box light">
          <lamad-value-scanner-diagram></lamad-value-scanner-diagram>
        </div>
      </section>

      <section id="governance" class="section">
        <div class="section-header">
          <h2>🛡️ Governance Architecture</h2>
          <p>Constitutional guardianship with flexible local adaptation: Immutable principles, autonomous stewards, community flexibility.</p>
        </div>

        <div class="demo-box dark">
          <lamad-governance-diagram></lamad-governance-diagram>
        </div>
      </section>

      <section id="hexagon" class="section">
        <div class="section-header">
          <h2>🔷 Hexagon Grid</h2>
          <p>A responsive honeycomb grid with canvas rendering and affinity-based coloring.</p>
        </div>

        <div class="demo-box medium">
          <lamad-hexagon-grid
            [nodes]="demoNodes"
            [itemsPerRow]="12"
            (nodeClick)="onHexClick($event)">
          </lamad-hexagon-grid>
        </div>
      </section>

      <footer class="footer">
        <p>Built with 💛 by Ethosengine | Elohim Protocol</p>
      </footer>
    </div>
  `,
  styles: [`
    .playground-container {
      min-height: 100vh;
      background: linear-gradient(135deg, #f9f8f4 0%, #f5f4f0 100%);
    }

    .header {
      padding: 3rem 2rem 2rem;
      max-width: 1200px;
      margin: 0 auto;
      text-align: center;
    }

    .header h1 {
      font-family: 'Inter', sans-serif;
      font-size: 3rem;
      font-weight: 800;
      color: #1c1917;
      margin: 0 0 0.5rem;
      letter-spacing: -0.02em;
    }

    .subtitle {
      font-size: 1.1rem;
      color: #78716c;
      margin: 0;
    }

    .nav {
      display: flex;
      gap: 1rem;
      justify-content: center;
      padding: 1rem;
      margin-bottom: 2rem;
      border-bottom: 1px solid #e7e5e4;
    }

    .nav a {
      padding: 0.5rem 1rem;
      color: #57534e;
      text-decoration: none;
      font-weight: 600;
      border-radius: 8px;
      transition: all 0.2s;
    }

    .nav a:hover {
      background: #c9a961;
      color: white;
    }

    .section {
      padding: 2rem;
      max-width: 1200px;
      margin: 0 auto 3rem;
    }

    .section-header {
      margin-bottom: 2rem;
      text-align: center;
    }

    .section-header h2 {
      font-size: 2rem;
      color: #1c1917;
      margin: 0 0 0.5rem;
    }

    .section-header p {
      font-size: 1rem;
      color: #78716c;
      max-width: 700px;
      margin: 0 auto;
      line-height: 1.6;
    }

    .demo-box {
      border-radius: 16px;
      padding: 3rem 2rem;
      margin-top: 1rem;
      min-height: 450px;
      display: flex;
      align-items: center;
      justify-content: center;
      box-shadow: 0 10px 40px rgba(0, 0, 0, 0.08);
      transition: transform 0.2s, box-shadow 0.2s;
    }

    .demo-box:hover {
      transform: translateY(-2px);
      box-shadow: 0 15px 50px rgba(0, 0, 0, 0.12);
    }

    .demo-box.light {
      background: linear-gradient(135deg, #ffffff 0%, #f9f8f4 100%);
      border: 1px solid #e7e5e4;
    }

    .demo-box.dark {
      background: linear-gradient(135deg, #292524 0%, #1c1917 100%);
      border: 1px solid #44403c;
    }

    .demo-box.medium {
      background: linear-gradient(135deg, #f5f4f0 0%, #e7e5e4 100%);
      border: 1px solid #d6d3d1;
    }

    .footer {
      padding: 3rem 2rem;
      text-align: center;
      color: #78716c;
      font-size: 0.9rem;
      border-top: 1px solid #e7e5e4;
      background: #f9f8f4;
    }

    @media (max-width: 768px) {
      .header h1 {
        font-size: 2rem;
      }

      .nav {
        flex-wrap: wrap;
      }

      .section {
        padding: 1rem;
      }

      .demo-box {
        padding: 2rem 1rem;
        min-height: 350px;
      }
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