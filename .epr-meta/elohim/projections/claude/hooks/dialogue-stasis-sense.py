#!/usr/bin/env python3
import os
import sys
if os.environ.get("STASIS_KEEPER") == "1" or os.environ.get("STASIS_SENSE") == "off":
    sys.exit(0)

"""Private, observe-only dialogue journal and context-only harness exception relay.

Evidence: probe/20261030T162945Z and probe/20261030T163031Z establish serial
permission correlation, inferred denial, delayed dialog notifications and the
system-originated task-notification exclusion. No remote push/deny-relay claim.
Authoritative bytes: elohim-agent HookPackage dialogue-stasis-sense.
STASIS_JOURNAL_ROOT overrides the stasis root; STASIS_SEGMENT_BYTES overrides
5 MB for replay tests. Neither changes authority or enables session decisions.
"""
import datetime as dt
import fcntl
import hashlib
import json
import math
from pathlib import Path
import re
import shlex
import time

RETIRE_WHEN = "Native session runtimes provide private observation and exception context."
OPERATORS = ";&|()<>\n"
COMMANDS = {
    "git": {"status", "diff", "log", "show", "rebase", "push", "fetch", "pull", "clone", "ls-remote", "rev-parse", "branch", "switch", "checkout", "reset", "add", "commit", "merge", "stash"},
    "pnpm": {"exec", "run", "test", "build", "install"},
    "cargo": {"test", "check", "build", "run", "fmt", "clippy"},
    "vitest": {"run", "watch"},
    "just": {"gate", "test", "dev", "mesh", "seed", "status", "look", "codegen"},
}
EXECUTABLES = set(COMMANDS) | {"cd", "curl", "wget", "ssh", "scp", "bash", "sh", "python", "python3", "node", "echo", "printf", "cat", "rg", "ls", "env", "export", "true", "false"}


def digest(value):
    return "sha256:" + hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def placeholder(word, hint=None):
    if re.match(r"^[a-zA-Z][a-zA-Z0-9+.-]*://", word):
        return "<url>"
    if hint == "ref":
        return "<ref>"
    if re.fullmatch(r"-?\d+(\.\d+)?", word):
        return "<num>"
    if any(c in word for c in "*?["):
        return "<glob>"
    if hint == "path" or "/" in word or re.search(r"\.[a-zA-Z0-9]{1,8}$", word):
        return "<path>"
    return "<str>"


def shape(command):
    """Keep known command grammar and flags; never preserve arbitrary words.

    This is a redactor, not a shell parser or a concern/effect classifier.
    Heredoc bodies and interpreter programs are opaque arguments.
    """
    if not isinstance(command, str):
        return "<str>"
    if "<<" in command:
        return shape(command.split("<<", 1)[0]) + " << <str>"
    try:
        lexer = shlex.shlex(command, posix=True, punctuation_chars=OPERATORS)
        lexer.whitespace = " \t\r"
        lexer.whitespace_split = True
        words = list(lexer)
    except ValueError:
        return "<str>"
    out, executable, subcommand, hint = [], None, None, None
    grammar = True
    for word in words:
        if word and all(c in OPERATORS for c in word):
            out.append(word)
            executable, subcommand, hint, grammar = None, None, None, True
            continue
        if executable is None:
            if "=" in word:
                out.append("<str>")
                continue
            base = word.rsplit("/", 1)[-1]
            executable = base
            out.append(base if base in EXECUTABLES and "/" not in word else placeholder(word))
            continue
        if hint:
            out.append(placeholder(word, hint))
            hint = None
            continue
        if re.fullmatch(r"--?[A-Za-z][A-Za-z0-9-]*(=.*)?", word):
            flag, sep, value = word.partition("=")
            out.append(flag + ("=" + placeholder(value) if sep else ""))
            if not sep and flag in {"-C", "--manifest-path", "--git-dir", "--work-tree"}:
                hint = "path"
            elif not sep and flag in {"--onto", "--branch"}:
                hint = "ref"
            elif not sep and flag in {"-c", "-e", "-m", "-H", "-o", "--token", "--header", "--password"}:
                hint = "str"
            continue
        if grammar and word in COMMANDS.get(executable, set()) and subcommand is None:
            out.append(word)
            subcommand = word
        elif grammar and executable == "pnpm" and subcommand == "exec" and word == "vitest":
            out.append(word)
            executable, subcommand = "vitest", None
        else:
            out.append(placeholder(word, "path" if executable == "cd" else
                                   "ref" if executable == "git" and subcommand in {"rebase", "show", "diff", "merge"} else None))
            grammar = False
    return " ".join(out)


