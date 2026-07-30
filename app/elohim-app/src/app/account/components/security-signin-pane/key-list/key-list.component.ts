import { CommonModule, DatePipe, SlicePipe } from '@angular/common';
import { Component, inject } from '@angular/core';

import { AccountService } from '../../../services/account.service';

@Component({
  selector: 'app-key-list',
  standalone: true,
  imports: [CommonModule, DatePipe, SlicePipe],
  templateUrl: './key-list.component.html',
})
export class KeyListComponent {
  readonly account = inject(AccountService);
}
