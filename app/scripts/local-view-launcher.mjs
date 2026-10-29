#!/usr/bin/env node
/** Development delivery adapter. No projection logic, checkout browsing or workflow writes.
 * TODO(identity): replace capability/session adapter with verified signed-in identity + owner binding.
 */
import { createServer } from "node:http";
import { randomBytes, timingSafeEqual, createHash } from "node:crypto";
import {
  readFile,
  writeFile,
  mkdir,
  readdir,
  stat,
  rename,
} from "node:fs/promises";
import { resolve, join, relative, extname } from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { CID } from "multiformats/cid";
import { sha256 } from "multiformats/hashes/sha2";

export async function pinBuild(dist, destination) {
  const assets = [];
  let total = 0;
  async function walk(dir) {
    for (const entry of await readdir(dir, { withFileTypes: true })) {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) {
        await walk(path);
        continue;
      }
      if (!entry.isFile()) continue;
      const relativeName = relative(dist, path);
      if (relativeName === "avodah/views/build.json") continue;
      const name =
        relativeName === "index.csr.html" ? "index.html" : relativeName;
      const bytes = await readFile(path);
      total += bytes.length;
      if (total > 256 * 1024 * 1024 || assets.length >= 10000)
        throw Error("Application exceeds pinned build bounds");
      const address = CID.createV1(0x55, await sha256.digest(bytes)).toString();
      assets.push({ url: `/${name.split("\\").join("/")}`, address });
      await mkdir(join(destination, name, ".."), { recursive: true });
      await writeFile(join(destination, name), bytes);
    }
  }
  await walk(dist);
  if (
    !assets.some((a) => a.url === "/index.html") ||
    !assets.some((a) => a.url === "/avodah/views/offline-worker.js")
  )
    throw Error("Build the production application before launching");
  assets.sort((a, b) => a.url.localeCompare(b.url));
  const id = createHash("sha256").update(JSON.stringify(assets)).digest("hex");
  await writeFile(
    join(destination, "avodah/views/build.json"),
    JSON.stringify({ id, assets }),
  );
  return { id, assets };
}
const secret = () => randomBytes(32).toString("base64url");
const equal = (a, b) =>
  typeof a === "string" &&
  a.length === b.length &&
  timingSafeEqual(Buffer.from(a), Buffer.from(b));
