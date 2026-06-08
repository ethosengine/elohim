/**
 * EprRawNodeComponent Tests
 *
 * The raw-node inspector renders an EPR AS AN ATOM — its own stored fields and
 * provenance — distinct from the rich pillar experience. These specs pin:
 *  - identity/type/reach/relations/provenance/metadata are rendered from the
 *    RAW wire shape (no blob resolution, no computed content path),
 *  - dhtAnchorHash: null is the expected dev/alpha anchor gap, NOT an error,
 *  - a null node renders a clear "not found" state with the requested id
 *    (NOT a blank/raw-JSON fallback).
 */

import { vi, describe, it, expect, beforeEach } from 'vitest';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { RouterModule } from '@angular/router';
import { ActivatedRoute } from '@angular/router';
import { of } from 'rxjs';

import { EprRawNodeComponent } from './epr-raw-node.component';
import { StorageClientService } from '../../services/storage-client.service';

// ── Helpers ───────────────────────────────────────────────────────────────────

function query(fixture: ComponentFixture<EprRawNodeComponent>, testId: string): Element | null {
  return fixture.nativeElement.querySelector(`[data-testid="${testId}"]`);
}

function queryAll(
  fixture: ComponentFixture<EprRawNodeComponent>,
  testId: string
): NodeListOf<Element> {
  return fixture.nativeElement.querySelectorAll(`[data-testid="${testId}"]`);
}

function text(fixture: ComponentFixture<EprRawNodeComponent>): string {
  return fixture.nativeElement.textContent ?? '';
}

// Raw wire shape (ContentView), with the relations augmentation the inspector reads.
function makeNode(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    id: 'fair-exchange',
    hAppId: 'elohim',
    title: 'Fair Exchange',
    description: 'A concept node',
    contentType: 'concept',
    contentFormat: 'markdown',
    blobHash: 'sha256-abc123',
    blobCid: 'bafkreiabc',
    contentSizeBytes: 2048,
    metadata: { difficulty: 'intro', author: 'matthew' },
    reach: 'commons',
    validationStatus: 'valid',
    createdBy: 'agent-xyz',
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-02-01T00:00:00Z',
    contentBody: '# Fair Exchange',
    dhtAnchorHash: 'uhCkk-anchor',
    relatedNodeIds: ['reciprocity', 'trust-game'],
    ...overrides,
  };
}

function setup(
  getContentReturn: unknown,
  resourceId = 'fair-exchange'
): { fixture: ComponentFixture<EprRawNodeComponent> } {
  const storageMock = { getContent: vi.fn().mockReturnValue(of(getContentReturn)) };

  TestBed.configureTestingModule({
    imports: [EprRawNodeComponent, RouterModule.forRoot([])],
    providers: [
      { provide: StorageClientService, useValue: storageMock },
      {
        provide: ActivatedRoute,
        useValue: { snapshot: { paramMap: new Map([['resourceId', resourceId]]) } },
      },
    ],
  });

  const fixture = TestBed.createComponent(EprRawNodeComponent);
  fixture.detectChanges();
  return { fixture };
}

// ── Suite ─────────────────────────────────────────────────────────────────────

