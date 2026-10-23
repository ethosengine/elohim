// Serialized into the existing bounded CI pod read. Never permits grant issuance.
export async function inspectProviderBootstrap({
  env = process.env,
  fetcher = fetch,
  load = () => import("/opt/compute/grant-bootstrap.mjs"),
} = {}) {
  const result = {
    kind: "provider-read-observation",
    nativeStatusChecked: false,
  };
  const stop = new Error("issuance intentionally not attempted");
  const key = (value) =>
    typeof value === "string" && /^uhCAk[A-Za-z0-9_-]{48}$/.test(value);
  const reasons = new Map([
    ["grant declaration is not JSON", "declaration-invalid"],
    ["grant declaration must be explicitly enabled", "declaration-disabled"],
    [
      "worker performer differs from approved fixture provider",
      "performer-mismatch",
    ],
    ["provider compute capability is absent or invalid", "capability-invalid"],
    ["provider health unavailable", "health-unavailable"],
    ["provider health refused identity read", "health-refused"],
    [
      "live provider differs from approved fixture provider",
      "provider-identity-mismatch",
    ],
  ]);
  async function boundedJson(response) {
    const reader = response.body.getReader();
    const parts = [];
    let size = 0;
    try {
      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        size += value.length;
        if (size > 65536) throw new Error("response bound");
        parts.push(value);
      }
      return JSON.parse(Buffer.concat(parts).toString("utf8"));
    } finally {
      await reader.cancel().catch(() => {});
    }
  }
  try {
    const { bootstrapGrant } = await load();
    await bootstrapGrant({
      env,
      fetcher: async (url, options = {}) => {
        const target = new URL(url);
        // The transport never forwards POST, even if the imported module changes.
        if (
          options.method === "POST" &&
          target.href === "http://127.0.0.1:8090/api/v1/compute/grants"
        )
          throw stop;
        if (
          (options.method || "GET") !== "GET" ||
          target.href !== "http://127.0.0.1:8090/health"
        )
          throw new Error("transport refused");
        const response = await fetcher(target, {
          method: "GET",
          redirect: "error",
          signal: AbortSignal.timeout(5000),
        });
        result.healthStatus = response.status;
        if (!response.ok) {
          await response.body?.cancel();
          return { ok: false };
        }
        const body = await boundedJson(response);
        const observed = body.dhtParticipation?.agentKeys?.lamad;
        if (key(observed)) result.observedProvider = observed;
        return { ok: true, json: async () => body };
      },
      saveReceipt: async () => {
        throw new Error("receipt write refused");
      },
    });
    result.phase = "declaration-disabled";
    return result;
  } catch (error) {
    if (error !== stop) {
      result.phase =
        reasons.get(error?.message) || "pre-issuance-check-refused";
      return result;
    }
  }
  result.phase = "pre-issuance-checks-passed";
  // Projection-only read: neither authors nor proves current signed native state.
  try {
    const declaration = JSON.parse(env.COMPUTE_RUNNER_GRANT_JSON);
    const target = new URL("http://127.0.0.1:8090/api/v1/compute/grants");
    target.searchParams.set("scope", "measure-stage");
    target.searchParams.set("recipient", declaration.recipient);
    const response = await fetcher(target, {
      method: "GET",
      redirect: "error",
      signal: AbortSignal.timeout(5000),
      headers: {
        "x-elohim-verified-performer": declaration.provider,
        "x-elohim-compute-token": env.ELOHIM_COMPUTE_LOCAL_TOKEN,
      },
    });
    result.grantReadStatus = response.status;
    if (!response.ok) {
      await response.body?.cancel();
      return result;
    }
    const body = await boundedJson(response);
    if (body.provider !== declaration.provider || !Array.isArray(body.grants))
      throw new Error("projection shape");
    result.projectedGrantCount = body.grants.length;
    result.projectedGrants = body.grants.slice(0, 8).map((row) => {
      if (
        row.provider !== declaration.provider ||
        row.recipient !== declaration.recipient ||
        row.scope !== "measure-stage" ||
        !/^uhCEk[A-Za-z0-9_-]{48}$/.test(row.grantCid || "") ||
        !["active", "offered", "withdrawn", "expired", "revoked"].includes(
          row.state,
        )
      )
        throw new Error("projection shape");
      return { grantCid: row.grantCid, state: row.state };
    });
  } catch {
    delete result.projectedGrants;
    delete result.projectedGrantCount;
    result.grantReadUnavailable = true;
  }
  return result;
}
