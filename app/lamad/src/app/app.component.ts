import { Component } from '@angular/core';
import { RouterOutlet } from '@angular/router';

@Component({
  selector: 'lamad-root',
  standalone: true,
  imports: [RouterOutlet],
  template: `
    <main>
      <h1>Lamad</h1>
      <p>Bundle scaffold — B18 lands the pillar code here.</p>
      <router-outlet />
    </main>
  `,
})
export class AppComponent {}
