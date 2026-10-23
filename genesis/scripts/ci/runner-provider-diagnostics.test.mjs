import test from "node:test";
import assert from "node:assert/strict";
import { runInNewContext } from "node:vm";
import { inspectProviderBootstrap } from "./runner-provider-diagnostics.mjs";
import * as bootstrap from "../../agentic/compute/grant-bootstrap.mjs";

const provider = "uhCAk" + "a".repeat(48);
const recipient = "uhCAk" + "b".repeat(48);
function environment() {
  const now = Date.now();
  return {
    COMPUTE_API_URL: "http://127.0.0.1:8090",
    COMPUTE_PERFORMER: provider,
    ELOHIM_COMPUTE_LOCAL_TOKEN: "private-capability-never-exported-123456789",
    COMPUTE_RUNNER_GRANT_JSON: JSON.stringify({
      enabled: true,
      stewardOfRecord: "human:matthew",
      fixtureProvider: "adam",
      provider,
      recipient,
      grant: {
        recipient,
        scope: "measure-stage",
        issuedAt: new Date(now - 1000).toISOString(),
        validFrom: new Date(now - 1000).toISOString(),
        validUntil: new Date(now + 3600000).toISOString(),
        bounds: {
          epr_scope: ["*"],
          reach_ceiling: "commons",
          rate_per_hour: 4,
          rotation_ttl_days: 1,
        },
      },
    }),
  };
}
test("actual bootstrap validates parties but diagnostic transport never issues", async () => {
  const env = environment(),
    calls = [];
  const serialized = runInNewContext(
    `(${inspectProviderBootstrap.toString()})`,
    { Buffer, URL, AbortSignal },
  );
  const result = await serialized({
    env,
    load: async () => bootstrap,
    fetcher: async (url, options) => {
      calls.push({ url: String(url), method: options.method });
      assert.equal(options.method, "GET");
      if (url.pathname === "/health")
        return Response.json({
          dhtParticipation: { agentKeys: { lamad: provider } },
        });
      assert.equal(url.searchParams.get("recipient"), recipient);
      assert.equal(
        options.headers["x-elohim-compute-token"],
        env.ELOHIM_COMPUTE_LOCAL_TOKEN,
      );
      return Response.json({
        provider,
        grants: [
          {
            provider,
            recipient,
            scope: "measure-stage",
            grantCid: "uhCEk" + "c".repeat(48),
            state: "active",
            ignored: env.ELOHIM_COMPUTE_LOCAL_TOKEN,
          },
        ],
      });
    },
  });
  assert.equal(calls.length, 2);
  assert.equal(result.phase, "pre-issuance-checks-passed");
  assert.equal(result.nativeStatusChecked, false);
  assert.equal(result.projectedGrants[0].state, "active");
  assert(!JSON.stringify(result).includes(env.ELOHIM_COMPUTE_LOCAL_TOKEN));
});
test("identity mismatch stops before any authenticated read", async () => {
  let calls = 0;
  const result = await inspectProviderBootstrap({
    env: environment(),
    load: async () => bootstrap,
    fetcher: async () => {
      calls++;
      return Response.json({
        dhtParticipation: { agentKeys: { lamad: recipient } },
      });
    },
  });
  assert.equal(calls, 1);
  assert.equal(result.phase, "provider-identity-mismatch");
});
test("arbitrary exceptions and oversized bodies cannot leak private bytes", async () => {
  const secret = "SECRET-PRIVATE-BYTES";
  for (const load of [
    async () => {
      throw new Error(secret);
    },
    async () => bootstrap,
  ]) {
    const result = await inspectProviderBootstrap({
      env: environment(),
      load,
      fetcher: async () => new Response(secret.repeat(5000)),
    });
    assert(!JSON.stringify(result).includes(secret));
    assert(
      ["pre-issuance-check-refused", "health-unavailable"].includes(
        result.phase,
      ),
    );
  }
});
test("projection HTTP refusal reports only status and remains unproven", async () => {
  const result = await inspectProviderBootstrap({
    env: environment(),
    load: async () => bootstrap,
    fetcher: async (url) =>
      url.pathname === "/health"
        ? Response.json({
            dhtParticipation: { agentKeys: { lamad: provider } },
          })
        : new Response("private error", { status: 403 }),
  });
  assert.equal(result.grantReadStatus, 403);
  assert.equal(result.nativeStatusChecked, false);
  assert(!JSON.stringify(result).includes("private error"));
});
test("changed imported code cannot forward an arbitrary POST", async () => {
  let calls = 0;
  const result = await inspectProviderBootstrap({
    env: environment(),
    load: async () => ({
      bootstrapGrant: async ({ fetcher }) =>
        fetcher("http://127.0.0.1:8090/unexpected", { method: "POST" }),
    }),
    fetcher: async () => {
      calls++;
      throw new Error("must not run");
    },
  });
  assert.equal(calls, 0);
  assert.equal(result.phase, "pre-issuance-check-refused");
});

