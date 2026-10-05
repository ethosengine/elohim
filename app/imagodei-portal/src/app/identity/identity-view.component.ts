/**
 * The native portal's identity screen, as presentation only: beginning an
 * identity on this device, and then what it rests on. Inputs in, intents
 * out — no calls, no state of its own. The root and the device approval page
 * drive it with the shared identity controller; the development-only preview
 * drives it with sample states.
 *
 * Host words: on a person's own node the key is made and kept on "this
 * device". The begin step asks for one thing, the name the person wants
 * shown — no doorway and no password, because neither is needed.
 */

import { ChangeDetectionStrategy, Component, EventEmitter, Input, Output } from '@angular/core';
import {
  IDENTITY_CODE,
  beginCommand,
  standingLine,
  type IdentityPageState,
} from 'elohim-imagodei/identity-standing';

/** This host's name for the node answering. */
const HERE = { inSentence: 'this device' };

/** Why a begin did not happen, in this host's words. */
const BEGIN_REFUSAL: Record<string, string> = {
  [IDENTITY_CODE.nameMalformed]: 'A name is plain text on one line, at most 128 characters.',
  [IDENTITY_CODE.signingUnavailable]:
    'This device can’t reach its signer right now, so nothing was made. This is a wait: try again in a few minutes.',
  [IDENTITY_CODE.originRefused]:
    'This device takes this only from the portal it serves itself, so nothing was made.',
  [IDENTITY_CODE.secretTooShort]: 'A sign-in secret is at least 8 characters. Nothing was made.',
  [IDENTITY_CODE.secretTooLong]: 'A sign-in secret is at most 1024 characters. Nothing was made.',
  [IDENTITY_CODE.callerNotLocal]:
    'This page is open on a different machine from the one that would hold your key, so nothing was made. Beginning happens on that machine: open this page in a browser there, or run this in its terminal:',
};
const BEGIN_UNAVAILABLE =
  'This device can’t begin an identity right now, so nothing was made. Try again in a little while.';

