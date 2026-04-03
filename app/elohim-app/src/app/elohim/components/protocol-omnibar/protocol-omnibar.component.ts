import { CommonModule } from '@angular/common';
import { Component, EventEmitter, Input, Output } from '@angular/core';
import { RouterModule } from '@angular/router';

export interface OmnibarSteward {
  humanId: string;
  displayName: string;
  ratio: number;
}

export type OmnibarState = 'pill' | 'expanded';

/**
 * ProtocolOmnibarComponent — The protocol's equivalent of a browser address bar.
 *
 * Pill state: tiny badge proving "this is protocol-delivered." Like the SSL padlock.
 * Expanded state: EPR address, reach, stewards, delivery source. Like viewing a cert.
 * Actions: drill-down to /resource/{id} governance hub, report, feedback.
 *
 * No lamad dependencies. Reads from @Input() only.
 * Designed to eventually become a standalone web component.
 */
@Component({
  selector: 'app-protocol-omnibar',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './protocol-omnibar.component.html',
  styleUrls: ['./protocol-omnibar.component.css'],
})
export class ProtocolOmnibarComponent {
  @Input() contentId = '';
  @Input() contentAddress = '';
  @Input() stewards: OmnibarSteward[] = [];
  @Input() reach = '';
  @Input() deliverySource = '';

  /** Show a back/exit button (e.g., when used as focus mode in lesson view) */
  @Input() showBack = false;

  @Output() backRequested = new EventEmitter<void>();
  @Output() reportRequested = new EventEmitter<void>();
  @Output() feedbackRequested = new EventEmitter<void>();

  state: OmnibarState = 'pill';
  showActions = false;
  copyFeedback = '';
  readonly copyIcon = '\u{1F4CB}';

  get truncatedAddress(): string {
    if (!this.contentAddress) return '';
    if (this.contentAddress.length <= 16) return this.contentAddress;
    return `${this.contentAddress.slice(0, 7)}...${this.contentAddress.slice(-6)}`;
  }

  get reachIcon(): string {
    switch (this.reach) {
      case 'private':
      case 'self':
        return '\u{1F512}';
      case 'local':
      case 'community':
        return '\u{25CE}';
      case 'commons':
      case 'public':
      default:
        return '\u{25CB}\u{25CB}\u{25CB}';
    }
  }

  get inspectRoute(): string[] {
    return ['/resource', this.contentId];
  }

  expand(): void {
    this.state = 'expanded';
    this.showActions = false;
  }

  collapse(): void {
    this.state = 'pill';
    this.showActions = false;
  }

  toggleActions(): void {
    this.showActions = !this.showActions;
  }

  async copyAddress(): Promise<void> {
    if (!this.contentAddress) return;
    try {
      await navigator.clipboard.writeText(this.contentAddress);
      this.copyFeedback = 'Copied';
      setTimeout(() => (this.copyFeedback = ''), 1500);
    } catch {
      this.copyFeedback = 'Failed';
      setTimeout(() => (this.copyFeedback = ''), 1500);
    }
  }
}
