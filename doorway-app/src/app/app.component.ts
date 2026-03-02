import { Component, OnInit, inject } from '@angular/core';
import { RouterOutlet } from '@angular/router';

import { DoorwayToolbarComponent } from './components/toolbar/doorway-toolbar.component';
import { NotificationToastComponent } from './core/notifications/notification-toast.component';
import { AuthStateService } from './services/auth-state.service';

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, DoorwayToolbarComponent, NotificationToastComponent],
  templateUrl: './app.component.html',
  styleUrl: './app.component.css'
})
export class AppComponent implements OnInit {
  private readonly authState = inject(AuthStateService);

  ngOnInit(): void {
    this.authState.init();
  }
}
