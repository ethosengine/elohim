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
      /^ORPHANED-BINDING human=Matthew doorway=alpha identifier=matthew\.dowell@alpha\.elohim\.host reason=Session is missing its installed app binding/,
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

  it("names the identifier the doorway stored, not the one sent (2026-11-09: @test re-qualified to @alpha)", async () => {
    const fetch = fakeDoorway({
      agentPubKey: "uhCAk-old",
      doorwayId: "alpha",
      identifier: "ezra@alpha.elohim.host",
    });
    const r = await registerHuman(DOORWAY, {
      ...matthew,
      id: "human-ezra",
      displayName: "Ezra",
    });
    const sent = JSON.parse(String(fetch.mock.calls[0][1]?.body)).identifier;
    expect(sent).toBe("ezra@test.elohim.host");
    expect(r.identifier).toBe("ezra@alpha.elohim.host");
    expect(r.error).toMatch(
      /^ORPHANED-BINDING human=Ezra doorway=alpha identifier=ezra@alpha\.elohim\.host reason=/,
    );
  });

  it("a fresh registration reports the doorway's stored identifier, falling back to the one sent", async () => {
    for (const [answer, expected] of [
      [{ agentPubKey: "uhCAk-new", identifier: "ezra@alpha.elohim.host" }, "ezra@alpha.elohim.host"],
      [{ agentPubKey: "uhCAk-new" }, "ezra@test.elohim.host"],
    ] as const) {
      vi.stubGlobal(
        "fetch",
        vi.fn(async () => new Response(JSON.stringify(answer), { status: 201 })),
      );
      const r = await registerHuman(DOORWAY, { ...matthew, id: "human-ezra", displayName: "Ezra" });
      expect(r.result).toBe("registered");
      expect(r.identifier).toBe(expected);
      vi.unstubAllGlobals();
    }
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
