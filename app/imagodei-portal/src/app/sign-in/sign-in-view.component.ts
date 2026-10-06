/**
 * The native portal's sign-in, as presentation only: the sign-in word and
 * the sign-in secret set on this device, and the node's answers said
 * plainly. Inputs in, intents out — no calls. The root drives it with the
 * shared sign-in controller; the development-only preview with samples.
 *
 * No doorway is named here: this is the person's own device. When no
 * sign-in secret is set yet, the page says where it is set instead of
 * showing a form that cannot work.
 */

import { ChangeDetectionStrategy, Component, EventEmitter, Input, Output } from '@angular/core';
import type { SignInFailure, SignInPageState } from 'elohim-imagodei/node-sign-in';

/** What the page shows: the form, or where a secret is set when none is. */
export type SignInPage = 'form' | 'secret-unset';

/** The command that sets, or replaces, the sign-in secret on the node's own machine. */
export const SECRET_COMMAND = 'epr identity secret';

/** Why a sign-in did not go ahead, in this host's words. */
function refusalWords(refusal: SignInFailure | undefined): string | undefined {
  switch (refusal?.kind) {
    case undefined:
      return undefined;
    case 'invalid':
      return 'That sign-in word and secret don’t match. Check both and try again.';
    case 'slowed':
      return refusal.retryAfter
        ? `Too many tries for now. This is a wait: try again in ${refusal.retryAfter} seconds.`
        : 'Too many tries for now. This is a wait: try again in a little while.';
    case 'paused':
      return 'A witness attending you paused this sign-in.';
    case 'needs-session-key':
      return 'This device signs in only a browser that can keep a sign-in key, and this browser can’t keep one here. Open this page over a secure connection, or in a browser on this device.';
    case 'needs-secure-channel':
      return 'Signing in from another machine needs a secure connection to this device. Open this page over one, or sign in on this device itself.';
    case 'missing':
      return 'Enter your sign-in word and your sign-in secret.';
    case 'secret-unset':
      return undefined;
    default:
      return 'This device can’t sign you in right now. Try again in a little while.';
  }
}

@Component({
  selector: 'imagodei-portal-sign-in-view',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (page === 'secret-unset' || state.refusal?.kind === 'secret-unset') {
      <section
        class="sign-in"
        aria-labelledby="secret-unset-title"
        data-testid="sign-in-secret-unset"
      >
        <h2 id="secret-unset-title" class="sign-in__title">Set a sign-in secret first</h2>
        <p>
          Signing in from a browser needs a sign-in secret. It is set on the device that holds your
          key, by running this in its terminal:
        </p>
        <code class="sign-in__command" data-testid="sign-in-secret-command">
          {{ secretCommand }}
        </code>
        <p>This is also how a forgotten secret is replaced.</p>
      </section>
    } @else {
      <section class="sign-in" aria-labelledby="sign-in-title" data-testid="sign-in">
        <h2 id="sign-in-title" class="sign-in__title">Sign in</h2>
        <p>Sign in with your sign-in word and the sign-in secret set on this device.</p>
        @if (returnNote) {
          <p class="sign-in__muted">{{ returnNote }}</p>
        }

        <form class="sign-in__form" (submit)="onSubmit($event, word.value, secret.value)">
          <label for="sign-in-word">Sign-in word</label>
          <input
            #word
            id="sign-in-word"
            name="identifier"
            autocomplete="username"
            autocapitalize="none"
            spellcheck="false"
            data-testid="sign-in-word"
            [value]="state.identifier ?? ''"
            [disabled]="busy"
          />
          <label for="sign-in-secret">Sign-in secret</label>
          <input
            #secret
            id="sign-in-secret"
            name="password"
            type="password"
            autocomplete="current-password"
            data-testid="sign-in-secret"
            [disabled]="busy"
            [attr.aria-describedby]="refusal ? 'sign-in-refusal' : null"
          />
          <button type="submit" data-testid="sign-in-submit" [disabled]="busy">Sign in</button>
        </form>

        @if (state.phase === 'signing-in') {
          <p class="sign-in__status" role="status" data-testid="sign-in-busy">Signing in…</p>
        }
        @if (refusal; as sentence) {
          <div
            id="sign-in-refusal"
            class="sign-in__refusal"
            role="alert"
            data-testid="sign-in-refusal"
          >
            <p>{{ sentence }}</p>
            @if (pausedReason; as reason) {
              <p class="sign-in__muted">The reason given:</p>
              <p class="sign-in__reason" data-testid="sign-in-paused-reason">{{ reason }}</p>
            }
          </div>
        }
      </section>
    }
  `,
  styles: [
    `
      :host {
        display: block;
      }

      .sign-in {
        display: grid;
        gap: 0.75rem;
        min-inline-size: 0;
      }

      .sign-in p {
        margin: 0;
        line-height: 1.5;
      }

      .sign-in__title {
        margin: 0;
        font-size: 1.25rem;
      }

      .sign-in__muted {
        opacity: 0.85;
      }

      .sign-in__form {
        display: grid;
        gap: 0.5rem;
      }

      .sign-in__form input {
        font: inherit;
        padding: 0.5rem 0.75rem;
        min-block-size: 44px;
        box-sizing: border-box;
        inline-size: 100%;
        color: FieldText;
        background: Field;
        border: 1px solid color-mix(in srgb, currentColor 40%, transparent);
        border-radius: 6px;
      }

      .sign-in__form button {
        font: inherit;
        font-weight: 600;
        justify-self: end;
        min-block-size: 44px;
        padding-inline: 1.25rem;
        color: ButtonText;
        background: ButtonFace;
        border: 1px solid ButtonText;
        border-radius: 6px;
      }

      .sign-in__form :disabled {
        opacity: 0.6;
        cursor: progress;
      }

      .sign-in__form :focus-visible {
        outline: 2px solid currentColor;
        outline-offset: 2px;
      }

      .sign-in__refusal {
        display: grid;
        gap: 0.5rem;
        padding-inline-start: 0.75rem;
        border-inline-start: 3px solid color-mix(in srgb, currentColor 30%, transparent);
      }

      .sign-in__reason {
        overflow-wrap: anywhere;
      }

      .sign-in__command {
        display: block;
        padding: 0.75rem 1rem;
        overflow-wrap: anywhere;
        user-select: all;
        font-family: ui-monospace, monospace;
        background: color-mix(in srgb, currentColor 6%, transparent);
        border-radius: 6px;
      }
    `,
  ],
})
export class SignInViewComponent {
  // Decorator inputs, like the rest of this bundle: its specs compile in JIT,
  // which does not see signal inputs.
  @Input({ required: true }) state!: SignInPageState;
  @Input() page: SignInPage = 'form';
  /** Said when the person will be brought straight back afterwards. */
  @Input() returnNote?: string;

  @Output() readonly signIn = new EventEmitter<{ identifier: string; password: string }>();

  readonly secretCommand = SECRET_COMMAND;

  get busy(): boolean {
    return this.state.phase === 'signing-in';
  }

  get refusal(): string | undefined {
    return refusalWords(this.state.refusal);
  }

  get pausedReason(): string | undefined {
    const refusal = this.state.refusal;
    return refusal?.kind === 'paused' ? refusal.reason : undefined;
  }

  onSubmit(event: Event, identifier: string, password: string): void {
    event.preventDefault();
    this.signIn.emit({ identifier, password });
  }
}
