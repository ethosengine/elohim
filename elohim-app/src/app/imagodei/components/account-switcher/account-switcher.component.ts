/**
 * AccountSwitcherComponent - Multi-account selector for lock screen.
 *
 * Shows a list of saved accounts with the active one highlighted.
 * Allows switching between accounts (requires restart) and adding new ones.
 */

import { CommonModule } from '@angular/common';
import { Component, OnInit, inject, signal, output } from '@angular/core';

import { TauriAuthService, type AccountSummary } from '../../services/tauri-auth.service';

@Component({
  selector: 'app-account-switcher',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './account-switcher.component.html',
  styleUrls: ['./account-switcher.component.css'],
})
export class AccountSwitcherComponent implements OnInit {
  private readonly tauriAuth = inject(TauriAuthService);

  readonly accounts = signal<AccountSummary[]>([]);
  readonly isLoading = signal(false);

  /** Emitted when user wants to add a new account */
  readonly addAccount = output<void>();

  /** Emitted when account switch requires restart */
  readonly needsRestart = output<void>();

  /** Emitted when user cancels the switcher */
  readonly cancelled = output<void>();

  async ngOnInit(): Promise<void> {
    await this.loadAccounts();
  }

  async loadAccounts(): Promise<void> {
    this.isLoading.set(true);
    try {
      const accounts = await this.tauriAuth.listAccounts();
      this.accounts.set(accounts);
    } finally {
      this.isLoading.set(false);
    }
  }

  async onSwitchAccount(humanId: string): Promise<void> {
    this.isLoading.set(true);
    try {
      const result = await this.tauriAuth.switchAccount(humanId);
      if (result.needsRestart) {
        this.needsRestart.emit();
      }
    } finally {
      this.isLoading.set(false);
    }
  }

  onAddAccount(): void {
    this.addAccount.emit();
  }

  onCancel(): void {
    this.cancelled.emit();
  }
}
