---
name: New Jenkinsfiles need safe.directory before in-container git operations
description: Any new pipeline that runs git commands inside a container (other than checkout scm) must set safe.directory first or git fails with dubious-ownership
type: feedback
originSessionId: cc51fa69-af87-4c58-a30c-b86120b754fc
---
When writing a new Jenkinsfile that uses the kubernetes pod-template agent and runs `git rev-parse`, `git log`, `git describe`, or any other git command inside `container('builder')` (or similar), you MUST add `sh 'git config --global --add safe.directory "*"'` BEFORE the first git command in that container.

**Why:** `checkout scm` runs as the JNLP agent's user (UID matches the workspace owner). When you `container('builder') { sh 'git ...' }` afterwards, the build container runs as a different UID. Git 2.35+ refuses to operate on a workspace owned by a different UID and emits `fatal: detected dubious ownership in repository at '/home/jenkins/agent/workspace/<job>'`. The stage fails and all downstream stages skip.

Caught on the first build of `elohim-storybook` (build #1, 2026-05-04). The Checkout stage's `git rev-parse --short HEAD` failed immediately, killing Setup Version → Install Deps → Build → Push → Deploy → Verify. Fixed in `fc9febcd`.

**How to apply:** When writing or reviewing a new Jenkinsfile, scan for in-container git operations. Add the safe.directory config inside every container block that uses git, BEFORE the first git command. Example:

```groovy
stage('Checkout') {
    steps {
        container('builder') {
            checkout scm
            script {
                sh 'git config --global --add safe.directory "*"'   // <-- required
                env.GIT_COMMIT_HASH = sh(script: 'git rev-parse --short HEAD', returnStdout: true).trim()
            }
        }
    }
}
```

The root `Jenkinsfile` (elohim-app) sets it at lines 357 and 393 — pattern reference.

This is also why the writing-plans skill should add a "in-container git ops" check to its self-review section for any plan that creates a Jenkinsfile.
