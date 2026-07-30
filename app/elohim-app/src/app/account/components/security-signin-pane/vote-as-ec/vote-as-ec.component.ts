import { CommonModule, DatePipe, SlicePipe } from '@angular/common';
import { Component, inject, signal } from '@angular/core';

import { AccountService } from '../../../services/account.service';
import { RevocationService } from '../../../services/revocation.service';

@Component({
  selector: 'app-vote-as-ec',
  standalone: true,
  imports: [CommonModule, DatePipe, SlicePipe],
  templateUrl: './vote-as-ec.component.html',
})
export class VoteAsEcComponent {
  readonly account = inject(AccountService);
  private readonly rev = inject(RevocationService);

  /** Local error surfaced per-vote action; clears on next action. */
  readonly errorMsg = signal<string | null>(null);

  async vote(id: string, decision: 'approve' | 'reject'): Promise<void> {
    this.errorMsg.set(null);
    const ok = await this.rev.voteOnRecovery(id, decision);
    if (ok) {
      await this.account.refresh();
    } else {
      this.errorMsg.set(this.rev.error() ?? '[M5 stub — recovery voting lands in Phase 11]');
    }
  }
}
