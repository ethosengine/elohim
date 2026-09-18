import { Component, signal } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';

import { ExternalEmbedComponent } from './external-embed.component';

/**
 * Tested through a REAL host component with a REAL click, never via
 * `overrideComponent`/fixture-root `autoDetect` — those hide OnPush freezes, and OnPush
 * is the implicit default in this workspace. If the facade ever stops reacting to the
 * person's click, these specs must be the thing that notices.
 */
@Component({
  standalone: true,
  imports: [ExternalEmbedComponent],
  template: `
    <app-external-embed
      [url]="url()"
      [embedTitle]="'The vision, in 7 minutes'"
      [providerName]="'YouTube'"
      [poster]="poster()"
    />
  `,
})
class EmbedHostComponent {
  readonly url = signal('https://www.youtube.com/embed/6g6v7ZMEAxk');
  readonly poster = signal<string | null>(null);
}

/** Every attribute that could pull bytes from somewhere else. */
const FETCHING_ATTRS = ['src', 'href', 'srcset', 'data', 'poster', 'action'];

function externalReferences(root: HTMLElement): string[] {
  const found: string[] = [];
  for (const el of Array.from(root.querySelectorAll<HTMLElement>('*'))) {
    for (const attr of FETCHING_ATTRS) {
      const value = el.getAttribute(attr);
      if (!value) continue;
      if (/^(https?:)?\/\//i.test(value)) found.push(`${el.tagName.toLowerCase()}[${attr}]=${value}`);
    }
  }
  return found;
}

describe('ExternalEmbedComponent', () => {
  let fixture: ComponentFixture<EmbedHostComponent>;
  let host: EmbedHostComponent;
  let root: HTMLElement;

  const facadeButton = () =>
    root.querySelector<HTMLButtonElement>('[data-testid="external-embed-load"]');

  beforeEach(async () => {
    await TestBed.configureTestingModule({ imports: [EmbedHostComponent] }).compileComponents();
    fixture = TestBed.createComponent(EmbedHostComponent);
    host = fixture.componentInstance;
    root = fixture.nativeElement as HTMLElement;
    fixture.detectChanges();
  });

  describe('before the person asks', () => {
    it('renders no iframe', () => {
      expect(root.querySelector('iframe')).toBeNull();
    });

    it('pulls nothing from any outside host', () => {
      expect(externalReferences(root)).toEqual([]);
    });

    it('offers a real, keyboard-operable button', () => {
      const button = facadeButton();
      expect(button).not.toBeNull();
      expect(button?.tagName).toBe('BUTTON');
      expect(button?.disabled).toBe(false);
      expect(button?.textContent?.trim()).toBe('Play from YouTube');
    });

    it('discloses exactly the host the URL parses to', () => {
      const disclosure = root.querySelector('[data-testid="external-embed-disclosure"]');
      const parsedHost = new URL(host.url()).hostname;
      expect(disclosure?.textContent).toContain(parsedHost);
      expect(
        root.querySelector('[data-testid="external-embed-load"]')?.getAttribute('aria-describedby')
      ).toBe(disclosure?.id);
      expect(disclosure?.id).toBeTruthy();
    });

    it('names what loading it costs the person', () => {
      const disclosure = root.querySelector('[data-testid="external-embed-disclosure"]');
      expect(disclosure?.textContent).toContain("isn't part of this place");
      expect(disclosure?.textContent).toContain('see that you watched');
    });

    it('ignores a poster that would reach an outside host', () => {
      host.poster.set('https://i.ytimg.com/vi/6g6v7ZMEAxk/hqdefault.jpg');
      fixture.detectChanges();
      expect(externalReferences(root)).toEqual([]);
      expect(root.querySelector('img')).toBeNull();
    });

    it('shows a local poster', () => {
      host.poster.set('/images/elohim_logo_light.png');
      fixture.detectChanges();
      expect(root.querySelector('img')?.getAttribute('src')).toBe('/images/elohim_logo_light.png');
      expect(externalReferences(root)).toEqual([]);
    });
  });

  describe('when the person asks', () => {
    beforeEach(() => {
      facadeButton()?.click();
      fixture.detectChanges();
    });

    it('inserts exactly one iframe, on the privacy-enhanced host', () => {
      const frames = root.querySelectorAll('iframe');
      expect(frames.length).toBe(1);
      const src = frames[0].getAttribute('src') ?? '';
      expect(new URL(src).hostname).toBe('www.youtube-nocookie.com');
      expect(new URL(src).pathname).toBe('/embed/6g6v7ZMEAxk');
    });

    it('carries the restrictive embed attributes', () => {
      const frame = root.querySelector('iframe');
      expect(frame?.getAttribute('sandbox')).toBe(
        'allow-scripts allow-same-origin allow-presentation allow-popups'
      );
      expect(frame?.getAttribute('referrerpolicy')).toBe('strict-origin-when-cross-origin');
      expect(frame?.getAttribute('loading')).toBe('lazy');
      expect(frame?.getAttribute('title')).toBe('The vision, in 7 minutes');
    });

    it('keeps the seam visible with a persistent caption', () => {
      const caption = root.querySelector('[data-testid="external-embed-caption"]');
      expect(caption?.textContent).toContain('Playing from www.youtube-nocookie.com');
      expect(caption?.textContent).toContain('outside content');
    });

    it('moves focus into the player', () => {
      expect(document.activeElement?.tagName).toBe('IFRAME');
    });

    it('persists nothing about the consent', () => {
      expect(localStorage.length).toBe(0);
      expect(document.cookie).toBe('');
    });
  });

  describe('an unrecognised host', () => {
    beforeEach(() => {
      host.url.set('https://player.vimeo.com/video/12345');
      fixture.detectChanges();
    });

    it('never embeds', () => {
      facadeButton()?.click();
      fixture.detectChanges();
      expect(root.querySelector('iframe')).toBeNull();
      expect(externalReferences(root)).toEqual([]);
    });

    it('says so honestly, naming the host', () => {
      const disclosure = root.querySelector('[data-testid="external-embed-disclosure"]');
      expect(disclosure?.textContent).toContain('player.vimeo.com');
      expect(disclosure?.textContent).toContain('Nothing is loaded from it here');
      expect(facadeButton()?.disabled).toBe(true);
    });

    it('refuses a non-https address on a known host', () => {
      host.url.set('http://www.youtube.com/embed/6g6v7ZMEAxk');
      fixture.detectChanges();
      facadeButton()?.click();
      fixture.detectChanges();
      expect(root.querySelector('iframe')).toBeNull();
    });

    it('survives an address it cannot parse at all', () => {
      host.url.set('not-a-url');
      fixture.detectChanges();
      expect(root.querySelector('iframe')).toBeNull();
      expect(facadeButton()?.disabled).toBe(true);
    });
  });
});
