import { CommonModule } from '@angular/common';
import { Component, inject, signal } from '@angular/core';

import { AccountService } from '../../../services/account.service';
import { RevocationService } from '../../../services/revocation.service';

@Component({
  selector: 'app-self-revoke',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './self-revoke.component.html',
})
export class SelfRevokeComponent {
  readonly account = inject(AccountService);
  private readonly rev = inject(RevocationService);

  readonly confirming = signal(false);
  readonly working = signal(false);

  /** Local error that surfaces Phase 11 stub messaging or any other failure. */
  readonly errorMsg = signal<string | null>(null);

  async confirm(): Promise<void> {
    const pubKey = this.account.account()?.activeKeyRotation?.newAgentPubkey;
    if (!pubKey) return;

    this.working.set(true);
    this.errorMsg.set(null);

    const ok = await this.rev.selfRevoke(pubKey);
    if (ok) {
      await this.account.refresh();
      this.confirming.set(false);
    } else {
      // Service sets rev.error; mirror it here so the template can render it
      // without depending on a shared service signal that could be overwritten.
      this.errorMsg.set(this.rev.error() ?? '[M5 stub — full self-revocation lands in Phase 11]');
    }

    this.working.set(false);
  }
}
