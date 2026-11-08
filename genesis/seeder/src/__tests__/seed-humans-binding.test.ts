/**
 * The seeder verifies the outcome it claims: a 409 + working login is not "already good"
 * unless the login also proves the session's installed-app binding resolves (alpha,
 * 2026-11-08: re-genesised fleet, surviving doorway `users` rows, /hc/connect 401).
 */
import { afterEach, describe, expect, it, vi } from "vitest";

import { registerHuman, selectActiveHumans } from "../seed-humans.js";

const DOORWAY = "http://fake-doorway.test";
const matthew = {
  id: "human-matthew-manager",
  displayName: "Matthew",
  bio: "b",
  category: "steward",
  profileReach: "public",
  agencyPhase: "hosted",
};

function fakeDoorway(login: Record<string, unknown>): ReturnType<typeof vi.fn> {
  const fn = vi.fn(async (url: string) => {
    if (url.endsWith("/auth/register"))
      return new Response('{"error":"exists"}', { status: 409 });
    if (url.endsWith("/auth/login"))
      return new Response(JSON.stringify(login), { status: 200 });
    return new Response("unexpected", { status: 500 });
  });
  vi.stubGlobal("fetch", fn);
  return fn;
}

afterEach(() => vi.unstubAllGlobals());

describe("seed-humans binding verification", () => {
  it("409 + login ok + binding resolves -> exists (pass)", async () => {
    fakeDoorway({
      agentPubKey: "uhCAk-new",
      installedAppId: "elohim-matthew",
      doorwayId: "alpha",
    });
    const r = await registerHuman(DOORWAY, matthew);
    expect(r.result).toBe("exists");
    expect(r.error).toBeUndefined();
  });

  it("409 + login ok + binding missing -> ORPHANED-BINDING failure", async () => {
    fakeDoorway({ agentPubKey: "uhCAk-old", doorwayId: "alpha" });
    const r = await registerHuman(DOORWAY, matthew);
    expect(r.result).toBe("failed");
    expect(r.error).toMatch(
      /^ORPHANED-BINDING human=Matthew doorway=alpha reason=Session is missing its installed app binding/,
    );
  });

  it("a doorway-phase operator whose row outlived a re-key -> ORPHANED-BINDING (the 2026-11-08 alpha case)", async () => {
    fakeDoorway({ agentPubKey: "uhCAk-pre-rekey", doorwayId: "alpha" });
    const r = await registerHuman(DOORWAY, {
      ...matthew,
      agencyPhase: "doorway",
    });
    expect(r.result).toBe("failed");
    expect(r.error).toMatch(/^ORPHANED-BINDING human=Matthew doorway=alpha /);
  });

  it("does not demand a doorway-installed app of a node human", async () => {
    fakeDoorway({ agentPubKey: "uhCAk-node" });
    const r = await registerHuman(DOORWAY, {
      ...matthew,
      displayName: "Jessica",
      agencyPhase: "node",
    });
    expect(r.result).toBe("exists");
  });

  it("suspended humans are skipped, not failed or probed", async () => {
    const fn = fakeDoorway({});
    const active = selectActiveHumans([matthew], new Set(["matthew"]));
    expect(active).toEqual([]);
    expect(fn).not.toHaveBeenCalled();
    expect(selectActiveHumans([matthew], new Set())).toEqual([matthew]);
  });
});
