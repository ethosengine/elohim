// The native boundary owns these types; formatting is the only projection step.
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
const binary = process.argv[2];
if (!binary) throw Error("Provide the built elohim-hub local-view example");
const native = execFileSync(binary, ["--contract"], { encoding: "utf8" });
const formatted = execFileSync(
  "pnpm",
  ["exec", "prettier", "--parser", "typescript"],
  { input: native, encoding: "utf8" },
);
const destination = resolve(
  "app/elohim-app/src/app/avodah/declared-views/view-contract.ts",
);
if (process.argv.includes("--write")) writeFileSync(destination, formatted);
else if (readFileSync(destination, "utf8") !== formatted)
  throw Error("Local-view contract drift; run this command with --write");
