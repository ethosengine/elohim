import { parseEpr, formatEpr, epr, eprToRoute, eprToDid } from './epr-ref';

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

  describe('eprToRoute', () => {
    it('returns resource route for head tier', () => {
      expect(eprToRoute({ id: 'manifesto', tier: 'head' })).toEqual([
        '/lamad/resource',
        'manifesto',
      ]);
    });

    it('returns path step route for step fragment', () => {
      expect(
        eprToRoute({
          id: 'elohim-protocol',
          tier: 'head',
          fragment: { type: 'step', value: '2' },
        })
      ).toEqual(['/lamad/path', 'elohim-protocol', 'step', '2']);
    });

    it('returns null for blob tier', () => {
      expect(eprToRoute({ id: 'manifesto', tier: 'blob' })).toBeNull();
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
