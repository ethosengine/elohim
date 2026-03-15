import { CommonModule } from '@angular/common';
import {
  Component,
  EventEmitter,
  Input,
  OnChanges,
  Output,
  SimpleChanges,
} from '@angular/core';

/**
 * FaceCardComponent - Identity card with initials-based avatar
 *
 * Shows a person's identity with a colored circle derived from their name.
 * Supports optional profile photo, subtitle (household name), and role tag.
 */
@Component({
  selector: 'app-face-card',
  standalone: true,
  imports: [CommonModule],
  template: `
    <button
      class="face-card"
      type="button"
      (click)="selected.emit(humanId)"
      [attr.aria-label]="'Select ' + displayName"
    >
      <div class="avatar" [style.background-color]="avatarColor">
        @if (profilePhotoUrl) {
          <img
            [src]="profilePhotoUrl"
            [alt]="displayName"
            class="avatar-img"
          />
        } @else {
          <span class="initials">{{ initials }}</span>
        }
      </div>
      <span class="name">{{ displayName }}</span>
      @if (subtitle) {
        <span class="subtitle">{{ subtitle }}</span>
      }
      @if (roleTag) {
        <span class="role-tag" [class.leader]="roleTag === 'leader'">{{
          roleTag
        }}</span>
      }
    </button>
  `,
  styles: [
    `
      .face-card {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: 0.375rem;
        padding: 0.75rem;
        border: 1px solid var(--border, #e5e5e5);
        border-radius: 12px;
        background: var(--surface, #fff);
        cursor: pointer;
        transition:
          transform 0.15s,
          box-shadow 0.15s;
        width: 100%;
        font-family: inherit;
        color: inherit;
      }

      .face-card:hover {
        transform: translateY(-2px);
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
      }

      .face-card:focus-visible {
        outline: 2px solid var(--primary, #6366f1);
        outline-offset: 2px;
      }

      .avatar {
        width: 56px;
        height: 56px;
        border-radius: 50%;
        display: flex;
        align-items: center;
        justify-content: center;
        overflow: hidden;
        flex-shrink: 0;
      }

      .avatar-img {
        width: 100%;
        height: 100%;
        object-fit: cover;
      }

      .initials {
        color: #fff;
        font-weight: 600;
        font-size: 1.125rem;
        text-transform: uppercase;
        user-select: none;
      }

      .name {
        font-size: 0.8125rem;
        font-weight: 500;
        text-align: center;
        color: var(--text-primary, #1a1a1a);
        line-height: 1.3;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        max-width: 100%;
      }

      .subtitle {
        font-size: 0.6875rem;
        color: var(--text-secondary, #666);
        text-align: center;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
        max-width: 100%;
      }

      .role-tag {
        font-size: 0.625rem;
        padding: 0.125rem 0.5rem;
        border-radius: 9999px;
        background: var(--surface-elevated, #f5f5f5);
        color: var(--text-secondary, #666);
        text-transform: capitalize;
      }

      .role-tag.leader {
        background: var(--primary, #6366f1);
        color: #fff;
      }

      @media (prefers-color-scheme: dark) {
        .face-card {
          background: var(--surface, #1a1a1a);
          border-color: var(--border, #333);
        }

        .name {
          color: var(--text-primary, #f5f5f5);
        }

        .role-tag {
          background: var(--surface-elevated, #2a2a2a);
        }
      }
    `,
  ],
})
export class FaceCardComponent implements OnChanges {
  @Input({ required: true }) humanId!: string;
  @Input({ required: true }) displayName!: string;
  @Input() profilePhotoUrl: string | null = null;
  @Input() subtitle: string | null = null;
  @Input() roleTag: string | null = null;

  @Output() selected = new EventEmitter<string>();

  initials = '';
  avatarColor = '';

  private static readonly COLORS = [
    '#6366f1',
    '#8b5cf6',
    '#a855f7',
    '#d946ef',
    '#ec4899',
    '#f43f5e',
    '#ef4444',
    '#f97316',
    '#eab308',
    '#84cc16',
    '#22c55e',
    '#14b8a6',
    '#06b6d4',
    '#0ea5e9',
    '#3b82f6',
    '#6366f1',
  ];

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['displayName']) {
      this.initials = FaceCardComponent.computeInitials(this.displayName);
      this.avatarColor = FaceCardComponent.computeColor(this.displayName);
    }
  }

  static computeInitials(name: string): string {
    if (!name) return '?';
    const parts = name.trim().split(/\s+/);
    if (parts.length >= 2) {
      return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
    }
    return parts[0][0].toUpperCase();
  }

  static computeColor(name: string): string {
    if (!name) return FaceCardComponent.COLORS[0];
    let hash = 0;
    for (let i = 0; i < name.length; i++) {
      hash = (hash << 5) - hash + name.charCodeAt(i);
      hash |= 0; // Convert to 32-bit integer
    }
    const index =
      Math.abs(hash) % FaceCardComponent.COLORS.length;
    return FaceCardComponent.COLORS[index];
  }
}
