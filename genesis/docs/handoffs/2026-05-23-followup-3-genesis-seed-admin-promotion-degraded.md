# Follow-up Prompt 3 — Genesis seed admin-promotion has been silently degraded

**Created:** 2026-05-23 from the alpha-landing-page dual-doorway shift.
**For:** A fresh session investigating an incidental discovery about elohim-genesis seed stages.
**Owner role:** content-pipeline (knows the seed flow) + ci-investigator (for log archaeology) + rust-architect (if the admin-promotion logic itself needs change).

---

## Context

While debugging credential visibility for the `alpha.elohim.host` work, I dispatched ci-investigator against the most-recent successful `elohim-genesis/dev #1022` build. The investigation surfaced an unrelated but serious finding:

**The genesis seed stages have been silently running without admin auth on every recent run.**

---

## Evidence (log-grounded, quoted verbatim)

`genesis/Jenkinsfile` has admin-promotion logic at lines `997` and `1035`:

```groovy
withCredentials([string(credentialsId: 'doorway-admin-bootstrap-key', variable: 'ADMIN_KEY')]) {
    // body that does admin-promotion on seeded entities
}
```

ci-investigator confirmed by reading `elohim-genesis/dev #1022` log (a SUCCESSFUL run):

> "Both seed stages (lines 1356 and 1438 in the build log) emit the same echo immediately after `withCredentials` closes without executing its body:
> ```
> [Pipeline] withCredentials
> [Pipeline] // withCredentials
> [Pipeline] echo
> doorway-admin-bootstrap-key credential not found — seeding without admin promotion
> ```
> The `// withCredentials` closing tag appearing before any inner steps means the block body never ran — Jenkins silently skipped it because the credential ID resolved to nothing. The Jenkinsfile's own fallback echo fires immediately after. This pattern repeats identically at both seed stages."

So the credential **does not exist at the `elohim-genesis` job's scope** (and per the parallel investigation, it doesn't exist at the `elohim/dev` job's scope either — see follow-up 2 for the credential-add).

The fallback echo is treated as a warning, not a failure. The pipeline reports SUCCESS. Nobody noticed.

---

## Open questions for this session

### 1. What is the admin-promotion supposed to do?

Read the body of the `withCredentials` block at `genesis/Jenkinsfile:997` and `:1035`. Trace what it calls and what those endpoints do. Specifically:

- What HTTP endpoint does the admin auth gate? (Probably an `/admin/*` route under the doorway-proxied storage.)
- What state does it modify when it runs? (Reach promotions? Account-creation handoff? Bootstrap content stamping?)
- What downstream consumers depend on that state being correctly set?

### 2. What's the impact of this having been skipped for an unknown time?

Once you know what admin-promotion does, assess:

- **Alpha cluster state today** — what's the data shape if admin-promotion never ran? Does any content have wrong reach? Are any account bindings unset?
- **Other environments seeded by this pipeline** — staging? prod? Same problem?
- **Visible symptoms** — are there things that "mostly work but..." patterns the operator might recognize as related?

### 3. Should the silent-failure pattern itself be fixed?

The current shape — `withCredentials` silently no-ops → fallback echo → pipeline reports SUCCESS — is the root cause of this staying hidden. Should genesis seed:

- Hard-fail when admin auth is missing (the way I made `Jenkinsfile:stageSpaBlob` do)? Risk: makes the seed pipeline less forgiving during pre-credential setup.
- Stay soft but emit a `UNSTABLE` marker so the result color signals the degradation? Better — failure-shape that's visible without being fatal.
- Refactor admin-promotion to use a different auth path that doesn't depend on a Jenkins credential at all (e.g., k8s ServiceAccount, mounted secret)? Reduces the credential-coordination burden going forward.

### 4. Is retroactive cleanup needed?

Once the credential is in place (per follow-up 2), the next genesis run will execute admin-promotion. Will that **correctly retroactively-promote** the already-seeded entities? Or do the seed entities now need explicit clean-up + re-seed?

---

## Where to start

1. Read `genesis/Jenkinsfile:997-1100` and `:1035-1100` — the full `withCredentials` blocks.
2. Find the HTTP endpoints those scripts call. Are they declared in `elohim/elohim-storage/src/http.rs`? Are they part of an admin module under `doorway-service/src/routes/`?
3. Read the route handlers — what entities do they touch, what reach/state do they set?
4. Cross-reference with `elohim/sdk/domains/lamad/manifest.json` and any seed data files under `genesis/data/` — what's the expected post-seed state vs. what's actually in `elohim-storage` on alpha today?
5. Decide the disposition: fix in next pipeline run automatically, vs. one-shot retroactive cleanup, vs. open a plan if the impact is broad.

---

## Constraints

- This is **read-only investigation first**. Don't push changes until the scope of the bug is understood and the operator has confirmed the cleanup approach.
- If retroactive cleanup is needed, surface that as a separate plan doc with a measure command and budget — don't bundle it into the credential-fix path.
- Be especially careful with anything that modifies seed-row state on a deployed environment. Backups + verification before mutation.

---

## Related artifacts

- ci-investigator finding (above) is from `elohim-genesis/dev #1022` log read via `mcp__jenkins__getBuildLog`.
- Prior shift journal: `.claude/shifts/2026-05-23T05-25-alpha-landing-page-dual-doorway.journal.md` — Iteration 5 documents the discovery.
- Credential add (when complete): see `followup-2-k8s-handoff-summary.md`.
- The fallback echo string `doorway-admin-bootstrap-key credential not found — seeding without admin promotion` is in `genesis/Jenkinsfile` — search for it to find the exact lines.
