import { Component } from '@angular/core';
import { RouterLink } from '@angular/router';

/**
 * AvodahHomeComponent - Landing page for the Avodah work management pillar
 */
@Component({
  selector: 'app-avodah-home',
  standalone: true,
  imports: [RouterLink],
  template: `
    <div class="avodah-home">
      <h1>Avodah</h1>
      <p>Work management — projects, boards, and tasks.</p>
      <a routerLink="projects">View Projects</a>
    </div>
  `,
  styles: [
    `
      .avodah-home {
        padding: 2rem;
      }

      h1 {
        font-size: 2rem;
        margin-bottom: 0.5rem;
      }

      a {
        color: var(--lamad-accent-primary, #6366f1);
      }
    `,
  ],
})
export class AvodahHomeComponent {}
