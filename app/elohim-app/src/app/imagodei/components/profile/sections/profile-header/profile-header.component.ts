import { CommonModule } from '@angular/common';
import { Component, input, output, computed, ChangeDetectionStrategy } from '@angular/core';

// @coverage: 5.9% (2026-02-24)

import {
  type HumanProfile,
  type IdentityMode,
  getInitials,
} from '../../../../models/identity.model';

@Component({
  selector: 'app-profile-header',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './profile-header.component.html',
  // OnPush-unsafe: clipboard write + setTimeout flag mutation — see backlog-onpush-eager-debt-inventory
  changeDetection: ChangeDetectionStrategy.Eager,
  styleUrls: ['./profile-header.component.css'],
})
export class ProfileHeaderComponent {
  readonly displayName = input.required<string>();
  readonly mode = input.required<IdentityMode>();
  readonly profile = input<HumanProfile | null>(null);
  readonly did = input<string | null>(null);
  readonly canEdit = input(false);

  readonly editProfile = output<void>();

  readonly initials = computed(() => getInitials(this.displayName()));

  didCopied = false;

  async copyDid(): Promise<void> {
    const did = this.did();
    if (!did) return;
    try {
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside try/catch SSR fallback
      await navigator.clipboard.writeText(did);
      this.didCopied = true;
      setTimeout(() => (this.didCopied = false), 2000);
    } catch {
      // Clipboard API not available
    }
  }

  truncateDid(did: string): string {
    if (did.length <= 32) return did;
    return `${did.substring(0, 20)}...${did.substring(did.length - 8)}`;
  }
}
