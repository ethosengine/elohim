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

test("normal attempt emits bounded native categories without extra POST or secret output", async () => {
  for (const [status, body, category] of [
    [
      500,
      "Error: Invalid input: compute grant: native consent record unavailable",
      "consent-unavailable",
    ],
    [
      500,
      "Error: Conductor error: secret-token-dynamic-detail",
      "conductor-error",
    ],
    [
      403,
      '{"reason":"local-compute-capability-required","secret":"secret-token-dynamic-detail"}',
      "local-capability-refused",
    ],
    [503, '{"status":"catching-up","retryAfter":7}', "catching-up"],
    [503, '{"error":"local conductor unavailable"}', "conductor-unavailable"],
    [500, "secret-token-dynamic-detail".repeat(4000), "body-limit"],
    [200, "not JSON secret-token-dynamic-detail", "response-json-invalid"],
    [
      200,
      JSON.stringify({ ...native(), provider: recipient }),
      "native-response-invalid",
    ],
  ]) {
    const f = setup();
    const diagnostics = [];
    const original = f.fetcher;
    f.fetcher = async (url, init) => {
      if (init?.method === "POST") {
        f.calls.push({ url: String(url), options: init });
        return new Response(body, { status });
      }
      return original(url, init);
    };
    await assert.rejects(
      bootstrapGrant({ ...f, observe: (d) => diagnostics.push(d) }),
    );
    assert.equal(f.calls.filter((c) => c.options.method === "POST").length, 1);
    assert.equal(f.calls[1].options.body, JSON.stringify(declaration().grant));
    assert.equal(f.calls[1].options.redirect, "error");
    assert.equal(f.receipts.length, 0);
    assert.equal(diagnostics.length, 1);
    assert.equal(diagnostics[0].category, category);
    assert.equal(diagnostics[0].httpStatus, status);
    assert.equal(diagnostics[0].outcome, "unknown");
    assert.deepEqual(Object.keys(diagnostics[0]).sort(), [
      "category",
      "httpStatus",
      "outcome",
      "phase",
    ]);
    const output = JSON.stringify(diagnostics);
    for (const secret of [
      "secret-token-dynamic-detail",
      provider,
      recipient,
      f.env.ELOHIM_COMPUTE_LOCAL_TOKEN,
      f.env.COMPUTE_RUNNER_GRANT_JSON,
    ])
      assert.ok(!output.includes(secret));
  }
});

test("timeout, connection, body and receipt failures after POST have unknown outcome", async (t) => {
  const deadlines = [];
  const controllers = [];
  t.mock.method(AbortSignal, "timeout", (ms) => {
    deadlines.push(ms);
    const controller = new AbortController();
    controllers.push(controller);
    return controller.signal;
  });
  for (const kind of [
    "timeout",
    "connection",
    "body",
    "stalled-body",
    "receipt",
  ]) {
    const f = setup();
    const original = f.fetcher;
    const diagnostics = [];
    f.fetcher = async (url, init) => {
      if (init?.method !== "POST") return original(url, init);
      f.calls.push({ url: String(url), options: init });
      if (kind === "timeout")
        throw Object.assign(new Error("SECRET"), { name: "TimeoutError" });
      if (kind === "connection") throw new Error("SECRET");
      if (kind === "body")
        return new Response(
          new ReadableStream({
            start(c) {
              c.error(new Error("SECRET"));
            },
          }),
        );
      if (kind === "stalled-body") {
        setImmediate(() => controllers.at(-1).abort());
        return new Response(
          new ReadableStream({
            pull() {
              return new Promise(() => {});
            },
          }),
        );
      }
      return Response.json(native());
    };
    if (kind === "receipt")
      f.saveReceipt = async () => {
        throw new Error("SECRET");
      };
    await assert.rejects(
      bootstrapGrant({ ...f, observe: (d) => diagnostics.push(d) }),
    );
    assert.equal(f.calls.filter((c) => c.options.method === "POST").length, 1);
    assert.equal(diagnostics[0].outcome, "unknown");
    assert.equal(
      diagnostics[0].category,
      {
        timeout: "request-timeout",
        connection: "connection-failed",
        body: "body-read-failed",
        "stalled-body": "request-timeout",
        receipt: "receipt-write-failed",
      }[kind],
    );
    assert.ok(!JSON.stringify(diagnostics).includes("SECRET"));
  }
  assert.ok(deadlines.length === 10 && deadlines.every((ms) => ms === 30000));
});

test("pre-POST refusal stays not-submitted; an observer cannot swallow a receipt error", async () => {
  const f = setup({ healthProvider: recipient });
  const diagnostics = [];
  await assert.rejects(
    bootstrapGrant({ ...f, observe: (d) => diagnostics.push(d) }),
  );
  assert.equal(diagnostics[0].phase, "health-identity");
  assert.equal(diagnostics[0].outcome, "not-submitted");
  assert.equal(f.calls.filter((c) => c.options.method === "POST").length, 0);
  const receiptFailure = setup();
  let observed = 0;
  await assert.rejects(
    bootstrapGrant({
      ...receiptFailure,
      saveReceipt: async () => {
        throw new Error("receipt not saved");
      },
      observe: () => {
        observed++;
        throw new Error("observer failure");
      },
    }),
    /receipt not saved/,
  );
  assert.equal(observed, 1);
  assert.equal(
    receiptFailure.calls.filter((c) => c.options.method === "POST").length,
    1,
  );
});

test("successful observed attempt preserves the exact receipt and fixed grant without failure observation", async () => {
  const original = setup();
  const expected = await bootstrapGrant(original);
  const observed = setup();
  const diagnostics = [];
  const receipt = await bootstrapGrant({
    ...observed,
    observe: (d) => diagnostics.push(d),
  });
  assert.deepEqual(receipt, expected);
  assert.deepEqual(observed.receipts, original.receipts);
  assert.deepEqual(diagnostics, []);
  const posts = observed.calls.filter((c) => c.options.method === "POST");
  assert.equal(posts.length, 1);
  assert.equal(posts[0].options.body, original.calls[1].options.body);
  assert.deepEqual(JSON.parse(posts[0].options.body), declaration().grant);
  assert.equal(receipt.provider, provider);
  assert.equal(receipt.recipient, recipient);
});

test("health refusal cancels its unread body without waiting on cancellation", async () => {
  const f = setup();
  let cancelled = false;
  const diagnostics = [];
  f.fetcher = async () =>
    new Response(
      new ReadableStream({
        cancel() {
          cancelled = true;
          return new Promise(() => {});
        },
      }),
      { status: 503 },
    );
  await assert.rejects(
    bootstrapGrant({ ...f, observe: (d) => diagnostics.push(d) }),
    /health refused/,
  );
  assert.equal(cancelled, true);
  assert.equal(diagnostics[0].category, "health-http-refused");
  assert.equal(diagnostics[0].outcome, "not-submitted");
  assert.deepEqual(f.receipts, []);
});
