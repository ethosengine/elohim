---
id: "backlog-bounds-validator-lexicographic-timestamp-compare"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "bounds_validator active-window check compares mixed-format ISO strings lexicographically"
slug: "bounds-validator-lexicographic-timestamp-compare"
written: "2026-08-19"
author: "claude (shift 2026-08-19T03-37-operator-positive-path-green)"
status: "backlog"
priority: "medium"
cites:
  - elohim/elohim-storage/src/services/bounds_validator.rs
tags: [correctness, mishpat, commitments, time]
shift_objective: |
  Make bounds_validator check 3 (active window) parse timestamps
  (chrono::DateTime<FixedOffset>) instead of comparing raw strings. The
  comparison `now < valid_from || now > valid_until` runs on strings whose
  formats differ by producer: signed_at is chrono to_rfc3339 (fractional +
  "+00:00"), while seeded/DHT-projected valid_from/valid_until commonly use
  "Z"-suffixed forms. Lexicographic order across those suffixes is wrong at
  second/sub-second boundaries: e.g. valid_from "…T13:19:11Z" reads as AFTER
  signed_at "…T13:19:11.460+00:00" ('.' < 'Z'), denying a just-granted
  commitment with commitment-inactive for up to a second. Add red tests with
  mixed-suffix same-second inputs. Discovered live 2026-08-19 during the
  operator-runtime-surface local proof (the proof script's seconds-precision
  validFrom tripped it deterministically).
---

# bounds_validator: lexicographic timestamp comparison across mixed ISO formats

See `shift_objective`. The check is `elohim/elohim-storage/src/services/bounds_validator.rs`
step 3 ("Active-window check", `now < &commitment.valid_from || now > &commitment.valid_until`
on `String`s). All other date logic that parses (e.g. chrono) is unaffected.
Low blast radius today (real grants have windows far from `now`), but any
grant-then-immediately-use flow — exactly the a2o operator-verbs fixture
shape — can flake on the boundary second.
