#!/usr/bin/env node
// Provider-side consent. Matthew's operational accountability is declared here;
// this is not a newly notarized human-to-fixture stewardship relationship.
import { mkdir, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { api } from "./common.mjs";
const insist = (condition, message) => {
  if (!condition) throw new Error(message);
};
const agentKey = (value) =>
  typeof value === "string" && /^uhCAk[A-Za-z0-9_-]{48}$/.test(value);
export function validateGrantDeclaration(config, now = Date.now()) {
  insist(
    config?.enabled === true,
    "grant declaration must be explicitly enabled",
  );
  insist(
    config.stewardOfRecord === "human:matthew" &&
      config.fixtureProvider === "adam",
    "grant requires Matthew accountability and the Adam fixture provider",
  );
  insist(
    agentKey(config.provider) &&
      agentKey(config.recipient) &&
      config.provider !== config.recipient,
    "grant requires distinct canonical provider and recipient agent keys",
  );
  const grant = config.grant;
  insist(
    grant?.scope === "measure-stage" && grant.recipient === config.recipient,
    "grant recipient/scope differs from approved declaration",
  );
  const stamps = ["issuedAt", "validFrom", "validUntil"].map((k) =>
    Date.parse(grant[k]),
  );
  insist(
    stamps.every(Number.isFinite) &&
      stamps[0] <= now &&
      stamps[1] <= now &&
      stamps[1] < stamps[2] &&
      stamps[2] > now &&
      stamps[0] < stamps[2],
    "grant requires fixed, finite, active issuance timestamps",
  );
  const bounds = grant.bounds;
  insist(
    bounds?.reach_ceiling === "commons" &&
      Number.isSafeInteger(bounds.rate_per_hour) &&
      bounds.rate_per_hour > 0 &&
      Number.isSafeInteger(bounds.rotation_ttl_days) &&
      bounds.rotation_ttl_days > 0 &&
      stamps[2] - stamps[1] <= bounds.rotation_ttl_days * 86400000,
    "grant requires finite rate, rotation lifetime and commons reach",
  );
  insist(
    Array.isArray(bounds.epr_scope) &&
      bounds.epr_scope.length > 0 &&
      bounds.epr_scope.length <= 64 &&
      bounds.epr_scope.every(
        (v) => typeof v === "string" && v.length > 0 && v.length <= 256,
      ),
    "grant requires explicit task scope",
  );
  return config;
}
// Diagnostics are an allowlist, never a rendering of native error text.
function nativeFailureCategory(status, text) {
  if (status === 403) {
    if (text.includes('"local-compute-capability-required"'))
      return "local-capability-refused";
    if (text.includes('"local-cell-actor-required"'))
      return "local-actor-refused";
  }
  if (status === 503) {
    if (text.includes('"catching-up"')) return "catching-up";
    if (text.includes("local compute API disabled"))
      return "local-api-disabled";
    if (text.includes("local conductor unavailable"))
      return "conductor-unavailable";
  }
  const reasons = {
    "native grant entry differs from submitted content":
      "grant-content-mismatch",
    "signed grant unavailable": "signed-grant-unavailable",
    "existing grant differs from explicit consent": "existing-grant-mismatch",
    "withdrawn grant cannot be reactivated": "grant-withdrawn",
    "withdrawn projection requires reconciliation, never reactivation":
      "projection-withdrawn",
    "native consent record unavailable": "consent-unavailable",
    "provider activation not confirmed": "activation-unconfirmed",
  };
  const prefix = "Error: Invalid input: compute grant: ";
  if (status === 500 && text.startsWith(prefix)) {
    const reason = text.slice(prefix.length).trim();
    if (Object.hasOwn(reasons, reason)) return reasons[reason];
    if (
      [
        "consent action or author mismatch",
        "consent requires Create",
        "consent requires an app entry",
        "consent must be the native EconomicEvent entry",
        "consent entry missing",
        "consent entry hash mismatch",
        "consent signature mismatch",
        "existing consent differs from explicit grant",
      ].includes(reason)
    )
      return "consent-verification-refused";
    return "grant-native-refused";
  }
  for (const [prefix, category] of [
    ["Error: Conductor error: ", "conductor-error"],
    ["Error: Serialization error: ", "serialization-error"],
    ["Error: Database error: ", "database-error"],
    ["Error: Diesel error: ", "database-error"],
    ["Error: plane_violation: ", "plane-refused"],
  ])
    if (status === 500 && text.startsWith(prefix)) return category;
  return "http-refused";
}

async function boundedBody(response, signal) {
  const reader = response.body?.getReader();
  if (!reader) return "";
  let listener;
  const aborted = new Promise((_, reject) => {
    listener = () =>
      reject(
        Object.assign(new Error("request deadline"), { name: "TimeoutError" }),
      );
    if (signal?.aborted) listener();
    else signal?.addEventListener("abort", listener, { once: true });
  });
  try {
    return await Promise.race([
      aborted,
      (async () => {
        const chunks = [];
        let length = 0;
        while (true) {
          const { done, value } = await reader.read();
          if (done) return Buffer.concat(chunks, length).toString("utf8");
          length += value.byteLength;
          if (length > 65536)
            throw Object.assign(new Error("body bound"), {
              name: "BodyLimitError",
            });
          chunks.push(Buffer.from(value));
        }
      })(),
    ]);
  } finally {
    signal?.removeEventListener("abort", listener);
    void reader.cancel().catch(() => {});
  }
}

export async function bootstrapGrant(options = {}) {
  // Existing read-only transports may deliberately stop before issuance.
  if (!options.observe)
    return bootstrapGrantAttempt({ ...options, phase: () => {} });
  const diagnostic = {
    phase: "declaration",
    httpStatus: null,
    category: "declaration-refused",
    outcome: "not-submitted",
  };
  const fetcher = options.fetcher || fetch;
  const observedFetch = async (url, init) => {
    const post = init?.method === "POST";
    diagnostic.phase = post ? "grant-request" : "health-request";
    diagnostic.httpStatus = null;
    if (post) diagnostic.outcome = "unknown"; // Handed off, never assume absence after an error.
    let response;
    try {
      response = await fetcher(url, init);
    } catch (error) {
      diagnostic.category = ["TimeoutError", "AbortError"].includes(error?.name)
        ? "request-timeout"
        : "connection-failed";
      throw error;
    }
    diagnostic.httpStatus =
      Number.isInteger(response.status) &&
      response.status >= 100 &&
      response.status <= 599
        ? response.status
        : null;
    diagnostic.phase = post ? "grant-response" : "health-response";
    const body = async () => {
      try {
        return await boundedBody(response, init?.signal);
      } catch (error) {
        diagnostic.category =
          error?.name === "BodyLimitError"
            ? "body-limit"
            : ["TimeoutError", "AbortError"].includes(error?.name)
              ? "request-timeout"
              : "body-read-failed";
        throw new Error("provider response unavailable");
      }
    };
    if (!response.ok) {
      if (!post) void response.body?.cancel().catch(() => {});
      diagnostic.category = post
        ? nativeFailureCategory(response.status, await body())
        : "health-http-refused";
      return { ok: false, status: response.status };
    }
    return {
      ok: true,
      status: response.status,
      json: async () => {
        const text = await body();
        try {
          return JSON.parse(text);
        } catch {
          diagnostic.category = "response-json-invalid";
          throw new Error("provider response is not JSON");
        }
      },
    };
  };
  try {
    return await bootstrapGrantAttempt({
      ...options,
      fetcher: observedFetch,
      phase: (phase, category) => {
        diagnostic.phase = phase;
        diagnostic.category = category;
      },
    });
  } catch (error) {
    // Observation cannot alter issuance, mask refusal, or trigger a retry.
    try {
      options.observe?.(Object.freeze({ ...diagnostic }));
    } catch {
      /* observer is advisory */
    }
    throw error;
  }
}

async function bootstrapGrantAttempt({
  env = process.env,
  fetcher = fetch,
  saveReceipt,
  now = Date.now(),
  phase,
} = {}) {
  if (!env.COMPUTE_RUNNER_GRANT_JSON) return null;
  let config;
  try {
    config = JSON.parse(env.COMPUTE_RUNNER_GRANT_JSON);
  } catch {
    throw new Error("grant declaration is not JSON");
  }
  if (config.enabled === false) return null;
  validateGrantDeclaration(config, now);
  const base = env.COMPUTE_API_URL || "http://127.0.0.1:8090";
  const request = api(
    base,
    config.provider,
    fetcher,
    env.ELOHIM_COMPUTE_LOCAL_TOKEN || "",
  );
  insist(
    env.COMPUTE_PERFORMER === config.provider,
    "worker performer differs from approved fixture provider",
  );
  insist(
    (env.ELOHIM_COMPUTE_LOCAL_TOKEN || "").length >= 32,
    "provider compute capability is absent or invalid",
  );
  let response;
  try {
    response = await fetcher(new URL("/health", base), {
      redirect: "error",
      signal: AbortSignal.timeout(30000),
    });
  } catch {
    throw new Error("provider health unavailable");
  }
  insist(response.ok, "provider health refused identity read");
  const health = await response.json();
  phase("health-identity", "provider-identity-refused");
  insist(
    health.dhtParticipation?.agentKeys?.lamad === config.provider,
    "live provider differs from approved fixture provider",
  );
  // Fixed timestamps never regenerate: retries submit identical native content.
  const issued = await request("/api/v1/compute/grants", config.grant);
  phase("grant-validation", "native-response-invalid");
  insist(
    issued.state === "active" &&
      issued.provider === config.provider &&
      issued.recipient === config.recipient,
    "native grant response differs from approved parties/state",
  );
  insist(
    /^uhCEk[A-Za-z0-9_-]{48}$/.test(issued.grantCid || "") &&
      /^uhCkk[A-Za-z0-9_-]{48}$/.test(issued.grantActionHash || "") &&
      /^uhCkk[A-Za-z0-9_-]{48}$/.test(issued.consentActionHash || ""),
    "native grant response has no canonical grant/consent hashes",
  );
  const receipt = {
    declarationProvenance: "operator-approved-config",
    stewardOfRecord: config.stewardOfRecord,
    fixtureProvider: config.fixtureProvider,
    provider: issued.provider,
    recipient: issued.recipient,
    state: issued.state,
    grantCid: issued.grantCid,
    grantActionHash: issued.grantActionHash,
    consentActionHash: issued.consentActionHash,
  };
  phase("receipt-write", "receipt-write-failed");
  if (saveReceipt) await saveReceipt(receipt);
  else {
    const state = env.COMPUTE_STATE_ROOT || "/var/lib/compute";
    await mkdir(state, { recursive: true });
    await writeFile(
      join(state, "runner-grant.json"),
      JSON.stringify(receipt, null, 2) + "\n",
      { mode: 0o600 },
    );
  }
  return receipt;
}
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  bootstrapGrant({
    observe: (diagnostic) => {
      process.stderr.write(
        "provider grant diagnostic: " + JSON.stringify(diagnostic) + "\n",
      );
    },
  }).then(
    (receipt) => {
      if (receipt)
        process.stdout.write(
          `provider grant ready: ${receipt.grantActionHash}; accountable steward ${receipt.stewardOfRecord}; fixture ${receipt.fixtureProvider}\n`,
        );
    },
    () => {
      process.stderr.write(
        "REFUSED provider grant bootstrap: verify approved declaration, provider identity and native grant availability\n",
      );
      process.exitCode = 2;
    },
  );
}
