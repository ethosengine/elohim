import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { request as httpRequest } from "node:http";
import { accessServer, viewerListenHost } from "./local-view-launcher.mjs";

test("launch capability is single-use, origin-bound, expiring and view-scoped", async () => {
  const server = accessServer({
    origin: "http://localhost:14287",
    upstream: "http://127.0.0.1:1",
    token: "reader-secret",
    capability: "one-use-secret",
    viewId: "orchard",
    build: "/not-served",
  });
  await new Promise((resolve) => server.listen(14287, resolve));
  try {
    const request = (path, init = {}) =>
      fetch(`http://localhost:14287/api/local-views${path}`, init);
    assert.equal((await request("/orchard/history")).status, 401);
    assert.equal(
      (
        await request("/session", {
          method: "POST",
          headers: { origin: "http://evil.invalid" },
          body: "{}",
        })
      ).status,
      403,
    );
    const init = {
      method: "POST",
      headers: { origin: "http://localhost:14287" },
      body: JSON.stringify({ capability: "one-use-secret" }),
    };
    const exchange = await request("/session", init);
    assert.equal(exchange.status, 200);
    assert.match(exchange.headers.get("cache-control"), /no-store/);
    const cookie = exchange.headers.get("set-cookie").split(";")[0];
    assert.match(exchange.headers.get("set-cookie"), /HttpOnly/);
    assert.equal((await request("/session", init)).status, 401);
    server.renewCapability("renewed-secret");
    assert.equal((await request("/session", init)).status, 401);
    const renewed = {
      ...init,
      body: JSON.stringify({ capability: "renewed-secret" }),
    };
    assert.equal((await request("/session", renewed)).status, 200);
    assert.equal((await request("/session", renewed)).status, 401);

    assert.equal(
      (await request("/other/history", { headers: { cookie } })).status,
      404,
    );
    assert.equal(
      (await request("/orchard/history?root=/etc", { headers: { cookie } }))
        .status,
      400,
    );
    assert.equal(
      (await request("/orchard/snapshots/etc", { headers: { cookie } })).status,
      404,
    );
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
});

test("an expired launch capability cannot create a session", async () => {
  const server = accessServer({
    origin: "http://localhost:14288",
    upstream: "http://127.0.0.1:1",
    token: "unused",
    capability: "expired",
    viewId: "orchard",
    build: "/not-served",
    expires: 0,
  });
  await new Promise((resolve) => server.listen(14288, resolve));
  try {
    const response = await fetch(
      "http://localhost:14288/api/local-views/session",
      {
        method: "POST",
        headers: { origin: "http://localhost:14288" },
        body: JSON.stringify({ capability: "expired" }),
      },
    );
    assert.equal(response.status, 401);
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
});

test("workspace redirect serves the shell but never relaxes API or host checks", async () => {
  const build = await mkdtemp(join(tmpdir(), "local-view-origin-"));
  await writeFile(join(build, "index.html"), "viewer shell");
  const host = "workspace-angular-dev.code.ethosengine.com";
  const origin = `https://${host}`;
  const server = accessServer({
    origin,
    upstream: "http://127.0.0.1:1",
    token: "unused",
    capability: "workspace-capability",
    viewId: "orchard",
    build,
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const request = (path, headers, method = "GET", body = "") =>
    new Promise((resolve, reject) => {
      const req = httpRequest(
        {
          hostname: "127.0.0.1",
          port: server.address().port,
          path,
          method,
          headers: { host, ...headers },
        },
        (res) => {
          res.resume();
          res.on("end", () => resolve(res));
        },
      );
      req.on("error", reject);
      req.end(body);
    });
  const redirect = {
    "sec-fetch-site": "cross-site",
    "sec-fetch-mode": "navigate",
    "sec-fetch-dest": "document",
  };
  try {
    assert.equal(
      (await request("/avodah/views/orchard", redirect)).statusCode,
      200,
    );
    assert.equal((await request("/api/local-views", redirect)).statusCode, 403);
    assert.equal(
      (
        await request("/avodah/views/orchard", {
          ...redirect,
          host: "evil.invalid",
        })
      ).statusCode,
      403,
    );
    assert.equal(
      (
        await request("/avodah/views/orchard", {
          "sec-fetch-site": "cross-site",
        })
      ).statusCode,
      403,
    );
    const exchange = await request(
      "/api/local-views/session",
      { origin, "sec-fetch-site": "same-origin" },
      "POST",
      JSON.stringify({ capability: "workspace-capability" }),
    );
    assert.equal(exchange.statusCode, 200);
    assert.match(exchange.headers["set-cookie"][0], /Secure/);
  } finally {
    await new Promise((resolve) => server.close(resolve));
    await rm(build, { recursive: true, force: true });
  }
});

test("a loopback origin binds the viewer to loopback; only a workspace origin binds every interface", () => {
  assert.equal(viewerListenHost("http://localhost:4200"), "127.0.0.1");
  assert.equal(viewerListenHost("http://127.0.0.1:4200"), "127.0.0.1");
  assert.equal(viewerListenHost("http://[::1]:4200"), "::1");
  assert.equal(
    viewerListenHost("https://workspace-angular-dev.code.ethosengine.com"),
    "0.0.0.0",
  );
  assert.equal(viewerListenHost("http://localhost.evil.invalid:4200"), "0.0.0.0");
});
