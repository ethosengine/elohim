import { CommonModule } from '@angular/common';
import { Component, input, output, computed } from '@angular/core';

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
