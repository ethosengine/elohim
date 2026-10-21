import test from "node:test";
import assert from "node:assert/strict";
import {
  bootstrapGrant,
  validateGrantDeclaration,
} from "./grant-bootstrap.mjs";
const provider = "uhCAk" + "a".repeat(48);
const recipient = "uhCAk" + "b".repeat(48);
const now = Date.parse("2026-10-21T12:00:00Z");
const declaration = () => ({
  enabled: true,
  stewardOfRecord: "human:matthew",
  fixtureProvider: "adam",
  provider,
  recipient,
  grant: {
    recipient,
    scope: "measure-stage",
    issuedAt: "2026-10-21T00:00:00Z",
    validFrom: "2026-10-21T00:00:00Z",
    validUntil: "2026-10-22T00:00:00Z",
    bounds: {
      epr_scope: ["*"],
      reach_ceiling: "commons",
      rate_per_hour: 8,
      rotation_ttl_days: 1,
    },
  },
});
const native = () => ({
  provider,
  recipient,
  state: "active",
  grantCid: "uhCEk" + "c".repeat(48),
  grantActionHash: "uhCkk" + "d".repeat(48),
  consentActionHash: "uhCkk" + "e".repeat(48),
});
function setup({
  config = declaration(),
  healthProvider = provider,
  result = native(),
  refuse = false,
} = {}) {
  const calls = [];
  const receipts = [];
  const env = {
    COMPUTE_RUNNER_GRANT_JSON: JSON.stringify(config),
    COMPUTE_PERFORMER: provider,
    ELOHIM_COMPUTE_LOCAL_TOKEN: "private-provider-token-".repeat(3),
  };
  const fetcher = async (url, options) => {
    calls.push({ url: String(url), options });
    if (String(url).endsWith("/health"))
      return Response.json({
        dhtParticipation: { agentKeys: { lamad: healthProvider } },
      });
    return refuse
      ? new Response("sensitive response", { status: 403 })
      : Response.json(result);
  };
  return {
    calls,
    receipts,
    env,
    fetcher,
    saveReceipt: async (receipt) => receipts.push(receipt),
    now,
  };
}
test("absent or disabled declarations never issue grants", async () => {
  const f = setup();
  await bootstrapGrant({ ...f, env: {} });
  await bootstrapGrant({
    ...f,
    env: { COMPUTE_RUNNER_GRANT_JSON: '{"enabled":false}' },
  });
  assert.equal(f.calls.length, 0);
});
test("fixed approved consent retries byte-identically and keeps token out of receipts", async () => {
  const f = setup();
  await bootstrapGrant(f);
  await bootstrapGrant(f);
  assert.equal(f.calls[1].options.body, f.calls[3].options.body);
  assert.deepEqual(JSON.parse(f.calls[1].options.body), declaration().grant);
  assert.equal(f.calls[0].options.headers, undefined);
  assert.equal(
    f.calls[1].options.headers["x-elohim-verified-performer"],
    provider,
  );
  assert.equal(
    f.calls[1].options.headers["x-elohim-compute-token"],
    f.env.ELOHIM_COMPUTE_LOCAL_TOKEN,
  );
  assert.equal(f.receipts[0].stewardOfRecord, "human:matthew");
  assert.equal(f.receipts[0].provider, provider);
  assert.equal(f.receipts[0].declarationProvenance, "operator-approved-config");
  assert.ok(
    !JSON.stringify(f.receipts).includes(f.env.ELOHIM_COMPUTE_LOCAL_TOKEN),
  );
});
test("wrong live provider refuses before writing; mismatched native response refuses startup", async () => {
  const wrongHealth = setup({ healthProvider: recipient });
  await assert.rejects(bootstrapGrant(wrongHealth), /live provider differs/);
  assert.equal(wrongHealth.calls.length, 1);
  for (const result of [
    { ...native(), provider: recipient },
    { ...native(), recipient: provider },
    { ...native(), state: "revoked" },
    { ...native(), grantActionHash: "fabricated" },
  ]) {
    const f = setup({ result });
    await assert.rejects(bootstrapGrant(f));
    assert.equal(f.receipts.length, 0);
  }
  const refused = setup({ refuse: true });
  await assert.rejects(
    bootstrapGrant(refused),
    (error) =>
      error.message.includes("403") &&
      !error.message.includes("sensitive response"),
  );
});
test("approval must name Matthew, Adam, active fixed bounds and separate actors", () => {
  for (const change of [
    (c) => (c.stewardOfRecord = "human:adam"),
    (c) => (c.fixtureProvider = "other"),
    (c) => (c.recipient = c.provider),
    (c) => (c.grant.validUntil = "2026-10-20T00:00:00Z"),
    (c) => (c.grant.bounds.rotation_ttl_days = 0),
    (c) => (c.grant.bounds.reach_ceiling = "intimate"),
    (c) => (c.grant.scope = "hosted-cell"),
    (c) => (c.grant.recipient = provider),
  ]) {
    const c = declaration();
    change(c);
    assert.throws(() => validateGrantDeclaration(c, now));
  }
});
test("remote adapter, missing token and wrong performer refuse without grant calls", async () => {
  for (const edit of [
    (env) => (env.COMPUTE_API_URL = "https://public.example"),
    (env) => (env.ELOHIM_COMPUTE_LOCAL_TOKEN = ""),
    (env) => (env.COMPUTE_PERFORMER = recipient),
  ]) {
    const f = setup();
    edit(f.env);
    await assert.rejects(bootstrapGrant(f));
    assert.equal(f.calls.length, 0);
  }
});
