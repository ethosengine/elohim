import { CommonModule } from '@angular/common';
import { Component, inject } from '@angular/core';

import { AccountService } from '../../services/account.service';

import { KeyListComponent } from './key-list/key-list.component';
import { LostKeyEntryComponent } from './lost-key-entry/lost-key-entry.component';
import { SelfRevokeComponent } from './self-revoke/self-revoke.component';
import { VoteAsEcComponent } from './vote-as-ec/vote-as-ec.component';

@Component({
  selector: 'app-security-signin-pane',
  standalone: true,
  imports: [
    CommonModule,
    KeyListComponent,
    SelfRevokeComponent,
    VoteAsEcComponent,
    LostKeyEntryComponent,
  ],
  templateUrl: './security-signin-pane.component.html',
  styleUrl: './security-signin-pane.component.css',
})
export class SecuritySigninPaneComponent {
  readonly account = inject(AccountService);
}
