import { describe, it, expect } from "vitest";
import {
  validateRecord,
  type DeploymentRecord,
} from "../validate-deployments.js";

// The shared consolidated template all non-adam humans sed-render from. It
// exists in the repo, so validateRecord's existsSync check resolves it.
const TEMPLATE_REL =
  "genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml";

const knownHumans = new Set(["human-testperson"]);
const knownDevices = new Set(["device-family-node-base"]);

function consolidated(
  overrides: Partial<DeploymentRecord> = {},
): DeploymentRecord {
  return {
    name: "testperson",
    role: "tester",
    humanLabel: "testperson",
    humanId: "human-testperson",
    pattern: "consolidated",
    template: TEMPLATE_REL,
    deviceArchetype: "device-family-node-base",
    nodeTypes: ["edge"],
    ...overrides,
  } as DeploymentRecord;
}

describe("validateRecord — consolidated pattern accepts template OR manifest", () => {
  it("accepts a consolidated record that sed-renders from a template (the 13-of-14 convention)", () => {
    // RED before the fix: the validator hardcoded `consolidated → manifest`.
    expect(validateRecord(consolidated(), knownHumans, knownDevices)).toEqual(
      [],
    );
  });

  it("still accepts a consolidated record with an explicit manifest (adam-style)", () => {
    const rec = consolidated({ template: undefined, manifest: TEMPLATE_REL });
    expect(validateRecord(rec, knownHumans, knownDevices)).toEqual([]);
  });

  it("rejects a consolidated record with neither manifest nor template", () => {
    const rec = consolidated({ template: undefined, manifest: undefined });
    const errors = validateRecord(rec, knownHumans, knownDevices);
    expect(
      errors.some((e) => /requires 'manifest' or 'template'/.test(e)),
    ).toBe(true);
  });

  it("rejects a consolidated record whose deployment source file is missing", () => {
    const rec = consolidated({
      template: "genesis/orchestrator/manifests/humans/does-not-exist.yaml",
    });
    const errors = validateRecord(rec, knownHumans, knownDevices);
    expect(errors.some((e) => /missing/.test(e))).toBe(true);
  });
});

describe("validateRecord — edgenodeDbPoolSize is a sed-interpolated positive integer", () => {
  it("accepts a positive integer string", () => {
    expect(
      validateRecord(
        consolidated({ edgenodeDbPoolSize: "20" }),
        knownHumans,
        knownDevices,
      ),
    ).toEqual([]);
  });

  it("accepts an absent edgenodeDbPoolSize (optional)", () => {
    expect(
      validateRecord(
        consolidated({ edgenodeDbPoolSize: undefined }),
        knownHumans,
        knownDevices,
      ),
    ).toEqual([]);
  });

  it("rejects a non-numeric value", () => {
    const errors = validateRecord(
      consolidated({ edgenodeDbPoolSize: "twenty" }),
      knownHumans,
      knownDevices,
    );
    expect(
      errors.some((e) =>
        /edgenodeDbPoolSize must be a positive integer/.test(e),
      ),
    ).toBe(true);
  });

  it('rejects "0" (a zero pool is meaningless; the Rust parser would discard it)', () => {
    const errors = validateRecord(
      consolidated({ edgenodeDbPoolSize: "0" }),
      knownHumans,
      knownDevices,
    );
    expect(
      errors.some((e) =>
        /edgenodeDbPoolSize must be a positive integer/.test(e),
      ),
    ).toBe(true);
  });

  it("catches a sed-delimiter pipe in the value (sedInterpolatedFields)", () => {
    const errors = validateRecord(
      consolidated({ edgenodeDbPoolSize: "20|bad" }),
      knownHumans,
      knownDevices,
    );
    expect(errors.some((e) => /edgenodeDbPoolSize contains '\|'/.test(e))).toBe(
      true,
    );
  });
});

describe("validateRecord — legacy pattern still requires template + sizing (regression)", () => {
  it("accepts a legacy record with template + resource sizing", () => {
    const rec = consolidated({
      pattern: "legacy",
      manifest: undefined,
      template: TEMPLATE_REL,
      edgenodeMemoryRequest: "1Gi",
      edgenodeMemoryLimit: "2Gi",
      edgenodeCpuRequest: "250m",
      edgenodeCpuLimit: "2000m",
    });
    expect(validateRecord(rec, knownHumans, knownDevices)).toEqual([]);
  });
});
