import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import {
  availableOffline,
  UnsupportedCapabilityError,
  UnsupportedLifecycle,
  type AppRuntime,
  type AppReleaseView,
  type AppReleaseAnswer,
  type AppInstanceAnswer,
} from "../src/app-lifecycle";

const release: AppReleaseView = {
  appId: "orchard",
  releaseCid: "fixture-release",
  target: "browser",
  artifactCids: ["fixture-artifact"],
  requiredCapabilities: [],
};

describe("portable lifecycle contract", () => {
  it("types the same tagged answers that Rust serializes", () => {
    const fixtures = JSON.parse(
      readFileSync(
        new URL(
          "../../../../crates/elohim-views/tests/fixtures/app-lifecycle.json",
          import.meta.url,
        ),
        "utf8",
      ),
    );
    const present: AppReleaseAnswer = { state: "present", value: release };
    const unreachable: AppInstanceAnswer = {
      state: "unreachable",
      reason: "timeout",
    };
    expect(present).toEqual(fixtures["app-release-answer"]);
    expect(unreachable).toEqual(fixtures["app-instance-answer"]);
    // @ts-expect-error A timeout cannot claim observed absence.
    const invalid: AppReleaseAnswer = { state: "absent", reason: "timeout" };
    expect(invalid).toEqual({ state: "absent", reason: "timeout" });
  });
  it("refuses unsupported operations instead of claiming execution or durability", async () => {
    const adapter = new UnsupportedLifecycle();
    const pending = [
      adapter.resolve("orchard", "browser"),
      adapter.ensureInstance({
        release,
        operationId: "open-1",
        grantRef: "not-authority",
      }),
      adapter.acquire(release),
      adapter.prepare("instance"),
      adapter.attach({
        instanceId: "instance",
        grantRef: "grant",
        origin: "https://example.test",
      }),
      adapter.request({
        scope: "own:observations",
        urgency: "foreground",
        targets: [],
      }),
    ];
    const results = await Promise.allSettled(pending);
    for (const result of results) {
      expect(result.status).toBe("rejected");
      if (result.status === "rejected")
        expect(result.reason).toBeInstanceOf(UnsupportedCapabilityError);
    }
  });
  it("replaces an adapter without importing mutation services", async () => {
    class Reader extends UnsupportedLifecycle {
      override async resolve() {
        return { state: "present" as const, value: release };
      }
    }
    const reader: AppRuntime = new Reader();
    expect(await reader.resolve("orchard", "browser")).toEqual({
      state: "present",
      value: release,
    });
    await expect(reader.observe("unknown")).rejects.toBeInstanceOf(
      UnsupportedCapabilityError,
    );
  });
  it("requires verified code and data regardless of pin/running status", () => {
    for (const artifactsVerified of [null, false, true])
      for (const dataVerified of [null, false, true]) {
        expect(
          availableOffline({
            releaseCid: "fixture",
            pinned: true,
            running: true,
            artifactsVerified,
            dataVerified,
          }),
        ).toBe(artifactsVerified === true && dataVerified === true);
      }
  });
});
