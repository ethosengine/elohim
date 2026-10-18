import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, OnInit, inject, signal } from '@angular/core';
import {
  parseAccountBindChallenge,
  returnFragment,
  type AccountBindChallenge,
} from 'elohim-imagodei';
import { ACCOUNT_BIND_PORT } from './account-bind-port';

/** An absolute web return address, restricted to the doorway named in the challenge. */
export function accountBindReturn(value: string, audience: string): URL | null {
  try {
    const url = new URL(value);
    const local = url.hostname === 'localhost' || url.hostname === '127.0.0.1';
    if (url.protocol !== 'https:' && !(url.protocol === 'http:' && local)) return null;
    if (url.username || url.password) return null;
    // The audience names a host, with or without a port; a doorway id is not a host.
    const namesHost = audience.includes('.') || audience.split(':')[0] === 'localhost';
    if (namesHost && url.host !== audience && url.hostname !== audience) return null;
    url.hash = '';
    return url;
  } catch {
    return null;
  }
}

@Component({
  selector: 'imagodei-portal-account-bind',
  standalone: true,
  imports: [CommonModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <section data-testid="account-bind-view" aria-labelledby="account-bind-title">
      <h2 id="account-bind-title">Move your account onto this node's key</h2>
      <ng-container *ngIf="challenge() as request">
        <p>
          <span data-testid="account-bind-audience">{{ request.audience }}</span>
          is asking to move the account
          <span data-testid="account-bind-identifier">{{ request.identifier }}</span>
          onto this node's key.
        </p>
        <p>
          This request expires at
          <time [attr.datetime]="request.expires">{{ request.expires }}</time>
          .
        </p>
        <p>The doorway will stop hosting a cell for you. This node will hold your key.</p>
      </ng-container>
      <p *ngIf="error()" data-testid="account-bind-error" role="alert">
        {{ error() }}
      </p>
      <button
        type="button"
        data-testid="account-bind-approve"
        [disabled]="!ready() || busy() || !!error()"
        (click)="approve()"
      >
        Approve
      </button>
      <button
        type="button"
        data-testid="account-bind-decline"
        [disabled]="!returnTo || busy()"
        (click)="decline()"
      >
        Decline
      </button>
    </section>
  `,
})
export class AccountBindComponent implements OnInit {
  private readonly port = inject(ACCOUNT_BIND_PORT);
  readonly challenge = signal<AccountBindChallenge | null>(null);
  readonly error = signal('');
  readonly ready = signal(false);
  readonly busy = signal(false);
  returnTo: URL | null = null;
  private text = '';

  async ngOnInit(): Promise<void> {
    const params = new URLSearchParams(window.location.search);
    // A malformed challenge carries no usable audience. Only a web URL may
    // receive that refusal; a parsed challenge adds the audience restriction.
    this.returnTo = accountBindReturn(params.get('return') ?? '', '');
    try {
      const encoded = params.get('challenge') ?? '';
      if (!/^[A-Za-z0-9_-]+$/.test(encoded) || encoded.length > 1366) throw new Error();
      this.text = new TextDecoder('utf-8', { fatal: true }).decode(
        Uint8Array.from(atob(encoded.replace(/-/g, '+').replace(/_/g, '/')), c => c.charCodeAt(0))
      );
    } catch {
      this.refuse('NOT_A_BIND_CHALLENGE', 'This is not a valid account move request.');
      return;
    }
    // Read the audience before checking expiry, so even an expired request can
    // return its refusal only to the doorway it names.
    const parsed = parseAccountBindChallenge(this.text, () => Number.NEGATIVE_INFINITY);
    if ('refused' in parsed) {
      this.refuse('NOT_A_BIND_CHALLENGE', 'This is not a valid account move request.');
      return;
    }
    this.challenge.set(parsed);
    this.returnTo = accountBindReturn(params.get('return') ?? '', parsed.audience);
    if (!this.returnTo) {
      this.error.set(
        'The return address must use HTTPS (or local HTTP) and match the asking doorway (INVALID_RETURN_URL).'
      );
      return;
    }
    const current = parseAccountBindChallenge(this.text);
    if ('refused' in current) {
      this.refuse(current.refused, 'This request has expired. Start again at your doorway.');
      return;
    }
    try {
      const standing = await this.port.standing();
      if (!standing.ok && standing.status === 401) {
        this.port.signIn();
        return;
      }
      if (!standing.ok) {
        this.refuse(
          'NODE_UNREACHABLE',
          'This node could not confirm your sign-in. Try opening this request again.'
        );
        return;
      }
      if (!(await this.port.hasSessionKey())) {
        this.port.signIn();
        return;
      }
      this.ready.set(true);
    } catch {
      this.refuse(
        'NODE_UNREACHABLE',
        'This node could not confirm your sign-in. Try opening this request again.'
      );
    }
  }

  async approve(): Promise<void> {
    if (!this.ready() || this.busy() || this.error() || !this.returnTo) return;
    const parsed = parseAccountBindChallenge(this.text);
    if ('refused' in parsed) {
      this.refuse(parsed.refused, 'This request has expired. Start again at your doorway.');
      return;
    }
    this.busy.set(true);
    try {
      const result = await this.port.sign(this.text);
      if (result.ok) {
        this.port.navigate(
          this.returnTo.href + returnFragment({ ...result.body, challengeId: parsed.nonce })
        );
      } else {
        const body = result.body as { code?: string; error?: string } | null;
        this.refuse(
          body?.code ?? 'NODE_UNREACHABLE',
          body?.error ?? 'This node could not sign the request.'
        );
      }
    } catch {
      this.refuse('NODE_UNREACHABLE', 'This node could not sign the request.');
    } finally {
      this.busy.set(false);
    }
  }

  decline(): void {
    if (!this.busy()) this.refuse('DECLINED', 'You declined this account move.');
  }

  private refuse(code: string, message: string): void {
    this.error.set(`${message} (${code})`);
    if (this.returnTo) this.port.navigate(this.returnTo.href + returnFragment({ refused: code }));
  }
}
