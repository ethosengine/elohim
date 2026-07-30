import { Component, inject, signal } from '@angular/core';
import { RouterLink, RouterLinkActive } from '@angular/router';

import { AuthStateService } from '../../services/auth-state.service';
import { ThemeToggleComponent } from '../theme-toggle/theme-toggle.component';

@Component({
  selector: 'app-doorway-toolbar',
  standalone: true,
  imports: [RouterLink, RouterLinkActive, ThemeToggleComponent],
  templateUrl: './doorway-toolbar.component.html',
  styleUrl: './doorway-toolbar.component.css',
})
export class DoorwayToolbarComponent {
  readonly authState = inject(AuthStateService);
  readonly dropdownOpen = signal(false);

  toggleDropdown(event: Event): void {
    event.stopPropagation();
    this.dropdownOpen.update(open => !open);
  }

  closeDropdown(): void {
    this.dropdownOpen.set(false);
  }

  async onLogout(): Promise<void> {
    this.closeDropdown();
    await this.authState.logout();
  }
}
