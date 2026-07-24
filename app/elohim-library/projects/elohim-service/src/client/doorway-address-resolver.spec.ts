import {
  ConfiguredDoorwayResolver,
  gatewayCandidates,
  normalizeDoorwayUrl,
} from './doorway-address-resolver';

describe('ConfiguredDoorwayResolver', () => {
  it('resolves an opaque identity to ordered, deduplicated gateway candidates', async () => {
    const resolver = new ConfiguredDoorwayResolver([
      {
        identity: 'uhCAk-doorway-key',
        primaryUrl: 'https://primary.example/',
        fallbackUrls: [
          'https://fallback.example',
          'https://primary.example',
          'https://second.example/',
        ],
      },
    ]);

    const resolution = await resolver.resolve('uhCAk-doorway-key');

    expect(resolution.source).toBe('config');
    expect(gatewayCandidates(resolution)).toEqual([
      'https://primary.example',
      'https://fallback.example',
      'https://second.example',
    ]);
  });

  it('fails honestly when the identity has no configured record', async () => {
    const resolver = new ConfiguredDoorwayResolver([]);

    expect(() => resolver.resolve('unknown-key')).toThrow(
      'No configured addresses for doorway identity "unknown-key"'
    );
  });

  it('orders signed-style endpoint projections by priority with stable ties', () => {
    expect(
      gatewayCandidates({
        identity: 'uhCAk-doorway-key',
        source: 'registration',
        endpoints: [
          { service: 'signal', url: 'wss://signal.example', priority: 0 },
          { service: 'gateway', url: 'https://third.example', priority: 20 },
          { service: 'gateway', url: 'https://first.example', priority: 0 },
          { service: 'gateway', url: 'https://second.example', priority: 0 },
        ],
      })
    ).toEqual(['https://first.example', 'https://second.example', 'https://third.example']);
    expect(normalizeDoorwayUrl('https://first.example///')).toBe('https://first.example');
  });
});