def number(value):
    return value if type(value) in (int, float) and math.isfinite(value) and value >= 0 else None


def text_or_none(value):
    return value if isinstance(value, str) else None


def wall(seconds=None):
    return dt.datetime.fromtimestamp(time.time() if seconds is None else seconds, dt.timezone.utc).isoformat().replace("+00:00", "Z")


def clock():
    return {"wall": wall(), "bootId": Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
            "uptimeSeconds": float(Path("/proc/uptime").read_text().split()[0]),
            "provenance": "claude-hook/linux-proc"}


def initial_state():
    return {"schemaVersion": 1, "nextSeq": 1, "lastPreToolUse": {}, "activeCalls": {},
            "openPrompts": {}, "presence": {}, "exceptions": {}, "transcripts": {}}


class Journal:
    """Caller holds journal.lock across recovery, append, rotation and state save."""
    def __init__(self, directory, payload):
        self.directory, self.payload = directory, payload
        self.path = directory / "events.jsonl"
        self.state_path = directory / "state.json"
        self.now = clock()
        self.agent = text_or_none(payload.get("agent_id")) or "main"
        self.source = {"hookEvent": text_or_none(payload.get("hook_event_name")) or "unknown",
                       "promptId": text_or_none(payload.get("prompt_id")),
                       "toolUseId": text_or_none(payload.get("tool_use_id"))}
        self.state = initial_state()
        stale = False
        try:
            self.state.update(json.loads(self.state_path.read_text()))
        except FileNotFoundError:
            stale = bool(self.segments() or self.path.exists())
        except (ValueError, TypeError):
            stale = True
        files = self.segments() + ([self.path] if self.path.exists() else [])
        tail = None
        if files:
            with files[-1].open("rb") as stream:
                stream.seek(max(0, files[-1].stat().st_size - 65536))
                lines = stream.read().splitlines()
            try:
                tail = json.loads(lines[-1]) if lines else None
            except (ValueError, IndexError):
                stale = True
        expected = tail["seq"] + 1 if tail else 1
        if stale or self.state["nextSeq"] != expected:
            self.recover(files)
            self.gap("sequence-discontinuity")
        self.retain()

    def segments(self):
        return sorted((p for p in self.directory.glob("events.*.jsonl")
                       if p.name.split(".")[1].isdigit()), key=lambda p: int(p.name.split(".")[1]))

    def recover(self, files):
        # PreToolUse is cache-only in v1. Rebuild incident evidence, and explicitly
        # drop unprovable pre-tool matches. Keep valid transcript cursors: no replay.
        old = self.state
        self.state = initial_state()
        self.state["transcripts"] = old.get("transcripts", {})
        self.state["exceptions"] = old.get("exceptions", {})
        self.state["transcriptHistoryLost"] = not bool(self.state["transcripts"])
        highest = 0
        for path in files:
            with path.open("rb+") as stream:
                while raw := stream.readline():
                    try:
                        event = json.loads(raw)
                        highest = max(highest, event["seq"])
                        agent = event["agentId"]
                        if event["kind"] == "permission-requested":
                            self.state["openPrompts"].setdefault(agent, []).append(self.open_record(event))
                        elif event["kind"] == "prompt-closed":
                            incident = event["correlation"]["incidentId"]
                            self.state["openPrompts"][agent] = [p for p in self.state["openPrompts"].get(agent, []) if p["incidentId"] != incident]
                    except (ValueError, KeyError, TypeError):
                        pass
                # An interrupted append must not concatenate the next JSON record.
                if path.stat().st_size:
                    stream.seek(-1, 2)
                    if stream.read(1) != b"\n":
                        stream.seek(0, 2)
                        stream.write(b"\n")
        self.state["nextSeq"] = max(highest + 1, old.get("nextSeq", 1))

    def presence(self):
        p = self.state["presence"]
        age = None
        if p.get("lastWitnessWall"):
            age = max(0, time.time() - dt.datetime.fromisoformat(p["lastWitnessWall"].replace("Z", "+00:00")).timestamp())
            if p.get("bootId") == self.now["bootId"]:
                age = max(0, self.now["uptimeSeconds"] - p["lastWitnessUptimeSeconds"])
        return {"disposition": "unknown" if age is None else "collaborative" if age <= 600 else "automated",
                "basis": {"lastWitnessedAgeSeconds": age, "rulingRef": None}}

    def witness(self, duration=0):
        seconds = (duration or 0) / 1000
        self.state["presence"] = {"lastWitnessWall": wall(time.time() - seconds),
                                  "lastWitnessUptimeSeconds": max(0, self.now["uptimeSeconds"] - seconds),
                                  "bootId": self.now["bootId"]}

    def append(self, kind, **fields):
        event = {"schemaVersion": 1, "kind": kind,
                 "session": {"runtime": "claude-code", "session": self.payload["session_id"]},
                 "agentId": self.agent, "seq": self.state["nextSeq"],
                 "clock": self.now, "presence": self.presence(), "source": self.source, **fields}
        raw = (json.dumps(event, separators=(",", ":"), allow_nan=False) + "\n").encode()
        limit = max(1, int(os.environ.get("STASIS_SEGMENT_BYTES", 5 * 1024 * 1024)))
        if self.path.exists() and self.path.stat().st_size and self.path.stat().st_size + len(raw) > limit:
            segments = self.segments()
            index = max(self.state.get("nextSegment", 1), int(segments[-1].name.split(".")[1]) + 1 if segments else 1)
            self.path.rename(self.directory / f"events.{index}.jsonl")
            self.state["nextSegment"] = index + 1
        with self.path.open("ab") as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        self.state["nextSeq"] += 1
        return event

    def gap(self, reason, start=None, end=None):
        self.append("observation-gap", gap={"reason": reason, "from": start, "to": end})

    def retain(self):
        removed = False
        for path in self.segments():
            if path.stat().st_mtime < time.time() - 30 * 86400:
                path.unlink()
                removed = True
        if removed:
            self.gap("retention")

    @staticmethod
    def open_record(event):
        return {"incidentId": event["correlation"]["incidentId"], "seq": event["seq"],
                "tool": event["tool"], "precedingToolUseId": event["source"].get("precedingToolUseId"),
                "inputDigest": event.get("inputDigest"), "promptId": event["source"].get("promptId"),
                "openedAt": event["clock"], "shape": event.get("shape", "<str>")}

    def save(self):
        temp = self.directory / "state.json.tmp"
        with temp.open("w") as stream:
            json.dump(self.state, stream, separators=(",", ":"), allow_nan=False)
            stream.flush()
            os.fsync(stream.fileno())
        temp.replace(self.state_path)


