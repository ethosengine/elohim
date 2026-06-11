import { expect } from '@open-wc/testing';
import { parseFederatedIdentifier, resolveGatewayToDoorwayUrl } from './federated-identifier.js';

// ─── parseFederatedIdentifier ────────────────────────────────────────────────

describe('parseFederatedIdentifier', () => {
  it('parses a well-formed identifier', () => {
    const result = parseFederatedIdentifier('matthew@alpha.elohim.host');
    expect(result.ok).to.be.true;
    if (result.ok) {
      expect(result.value.username).to.equal('matthew');
      expect(result.value.gatewayDomain).to.equal('alpha.elohim.host');
      expect(result.value.identifier).to.equal('matthew@alpha.elohim.host');
    }
  });

  it('strips a leading @ before parsing', () => {
    const result = parseFederatedIdentifier('@matthew@alpha.elohim.host');
    expect(result.ok).to.be.true;
    if (result.ok) {
      expect(result.value.username).to.equal('matthew');
      expect(result.value.gatewayDomain).to.equal('alpha.elohim.host');
    }
  });

  it('trims surrounding whitespace', () => {
    const result = parseFederatedIdentifier('  matthew@alpha.elohim.host  ');
    expect(result.ok).to.be.true;
    if (result.ok) {
      expect(result.value.username).to.equal('matthew');
      expect(result.value.gatewayDomain).to.equal('alpha.elohim.host');
    }
  });

  it('uses the last @ as separator for email-like local-parts', () => {
    const result = parseFederatedIdentifier('user@email.com@gateway.host');
    expect(result.ok).to.be.true;
    if (result.ok) {
      expect(result.value.username).to.equal('user@email.com');
      expect(result.value.gatewayDomain).to.equal('gateway.host');
    }
  });

  it('rejects an identifier with no @ sign', () => {
    const result = parseFederatedIdentifier('nope');
    expect(result.ok).to.be.false;
    if (!result.ok) expect(result.reason).to.equal('no-at-sign');
  });

  it('rejects an empty string', () => {
    const result = parseFederatedIdentifier('');
    expect(result.ok).to.be.false;
    if (!result.ok) expect(result.reason).to.equal('no-at-sign');
  });

  it('rejects an identifier with an empty local-part (@gateway.host)', () => {
    // Leading @ is stripped first, leaving "gateway.host" — no remaining @,
    // so the outcome is no-at-sign (not empty-local).
    const result = parseFederatedIdentifier('@gateway.host');
    expect(result.ok).to.be.false;
    if (!result.ok) expect(result.reason).to.equal('no-at-sign');
  });

  it('rejects an identifier with an empty gateway (user@)', () => {
    const result = parseFederatedIdentifier('user@');
    expect(result.ok).to.be.false;
    if (!result.ok) expect(result.reason).to.equal('empty-gateway');
  });

  it('rejects a bare @ sign', () => {
    // Leading @ is stripped first, leaving "" — no remaining @, so no-at-sign.
    const result = parseFederatedIdentifier('@');
    expect(result.ok).to.be.false;
    if (!result.ok) expect(result.reason).to.equal('no-at-sign');
  });
});

// ─── resolveGatewayToDoorwayUrl ─────────────────────────────────────────────

describe('resolveGatewayToDoorwayUrl', () => {
  it('uses the convention for an unknown 3-part domain', () => {
    const result = resolveGatewayToDoorwayUrl('staging.elohim.host', []);
    expect(result.ok).to.be.true;
    if (result.ok) expect(result.doorway.url).to.equal('https://doorway-staging.elohim.host');
  });

  it('falls back to the domain directly for a 2-part domain', () => {
    const result = resolveGatewayToDoorwayUrl('elohim.host', []);
    expect(result.ok).to.be.true;
    if (result.ok) expect(result.doorway.url).to.equal('https://elohim.host');
  });

  it('does not double-prefix an already-prefixed doorway domain', () => {
    const result = resolveGatewayToDoorwayUrl('doorway-alpha.elohim.host', []);
    expect(result.ok).to.be.true;
    if (result.ok) expect(result.doorway.url).to.not.include('doorway-doorway-');
  });

  it('returns a known doorway URL when the domain matches by URL content', () => {
    const knownDoorways = [{ url: 'https://doorway-custom.example.com', id: 'custom' }];
    const result = resolveGatewayToDoorwayUrl('custom.example.com', knownDoorways);
    expect(result.ok).to.be.true;
    if (result.ok) {
      expect(result.doorway.url).to.equal('https://doorway-custom.example.com');
      expect(result.doorway.id).to.equal('custom');
    }
  });

  it('defaults knownDoorways to [] when omitted', () => {
    // Should not throw; uses convention or fallback
    const result = resolveGatewayToDoorwayUrl('test.elohim.host');
    expect(result.ok).to.be.true;
    if (result.ok) expect(result.doorway.url).to.equal('https://doorway-test.elohim.host');
  });
});