@Component({
  selector: 'imagodei-portal-identity-view',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    @if (state.phase === 'standing') {
      <section
        class="identity"
        aria-labelledby="identity-title"
        data-testid="identity-standing-panel"
      >
        <h2 id="identity-title" class="identity__title">Your identity</h2>
        <p class="identity__standing" data-testid="identity-standing">{{ standing }}</p>
        @if (state.created?.session) {
          <p class="identity__muted">You’re signed in on this device.</p>
        }
        @if (canSignOut) {
          <button
            type="button"
            class="identity__quiet"
            data-testid="identity-sign-out"
            [disabled]="signingOut"
            (click)="signOut.emit()"
          >
            Sign out
          </button>
        }
      </section>
    } @else if (state.phase === 'begin' || state.phase === 'beginning') {
      <section class="identity" aria-labelledby="identity-begin-title" data-testid="identity-begin">
        <h2 id="identity-begin-title" class="identity__title">
          {{
            elsewhere
              ? 'Begin on the machine that will hold your key'
              : 'Begin your identity on this device'
          }}
        </h2>
        @if (!elsewhere) {
          <p>Your key is made and kept on this device. Nothing is sent to any host.</p>
          <p>Until you add another device, this device alone speaks for you.</p>
        }
        @if (returnNote) {
          <p class="identity__muted">{{ returnNote }}</p>
        }

        <form
          class="identity__form"
          (submit)="onSubmit($event, nameInput.value, secretInput.value)"
        >
          <label for="identity-name">The name you want shown</label>
          <input
            #nameInput
            id="identity-name"
            name="displayName"
            autocomplete="name"
            data-testid="identity-name"
            [value]="state.displayName ?? ''"
            [disabled]="state.phase === 'beginning'"
            [attr.aria-describedby]="refusal ? 'identity-refusal' : null"
            [attr.aria-invalid]="state.beginRefusal === nameMalformed ? 'true' : null"
          />
          <label for="identity-secret">Sign-in secret (optional)</label>
          <input
            #secretInput
            id="identity-secret"
            name="secret"
            type="password"
            autocomplete="new-password"
            data-testid="identity-secret"
            aria-describedby="identity-secret-what"
            [disabled]="state.phase === 'beginning'"
          />
          <p id="identity-secret-what" class="identity__muted identity__hint">
            Lets you sign in to this device from a browser later. At least 8 characters. You can
            leave it empty, and set or replace it any time by running
            <code>epr identity secret</code>
            on this device.
          </p>
          <button
            type="submit"
            data-testid="identity-begin-submit"
            [disabled]="state.phase === 'beginning'"
          >
            Begin
          </button>
        </form>

        @if (state.phase === 'beginning') {
          <p class="identity__status" role="status" data-testid="identity-beginning">
            Making your key on this device…
          </p>
        }
        @if (refusal; as sentence) {
          <p
            id="identity-refusal"
            class="identity__refusal"
            role="alert"
            data-testid="identity-refusal"
          >
            {{ sentence }}
          </p>
          @if (elsewhere) {
            <code class="identity__command" data-testid="identity-command">{{ command }}</code>
          }
        }
      </section>
    }
  `,
  styles: [
    `
      :host {
        display: block;
      }

      .identity {
        display: grid;
        gap: 0.75rem;
        min-inline-size: 0;
      }

      .identity p {
        margin: 0;
        line-height: 1.5;
      }

      .identity__title {
        margin: 0;
        font-size: 1.25rem;
      }

      .identity__muted {
        opacity: 0.85;
      }

      .identity__form {
        display: grid;
        gap: 0.5rem;
      }

      .identity__form input {
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

      .identity__form button {
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

      .identity__hint {
        font-size: 0.875rem;
      }

      .identity__quiet {
        justify-self: start;
        font: inherit;
        min-block-size: 44px;
        padding-inline: 1rem;
        color: ButtonText;
        background: ButtonFace;
        border: 1px solid color-mix(in srgb, currentColor 40%, transparent);
        border-radius: 6px;
      }

      .identity__form :disabled {
        opacity: 0.6;
        cursor: progress;
      }

      .identity__form :focus-visible {
        outline: 2px solid currentColor;
        outline-offset: 2px;
      }

      .identity__refusal {
        padding-inline-start: 0.75rem;
        border-inline-start: 3px solid currentColor;
      }

      /* A command is selected whole to copy; it may wrap to fit, since its
         words are pasted as one line whatever the screen showed. */
      .identity__command {
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
export class IdentityViewComponent {
  // Decorator inputs, like the rest of this bundle: its specs compile in JIT,
  // which does not see signal inputs.
  @Input({ required: true }) state!: IdentityPageState;
  /** Said under the begin step when the person will be brought back afterwards. */
  @Input() returnNote?: string;

  /** Offer sign-out on the standing panel (the signed-in root). */
  @Input() canSignOut = false;
  /** A sign-out is on its way. */
  @Input() signingOut = false;

  @Output() readonly begin = new EventEmitter<{ displayName: string; secret?: string }>();
  @Output() readonly signOut = new EventEmitter<void>();

  readonly nameMalformed = IDENTITY_CODE.nameMalformed;

  get standing(): string | undefined {
    return standingLine(this.state.standing, HERE, this.state.displayName);
  }

  /** The begin was refused because this page is on another machine. */
  get elsewhere(): boolean {
    return this.state.beginRefusal === IDENTITY_CODE.callerNotLocal;
  }

  get refusal(): string | undefined {
    const code = this.state.beginRefusal;
    if (!code) return undefined;
    return BEGIN_REFUSAL[code] ?? BEGIN_UNAVAILABLE;
  }

  /** The same begin, from that machine's terminal, with the name already typed. */
  get command(): string {
    return beginCommand(this.state.displayName?.trim() || 'Your name');
  }

  onSubmit(event: Event, displayName: string, secret: string): void {
    event.preventDefault();
    this.begin.emit(secret ? { displayName, secret } : { displayName });
  }
}
