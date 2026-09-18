/**
 * ExternalEmbedComponent — the visible seam where outside web content meets a governed place.
 *
 * A raw third-party `<iframe>` inside a notarized ContentNode lets an outside party watch a
 * person who never asked it to, and nothing on the page says so. This component is the
 * protection surface for that boundary (slice 1 of
 * `genesis/data/timeline/backlog/legacy-web-content-projected-inward.md`, direction 4):
 *
 *   - NOTHING third-party is fetched before the person asks. No iframe in the DOM, no
 *     remote thumbnail, no preconnect/prefetch hint. An optional poster must be a LOCAL
 *     path; anything else is ignored, so a poster can never smuggle an outside request in.
 *   - The disclosed host is derived from the parsed URL, never hand-typed — the label
 *     cannot lie about where the bytes come from.
 *   - Only hosts in RECOGNISED_PROVIDERS are ever embedded; an unknown host renders an
 *     honest closed state instead of an arbitrary frame.
 *   - After loading, a small caption keeps the seam visible. The player is theirs, the
 *     page is ours, and the person can see the line.
 *
 * Consent here is per-embed and per-page-view. It is deliberately NOT persisted — where a
 * consent for a live outside load should be recorded (and at what reach it is refused) is
 * an open question in the backlog above, not something the UI layer may decide.
 */

import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  computed,
  effect,
  inject,
  input,
  signal,
  viewChild,
} from '@angular/core';
import { DomSanitizer, type SafeResourceUrl } from '@angular/platform-browser';

/** An outside host this place is willing to open, and how it prefers to be opened. */
interface EmbedProvider {
  /** Exact hostnames, matched literally — no suffix matching, no wildcards. */
  readonly hosts: readonly string[];
  /** Host to load from instead, when the provider offers a less-watchful one. */
  readonly privacyHost?: string;
}

/**
 * The short list. Adding a row here is a decision about who may observe a person inside a
 * governed place — make it deliberately, and keep the hosts exact.
 */
const RECOGNISED_PROVIDERS: readonly EmbedProvider[] = [
  {
    hosts: ['www.youtube.com', 'www.youtube-nocookie.com'],
    privacyHost: 'www.youtube-nocookie.com',
  },
];

/** Unique-per-instance ids so two facades on one page keep distinct aria wiring. */
let facadeSequence = 0;

@Component({
  selector: 'app-external-embed',
  standalone: true,
  imports: [],
  templateUrl: './external-embed.component.html',
  styleUrl: './external-embed.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ExternalEmbedComponent {
  /** The outside embed address, exactly as authored. */
  readonly url = input.required<string>();
  /** What this is, in the person's words — also the iframe's accessible name. */
  readonly embedTitle = input.required<string>();
  /** How the provider calls itself, for the action label ("Play from YouTube"). */
  readonly providerName = input.required<string>();
  /** Optional LOCAL poster path (must start with a single `/`); anything else is dropped. */
  readonly poster = input<string | null>(null);

  private readonly sanitizer = inject(DomSanitizer);
  private readonly playerFrame = viewChild<ElementRef<HTMLIFrameElement>>('playerFrame');

  readonly disclosureId = `external-embed-${++facadeSequence}-disclosure`;

  private readonly loaded = signal(false);
  /** True once the person has asked for the outside content on this page view. */
  readonly activated = this.loaded.asReadonly();

  private readonly parsedUrl = computed<URL | null>(() => {
    try {
      return new URL(this.url());
    } catch {
      return null;
    }
  });

  /** The host as the browser parses it — the one label on this surface that cannot lie. */
  readonly sourceHost = computed(() => this.parsedUrl()?.hostname ?? '');

  private readonly provider = computed<EmbedProvider | null>(() => {
    const parsed = this.parsedUrl();
    if (parsed?.protocol !== 'https:') return null;
    return (
      RECOGNISED_PROVIDERS.find(candidate => candidate.hosts.includes(parsed.hostname)) ?? null
    );
  });

  /** Whether this address is one this place is willing to open at all. */
  readonly recognised = computed(() => this.provider() !== null);

  /** A poster is only honoured when it is a same-origin path — never an outside fetch. */
  readonly localPoster = computed(() => {
    const candidate = this.poster()?.trim();
    if (!candidate) return null;
    return candidate.startsWith('/') && !candidate.startsWith('//') ? candidate : null;
  });

  private readonly embedUrl = computed<URL | null>(() => {
    const parsed = this.parsedUrl();
    const provider = this.provider();
    if (!parsed || !provider) return null;
    const target = new URL(parsed.href);
    if (provider.privacyHost) target.hostname = provider.privacyHost;
    // The person pressed "play", so play — the click is the gesture the browser needs.
    target.searchParams.set('autoplay', '1');
    return target;
  });

  /** The host actually serving the player, named in the caption under it. */
  readonly playingHost = computed(() => this.embedUrl()?.hostname ?? '');

  readonly playerSrc = computed<SafeResourceUrl | null>(() => {
    const target = this.embedUrl();
    if (!target) return null;
    // Safe: embedUrl() is null unless the address parsed as https with an EXACT hostname
    // match in RECOGNISED_PROVIDERS, and the host is then rewritten to the provider's own
    // privacy host. No caller-supplied string reaches this call unvalidated.
    // eslint-disable-next-line sonarjs/no-angular-bypass-sanitization
    return this.sanitizer.bypassSecurityTrustResourceUrl(target.toString());
  });

  constructor() {
    // Asking for the content should land you in it — move focus to the player once it exists.
    effect(() => {
      this.playerFrame()?.nativeElement.focus();
    });
  }

  /** The person asked. Load the outside content — for this page view only. */
  load(): void {
    if (!this.recognised()) return;
    this.loaded.set(true);
  }
}
