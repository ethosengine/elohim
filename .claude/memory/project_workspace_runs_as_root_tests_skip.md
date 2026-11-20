---
name: project-workspace-runs-as-root-tests-skip
title: "Workspace runs as root; permission tests skip here"
description: "The Che workspace runs as uid 0, so permission tests skip or pass vacuously; run built test binaries as nobody with setpriv before trusting a green."
metadata:
  node_type: memory
  title: Workspace runs as root; permission tests skip here
  type: project
  originSessionId: 0e0924d5-52e0-4709-b329-ea2463064148
  modified: 2026-11-19T15:51:52.238Z
---

The ethosengine Che workspace container runs every agent command as root (uid 0). Root ignores directory and file modes, so a test that stages "unreadable" with chmod, or that guards itself with `uid() == 0` / an EPERM skip, never exercises its claim on this host. CI runners are not root.

Seen 2026-11-19 (brit sprint 3, session 0e0924d5): `brit-cli::repository_slots::an_unreadable_alternate_pack_directory_does_not_fail_the_open` skipped here through the author's run, three review passes and the integrator's verification, then failed on every non-root Linux CI job (brit run 44544779620) with a real PermissionDenied at open. The reviewer had marked the case "inferred, not measured" for exactly this reason.

Correction the same day: the product claim was right. The TEST was unsound — it opened the repository without process isolation, so a host-wide `safe.directory = *` (CI runners appear to have one; inferred from an exact reproduction) raised it to full trust, where the store lists pack directories at open. Run as nobody it passed with a plain HOME and failed with that setting.

**Why:** a test that skips as root was never exercised here, so nobody saw that it depended on host git config; it cost one CI run after the push.

**How to apply:** running as an unprivileged user works here — verified: `setpriv --reuid=65534 --regid=65534 --clear-groups <cmd>`; the cargo pool is readable by that user, `/tmp` is writable, `git` runs; session scratch under `/tmp/claude-0` is NOT reachable (0700). Build as root, then execute the already-built test binaries as nobody with `TMPDIR=/tmp`, a throwaway `HOME` and `GIT_CONFIG_NOSYSTEM=1` (this host's /etc/gitconfig is unreadable to nobody). Run twice: plain HOME, and a HOME whose `.gitconfig` has `[safe] directory = *`. For any lane touching permissions, ownership or trust, ask for that non-root leg and a list of the tests' root-dependent branches before accepting "green". Related: [[project_rust_test_stub_etxtbsy_race]], [[project-experiential-evidence-boundary]].
