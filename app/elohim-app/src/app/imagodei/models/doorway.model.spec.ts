/**
 * Doorway Model Tests
 *
 * Tests for federated identifier parsing and gateway resolution.
 */

import {
  parseFederatedIdentifier,
  resolveGatewayToDoorwayUrl,
  BOOTSTRAP_DOORWAYS,
  type DoorwayInfo,
} from './doorway.model';

describe('parseFederatedIdentifier', () => {
  it('should parse standard user@domain format', () => {
    const result = parseFederatedIdentifier('matthew@alpha.elohim.host');
    expect(result).toEqual({
      username: 'matthew',
      gatewayDomain: 'alpha.elohim.host',
    });
  });

  it('should handle leading @ sign', () => {
    const result = parseFederatedIdentifier('@matthew@alpha.elohim.host');
    expect(result).toEqual({
      username: 'matthew',
      gatewayDomain: 'alpha.elohim.host',
    });
  });

  it('should trim whitespace', () => {
    const result = parseFederatedIdentifier('  matthew@alpha.elohim.host  ');
    expect(result).toEqual({
      username: 'matthew',
      gatewayDomain: 'alpha.elohim.host',
    });
  });

  it('should handle email-like usernames with last @ as separator', () => {
    const result = parseFederatedIdentifier('user@email.com@gateway.host');
    expect(result).toEqual({
      username: 'user@email.com',
      gatewayDomain: 'gateway.host',
    });
  });

  it('should return null for missing @ sign', () => {
    expect(parseFederatedIdentifier('nope')).toBeNull();
  });

  it('should return null for empty string', () => {
    expect(parseFederatedIdentifier('')).toBeNull();
  });

  it('should return null for @ at start (no username)', () => {
    expect(parseFederatedIdentifier('@gateway.host')).toBeNull();
  });

  it('should return null for @ at end (no domain)', () => {
    expect(parseFederatedIdentifier('user@')).toBeNull();
  });

  it('should return null for just @', () => {
    expect(parseFederatedIdentifier('@')).toBeNull();
  });
});

describe('resolveGatewayToDoorwayUrl', () => {
  it('should resolve known bootstrap doorway', () => {
    const result = resolveGatewayToDoorwayUrl('alpha.elohim.host');
    // BOOTSTRAP_DOORWAYS contains doorway-alpha.elohim.host
    expect(result).toBe('https://doorway-alpha.elohim.host');
  });

  // The four tests below previously asserted the SYNTHESIS behaviour — that an
  // unknown gateway host becomes `https://doorway-{host}` or `https://{host}`.
  // That convention is what let a typed identifier name the origin a plaintext
  // password was POSTed to. They now assert the refusal, and they are the
  // regression guard: if any of them goes back to returning a URL, the
  // credential-exfiltration path is open again.

  it('refuses to invent a doorway for an unknown 3+ part host', () => {
    expect(resolveGatewayToDoorwayUrl('staging.elohim.host', [])).toBeNull();
  });

  it('refuses to invent a doorway for an unknown 2-part host', () => {
    expect(resolveGatewayToDoorwayUrl('elohim.host', [])).toBeNull();
  });

  it('does not resolve a host merely because it looks like a doorway', () => {
    // Starting with `doorway-` is not evidence of anything; only a declaration
    // or a probe is.
    expect(resolveGatewayToDoorwayUrl('doorway-alpha.elohim.host', [])).toBeNull();
  });

  it('matches a known doorway only by DECLARED gatewayDomain, not by URL substring', () => {
    const undeclared: DoorwayInfo[] = [
      {
        id: 'custom',
        name: 'Custom',
        url: 'https://doorway-custom.example.com',
        description: '',
        region: 'global',
        operator: '',
        features: [],
        status: 'online',
        registrationOpen: true,
      },
    ];
    // The URL contains 'custom.example.com', but the doorway never said it
    // vouches for that gateway. Inferring it from the URL is string surgery.
    expect(resolveGatewayToDoorwayUrl('custom.example.com', undeclared)).toBeNull();

    // With the declaration present, it resolves.
    const declared: DoorwayInfo[] = [{ ...undeclared[0], gatewayDomain: 'custom.example.com' }];
    expect(resolveGatewayToDoorwayUrl('custom.example.com', declared)).toBe(
      'https://doorway-custom.example.com'
    );
  });

  describe('hostile input', () => {
    // Each of these returned a usable https:// origin before the fix, and each
    // is a live credential-exfiltration vector: whatever comes back here is
    // the origin the human is REDIRECTED to in order to type their password.
    // The app no longer posts the credential itself, but sending someone to a
    // hostile sign-in page is the same theft with an extra step.
    it('refuses a bare attacker domain', () => {
      expect(resolveGatewayToDoorwayUrl('evil.tld')).toBeNull();
    });

    it('refuses an attacker subdomain', () => {
      expect(resolveGatewayToDoorwayUrl('x.evil.tld')).toBeNull();
    });

    it('refuses a known host used as a PREFIX of an attacker domain', () => {
      // The old matcher used `.includes`, so this resolved to the real alpha
      // doorway's URL — the substring accident. Host equality closes it.
      expect(resolveGatewayToDoorwayUrl('alpha.elohim.host.evil.tld')).toBeNull();
    });

    it('refuses an attacker domain that merely CONTAINS a known host', () => {
      expect(resolveGatewayToDoorwayUrl('evil.tld/alpha.elohim.host')).toBeNull();
    });
  });
});