def correlation(incident=None, confidence="unknown", method="none"):
    return {"incidentId": incident, "confidence": confidence, "method": method}


def map_event(j):
    p, s, agent = j.payload, j.state, j.agent
    event, tool = p.get("hook_event_name"), text_or_none(p.get("tool_name")) or "unknown"
    prompt, tool_id = p.get("prompt_id"), p.get("tool_use_id")
    inputs = p.get("tool_input", {})
    hashed = digest(inputs)
    redacted = shape(inputs.get("command")) if tool == "Bash" and isinstance(inputs, dict) else "<str>"
    opens = s["openPrompts"].setdefault(agent, [])
    calls = s["activeCalls"].setdefault(agent, {})
    key = json.dumps([agent, prompt, tool])
    if event in {"PreToolUse", "Stop", "UserPromptSubmit"}:
        for opened in opens[:]:
            known = bool(opened["precedingToolUseId"])
            j.append("prompt-closed", closedBy="denied-inferred" if known else "unknown",
                     correlation=correlation(opened["incidentId"], "low", "inferred-next-event"))
            if not known:
                j.gap("unknown-close", opened["openedAt"]["wall"], j.now["wall"])
            calls.pop(opened["precedingToolUseId"], None)
            opens.remove(opened)
    if event == "PreToolUse":
        record = {"toolUseId": tool_id, "seq": s["nextSeq"], "promptId": prompt,
                  "tool": tool, "inputDigest": hashed, "ambiguous": bool(calls)}
        if calls:
            for call in calls.values():
                call["ambiguous"] = True
        s["lastPreToolUse"][key] = record
        if tool_id:
            calls[tool_id] = record
    elif event == "PermissionRequest":
        candidate = s["lastPreToolUse"].get(key)
        same = [o for o in opens if o["inputDigest"] == hashed]
        reliable = (candidate and prompt and candidate["toolUseId"] in calls
                    and len(calls) == 1 and not calls[candidate["toolUseId"]]["ambiguous"]
                    and candidate["inputDigest"] == hashed and not same)
        preceding = candidate["toolUseId"] if reliable else None
        # Repeated prompts make prior cached correlation ambiguous too.
        for opened in same:
            opened["precedingToolUseId"] = None
        incident = f'{p["session_id"]}:{s["nextSeq"]}'
        recorded = j.append("permission-requested", tool=tool, shape=redacted, inputDigest=hashed,
                            source={**j.source, "precedingToolUseId": preceding},
                            correlation=correlation(incident, "high" if reliable else "low",
                                                    "prompt-id-preceding-pretooluse" if reliable else "none"))
        opens.append(j.open_record(recorded))
        if not reliable:
            j.gap("ambiguous-permission-correlation")
    elif event == "Notification" and p.get("notification_type") == "permission_prompt":
        candidates = [o for o in opens if not prompt or o["promptId"] == prompt]
        opened = candidates[0] if len(candidates) == 1 else None
        j.append("dialog-displayed", tool=opened["tool"] if opened else tool,
                 correlation=correlation(opened["incidentId"], "high", "prompt-id-preceding-pretooluse") if opened and opened["precedingToolUseId"] else correlation())
        if not opened or not opened["precedingToolUseId"]:
            j.gap("ambiguous-dialog-correlation")
    elif event in {"PostToolUse", "PostToolUseFailure"}:
        matches = [o for o in opens if tool_id and o["precedingToolUseId"] == tool_id]
        if len(matches) == 1:
            opened = matches[0]
            duration = number(p.get("duration_ms"))
            # A tool completion proves permission, including a failed tool.
            # Only a matching pending dialog supplies an interaction witness.
            if duration is not None:
                j.witness(duration)
            j.append("prompt-closed", closedBy="approved", durationMs=duration,
                     correlation=correlation(opened["incidentId"], "high", "tool-use-id"))
            opens.remove(opened)
        elif opens:
            j.gap("unmatched-tool-completion")
        calls.pop(tool_id, None)
    elif event == "PermissionDenied":
        j.append("classifier-denied", tool=tool, shape=redacted, inputDigest=hashed)
    elif event == "Stop":
        message = p.get("last_assistant_message")
        question = "uncertain" if not isinstance(message, str) or not message.strip() else "yes" if "?" in message else "no"
        j.append("turn-ended", question=question)
        calls.clear()
    elif event == "UserPromptSubmit":
        text = p.get("prompt")
        if isinstance(text, str) and not text.startswith("<task-notification>"):
            j.witness()
            if re.search(r"\b(stuck|status|progress|still there)\b", text, re.I):
                j.append("steward-ping")
        calls.clear()
    elif event == "SessionEnd":
        for opened in opens:
            j.append("prompt-closed", closedBy="unknown",
                     correlation=correlation(opened["incidentId"], "low", "none"))
            j.gap("unknown-close", opened["openedAt"]["wall"], j.now["wall"])
        opens.clear()
        j.append("session-ended")
    elif event in {"HookError", "StopFailure"}:
        j.gap("hook-error")


