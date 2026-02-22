/**
 * Base class for devices that a Human uses to interact with the network.
 *
 * Each device type wraps a different transport:
 *   - BrowserDevice: HTTP client talking to a doorway (undici, fast, CI-friendly)
 *   - PlaywrightDevice: real Chromium browser via Playwright
 *   - StewardDevice: direct HTTP to local elohim-storage at :8090
 *   - claude: Claude-in-Chrome interactive exploration
 */

export type DeviceType = 'browser' | 'playwright' | 'steward' | 'claude';

export abstract class Device {
  abstract readonly type: DeviceType;
  abstract readonly label: string;

  /** Whether this device has an active authenticated session */
  abstract get isAuthenticated(): boolean;

  /** Human-readable description for test output */
  toString(): string {
    return `${this.type}:${this.label}`;
  }
}
