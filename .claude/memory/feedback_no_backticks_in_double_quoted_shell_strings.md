---
name: feedback_no_backticks_in_double_quoted_shell_strings
title: "Backticks in double-quoted shell strings execute commands"
description: "2026-11-05: a `berth say \"… \\`just gate elohim-storage\\` …\"` ran the storage gate in the main checkout; messages/prompts built in Bash must be single-quoted or written via a heredoc/file."
metadata:
  node_type: memory
  title: Backticks in double-quoted shell strings execute commands
  type: feedback
  originSessionId: 184beb01-43bc-4c74-9212-9f7189301998
  modified: 2026-11-15T08:47:23.696Z
---

Twice on 2026-11-05 a `berth say "<message>"` whose message mentioned a command in backticks
(`` `just gate elohim-storage` ``, `` `just gate agentic-shift` ``) executed that command through
shell command substitution — one of them a 15-minute storage gate in the main checkout while a
mesh was up, which also made the message never reach the ledger and the Bash call hang until
backgrounded. The same slip mangled an Astra challenge prompt written with a heredoc inside a
double-quoted variable.

**Why:** `"…\`cmd\`…"` and `"…$(cmd)…"` are substitutions in bash; a prose message about a
command is not a command.

**How to apply:** build any multi-line prose for a CLI (`berth say`, `codex exec` prompts, commit
bodies) with the Write tool into a file, or single-quote it and avoid backticks entirely; never
embed `` ` `` or `$(` inside a double-quoted shell argument. If a prose-only command unexpectedly
runs tests or gates, this is the first suspect. Related: [[feedback_agent_fleet_and_harness]].

Single quotes are not safe either: on 2026-11-15 a single-quoted `berth say '…'` containing the
word "exec'ing" ended the quote early. bash then ran fragments of the prose as commands and sent
a truncated note before a syntax error. For any long note, use the Write tool (or a quoted
`<<'EOF'` heredoc into a file) and send `"$(cat file)"`. Check the sent text when the call errors.
