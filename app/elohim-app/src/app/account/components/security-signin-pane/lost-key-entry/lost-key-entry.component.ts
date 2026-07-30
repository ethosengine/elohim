import { CommonModule } from '@angular/common';
import { Component, inject, ChangeDetectionStrategy } from '@angular/core';
import { Router } from '@angular/router';

import { AccountService } from '../../../services/account.service';

@Component({
  selector: 'app-lost-key-entry',
  standalone: true,
  imports: [CommonModule],
  changeDetection: ChangeDetectionStrategy.Eager,
  templateUrl: './lost-key-entry.component.html',
})
export class LostKeyEntryComponent {
  readonly account = inject(AccountService);
  private readonly router = inject(Router);

  goRecovery(): void {
    void this.router.navigate(['/identity/recover']);
  }
}
