import { describe, expect, it } from 'vitest';

import { anchorWords, focalShape, reachSubtitle, shortAnchor, toAtom } from './epr-home.model';

describe('epr-home.model', () => {
  it('shapes the focal slot by contentFormat', () => {
    expect(focalShape('html5-app')).toBe('immersive');
    expect(focalShape('markdown')).toBe('reading');
    expect(focalShape('')).toBe('reading');
  });

  it('says the anchor state in words', () => {
    expect(anchorWords('notarized', 'unverified')).toBe('anchor not yet verified here');
    expect(anchorWords('notarized', 'verified')).toBe('anchor verified here');
    expect(anchorWords(null, null)).toBe('Not yet notarized');
  });

  it('projects the raw wire shape without reshaping identity', () => {
    const atom = toAtom({
      id: 'evolution-of-trust',
      title: 'The Evolution of Trust',
      contentType: 'collective',
      contentFormat: 'html5-app',
      reach: 'commons',
      trust: 'notarized',
      dhtAnchorHash: 'uhCkk_D-fLh9hgcSAk4ZE6375dJuKrzf4Y9CDEOoX4e9fKujiEm8f',
      dhtAnchorState: 'unverified',
      metadata: { author: 'Nicky Case', license: 'CC0 Public Domain', relatedNodeIds: ['a', 'b'] },
    });
    expect(atom.shape).toBe('immersive');
    expect(atom.author).toBe('Nicky Case');
    expect(atom.relatedIds).toEqual(['a', 'b']);
    expect(reachSubtitle(atom.reach)).toBe('anyone can reach this');
    expect(shortAnchor(atom.dhtAnchorHash!)).toBe('uhCkk_D-fLh9…KujiEm8f');
  });
});

import { Component, EventEmitter, Input, Output } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute, convertToParamMap, provideRouter } from '@angular/router';
import { of, throwError } from 'rxjs';
import { beforeEach, vi } from 'vitest';

import { EprHomeComponent } from './epr-home.component';
import { EprFocalComponent } from '../epr-focal/epr-focal.component';
import { StorageClientService } from '../../services/storage-client.service';

@Component({ selector: 'app-epr-focal', standalone: true, template: '<div class="focal-stub"></div>' })
class EprFocalStub {
  @Input() slug = '';
  @Output() nodeLoaded = new EventEmitter<unknown>();
  @Output() notFound = new EventEmitter<string>();
  @Output() failed = new EventEmitter<string>();
}

const rawSimulation = {
  id: 'evolution-of-trust',
  title: 'The Evolution of Trust',
  description: 'An interactive guide to the game theory of trust.',
  contentType: 'collective',
  contentFormat: 'html5-app',
  reach: 'commons',
  trust: 'notarized',
  dhtAnchorHash: 'uhCkk_D-fLh9hgcSAk4ZE6375dJuKrzf4Y9CDEOoX4e9fKujiEm8f',
  dhtAnchorState: 'unverified',
  createdAt: '2026-05-27 20:46:37',
  updatedAt: '2026-08-05 18:40:53',
  metadata: { author: 'Nicky Case', license: 'CC0 Public Domain', estimatedTime: '30 minutes' },
};

function q(fixture: ComponentFixture<EprHomeComponent>, id: string): Element | null {
  return fixture.nativeElement.querySelector(`[data-testid="${id}"]`);
}

describe('EprHomeComponent', () => {
  let fixture: ComponentFixture<EprHomeComponent>;
  let storage: { getContent: ReturnType<typeof vi.fn> };

  async function mount(resourceId: string): Promise<void> {
    await TestBed.configureTestingModule({
      imports: [EprHomeComponent],
      providers: [
        provideRouter([]),
        { provide: StorageClientService, useValue: storage },
        {
          provide: ActivatedRoute,
          useValue: { paramMap: of(convertToParamMap({ resourceId })) },
        },
      ],
    })
      .overrideComponent(EprHomeComponent, {
        remove: { imports: [EprFocalComponent] },
        add: { imports: [EprFocalStub] },
      })
      .compileComponents();
    fixture = TestBed.createComponent(EprHomeComponent);
    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();
  }

  beforeEach(() => {
    storage = { getContent: vi.fn().mockReturnValue(of(rawSimulation)) };
  });

  it('renders the frame with identity and chips for a reachable atom', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home')).not.toBeNull();
    expect(q(fixture, 'epr-home-title')?.textContent).toContain('The Evolution of Trust');
    expect(q(fixture, 'epr-home-chip-reach')?.textContent).toContain('Commons');
    expect(q(fixture, 'epr-home-chip-notarized')?.textContent).toContain(
      'anchor not yet verified here'
    );
    expect(fixture.nativeElement.textContent).not.toContain('Back to Lamad');
  });

  it('hands the slug to the focal slot in the immersive shape', async () => {
    await mount('evolution-of-trust');
    const focal = q(fixture, 'epr-home-focal');
    expect(focal?.classList.contains('epr-home__focal--immersive')).toBe(true);
    expect(focal?.querySelector('.focal-stub')).not.toBeNull();
  });

  it('uses the reading shape for markdown', async () => {
    storage.getContent.mockReturnValue(of({ ...rawSimulation, contentFormat: 'markdown' }));
    await mount('succession');
    expect(q(fixture, 'epr-home-focal')?.classList.contains('epr-home__focal--reading')).toBe(
      true
    );
  });

  it('renders the out-of-reach gate for a null atom, with no chrome', async () => {
    storage.getContent.mockReturnValue(of(null));
    await mount('concept-bidirectional-trust');
    expect(q(fixture, 'epr-home-gate')?.textContent).toContain("We can't reach this one from here");
    expect(q(fixture, 'epr-home-gate')?.textContent).toContain('concept-bidirectional-trust');
    expect(q(fixture, 'epr-home-your-mark')).toBeNull();
    expect(q(fixture, 'epr-home-focal')).toBeNull();
  });

  it('renders the error state when the load fails', async () => {
    storage.getContent.mockReturnValue(throwError(() => new Error('boom')));
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-error')).not.toBeNull();
  });

  it('carries the universal address line', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-address')?.textContent).toContain('/epr/evolution-of-trust');
  });
});
