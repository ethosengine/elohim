import { Component, OnInit, inject } from '@angular/core';
import { RouterOutlet } from '@angular/router';

import { DoorwayToolbarComponent } from './components/toolbar/doorway-toolbar.component';
import { AuthStateService } from './services/auth-state.service';

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, DoorwayToolbarComponent],
  templateUrl: './app.component.html',
  styleUrl: './app.component.css'
})
export class AppComponent implements OnInit {
  private readonly authState = inject(AuthStateService);

  ngOnInit(): void {
    this.authState.init();
  }
}
