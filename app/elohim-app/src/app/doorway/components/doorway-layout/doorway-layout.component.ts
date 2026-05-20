import { ChangeDetectionStrategy, Component } from '@angular/core';
import { RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';

@Component({
  selector: 'app-doorway-layout',
  standalone: true,
  imports: [RouterOutlet, RouterLink, RouterLinkActive],
  templateUrl: './doorway-layout.component.html',
  styleUrls: ['./doorway-layout.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DoorwayLayoutComponent {}
