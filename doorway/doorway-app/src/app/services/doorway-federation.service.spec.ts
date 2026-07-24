import { gatewayCandidates } from '@elohim/service/client/doorway-address-resolver';

import { federationDoorwayResolution } from './doorway-federation.service';

describe('federationDoorwayResolution', () => {
  it('uses key identity and signed endpoint priority instead of hostname identity', () => {
    const resolution = federationDoorwayResolution({
      id: 'alpha-elohim-host',
      url: 'https://legacy.example',
      identity_root: 'uhCAk-root',
      signing_key: 'uhCAk-head',
      endpoints: [
        {
          service: 'gateway',
          url: 'https://backup.example',
          priority: 10,
          ttl_secs: 300,
        },
        {
          service: 'gateway',
          url: 'https://primary.example',
          priority: 0,
          ttl_secs: 300,
        },
      ],
      record_signature: [1, 2, 3],
      tier: 'Emerging',
      capabilities: ['gateway'],
      status: 'online',
    });

    expect(resolution.identity).toBe('uhCAk-root');
    expect(resolution.source).toBe('registration');
    expect(gatewayCandidates(resolution)).toEqual([
      'https://primary.example',
      'https://backup.example',
    ]);
  });

  it('falls back to the legacy configured URL when no signed record is present', () => {
    const resolution = federationDoorwayResolution({
      id: 'legacy',
      url: 'https://legacy.example/',
      tier: 'Federated',
      capabilities: ['gateway'],
      status: 'online',
    });

    expect(resolution.identity).toBe('legacy');
    expect(resolution.source).toBe('config');
    expect(gatewayCandidates(resolution)).toEqual(['https://legacy.example']);
  });
});
