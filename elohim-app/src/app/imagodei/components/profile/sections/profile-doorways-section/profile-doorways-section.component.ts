import { CommonModule } from '@angular/common';
import { Component, input, output, signal } from '@angular/core';

import { type DoorwayWithHealth } from '../../../../models/doorway.model';

export interface DoorwayRegistrationContext {
  identifier: string | null;
  registeredSince: string | null;
  credentialStorage: 'browser' | 'device' | null;
}

@Component({
  selector: 'app-profile-doorways-section',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './profile-doorways-section.component.html',
  styleUrls: ['./profile-doorways-section.component.css'],
})
export class ProfileDoorwaysSectionComponent {
  readonly doorways = input.required<DoorwayWithHealth[]>();
  readonly activeDoorwayId = input<string | null>(null);
  readonly registrationContext = input<DoorwayRegistrationContext | null>(null);

  readonly setAsPrimary = output<string>();
  readonly validateDoorway = output<string>();
  readonly addDoorway = output<string>();

  readonly showAddDoorway = signal(false);
  readonly newDoorwayUrl = signal('');
  readonly validating = signal(false);
  readonly validationError = signal<string | null>(null);
  readonly validationResult = signal<{ name: string; url: string } | null>(null);

  onDoorwayUrlInput(event: Event): void {
    const input = event.target as HTMLInputElement;
    this.newDoorwayUrl.set(input.value);
  }

  toggleAddDoorway(): void {
    this.showAddDoorway.update(v => !v);
    this.validationError.set(null);
    this.validationResult.set(null);
    this.newDoorwayUrl.set('');
  }

  onValidate(): void {
    const url = this.newDoorwayUrl().trim();
    if (url) {
      this.validateDoorway.emit(url);
    }
  }

  onAdd(): void {
    const result = this.validationResult();
    if (result) {
      this.addDoorway.emit(result.url);
      this.toggleAddDoorway();
    }
  }

  formatDate(iso: string): string {
    try {
      return new Date(iso).toLocaleDateString(undefined, {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
      });
    } catch {
      return iso;
    }
  }
}