def exception_record(record):
    data = record.get("attachment") or record
    if not isinstance(data, dict):
        return None
    kind = data.get("hook_event_name") or data.get("subtype") or data.get("type")
    source = {"hook_cancelled": "hook_cancelled", "hook_error": "hook_error",
              "hook-error": "hook_error", "StopFailure": "stop_failure",
              "stop_failure": "stop_failure"}.get(kind)
    if source is None:
        return None
    name = data.get("hookName") or ("StopFailure" if source == "stop_failure" else "unknown")
    # Hook identifiers enter a single context line; prohibit markup/newline injection.
    name = re.sub(r"[^A-Za-z0-9_.:-]", "_", str(name))[:160] or "unknown"
    command = data.get("command")
    key = digest([source, name, command])
    return key, {"source": source, "hookName": name,
                 "command": shape(command) if command is not None else None,
                 "durationMs": number(data.get("durationMs")), "timeoutMs": number(data.get("timeoutMs")), "count": 1}


def transcript(j):
    path = j.payload.get("transcript_path")
    if not isinstance(path, str) or not path:
        j.gap("transcript-path-absent")
        return
    key = digest(path)
    cursors = j.state["transcripts"]
    try:
        with open(path, "rb") as stream:
            stat = os.fstat(stream.fileno())
            identity = [stat.st_dev, stat.st_ino]
            cursor = cursors.get(key)
            baseline = cursor is None
            if cursor is not None and (cursor["identity"] != identity or cursor["offset"] > stat.st_size):
                baseline = True
                j.gap("transcript-discontinuity")
            if baseline:
                if cursor is not None or j.state.get("transcriptHistoryLost"):
                    j.gap("transcript-cursor-lost")
                # Preserve historical evidence without replaying it into context.
                cursor = {"offset": 0, "identity": identity, "partial": False}
                cursors[key] = cursor
            stream.seek(cursor["offset"])
            batch = {}
            for raw in stream:
                cursor["offset"] += len(raw)
                partial = not raw.endswith(b"\n")
                if cursor["partial"] or partial:
                    if not cursor["partial"]:
                        j.gap("transcript-partial-record")
                    cursor["partial"] = partial
                    continue
                try:
                    record = json.loads(raw)
                    item = exception_record(record) if isinstance(record, dict) else None
                except (ValueError, TypeError):
                    j.gap("transcript-invalid-record")
                    continue
                if item:
                    item_key, exc = item
                    if item_key in batch:
                        exc["count"] += batch[item_key]["count"]
                    batch[item_key] = exc
            for item_key, exc in batch.items():
                if baseline:
                    exc["baseline"] = True
                j.append("harness-exception", exception=exc)
                if exc["source"] in {"hook_error", "stop_failure"}:
                    j.gap("hook-error")
                entry = j.state["exceptions"].setdefault(item_key, {"count": 0, "notifiedCount": 0, "noticeAt": 0})
                entry["count"] += exc["count"]
                entry["exception"] = exc
                if baseline:
                    entry.update(baselineCount=entry["count"], notifiedCount=entry["count"], noticeAt=0)
    except OSError:
        j.gap("transcript-unreadable")


