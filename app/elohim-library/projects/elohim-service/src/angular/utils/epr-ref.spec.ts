import {
  parseEpr,
  formatEpr,
  epr,
  eprToRoute,
  eprToUniversalHref,
  eprToDid,
  claimsFromDeclaration,
  type BundleRouteContext,
  type EprRef,
} from './epr-ref';
import vectors from '../../../../../../../elohim/sdk/fixtures/route-claims.vectors.json';

describe('EprRef', () => {
  describe('parseEpr', () => {
    it('parses bare ID as epr: head', () => {
      const ref = parseEpr('manifesto-foundations');
      expect(ref.id).toBe('manifesto-foundations');
      expect(ref.tier).toBe('head');
      expect(ref.version).toBeUndefined();
      expect(ref.fragment).toBeUndefined();
    });

    it('parses epr: prefix', () => {
      const ref = parseEpr('epr:manifesto-foundations');
      expect(ref.id).toBe('manifesto-foundations');
      expect(ref.tier).toBe('head');
    });

    it('parses version', () => {
      const ref = parseEpr('epr:manifesto-foundations@3');
      expect(ref.id).toBe('manifesto-foundations');
      expect(ref.version).toBe(3);
    });

    it('parses /blob tier', () => {
      const ref = parseEpr('epr:manifesto-foundations/blob');
      expect(ref.id).toBe('manifesto-foundations');
      expect(ref.tier).toBe('blob');
    });

    it('parses /doc tier', () => {
      const ref = parseEpr('epr:manifesto-foundations/doc');
      expect(ref.id).toBe('manifesto-foundations');
      expect(ref.tier).toBe('doc');
    });

    it('parses ?via query', () => {
      const ref = parseEpr('epr:manifesto?via=did:web:alpha.elohim.host');
      expect(ref.id).toBe('manifesto');
      expect(ref.via).toBe('did:web:alpha.elohim.host');
    });

    it('parses ?reach query', () => {
      const ref = parseEpr('epr:manifesto?reach=community');
      expect(ref.reach).toBe('community');
    });

    it('parses #step fragment', () => {
      const ref = parseEpr('epr:elohim-protocol#step/2');
      expect(ref.id).toBe('elohim-protocol');
      expect(ref.fragment).toEqual({ type: 'step', value: '2' });
    });

    it('parses #chapter fragment', () => {
      const ref = parseEpr('epr:elohim-protocol#chapter/economic');
      expect(ref.fragment).toEqual({ type: 'chapter', value: 'economic' });
    });

    it('parses #rel fragment', () => {
      const ref = parseEpr('epr:systems-thinking#rel/PREREQUISITE/feedback-loops');
      expect(ref.fragment).toEqual({
        type: 'rel',
        relType: 'PREREQUISITE',
        value: 'feedback-loops',
      });
    });

    it('parses full complex URI', () => {
      const ref = parseEpr('epr:manifesto@3/doc?via=did:web:alpha.elohim.host#step/2');
      expect(ref.id).toBe('manifesto');
      expect(ref.version).toBe(3);
      expect(ref.tier).toBe('doc');
      expect(ref.via).toBe('did:web:alpha.elohim.host');
      expect(ref.fragment).toEqual({ type: 'step', value: '2' });
    });

    it('parses did:web:content DID', () => {
      const ref = parseEpr('did:web:hosted.elohim.host:content:manifesto-foundations');
      expect(ref.id).toBe('manifesto-foundations');
      expect(ref.tier).toBe('head');
    });

    it('parses did:web:paths DID', () => {
      const ref = parseEpr('did:web:hosted.elohim.host:paths:elohim-protocol');
      expect(ref.id).toBe('elohim-protocol');
      expect(ref.tier).toBe('head');
    });
  });

  describe('formatEpr', () => {
    it('formats bare head reference', () => {
      expect(formatEpr({ id: 'manifesto', tier: 'head' })).toBe('epr:manifesto');
    });

    it('formats versioned reference', () => {
      expect(formatEpr({ id: 'manifesto', tier: 'head', version: 3 })).toBe('epr:manifesto@3');
    });

    it('formats blob tier', () => {
      expect(formatEpr({ id: 'manifesto', tier: 'blob' })).toBe('epr:manifesto/blob');
    });

    it('formats with via hint', () => {
      expect(formatEpr({ id: 'manifesto', tier: 'head', via: 'did:web:alpha.elohim.host' })).toBe(
        'epr:manifesto?via=did:web:alpha.elohim.host'
      );
    });

    it('formats with fragment', () => {
      expect(
        formatEpr({
          id: 'elohim-protocol',
          tier: 'head',
          fragment: { type: 'step', value: '2' },
        })
      ).toBe('epr:elohim-protocol#step/2');
    });
  });

  describe('epr()', () => {
    it('creates head ref by default', () => {
      const ref = epr('manifesto');
      expect(ref).toEqual({ id: 'manifesto', tier: 'head' });
    });

    it('creates blob ref', () => {
      const ref = epr('manifesto', 'blob');
      expect(ref).toEqual({ id: 'manifesto', tier: 'blob' });
    });
  });

  describe('eprToRoute (claims-aware, §12.3)', () => {
    const LAMAD_CTX: BundleRouteContext = {
      claims: [
        {
          contentType: 'path',
          commands: ref =>
            ref.fragment?.type === 'step'
              ? ['/path', ref.id, 'step', ref.fragment.value]
              : ['/path', ref.id],
        },
      ],
    };
    const SHELL_CTX: BundleRouteContext = { claims: [], ownsUniversalRoute: true };
    const EMPTY_CTX: BundleRouteContext = { claims: [] };

    it('unclaimed in a pillar bundle → no commands, universal href', () => {
      const res = eprToRoute({ id: 'manifesto', tier: 'head' }, LAMAD_CTX);
      expect(res).toEqual({ commands: null, href: '/epr/manifesto', claimed: false });
    });

    it('claimed contentType → in-bundle commands', () => {
      const res = eprToRoute({ id: 'elohim-protocol', tier: 'head' }, LAMAD_CTX, 'path');
      expect(res).toEqual({
        commands: ['/path', 'elohim-protocol'],
        href: '/epr/elohim-protocol',
        claimed: true,
      });
    });

    it('step fragment structurally implies the path claim', () => {
      const res = eprToRoute(
        { id: 'elohim-protocol', tier: 'head', fragment: { type: 'step', value: '2' } },
        LAMAD_CTX
      );
      expect(res?.commands).toEqual(['/path', 'elohim-protocol', 'step', '2']);
      expect(res?.claimed).toBe(true);
      expect(res?.href).toBe('/epr/elohim-protocol#step/2');
    });

    it('shell owns the universal route → /epr commands for unclaimed refs', () => {
      const res = eprToRoute({ id: 'manifesto', tier: 'head' }, SHELL_CTX);
      expect(res).toEqual({ commands: ['/epr', 'manifesto'], href: '/epr/manifesto', claimed: false });
    });

    it('step fragment in the shell (no path claim) degrades to /epr commands', () => {
      const res = eprToRoute(
        { id: 'elohim-protocol', tier: 'head', fragment: { type: 'step', value: '2' } },
        SHELL_CTX
      );
      expect(res?.commands).toEqual(['/epr', 'elohim-protocol']);
      expect(res?.href).toBe('/epr/elohim-protocol#step/2');
    });

    it('empty context (no claims, no universal route) is cross-bundle for everything', () => {
      const res = eprToRoute({ id: 'manifesto', tier: 'head' }, EMPTY_CTX);
      expect(res).toEqual({ commands: null, href: '/epr/manifesto', claimed: false });
    });

    it('returns null for blob tier', () => {
      expect(eprToRoute({ id: 'manifesto', tier: 'blob' }, SHELL_CTX)).toBeNull();
    });

    it('doc tier resolves like head (page address, not blob)', () => {
      const res = eprToRoute({ id: 'x', tier: 'doc' }, EMPTY_CTX);
      expect(res).toEqual({ commands: null, href: '/epr/x', claimed: false });
    });

    it('claim wins over ownsUniversalRoute when a context has both', () => {
      const HYBRID_CTX: BundleRouteContext = {
        claims: [{ contentType: 'path', commands: ref => ['/path', ref.id] }],
        ownsUniversalRoute: true,
      };
      const claimed = eprToRoute({ id: 'p1', tier: 'head' }, HYBRID_CTX, 'path');
      expect(claimed?.commands).toEqual(['/path', 'p1']);
      expect(claimed?.claimed).toBe(true);
      const unclaimed = eprToRoute({ id: 'c1', tier: 'head' }, HYBRID_CTX, 'concept');
      expect(unclaimed?.commands).toEqual(['/epr', 'c1']);
      expect(unclaimed?.claimed).toBe(false);
    });
  });

  describe('eprToUniversalHref', () => {
    it('mints the universal address', () => {
      expect(eprToUniversalHref({ id: 'manifesto', tier: 'head' })).toBe('/epr/manifesto');
    });
    it('carries the fragment', () => {
      expect(
        eprToUniversalHref({ id: 'p', tier: 'head', fragment: { type: 'step', value: '3' } })
      ).toBe('/epr/p#step/3');
    });
    it('URI-encodes the id', () => {
      expect(eprToUniversalHref({ id: 'a b', tier: 'head' })).toBe('/epr/a%20b');
    });
    it('targets the raw-node inspector for the raw subview', () => {
      expect(eprToUniversalHref({ id: 'manifesto', tier: 'head', subview: 'raw' })).toBe(
        '/epr/manifesto/raw'
      );
    });
    it('places /raw before the fragment', () => {
      expect(
        eprToUniversalHref({
          id: 'p',
          tier: 'head',
          subview: 'raw',
          fragment: { type: 'step', value: '3' },
        })
      ).toBe('/epr/p/raw#step/3');
    });
  });

  describe('eprToDid', () => {
    it('converts to did:web content DID', () => {
      expect(eprToDid('manifesto-foundations')).toBe(
        'did:web:hosted.elohim.host:content:manifesto-foundations'
      );
    });

    it('accepts custom host and namespace', () => {
      expect(eprToDid('elohim-protocol', 'hosted.elohim.host', 'paths')).toBe(
        'did:web:hosted.elohim.host:paths:elohim-protocol'
      );
    });
  });

  describe('roundtrip', () => {
    it('parse → format → parse produces same ref', () => {
      const uris = [
        'epr:manifesto-foundations',
        'epr:manifesto@3',
        'epr:manifesto/blob',
        'epr:manifesto/doc',
        'epr:path#step/2',
        'epr:node#rel/PREREQUISITE/other',
      ];
      for (const uri of uris) {
        const ref = parseEpr(uri);
        const formatted = formatEpr(ref);
        const reparsed = parseEpr(formatted);
        expect(reparsed).toEqual(ref);
      }
    });
  });
});

describe('claimsFromDeclaration (route-claims contract vectors)', () => {
  const ctx: BundleRouteContext = {
    claims: claimsFromDeclaration(vectors.lamadGrant.claims),
  };

  for (const v of vectors.mintVectors) {
    it(v.note, () => {
      const ref: EprRef = {
        id: v.refId,
        tier: 'head',
        ...(v.fragmentType
          ? { fragment: { type: v.fragmentType as 'step', value: v.fragmentValue! } }
          : {}),
      };
      const res = eprToRoute(ref, ctx, v.contentType);
      expect(res?.commands ?? null).toEqual(v.expectCommands);
      expect(res?.href).toBe(v.expectHref);
    });
  }
});
