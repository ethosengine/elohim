---
id: "backlog-elohim-epr-ts-rs-serde-attr-parse-warnings"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-epr: ts-rs emits 'failed to parse serde attribute' warnings on #[serde(skip_serializing_if, default)] during build/package"
slug: "elohim-epr-ts-rs-serde-attr-parse-warnings"
written: "2026-06-29"
author: "elohim-epr 0.1.0 publish (warnings surfaced during cargo publish verify)"
status: "backlog"
priority: "low"
jobs: [elohim]
---

## What
Building `elohim-epr` (seen during `cargo publish` verify, also any `cargo build`) emits repeated `ts-rs` warnings:

```
warning: failed to parse serde attribute
  | #[serde(skip_serializing_if = "Option::is_none", default)]
  = note: ts-rs failed to parse this attribute. It will be ignored.
```

One per `#[derive(TS)]` field carrying the combined `skip_serializing_if = "…", default` form. ts-rs (v10.1.0) doesn't parse that attribute shape and ignores it.

## Why low
Cosmetic only — the crate compiles, tests pass, and `elohim-epr 0.1.0` published clean. ts-rs only *ignores* the attribute for its TS generation; serde itself honors it at runtime, so wire behavior is unaffected. No correctness or codegen impact observed.

## Fix options (when picked up)
- Confirm the generated TS for the affected types is still correct (Optional fields), since ts-rs ignores the hint — verify the `?`-optionality matches intent.
- Either split the attribute into a form ts-rs parses, gate it behind `#[cfg_attr(not(feature = "ts"), …)]`-style separation, or upgrade ts-rs if a newer version parses the combined form.
- Locate the fields: `grep -rn 'skip_serializing_if' elohim/epr/src` over the `#[derive(TS)]` types.

Domain: EPR codec crate (`elohim/epr`). Surfaced while publishing the crate to the internal Nexus registry.
