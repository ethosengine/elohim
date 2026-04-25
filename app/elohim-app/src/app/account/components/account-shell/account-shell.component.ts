/**
 * Account Shell — placeholder stub.
 * Full implementation in Task 18.
 */

import { Component } from '@angular/core';
import { RouterOutlet } from '@angular/router';

@Component({
  selector: 'app-account-shell',
  standalone: true,
  imports: [RouterOutlet],
  template: `
    <router-outlet />
  `,
})
export class AccountShellComponent {}
