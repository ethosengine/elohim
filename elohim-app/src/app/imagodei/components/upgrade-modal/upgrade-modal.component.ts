/**
 * UpgradeModalComponent - Extracted modal for "Join the Elohim Network".
 *
 * Displays benefits, how-it-works steps, and current session summary
 * to encourage visitors to upgrade to a Holochain identity.
 */

import { CommonModule } from '@angular/common';
import { Component, EventEmitter, Input, Output } from '@angular/core';

import { SessionHuman } from '../../models/session-human.model';

@Component({
  selector: 'app-upgrade-modal',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './upgrade-modal.component.html',
  styleUrls: ['./upgrade-modal.component.css'],
})
export class UpgradeModalComponent {
  @Input() session: SessionHuman | null = null;
  @Input() open = false;
  @Output() closed = new EventEmitter<void>();

  close(): void {
    this.closed.emit();
  }

  onOverlayClick(): void {
    this.close();
  }

  onOverlayKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      this.close();
    }
  }
}
