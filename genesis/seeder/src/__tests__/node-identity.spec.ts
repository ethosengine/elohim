import { describe, expect, it, vi } from 'vitest';
import { createNodeIdentityResolver, type ConductorReader } from '../node-identity.js';
import { canonicalHumanIdFor } from '../seed-drill-fixtures.js';

const KEY = 'uhCAkmatthewagentkey';

describe('node identity — read from the human\'s OWN conductor', () => {
  it('connects to the name-affine conductor and returns agent key + embodied human', async () => {
    const reader = vi.fn<ConductorReader>(async () => ({
      agentPubKey: KEY,
      embodiedHumanId: '5f27bc9b-df99-4a94-9f68-b1d355b4ddef',
    }));
    const resolver = createNodeIdentityResolver({
      conductorUrls: 'jessica=ws://localhost:4455,matthew=ws://localhost:4445',
      appIdPrefix: 'elohim',
      timeoutMs: 1234,
      reader,
    });
    const id = await resolver.resolve('human-matthew-manager');
    expect(id).toEqual({
      humanId: 'human-matthew-manager',
      conductorUrl: 'ws://localhost:4445',
      agentPubKey: KEY,
      embodiedHumanId: '5f27bc9b-df99-4a94-9f68-b1d355b4ddef',
    });
    expect(reader).toHaveBeenCalledWith('ws://localhost:4445', 'elohim', 1234);
  });

  it('caches per run — one conductor read per human', async () => {
    const reader = vi.fn<ConductorReader>(async () => ({ agentPubKey: KEY, embodiedHumanId: null }));
    const resolver = createNodeIdentityResolver({ conductorUrls: 'matthew=ws://localhost:4445', reader });
    await resolver.resolve('human-matthew-manager');
    await resolver.resolve('human-matthew-manager');
    expect(reader).toHaveBeenCalledTimes(1);
  });

  it('honours the elohim-<name>-<env> hostname convention', async () => {
    const reader = vi.fn<ConductorReader>(async () => ({ agentPubKey: KEY, embodiedHumanId: null }));
    const resolver = createNodeIdentityResolver({
      conductorUrls: 'ws://elohim-jessica-alpha:4445,ws://elohim-matthew-alpha:4445',
      reader,
    });
    expect((await resolver.resolve('human-matthew-manager')).conductorUrl).toBe(
      'ws://elohim-matthew-alpha:4445',
    );
  });

  it('a human with no affine conductor resolves to nothing — never first-reachable', async () => {
    const reader = vi.fn<ConductorReader>(async () => ({ agentPubKey: KEY, embodiedHumanId: null }));
    const resolver = createNodeIdentityResolver({ conductorUrls: 'ws://localhost:4445', reader });
    await expect(resolver.resolve('human-adam-elder')).rejects.toThrow(/human-adam-elder: no conductor/);
    expect(reader).not.toHaveBeenCalled();
  });

  it('a connect failure names the human and the conductor URL tried', async () => {
    const resolver = createNodeIdentityResolver({
      conductorUrls: 'matthew=ws://localhost:4445',
      reader: async () => {
        throw new Error('admin connect timed out after 10ms');
      },
    });
    await expect(resolver.resolve('human-matthew-manager')).rejects.toThrow(
      'human-matthew-manager: conductor ws://localhost:4445 unreadable — admin connect timed out after 10ms',
    );
  });

  it('a conductor with no steward app is a named failure', async () => {
    const resolver = createNodeIdentityResolver({
      conductorUrls: 'matthew=ws://localhost:4445',
      reader: async () => null,
    });
    await expect(resolver.resolve('human-matthew-manager')).rejects.toThrow(/no steward app 'elohim'/);
  });

  it('drill fixtures read the canonical id the peer\'s conductor embodies', async () => {
    const resolver = createNodeIdentityResolver({
      conductorUrls: 'matthew=ws://localhost:4445,james=ws://localhost:4465',
      reader: async url =>
        url.endsWith('4445')
          ? { agentPubKey: KEY, embodiedHumanId: 'human-matthew-manager' }
          : { agentPubKey: KEY, embodiedHumanId: '0b1d-uuid' },
    });
    expect(await canonicalHumanIdFor('matthew', resolver)).toBe('human-matthew-manager');
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    expect(await canonicalHumanIdFor('james', resolver)).toBeUndefined();
  });
});