def relay(j):
    # gate-live-inject's verified transport is PostToolUse. UserPromptSubmit's
    # plain-text transport and PreToolUse JSON are the hooks manifest's ABI.
    event = j.payload.get("hook_event_name")
    if event not in {"PostToolUse", "PreToolUse", "UserPromptSubmit"}:
        return None
    lines = []
    for entry in j.state["exceptions"].values():
        # Rate limiting and doubling count only occurrences past the baseline.
        baseline = entry.get("baselineCount", 0)
        total, sent, at = entry["count"] - baseline, entry["notifiedCount"] - baseline, entry["noticeAt"]
        if total <= sent or (sent and time.time() - at < 1800 and total < sent * 2):
            continue
        exc = entry["exception"]
        duration = exc["durationMs"] if exc["durationMs"] is not None else "unknown"
        timeout = exc["timeoutMs"] if exc["timeoutMs"] is not None else "unknown"
        lines.append(f'harness exception: `{exc["hookName"]}` timed out (`{duration}` ms against `{timeout}` ms) ×{total - sent} since the last notice. Investigate and fix it now, or dispatch a subagent to.')
        entry.update(notifiedCount=entry["count"], noticeAt=time.time())
    if not lines:
        return None
    context = "\n".join(lines)
    if event == "UserPromptSubmit":
        return context
    return json.dumps({"hookSpecificOutput": {"hookEventName": event, "additionalContext": context}})


def main():
    payload = json.load(sys.stdin)
    if not isinstance(payload, dict):
        return
    session = payload.get("session_id")
    if not isinstance(session, str) or not session or session in {".", ".."} or not re.fullmatch(r"[A-Za-z0-9._-]+", session):
        return
    root = Path(os.environ.get("STASIS_JOURNAL_ROOT") or Path(os.environ.get("CLAUDE_PROJECT_DIR") or Path(__file__).resolve().parents[2]) / ".eprfs/status/stasis")
    directory = root / session
    directory.mkdir(parents=True, exist_ok=True, mode=0o700)
    output = None
    with (directory / "journal.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        j = Journal(directory, payload)
        try:
            if payload.get("hook_event_name") == "Notification" and payload.get("notification_type") == "idle_prompt":
                # Probe 20261030T162945Z: arrives ~60 s after Stop, never while
                # a dialog is open. Evidence only: no witness, inferred close,
                # transcript scan or relay. Slice 5's native fold owns silence
                # timing independently of notifications; no Python timer.
                j.append("idle-prompt")
            else:
                map_event(j)
                transcript(j)
                output = relay(j)
        except Exception:
            # Never persist the exception text: it can contain raw tool arguments.
            j.gap("adapter-error")
        finally:
            j.save()
    if output:
        print(output)


if __name__ == "__main__":
    try:
        main()
    except Exception:
        pass  # Fail open even when no safe journal append is possible.
