import { describe, it, expect } from 'vitest';
import { validateApiKey, extractApiKey } from './auth.js';
import { PermissionLevel } from './permissions.js';
import type { Config } from './config.js';

const mockConfig: Config = {
  devMode: false,
  conductorUrl: 'ws://localhost:4444',
  port: 8080,
  adminPort: 4444,
  appPortMin: 4445,
  appPortMax: 65535,
  apiKeyAuthenticated: 'test-auth-key',
  apiKeyAdmin: 'test-admin-key',
  logLevel: 'info',
};

const devModeConfig: Config = {
  ...mockConfig,
  devMode: true,
};

describe('auth', () => {
  describe('validateApiKey', () => {
    it('returns PUBLIC for null API key', () => {
      expect(validateApiKey(null, mockConfig)).toBe(PermissionLevel.PUBLIC);
    });

    it('returns AUTHENTICATED for authenticated key', () => {
      expect(validateApiKey('test-auth-key', mockConfig)).toBe(
        PermissionLevel.AUTHENTICATED
      );
    });

    it('returns ADMIN for admin key', () => {
      expect(validateApiKey('test-admin-key', mockConfig)).toBe(
        PermissionLevel.ADMIN
      );
    });

    it('returns null for invalid key', () => {
      expect(validateApiKey('invalid-key', mockConfig)).toBe(null);
      expect(validateApiKey('', mockConfig)).toBe(null);
    });

    it('returns ADMIN for any key in dev mode', () => {
      expect(validateApiKey(null, devModeConfig)).toBe(PermissionLevel.ADMIN);
      expect(validateApiKey('invalid-key', devModeConfig)).toBe(PermissionLevel.ADMIN);
      expect(validateApiKey('', devModeConfig)).toBe(PermissionLevel.ADMIN);
    });
  });

  describe('extractApiKey', () => {
    it('extracts API key from URL query param', () => {
      expect(extractApiKey('/?apiKey=my-key', 'localhost')).toBe('my-key');
      expect(extractApiKey('/path?apiKey=another-key', 'example.com')).toBe(
        'another-key'
      );
    });

    it('returns null when no API key present', () => {
      expect(extractApiKey('/', 'localhost')).toBe(null);
      expect(extractApiKey('/path?other=value', 'localhost')).toBe(null);
    });

    it('handles empty API key', () => {
      expect(extractApiKey('/?apiKey=', 'localhost')).toBe('');
    });

    it('handles complex URLs', () => {
      expect(
        extractApiKey('/?foo=bar&apiKey=test&baz=qux', 'localhost')
      ).toBe('test');
    });
  });
});