export function accessServer({
  origin,
  upstream,
  token,
  capability,
  viewId,
  build,
  expires = Date.now() + 300000,
}) {
  let unused = true;
  const sessions = new Map();
  const server = createServer(async (req, res) => {
    res.setHeader("Cache-Control", "private, no-store");
    res.setHeader("X-Content-Type-Options", "nosniff");
    res.setHeader("Referrer-Policy", "no-referrer");
    try {
      const url = new URL(req.url, origin);
      // Workspace sign-in may return through a cross-site top-level navigation.
      // Only the public application shell may use that path; API access stays strict.
      const shellNavigation =
        req.method === "GET" &&
        req.headers["sec-fetch-mode"] === "navigate" &&
        req.headers["sec-fetch-dest"] === "document" &&
        !url.pathname.startsWith("/api/local-views");
      if (
        req.headers.host !== new URL(origin).host ||
        (req.headers.origin && req.headers.origin !== origin) ||
        (req.headers["sec-fetch-site"] === "cross-site" && !shellNavigation)
      ) {
        res.writeHead(403).end();
        return;
      }
      if (url.pathname.startsWith("/api/local-views")) {
        if (req.method === "POST" && req.headers.origin !== origin) {
          res.writeHead(403).end();
          return;
        }
        if (url.search) {
          res.writeHead(400).end();
          return;
        }
        if (
          url.pathname === "/api/local-views/session" &&
          req.method === "POST"
        ) {
          let body = "";
          for await (const chunk of req) {
            body += chunk;
            if (body.length > 2048) {
              res.writeHead(413).end();
              return;
            }
          }
          const invalid = !equal(JSON.parse(body).capability, capability);
          if (!unused || Date.now() > expires || invalid) {
            res.writeHead(401, { "Content-Type": "application/json" }).end(
              JSON.stringify({
                error:
                  "Private launch link expired, was already used, or was replaced. Generate a fresh link with LOCAL_VIEW_RENEW=1 just dev view <declaration>, then open the complete URL.",
              }),
            );
            return;
          }
          unused = false;
          const session = secret();
          sessions.set(session, Date.now() + 8 * 3600000);
          res.setHeader(
            "Set-Cookie",
            `local_view_session=${session}; HttpOnly; SameSite=Strict; Path=/api/local-views; Max-Age=28800${origin.startsWith("https:") ? "; Secure" : ""}`,
          );
          res.setHeader("Content-Type", "application/json");
          res.end("{}");
          return;
        }
        const session = /(?:^|;\s*)local_view_session=([^;]+)/.exec(
          req.headers.cookie ?? "",
        )?.[1];
        if (!session || (sessions.get(session) ?? 0) <= Date.now()) {
          res.writeHead(401).end();
          return;
        }
        const prefix = `/api/local-views/${encodeURIComponent(viewId)}`;
        const allowed =
          (req.method === "GET" &&
            (url.pathname === "/api/local-views" ||
              url.pathname === `${prefix}/history` ||
              new RegExp(`^${prefix}/snapshots/bafkrei[a-z2-7]+$`).test(
                url.pathname,
              ))) ||
          (req.method === "POST" && url.pathname === `${prefix}/capture`);
        if (!allowed) {
          res.writeHead(404).end();
          return;
        }
        const response = await fetch(`${upstream}${url.pathname}`, {
          method: req.method,
          headers: { Authorization: `Bearer ${token}` },
          redirect: "error",
        });
        res.writeHead(response.status, { "Content-Type": "application/json" });
        res.end(Buffer.from(await response.arrayBuffer()));
        return;
      }
      if (req.method !== "GET" && req.method !== "HEAD") {
        res.writeHead(405).end();
        return;
      }
      const pathname = decodeURIComponent(url.pathname);
      const name =
        pathname === `/avodah/views/${viewId}`
          ? "index.html"
          : pathname.slice(1);
      const path = resolve(build, name);
      if (
        !path.startsWith(resolve(build) + "/") ||
        !(await stat(path)).isFile()
      ) {
        res.writeHead(404).end();
        return;
      }
      const types = {
        ".html": "text/html",
        ".js": "text/javascript",
        ".css": "text/css",
        ".json": "application/json",
        ".woff2": "font/woff2",
        ".svg": "image/svg+xml",
      };
      res.setHeader(
        "Content-Type",
        types[extname(path)] ?? "application/octet-stream",
      );
      res.end(req.method === "HEAD" ? undefined : await readFile(path));
    } catch (error) {
      if (!res.headersSent) res.writeHead(error.code === "ENOENT" ? 404 : 502);
      res.end();
    }
  });
  server.renewCapability = (next) => {
    capability = next;
    expires = Date.now() + 300000;
    unused = true;
  };
  return server;
}
async function main() {
  const repository = resolve(fileURLToPath(new URL("../..", import.meta.url)));
  const root = resolve(process.env.LOCAL_VIEW_ROOT ?? repository);
  const declaration = process.argv[2];
  if (
    !declaration ||
    declaration.startsWith("/") ||
    declaration.split("/").includes("..")
  )
    throw Error("Provide a repository-relative declaration");
  if (!(await stat(resolve(root, declaration))).isFile())
    throw Error("Declaration must be a file");
  const label = process.env.LOCAL_VIEW_ID ?? "idle-is-free";
  const binding = createHash("sha256")
    .update(`${root}\n${declaration}`)
    .digest("hex")
    .slice(0, 12);
  const viewId = `${label}-${binding}`;
  if (!/^[a-zA-Z0-9_-]{1,80}$/.test(viewId)) throw Error("Invalid view id");
  const state = resolve(repository, "genesis/local-dev/declared-views", viewId);
  const pidPath = join(state, "launcher.pid");
  const linkPath = join(state, "launch-link.txt");
  if (process.env.LOCAL_VIEW_RENEW === "1") {
    const pid = Number(await readFile(pidPath, "utf8"));
    if (!Number.isSafeInteger(pid) || pid <= 1)
      throw Error("Invalid launcher PID");
    // Validate the process before signalling: stale PID files must not target another service.
    const args = (await readFile(`/proc/${pid}/cmdline`, "utf8")).split("\0");
    if (
      !args.some(
        (arg) => resolve(repository, arg) === fileURLToPath(import.meta.url),
      ) ||
      !args.includes(declaration)
    )
      throw Error("Launcher is no longer running; start this view again");
    const previous = await readFile(linkPath, "utf8");
    process.kill(pid, "SIGUSR2");
    for (let attempt = 0; attempt < 100; attempt++) {
      await new Promise((done) => setTimeout(done, 50));
      if ((await readFile(linkPath, "utf8")) !== previous) {
        console.log(
          `Fresh single-use link ready (expires in five minutes).\nRead privately: cat ${linkPath}`,
        );
        return;
      }
    }
    throw Error("Launcher did not renew the link");
  }
  await mkdir(state, { recursive: true, mode: 0o700 });
  const build = join(state, `build-${Date.now()}`);
  const manifest = await pinBuild(
    resolve(
      repository,
      process.env.LOCAL_VIEW_DIST ?? "app/elohim-app/dist/elohim-app/browser",
    ),
    build,
  );
  const port = Number(process.env.LOCAL_VIEW_PORT ?? 4200);
  const readerPort = Number(process.env.LOCAL_VIEW_READER_PORT ?? 8097);
  const origin = process.env.LOCAL_VIEW_ORIGIN ?? `http://localhost:${port}`;
  if (new URL(origin).origin !== origin)
    throw Error("LOCAL_VIEW_ORIGIN must be an exact origin");
  const token = secret(),
    capability = secret();
  const config = join(state, "reader.json");
  await writeFile(
    config,
    JSON.stringify({
      root,
      store: join(state, "store"),
      views: { [viewId]: declaration },
      listen: `127.0.0.1:${readerPort}`,
    }),
    { mode: 0o600 },
  );
  const binary = process.env.ELOHIM_NODE_BIN;
  if (!binary)
    throw Error(
      "Set ELOHIM_NODE_BIN to the local-view-only node build (see local-view-launcher.sh)",
    );
  const child = spawn(binary, ["--local-view", config], {
    env: { ...process.env, LOCAL_VIEW_READER_TOKEN: token },
    stdio: ["ignore", "ignore", "inherit"],
  });
  // A Che workspace supplies its existing doorway URL; only that relay gets this token.
  const upstream =
    process.env.LOCAL_VIEW_DOORWAY_URL ?? `http://127.0.0.1:${readerPort}`;
  const server = accessServer({
    origin,
    upstream,
    token,
    capability,
    viewId,
    build,
  });
  server.on("error", (error) => {
    child.kill("SIGTERM");
    console.error(`Viewer could not listen: ${error.message}`);
    process.exitCode = 1;
  });
  child.on("error", (error) => {
    server.close();
    console.error(`Reader could not start: ${error.message}`);
    process.exitCode = 1;
  });
  process.on("SIGUSR2", () => {
    const next = secret();
    server.renewCapability(next);
    const temporary = `${linkPath}.next`;
    writeFile(
      temporary,
      `${origin}/avodah/views/${viewId}#capability=${next}\n`,
      { mode: 0o600 },
    )
      .then(() => rename(temporary, linkPath))
      .catch(() => console.error("Could not persist renewed launch link"));
  });
  await writeFile(pidPath, String(process.pid), { mode: 0o600 });
  await writeFile(
    linkPath,
    `${origin}/avodah/views/${viewId}#capability=${capability}\n`,
    { mode: 0o600 },
  );
  server.listen(port, "0.0.0.0", () =>
    console.log(
      `Pinned build ${manifest.id}\nPrivate launch link: ${linkPath}\nReader: loopback ${readerPort}; viewer: ${origin}`,
    ),
  );
  child.on("exit", (code) => {
    server.close();
    process.exitCode = code ?? 1;
  });
  for (const signal of ["SIGINT", "SIGTERM"])
    process.on(signal, () => {
      child.kill("SIGTERM");
      server.close();
    });
}
if (process.argv[1] === fileURLToPath(import.meta.url))
  main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
