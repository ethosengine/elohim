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

  it('should use convention for unknown 3+ part domains', () => {
    const result = resolveGatewayToDoorwayUrl('staging.elohim.host', []);
    expect(result).toBe('https://doorway-staging.elohim.host');
  });

  it('should use domain directly for 2-part domains', () => {
    const result = resolveGatewayToDoorwayUrl('elohim.host', []);
    expect(result).toBe('https://elohim.host');
  });

  it('should not double-prefix doorway domains', () => {
    const result = resolveGatewayToDoorwayUrl('doorway-alpha.elohim.host', []);
    // Already starts with doorway- so should not re-prefix
    expect(result).not.toContain('doorway-doorway-');
  });

  it('should match known doorways by URL content', () => {
    const knownDoorways: DoorwayInfo[] = [
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
    const result = resolveGatewayToDoorwayUrl('custom.example.com', knownDoorways);
    expect(result).toBe('https://doorway-custom.example.com');
  });
});
