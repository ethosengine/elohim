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
export async function bootstrapGrant({
  env = process.env,
  fetcher = fetch,
  saveReceipt,
  now = Date.now(),
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
  insist(
    health.dhtParticipation?.agentKeys?.lamad === config.provider,
    "live provider differs from approved fixture provider",
  );
  // Fixed timestamps never regenerate: retries submit identical native content.
  const issued = await request("/api/v1/compute/grants", config.grant);
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
  bootstrapGrant().then(
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
