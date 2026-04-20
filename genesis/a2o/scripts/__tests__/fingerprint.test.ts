import { strict as assert } from 'node:assert';
import { describe, it } from 'node:test';

import { fingerprint, normalizeMessage } from '../lib/fingerprint.js';

const REFERENCE_ERROR_MSG = 'ReferenceError: x is not defined';

void describe('normalizeMessage', () => {
  void it('strips UUIDs', () => {
    const m = 'Failed to load content 550e8400-e29b-41d4-a716-446655440000';
    assert.equal(normalizeMessage(m), 'Failed to load content <uuid>');
  });

  void it('strips ISO timestamps', () => {
    const m = 'Request at 2026-04-19T10:15:23.123Z timed out';
    assert.equal(normalizeMessage(m), 'Request at <ts> timed out');
  });

  void it('strips port numbers from URLs', () => {
    const m = 'fetch https://doorway-alpha.elohim.host:8443/foo failed';
    assert.equal(normalizeMessage(m), 'fetch https://doorway-alpha.elohim.host/foo failed');
  });

  void it('strips hex hashes (sha-256 style)', () => {
    const m = 'Shard sha256-abc123def456789012345678901234567890 missing';
    assert.equal(normalizeMessage(m), 'Shard sha256-<hash> missing');
  });

  void it('collapses multi-whitespace', () => {
    assert.equal(normalizeMessage('a  \n  b\t\tc'), 'a b c');
  });

  void it('is idempotent', () => {
    const once = normalizeMessage('id 550e8400-e29b-41d4-a716-446655440000');
    assert.equal(normalizeMessage(once), once);
  });
});

void describe('fingerprint', () => {
  void it('returns 12-hex-char prefix', () => {
    const fp = fingerprint('any message');
    assert.match(fp, /^[0-9a-f]{12}$/);
  });

  void it('is stable across calls', () => {
    assert.equal(fingerprint(REFERENCE_ERROR_MSG), fingerprint(REFERENCE_ERROR_MSG));
  });

  void it('ignores runtime noise so two realistic variants collide', () => {
    const a = fingerprint(
      'Failed to load 550e8400-e29b-41d4-a716-446655440000 at 2026-04-19T10:15:23.123Z'
    );
    const b = fingerprint(
      'Failed to load 660e8400-e29b-41d4-a716-446655441111 at 2026-04-19T10:20:00.000Z'
    );
    assert.equal(a, b);
  });

  void it('distinguishes genuinely different messages', () => {
    assert.notEqual(
      fingerprint(REFERENCE_ERROR_MSG),
      fingerprint('TypeError: Cannot read properties of undefined')
    );
  });
});