test("native proposed and withdrawn lifecycle rows remain projection-only observations", async () => {
  for (const state of ["proposed", "cancelled", "sunset"]) {
    const env = environment();
    const result = await inspectProviderBootstrap({
      env,
      load: async () => bootstrap,
      fetcher: async (url, options) => {
        assert.equal(options.method, "GET");
        return url.pathname === "/health"
          ? Response.json({
              dhtParticipation: { agentKeys: { lamad: provider } },
            })
          : Response.json({
              provider,
              grants: [
                {
                  provider,
                  recipient,
                  scope: "measure-stage",
                  grantCid: "uhCEk" + "c".repeat(48),
                  state,
                  private: env.ELOHIM_COMPUTE_LOCAL_TOKEN,
                },
              ],
            });
      },
    });
    assert.deepEqual(result.projectedGrants, [
      { grantCid: "uhCEk" + "c".repeat(48), state },
    ]);
    assert.equal(result.nativeStatusChecked, false);
    assert.equal(result.grantReadUnavailable, undefined);
    assert(!JSON.stringify(result).includes(env.ELOHIM_COMPUTE_LOCAL_TOKEN));
  }
});
test("projection refusal is classified without disclosing arbitrary response data", async () => {
  for (const [body, reason] of [
    [{ provider, grants: [{ state: "PRIVATE-UNKNOWN" }] }, "projection-row"],
    [{ secret: "PRIVATE-UNKNOWN" }, "projection-envelope"],
  ]) {
    const result = await inspectProviderBootstrap({
      env: environment(),
      load: async () => bootstrap,
      fetcher: async (url) =>
        url.pathname === "/health"
          ? Response.json({
              dhtParticipation: { agentKeys: { lamad: provider } },
            })
          : Response.json(body),
    });
    assert.equal(result.grantReadRefusal, reason);
    assert.equal(result.grantReadUnavailable, true);
    assert.equal(result.projectedGrants, undefined);
    assert(!JSON.stringify(result).includes("PRIVATE-UNKNOWN"));
  }
});

test("bounded projection body and transport failures keep fixed classifications", async () => {
  for (const [respond, reason] of [
    [() => new Response("PRIVATE-MALFORMED"), "invalid-json"],
    [() => new Response("x".repeat(65537)), "response-bound"],
    [
      () => {
        throw new TypeError("PRIVATE-TRANSPORT");
      },
      "read-unavailable",
    ],
    [
      () => {
        throw new DOMException("PRIVATE-TIMEOUT", "TimeoutError");
      },
      "timeout",
    ],
  ]) {
    const result = await inspectProviderBootstrap({
      env: environment(),
      load: async () => bootstrap,
      fetcher: async (url) =>
        url.pathname === "/health"
          ? Response.json({
              dhtParticipation: { agentKeys: { lamad: provider } },
            })
          : respond(),
    });
    assert.equal(result.grantReadRefusal, reason);
    assert.equal(result.grantReadUnavailable, true);
    assert(!JSON.stringify(result).includes("PRIVATE-"));
  }
});
