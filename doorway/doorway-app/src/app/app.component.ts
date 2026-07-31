import { ChangeDetectionStrategy, Component, OnInit, inject } from '@angular/core';
import { RouterOutlet } from '@angular/router';

import { DoorwayToolbarComponent } from './components/toolbar/doorway-toolbar.component';
import { NotificationToastComponent } from './core/notifications/notification-toast.component';
import { AuthStateService } from './services/auth-state.service';

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, DoorwayToolbarComponent, NotificationToastComponent],
  templateUrl: './app.component.html',
  styleUrl: './app.component.css',
  // OnPush-unsafe: ROOT view — an OnPush root is never marked dirty, so a global
  // ApplicationRef tick skips it and freezes change detection for the whole
  // non-signal tree. See backlog-onpush-eager-debt-inventory.
  // eslint-disable-next-line @angular-eslint/prefer-on-push-component-change-detection
  changeDetection: ChangeDetectionStrategy.Eager,
})
export class AppComponent implements OnInit {
  private readonly authState = inject(AuthStateService);

  ngOnInit(): void {
    this.authState.init();
  }
}
