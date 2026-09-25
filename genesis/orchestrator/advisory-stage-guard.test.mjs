/**
 * Static contract: the orchestrator's advisory stages cannot abort dispatch.
 *
 * Orchestrator #1904: a durable-task launch failure (`process apparently never
 * started`, exit -2 — an oversized env string; changed-paths-env-cap.test.mjs) inside `Brit Plan (advisory)` cascaded to FAILURE and
 * aborted the run before `Execute Builds` dispatched anything (7 pipelines
 * never started). Advisory means advisory: the stage's whole `steps {}` body
 * sits inside `catchError(buildResult: null, stageResult: 'UNSTABLE')`, so a
 * failure there marks only the stage and leaves the build result untouched.
 */

import { describe, test } from "node:test";
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";

const orchestrator = readFileSync(new URL("./Jenkinsfile", import.meta.url), "utf8");

// Brace matcher that skips comments and string literals (incl. ''' heredocs),
// so a brace inside a shell body or a comment never shifts the depth.
function balancedBlock(text, openBrace) {
  let depth = 1;
  let i = openBrace + 1;
  while (i < text.length && depth > 0) {
    if (text.startsWith("//", i)) {
      i = text.indexOf("\n", i + 2);
      if (i === -1) return text.slice(openBrace + 1);
      continue;
    }
    if (text.startsWith("/*", i)) {
      const end = text.indexOf("*/", i + 2);
      assert.notEqual(end, -1, "unterminated block comment while parsing Jenkinsfile");
      i = end + 2;
      continue;
    }
    if (text[i] === "'" || text[i] === '"') {
      const quote = text[i];
      const triple = text.startsWith(quote.repeat(3), i);
      i += triple ? 3 : 1;
      while (i < text.length) {
        if (triple && text.startsWith(quote.repeat(3), i)) {
          i += 3;
          break;
        }
        if (!triple && text[i] === quote) {
          i += 1;
          break;
        }
        if (text[i] === "\\") i += 1;
        i += 1;
      }
      continue;
    }
    if (text[i] === "{") depth += 1;
    if (text[i] === "}") depth -= 1;
    i += 1;
  }
  assert.equal(depth, 0, "unterminated Jenkinsfile block");
  return text.slice(openBrace + 1, i - 1);
}

function blockAfter(text, pattern, what) {
  const match = pattern.exec(text);
  assert.ok(match, `${what} not found`);
  const openBrace = match.index + match[0].lastIndexOf("{");
  return { body: balancedBlock(text, openBrace), start: match.index, openBrace };
}

function stripComments(text) {
  return text.replace(/\/\/[^\n]*/g, "").replace(/\/\*[\s\S]*?\*\//g, "");
}

describe("orchestrator advisory stage guard (B5)", () => {
  const stage = blockAfter(
    orchestrator,
    /stage\(\s*'Brit Plan \(advisory\)'\s*\)\s*\{/,
    "stage 'Brit Plan (advisory)'",
  ).body;
  const steps = blockAfter(stage, /\n\s*steps\s*\{/, "Brit Plan steps {}").body;

  test("the whole steps body is one catchError that leaves the build result alone", () => {
    const guard = /^\s*catchError\(\s*buildResult:\s*null\s*,\s*stageResult:\s*'UNSTABLE'\s*\)\s*\{/;
    const code = stripComments(steps);
    const match = guard.exec(code);
    assert.ok(
      match,
      "Brit Plan (advisory) steps{} must open with catchError(buildResult: null, stageResult: 'UNSTABLE') { … }",
    );
    const openBrace = match.index + match[0].lastIndexOf("{");
    const guarded = balancedBlock(code, openBrace);
    // Everything the stage does lives inside the guard: nothing precedes or
    // follows it in steps{}, so no step can run unguarded.
    const closeAt = code.indexOf(guarded, openBrace) + guarded.length + 1;
    assert.equal(code.slice(closeAt).trim(), "", "a step runs after the guard closes");
    assert.match(guarded, /container\('builder'\)/, "container launch sits inside the guard");
    assert.match(guarded, /brit-helper\.sh plan --since origin\/dev/, "the brit sh body sits inside the guard");
  });

  test("the advisory stage precedes dispatch, so its guard is what keeps dispatch reachable", () => {
    const advisory = orchestrator.indexOf("stage('Brit Plan (advisory)')");
    const execute = orchestrator.indexOf("stage('Execute Builds')");
    assert.ok(advisory !== -1 && execute !== -1 && advisory < execute);
  });
});