describe('EprRawNodeComponent', () => {
  beforeEach(() => {
    TestBed.resetTestingModule();
  });

  // ── Loaded state ──────────────────────────────────────────────────────────

  describe('loaded node', () => {
    it('renders the root inspector with its testid', () => {
      const { fixture } = setup(makeNode());
      expect(query(fixture, 'epr-raw-node')).toBeTruthy();
    });

    it('fetches via StorageClientService.getContent with the route resourceId', () => {
      const storageMock = { getContent: vi.fn().mockReturnValue(of(makeNode())) };
      TestBed.configureTestingModule({
        imports: [EprRawNodeComponent, RouterModule.forRoot([])],
        providers: [
          { provide: StorageClientService, useValue: storageMock },
          {
            provide: ActivatedRoute,
            useValue: { snapshot: { paramMap: new Map([['resourceId', 'reciprocity']]) } },
          },
        ],
      });
      const fixture = TestBed.createComponent(EprRawNodeComponent);
      fixture.detectChanges();

      expect(storageMock.getContent).toHaveBeenCalledWith('reciprocity');
    });

    it('shows identity, type, reach and validation as stored', () => {
      const { fixture } = setup(makeNode());
      const body = text(fixture);

      expect(body).toContain('fair-exchange'); // id / address
      expect(body).toContain('concept'); // contentType
      expect(body).toContain('markdown'); // contentFormat
      expect(body).toContain('sha256-abc123'); // blobHash
      expect(body).toContain('commons'); // reach
      expect(body).toContain('valid'); // validationStatus
    });

    it('links the blobHash to /blob/{hash}', () => {
      const { fixture } = setup(makeNode());
      const link = query(fixture, 'epr-raw-blob-link') as HTMLAnchorElement | null;

      expect(link).toBeTruthy();
      expect(link!.getAttribute('href')).toBe('/blob/sha256-abc123');
      // discernible link text for a11y
      expect((link!.textContent ?? '').trim().length).toBeGreaterThan(0);
    });

    it('renders one /epr/{relatedId} link per relatedNodeId', () => {
      const { fixture } = setup(makeNode());
      const links = queryAll(fixture, 'epr-raw-relation-link');

      expect(links.length).toBe(2);
      const hrefs = Array.from(links).map(a => a.getAttribute('href'));
      expect(hrefs).toContain('/epr/reciprocity');
      expect(hrefs).toContain('/epr/trust-game');
    });

    it('renders an empty-relations note when there are no related nodes', () => {
      const { fixture } = setup(makeNode({ relatedNodeIds: [] }));

      expect(queryAll(fixture, 'epr-raw-relation-link').length).toBe(0);
      expect(query(fixture, 'epr-raw-relations-empty')).toBeTruthy();
    });

    it('renders provenance including createdAt and createdBy', () => {
      const { fixture } = setup(makeNode());
      const prov = query(fixture, 'epr-raw-provenance');

      expect(prov).toBeTruthy();
      expect(prov!.textContent).toContain('2026-01-01T00:00:00Z'); // createdAt
      expect(prov!.textContent).toContain('agent-xyz'); // createdBy
    });

    it('shows "unattributed" when createdBy is null', () => {
      const { fixture } = setup(makeNode({ createdBy: null }));
      const prov = query(fixture, 'epr-raw-provenance');

      expect(prov!.textContent).toContain('unattributed');
    });

    it('renders metadata object keys as a definition list', () => {
      const { fixture } = setup(makeNode());
      const meta = query(fixture, 'epr-raw-metadata');

      expect(meta).toBeTruthy();
      expect(meta!.textContent).toContain('difficulty');
      expect(meta!.textContent).toContain('intro');
      expect(meta!.textContent).toContain('author');
      expect(meta!.textContent).toContain('matthew');
    });

    it('uses real heading structure (one h1, multiple h2)', () => {
      const { fixture } = setup(makeNode());
      const h1s = fixture.nativeElement.querySelectorAll('h1');
      const h2s = fixture.nativeElement.querySelectorAll('h2');

      expect(h1s.length).toBe(1);
      expect(h2s.length).toBeGreaterThanOrEqual(5); // identity/type/reach/relations/provenance/metadata
    });
  });

  // ── Anchor gap ────────────────────────────────────────────────────────────

  describe('dhtAnchorHash gap', () => {
    it('renders "not yet anchored" (NOT an error) when dhtAnchorHash is null', () => {
      const { fixture } = setup(makeNode({ dhtAnchorHash: null }));

      expect(query(fixture, 'epr-raw-error')).toBeNull();
      expect(query(fixture, 'epr-raw-node')).toBeTruthy();
      expect(text(fixture)).toContain('not yet anchored');
    });

    it('shows the anchor hash when present', () => {
      const { fixture } = setup(makeNode());
      expect(text(fixture)).toContain('uhCkk-anchor');
    });
  });

  // ── Not-found state ───────────────────────────────────────────────────────

  describe('not found', () => {
    it('renders a clear not-found state with the requested id when node is null', () => {
      const { fixture } = setup(null, 'ghost-id');

      const notFound = query(fixture, 'epr-raw-not-found');
      expect(notFound).toBeTruthy();
      expect(notFound!.textContent).toContain('ghost-id');
      // not a raw-JSON / blank fallback
      expect(query(fixture, 'epr-raw-metadata')).toBeNull();
      expect(text(fixture)).not.toContain('{');
    });
  });
});
