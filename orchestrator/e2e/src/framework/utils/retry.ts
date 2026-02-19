/**
 * Exponential backoff retry utility for eventual consistency assertions.
 */

export interface RetryOptions {
  /** Maximum number of attempts */
  maxAttempts?: number;
  /** Initial delay in milliseconds */
  initialDelayMs?: number;
  /** Backoff multiplier */
  backoffFactor?: number;
  /** Maximum delay between retries in milliseconds */
  maxDelayMs?: number;
  /** Overall timeout in milliseconds (overrides maxAttempts if hit first) */
  timeoutMs?: number;
}

const DEFAULTS: Required<RetryOptions> = {
  maxAttempts: 10,
  initialDelayMs: 1000,
  backoffFactor: 1.5,
  maxDelayMs: 15_000,
  timeoutMs: 60_000,
};

export async function retry<T>(
  fn: () => Promise<T>,
  opts: RetryOptions = {},
): Promise<T> {
  const config = { ...DEFAULTS, ...opts };
  const deadline = Date.now() + config.timeoutMs;
  let delay = config.initialDelayMs;
  let lastError: unknown;

  for (let attempt = 1; attempt <= config.maxAttempts; attempt++) {
    if (Date.now() >= deadline) break;

    try {
      return await fn();
    } catch (err) {
      lastError = err;
      if (attempt === config.maxAttempts || Date.now() + delay >= deadline) break;
      await sleep(Math.min(delay, deadline - Date.now()));
      delay = Math.min(delay * config.backoffFactor, config.maxDelayMs);
    }
  }

  throw lastError instanceof Error
    ? lastError
    : new Error(`Retry exhausted: ${String(lastError)}`);
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
