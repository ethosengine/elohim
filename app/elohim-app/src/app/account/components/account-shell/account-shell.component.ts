import { Component, inject, ChangeDetectionStrategy } from '@angular/core';
import { RouterOutlet, RouterLink, RouterLinkActive } from '@angular/router';

import { AccountService } from '../../services/account.service';

@Component({
  selector: 'app-account-shell',
  standalone: true,
  imports: [RouterOutlet, RouterLink, RouterLinkActive],
  templateUrl: './account-shell.component.html',
  changeDetection: ChangeDetectionStrategy.Eager,
  styleUrl: './account-shell.component.css',
})
export class AccountShellComponent {
  readonly account = inject(AccountService);
}
