import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import Ajv from "ajv";
import addFormats from "ajv-formats";
import YAML from "yaml";

const ajv = new Ajv({ allErrors: true, strict: false });
addFormats(ajv);
const validate = ajv.compile(
  JSON.parse(
    readFileSync(
      new URL("../../.claude/schemas/objective.schema.json", import.meta.url),
      "utf8",
    ),
  ),
);

/** One loader for both documented formats; never execute YAML tags. */
export function loadObjective(path) {
  const objective = YAML.parse(readFileSync(path, "utf8"), {
    uniqueKeys: true,
  });
  if (!validate(objective))
    throw new Error(`Invalid objective: ${ajv.errorsText(validate.errors)}`);
  return objective;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    console.log(JSON.stringify(loadObjective(process.argv[2])));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 2;
  }
}
