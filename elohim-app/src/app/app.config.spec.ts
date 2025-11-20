import { appConfig } from './app.config';

describe('App Config', () => {
  it('should have application config defined', () => {
    expect(appConfig).toBeDefined();
  });

  it('should have providers array', () => {
    expect(appConfig.providers).toBeDefined();
    expect(Array.isArray(appConfig.providers)).toBe(true);
  });

  it('should have at least 3 providers', () => {
    expect(appConfig.providers.length).toBeGreaterThanOrEqual(3);
  });
});
