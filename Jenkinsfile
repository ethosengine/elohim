/**
 * Elohim App Pipeline
 *
 * Builds and deploys the Angular web application to alpha/staging/production.
 * Triggered by orchestrator when app/elohim-app/ or app/elohim-library/ files change.
 *
 * What this pipeline builds:
 *   - elohim-app Angular application
 *   - Docker images pushed to Harbor registry
 *
 * Environment Architecture:
 *   - dev, feat-*, claude branches → alpha.elohim.host
 *   - staging* → staging.elohim.host
 *   - main → elohim.host (production)
 *
 * Trigger behavior:
 *   - Only runs when triggered by orchestrator or manual
 *   - Shows NOT_BUILT when triggered directly by webhook
 *
 * Artifact dependency:
 *   - Fetches elohim-cache-core WASM from elohim-holochain pipeline
 *
 * @see genesis/orchestrator/Jenkinsfile for central trigger logic
 */

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

def loadBuildVars() {
    def rootEnv = "${env.WORKSPACE}/build.env"
    def path = fileExists(rootEnv) ? rootEnv : 'build.env'
    
    echo "DEBUG: Looking for build.env at: ${path}"
    if (!fileExists(path)) {
        error "build.env not found at ${path}"
    }
    
    // Debug: Show actual file contents
    sh "echo '--- build.env content ---'; cat '${path}'"
    
    def props = readProperties file: path
    echo "DEBUG: Properties read from file: ${props}"
    
    // Return the properties instead of trying to set env
    return props
}

// Helper to setup environment from properties
def withBuildVars(props, Closure body) {
    withEnv([
        "BASE_VERSION=${props.BASE_VERSION ?: ''}",
        "GIT_COMMIT_HASH=${props.GIT_COMMIT_HASH ?: ''}",
        "IMAGE_TAG=${props.IMAGE_TAG ?: ''}",
        "BRANCH_NAME=${props.BRANCH_NAME ?: env.BRANCH_NAME}"
    ]) {
        body()
    }
}

// Helper to determine SonarQube project config based on branch
// Returns: [projectKey: String, shouldEnforce: Boolean, env: String]
@NonCPS
def getSonarProjectConfig() {
    def targetBranch = env.CHANGE_TARGET ?: env.BRANCH_NAME

    if (targetBranch == 'main') {
        return [projectKey: 'elohim-app', shouldEnforce: true, env: 'prod']
    } else if (targetBranch == 'staging' || targetBranch ==~ /staging-.+/) {
        return [projectKey: 'elohim-app-staging', shouldEnforce: false, env: 'staging']
    } else {
        return [projectKey: 'elohim-app-alpha', shouldEnforce: false, env: 'alpha']
    }
}

// ============================================================================
// STAGE HELPER METHODS (to reduce bytecode size)
// ============================================================================

/**
 * Check if a build step should run based on the STEPS parameter.
 * Returns true if STEPS is 'all' (default) or contains the step name.
 */
def shouldRunStep(String stepName) {
    def steps = (params.STEPS ?: 'all').split(',').collect { it.trim() }
    return steps.contains('all') || steps.contains(stepName)
}

def deployAppToEnvironment(String environment, String namespace, String deploymentName, String manifestPath, String imageTag) {
    def helpers = load 'genesis/orchestrator/scripts/deploy-helpers.groovy'
    helpers.deployAppToEnvironment(environment, namespace, deploymentName, manifestPath, imageTag)
}

def buildSophiaPlugin() {
    // Fetch pre-built sophia-element from Nexus instead of building from submodule
    echo 'Fetching sophia-element from Nexus...'
    sh '''#!/bin/bash
        set -euo pipefail
        ASSET_DIR=app/elohim-app/src/assets/sophia-plugin
        mkdir -p "$ASSET_DIR"
        WORKDIR=$(mktemp -d)

        # Fetch and extract the published package from Nexus
        cd "$WORKDIR"
        npm pack @ethosengine/sophia-element --registry=https://nexus.ethosengine.com/repository/npm/
        tar xzf ethosengine-sophia-element-*.tgz

        # Copy UMD bundle + CSS to elohim-app assets
        cp package/dist/sophia-element.umd.js "$OLDPWD/$ASSET_DIR/"
        cp package/dist/index.css "$OLDPWD/$ASSET_DIR/" 2>/dev/null || true
        cp package/dist/sophia-element.umd.css "$OLDPWD/$ASSET_DIR/" 2>/dev/null || true
        cd "$OLDPWD"

        # Create stub CSS files for theme overrides (actual theming is via Sophia.configure() API)
        echo "/* Sophia theme overrides - Configure via Sophia.configure() API */" > "$ASSET_DIR/sophia-theme-overrides.css"
        echo "/* Sophia styles - bundled in UMD */" > "$ASSET_DIR/sophia.css"

        # Verify UMD bundle is actually UMD format (not ESM)
        if head -c 50 "$ASSET_DIR/sophia-element.umd.js" | grep -q "^import "; then
            echo "ERROR: sophia-element.umd.js contains ESM syntax instead of UMD"
            exit 1
        fi
        echo "✅ sophia-element fetched from Nexus and verified"
        ls -la "$ASSET_DIR/"
        rm -rf "$WORKDIR"
    '''
}

def runE2ETests(String environment, String baseUrl, String gitCommitHash) {
    echo "Running E2E tests against ${environment}"
    env.E2E_TESTS_RAN = 'true'

    // Install Cypress if needed
    sh '''
        if [ ! -d "node_modules/cypress" ]; then
            pnpm add cypress @badeball/cypress-cucumber-preprocessor @cypress/browserify-preprocessor @bahmutov/cypress-esbuild-preprocessor
        fi
    '''

    // Verify environment is up
    sh """
        timeout 60s bash -c 'until curl -s -o /dev/null -w "%{http_code}" ${baseUrl} | grep -q "200\\|302\\|301"; do
            sleep 5
        done'
        echo "✅ ${environment} site is responding"
    """

    // Run tests
    sh """#!/bin/bash
        export CYPRESS_baseUrl=${baseUrl}
        export CYPRESS_ENV=${environment}
        export CYPRESS_EXPECTED_GIT_HASH=${gitCommitHash}
        export NO_COLOR=1
        export DISPLAY=:99

        Xvfb :99 -screen 0 1024x768x24 -ac > /dev/null 2>&1 &
        XVFB_PID=\\\$!
        sleep 2

        npx cypress verify > /dev/null
        mkdir -p cypress/reports

        npx cypress run \\
            --headless \\
            --browser chromium \\
            --spec "cypress/e2e/staging-validation.feature"

        kill \\\$XVFB_PID 2>/dev/null || true
    """

    echo "✅ ${environment} validation passed!"
}

def publishE2EReports(String environment) {
    if (env.E2E_TESTS_RAN == 'true') {
        echo '📊 Publishing cucumber reports...'

        if (environment == 'staging') {
            sh 'echo "DEBUG: Contents of cypress directory:"'
            sh 'find cypress -type f -name "*" 2>/dev/null || echo "cypress directory not found"'
            sh 'echo "DEBUG: Contents of cypress/reports directory:"'
            sh 'ls -la cypress/reports/ 2>/dev/null || echo "cypress/reports directory not found"'
            sh 'echo "DEBUG: Current working directory: $(pwd)"'
            sh 'echo "DEBUG: Absolute path to cucumber report: $(pwd)/cypress/reports/cucumber-report.json"'
            sh 'test -f cypress/reports/cucumber-report.json && echo "DEBUG: File exists and is readable" || echo "DEBUG: File does not exist or is not readable"'
        }

        if (fileExists('cypress/reports/cucumber-report.json')) {
            cucumber([
                reportTitle: "E2E Test Results (${environment})",
                fileIncludePattern: 'cucumber-report.json',
                jsonReportDirectory: 'cypress/reports',
                buildStatus: 'FAILURE',
                failedFeaturesNumber: -1,
                failedScenariosNumber: -1,
                failedStepsNumber: -1,
                skippedStepsNumber: -1,
                pendingStepsNumber: -1,
                undefinedStepsNumber: -1
            ])
            echo 'Cucumber reports published successfully'
        } else {
            echo 'No cucumber reports found to publish'
        }
    } else {
        echo 'E2E tests did not run - skipping cucumber report publishing'
    }

    // Archive test artifacts
    if (env.E2E_TESTS_RAN == 'true') {
        if (fileExists('cypress/screenshots')) {
            archiveArtifacts artifacts: 'cypress/screenshots/**/*.png', allowEmptyArchive: true
        }
        if (fileExists('cypress/videos')) {
            archiveArtifacts artifacts: 'cypress/videos/**/*.mp4', allowEmptyArchive: true
        }
        if (fileExists('cypress/reports/cucumber-report.json')) {
            archiveArtifacts artifacts: 'cypress/reports/cucumber-report.json', allowEmptyArchive: true
        }
    }
}

def stageSpaBlobs(String doorwayEprUrl, List<Map> bundles, String adminKey, Map outcomes) {
    // Byte-seed one OR MORE pillar-EPR browser bundles onto ONE backend. Each
    // bundle is a {distDir, slug} pair: the dist contents get zipped and PUT as
    // a content-addressed blob (/admin/seed/blob). The notarized head is NOT
    // written here — authorHeadOnce PATCHes it exactly once via a live conductor
    // bridge, and it gossips to every peer. Blob BYTES don't auto-replicate P2P
    // yet, so seeding them per backend is legitimate load-spread.
    //
    // Pillar-EPR decomposition (Task B21): each pillar projects its own
    // bundle onto its own content row. The previous "one blob, two slugs"
    // arrangement (elohim-app bundle on both elohim-host-landing AND
    // lamad-spa) was a coincidence of single-app deployment; with the
    // lamad SPA split out into app/lamad, each surface owns its bundle:
    //
    //   db/content/elohim-host-landing  — landing-page EPR projected by
    //                                     doorway-A (alpha.elohim.host)
    //                                     + doorway-B (elohim.host) as
    //                                     ROOT_APP_SLUG; served from
    //                                     app/elohim-app dist
    //   db/content/lamad-spa            — lamad pillar EPR served from
    //                                     app/lamad dist at /lamad/...
    //
    // The JSON source for these content nodes intentionally omits
    // blobHash; the seed-sqlite step does not overwrite the deploy-time
    // value written here.
    //
    // adminKey is passed for the PUT's X-API-Key (gated / non-DEV_MODE backends
    // require it to seed bytes). This helper never PATCHes the head. See:
    //   genesis/docs/superpowers/plans/2026-05-23-spa-blob-deploy-drift.md
    //   genesis/docs/handoffs/2026-05-23-followup-2-k8s-handoff-summary.md
    //
    // index.html: SSR-mode dists (elohim-app, Angular 19) emit
    // index.csr.html only; materialize to index.html since storage's
    // /apps lookup is literal-path. Pure SPAs (app/lamad) pass through.
    // Bash body lives in scripts/ci/stage-spa-blob.sh (extracted 2026-06-10:
    // the inline heredoc pushed the CPS method past the JVM 64KB
    // MethodTooLargeException limit — builds #1519/#1520 died at Jenkinsfile
    // compile, zero stages ran). Keep helpers heredoc-free.
    // Byte-seed pass ONLY (PUT /admin/seed/blob). The notarized head is authored
    // exactly once by authorHeadOnce (failover to a live conductor bridge); this
    // helper never PATCHes the head, so DO_PATCH stays 0. adminKey is still passed
    // for the PUT's X-API-Key (gated / non-DEV_MODE backends require it).
    def doPatch = '0'
    def host = doorwayEprUrl.replaceFirst(/^https?:\/\//, '')
    for (bundle in bundles) {
        def kind = bundle.kind ?: 'browser'
        // String key (NOT a GString) so the cross-method map lookup in
        // emitAppDeployJunit is reliable — a GString and an equal String are
        // not interchangeable map keys in Groovy.
        def outcomeKey = "${host}|${bundle.slug}|${kind}".toString()
        echo "stageSpaBlobs: host='${host}' distDir='${bundle.distDir}' slug='${bundle.slug}'"
        // Per-(host,bundle) isolation for the BYTE-SEED pass. Blob BYTES are
        // content-addressed and do not auto-replicate P2P yet, so each serving
        // backend must carry them (legitimate load-spread — NOT a divergent
        // write; the head is authored once and gossips). One backend's byte
        // upload failing must NOT skip the remaining bundles/hosts: catch per
        // (host,slug) so every still-seedable backend lands, the failed one goes
        // UNSTABLE (orchestrator treats UNSTABLE as success), and
        // emitAppDeployJunit NAMES it (Part B, 2026-06-27) instead of a buried
        // UNSTABLE. The notarized head is NOT authored here — authorHeadOnce does
        // that exactly once, via a live conductor bridge.
        def verdictFile = "${env.WORKSPACE}/.ci-deliverability-${bundle.slug}-${kind}.txt"
        catchError(buildResult: 'UNSTABLE', stageResult: 'UNSTABLE',
                   message: "seed ${host} ${bundle.slug} (${kind}): blob byte upload failed after retries; see junit testcase") {
            withEnv(["STORAGE_API_KEY_ADMIN=${adminKey ?: ''}", "DO_PATCH=${doPatch}", "DELIVERABILITY_VERDICT_FILE=${verdictFile}"]) {
                // Clear any stale marker from an earlier build/host in this
                // reused workspace before the byte-seed runs — otherwise a
                // BROKEN_HEAD written by host A's run survives into host B's
                // clean run's outcomes (the read below only assigns when the
                // file exists, so a leftover file is indistinguishable from
                // a fresh one).
                sh "rm -f '${verdictFile}'"
                sh "bash '${env.WORKSPACE}/scripts/ci/stage-spa-blob.sh' '${bundle.distDir}' '${bundle.slug}' '${doorwayEprUrl}' '${kind}'"
            }
            // Reached only on a clean return — catchError swallows exceptions
            // before this line on any failure path.
            outcomes[outcomeKey] = true
        }
        // Read the deliverability marker AFTER catchError so a failed (caught)
        // stage still records BROKEN_HEAD if stage-spa-blob.sh wrote one before
        // exiting non-zero — authorHeadOnce below is the sole consumer.
        if (fileExists(verdictFile)) {
            outcomes["deliverability|${bundle.slug}|${kind}".toString()] = readFile(verdictFile).trim()
        }
    }
}

// Author the single notarized head for ONE bundle, exactly once. The blobHash
// PATCH is a DNA-notarized write (patch_needs_conductor=true): the doorway
// routes it to a storage backend whose CONDUCTOR authors the DHT entry — the
// peer network (Holochain DHT) is the witness, the doorway is only the gateway.
// A backend with no live conductor bridge 503s (the script exits non-zero); we
// fail the PATCH over across doorways until one authors it, then STOP. The single
// witnessed head gossips to every peer (run_content_sweep), so the other
// backends converge WITHOUT any per-host head write. Returns the authoring host,
// or null if NO doorway in the fabric could witness the head. Own top-level def
// = own CPS method; no heredoc (CPS 64KB limit — bash lives in stage-spa-blob.sh).
def authorHeadOnce(List<String> doorwayEprUrls, Map bundle, String adminKey, Map outcomes) {
    def kind = bundle.kind ?: 'browser'
    def verdictKey = "deliverability|${bundle.slug}|${kind}".toString()
    if (outcomes[verdictKey]?.startsWith('BROKEN_HEAD')) {
        // A peer judged this bundle from its bytes: it cannot boot. Authoring
        // the head would mint a witnessed pointer to a blank page (2026-09-04).
        // Do NOT error() here: this call runs inside Phase 2's own catchError
        // (stageAndVerifyAllBundles), which would swallow the throw to UNSTABLE
        // (orchestrator treats UNSTABLE as success) AND unwind the bundle loop,
        // starving sibling bundles of authoring. Record + skip instead; the
        // hard FAILURE is raised once, after the loop, by the caller.
        outcomes["broken|${bundle.slug}|${kind}".toString()] = outcomes[verdictKey]
        echo "authorHeadOnce: ${bundle.slug} (${kind}) SKIPPED — ${outcomes[verdictKey]}; no head will be authored for a bundle that cannot boot"
        return null
    }
    def authorKey = "author|${bundle.slug}|${kind}".toString()
    // Hand-off file for verifyProjectedHeads (Track-4 T4-2): stage-spa-blob.sh
    // writes the content hash it just computed here so the Jenkinsfile can read
    // it back as the EXPECTED hash for the served-vs-declared propagation probe,
    // without a second independent zip/hash (see stage-spa-blob.sh comment).
    def hashFile = "${env.WORKSPACE}/.ci-authored-hash-${bundle.slug}-${kind}.txt"
    // Same marker convention as stageSpaBlobs' byte-seed pass (Phase 1): a
    // Phase-1 verdict was already checked above (the outcomes[verdictKey]
    // guard), but the PATCH call below re-runs the gate too — a bundle whose
    // bytes only become judged BROKEN between Phase 1 and this Phase-2 PATCH
    // must not fall silently into the "no live conductor bridge" fail-over
    // bucket (that swallows the verdict and never reaches the post-Phase-2
    // error()).
    def verdictFile = "${env.WORKSPACE}/.ci-deliverability-${bundle.slug}-${kind}.txt"
    for (int i = 0; i < doorwayEprUrls.size(); i++) {
        def doorwayEprUrl = doorwayEprUrls[i]
        def host = doorwayEprUrl.replaceFirst(/^https?:\/\//, '')
        def rc = 1
        // returnStatus (not throw): a 503 here is EXPECTED on a bridgeless
        // backend and means "try the next doorway", not "fail the build".
        withEnv(["STORAGE_API_KEY_ADMIN=${adminKey ?: ''}", "DO_PATCH=1", "HASH_OUTPUT_FILE=${hashFile}", "DELIVERABILITY_VERDICT_FILE=${verdictFile}"]) {
            // Clear any stale marker (an earlier doorway's fail-over attempt
            // in this same loop, or a leftover from a prior build in this
            // reused workspace) before this doorway's own run — mirrors the
            // per-call rm -f in stageSpaBlobs above.
            sh "rm -f '${verdictFile}'"
            rc = sh(returnStatus: true,
                    script: "bash '${env.WORKSPACE}/scripts/ci/stage-spa-blob.sh' '${bundle.distDir}' '${bundle.slug}' '${doorwayEprUrl}' '${kind}'")
        }
        if (rc == 0) {
            echo "authorHeadOnce: ${bundle.slug} (${kind}) — head authored via ${host}'s conductor bridge; DHT witnesses it, converges to all peers"
            outcomes[authorKey] = host
            if (fileExists(hashFile)) {
                outcomes["hash|${bundle.slug}|${kind}".toString()] = readFile(hashFile).trim()
            }
            // Propagate the canonical-head declaration to the OTHER doorways
            // (DECLARE_ONLY leg in the script). Under R1 (2026-07-31 decision:
            // genesis/data/timeline/backlog/content-head-election-vs-reach-fork-arbitration.md)
            // this IS the declared-vs-declared arbitration channel — no
            // automatic arbitration exists between two competing declared
            // heads, so a fan-out failure here must be visible rather than
            // buried. returnStatus: true still keeps this from hard-failing
            // the build (a target doorway staying stale is not fatal to the
            // deploy), but the script now exits non-zero on any fan-out that
            // never reached HTTP 2xx (source/hash unresolvable, curl error, or
            // retry ladder exhausted) — surface that as stage/build UNSTABLE
            // so it isn't silently swallowed.
            for (int j = 0; j < doorwayEprUrls.size(); j++) {
                if (j == i) { continue }
                // DECLARE_MAX_ATTEMPTS=24 (~36min worst-case): cross-conductor
                // retrievability lands 18-50min post-author on the live pair —
                // the default 12-attempt ladder (~18min) misses the tail. One
                // successful declare records ordering on the peer, after which
                // the monotonic heal converges it automatically each sweep.
                def declareRc = 1
                withEnv(["STORAGE_API_KEY_ADMIN=${adminKey ?: ''}", 'DECLARE_ONLY=1', 'DECLARE_MAX_ATTEMPTS=24', "SOURCE_DOORWAY_URL=${doorwayEprUrl}"]) {
                    declareRc = sh(returnStatus: true,
                       script: "bash '${env.WORKSPACE}/scripts/ci/stage-spa-blob.sh' '-' '${bundle.slug}' '${doorwayEprUrls[j]}' '${kind}'")
                }
                if (declareRc != 0) {
                    unstable("canonical-head declare ${bundle.slug} (${kind}): ${doorwayEprUrls[j]} did not confirm propagation (exit ${declareRc}) — that doorway may stay stale until the next declare-cycle or heal converges it; see stage-spa-blob.sh DECLARE_ONLY log above")
                }
            }
            return host
        }
        echo "authorHeadOnce: ${host} could not author ${bundle.slug} (${kind}) (no live conductor bridge / persistent 503) — failing over to next doorway"
    }
    // Every doorway failed to author. Before reporting a generic "no live
    // bridge" outcome, check whether the LAST verdict marker written during
    // this loop says the peer judged the bytes broken — that is a
    // Phase-2-first BROKEN_HEAD (Phase 1's byte-seed pass saw a healthy or
    // not-yet-judged verdict, but this Phase-2 gate run caught it). Record it
    // the same way the Phase-1-caught case is recorded above so the
    // post-Phase-2 hard error() in stageAndVerifyAllBundles still fires.
    if (fileExists(verdictFile)) {
        def verdict = readFile(verdictFile).trim()
        if (verdict.startsWith('BROKEN_HEAD')) {
            outcomes["broken|${bundle.slug}|${kind}".toString()] = verdict
            echo "authorHeadOnce: ${bundle.slug} (${kind}) SKIPPED — ${verdict}; the peer judged this bundle broken during the author-head gate run; no head will be authored"
            return null
        }
    }
    echo "authorHeadOnce: NO doorway could author ${bundle.slug} (${kind}) — no live conductor bridge in the fabric to witness the head"
    return null
}

def verifyEprMounts(String doorwayUrl, List<String> mounts) {
    // End-to-end EPR serving seatbelt (2026-06-09 regression class): content
    // rows can point at blob hashes the backing storage no longer holds — in
    // that state /apps/{slug}/* keeps serving 200 from the doorway's own app
    // cache while the EPR-routed mounts a human actually visits ('/', '/lamad')
    // 404 with "App ZIP blob not found" for days, invisibly. So probe the
    // routed mounts themselves, not /apps. Retries span the EPR router's 30s
    // self-heal refresh window. Caller wraps in catchError->UNSTABLE: drift is
    // surfaced without aborting the orchestrator dependency chain.
    // Bash body lives in scripts/ci/verify-epr-mount.sh (extracted 2026-06-10,
    // CPS 64KB limit — see stageSpaBlobs note). Keep helpers heredoc-free.
    for (mount in mounts) {
        sh "bash '${env.WORKSPACE}/scripts/ci/verify-epr-mount.sh' '${doorwayUrl}${mount}'"
    }
}

// Served-vs-declared propagation probe (Track-4 T4-2). verifyEprMounts (above)
// proves a routed mount answers 200; stageSpaBlobs/authorHeadOnce prove the
// content ROW's declared head was PATCHed. Neither proves the running doorway
// PROCESS has actually materialized that head — a stale-but-200 host passes
// both. This leg asks each doorway's health surface directly what server
// bundle head it has served and compares it to the hash authorHeadOnce just
// authored (outcomes["hash|slug|kind"]). Only server-kind bundles are probed:
// the T4-1 health-surface contract (servedBundleHeads[].serverBlobHash) only
// attests the SSR bundle — see verify-projected-head.sh header for the
// browser-bundle limitation. Records one outcome per (host, slug, kind) so
// emitAppDeployJunit can name each host's leg individually. Bash body lives in
// scripts/ci/verify-projected-head.sh (CPS 64KB limit — see stageSpaBlobs
// note; helpers stay heredoc-free).
def verifyProjectedHeads(List<String> doorwayEprUrls, List<Map> bundles, String gitCommitHash, Map outcomes) {
    for (bundle in bundles) {
        def kind = bundle.kind ?: 'browser'
        if (kind != 'server') { continue }
        def expectedHash = outcomes["hash|${bundle.slug}|${kind}".toString()]
        if (!expectedHash) {
            echo "verifyProjectedHeads: no authored hash recorded for ${bundle.slug} (${kind}) — skipping probe (author leg did not succeed)"
            continue
        }
        for (doorwayEprUrl in doorwayEprUrls) {
            def host = doorwayEprUrl.replaceFirst(/^https?:\/\//, '')
            def rc = sh(returnStatus: true,
                    script: "bash '${env.WORKSPACE}/scripts/ci/verify-projected-head.sh' '${doorwayEprUrl}' '${bundle.slug}' '${expectedHash}' '${gitCommitHash ?: ''}'")
            outcomes["projhead|${host}|${bundle.slug}|${kind}".toString()] = (rc == 0)
        }
    }
}

// The three helpers below carry the Upload-SPA-Blob stage's script-block body.
// Extracted 2026-06-10 (second cut of the CPS 64KB breach): the block grew
// 9,034 → 9,898 source bytes when the seatbelt call-sites landed, and THAT
// delta — not the helper heredocs — is what pushed WorkflowScript.___cps___7636
// over the JVM method limit (#1519/#1520/#1521 all died at Jenkinsfile
// compile). Split small on purpose: each top-level def is its own CPS method;
// one big helper would just relocate the breach.

def resolveDoorwayEprUrls() {
    // A doorway-EPR URL is a DNS-facilitated address routed and projected by
    // a doorway to a specific EPR's hosting contract (here: the
    // elohim-host-landing EPR + lamad-spa). It is NOT "the doorway URL" —
    // that name belongs to the doorway-service surface itself. A single
    // doorway projects/hosts many EPRs; each has its own DNS-facilitated URL
    // via the doorway's stewardship contract.
    //
    // We hit the doorway-EPR URL (not the storage tier directly) because
    // storage is peer-native and not reachable from outside the cluster. The
    // doorway proxies /blob/{hash} (PUT) and /db/content/{id} (PATCH)
    // through to storage. The previous default (a headless-service pod FQDN)
    // only resolved inside the elohim-alpha namespace; build pods in the
    // jenkins namespace got curl exit 6 — see App #1457.
    //
    // Alpha cluster has TWO storage backends — matthew (alpha.elohim.host) +
    // adam (elohim.host). Each must carry the SPA blob BYTES (bytes don't
    // auto-replicate P2P yet — legitimate per-host load-spread), but the
    // notarized blobHash HEAD is authored ONCE via a live conductor bridge and
    // gossips to every peer (authorHeadOnce) — NOT a per-storage write (that
    // minted divergent, un-witnessed heads). This list is both the byte-seed set
    // and the failover order for the single head author.
    // STORAGE_URL env still overrides for ad-hoc or in-cluster targeting.
    def branch = env.BRANCH_NAME ?: 'dev'
    def defaults
    if (branch == 'main') {
        defaults = ['https://elohim.host']
    } else if (branch == 'staging' || branch.startsWith('staging-')) {
        defaults = ['https://staging.elohim.host']
    } else {
        defaults = ['https://alpha.elohim.host', 'https://elohim.host']
    }
    return env.STORAGE_URL ? [env.STORAGE_URL] : defaults
}

def resolveStorageAdminKey() {
    // Auth for PATCH /db/content/{id}: try `storage-api-key-admin`
    // (k8s-provisioned) then fall back to `doorway-admin-bootstrap-key`
    // (genesis/Jenkinsfile seed stages). App pipeline credential scope is
    // sometimes folder-disjoint; the fallback keeps both visibility paths
    // working without operator coordination.
    def adminKey = ''
    def credUsed = ''
    try {
        withCredentials([string(credentialsId: 'storage-api-key-admin', variable: 'ADMIN_KEY')]) {
            adminKey = env.ADMIN_KEY
            credUsed = 'storage-api-key-admin'
        }
    } catch (e1) {
        try {
            withCredentials([string(credentialsId: 'doorway-admin-bootstrap-key', variable: 'ADMIN_KEY')]) {
                adminKey = env.ADMIN_KEY
                credUsed = 'doorway-admin-bootstrap-key'
            }
        } catch (e2) {
            adminKey = ''
            credUsed = ''
        }
    }
    if (credUsed) {
        echo "stageSpaBlobs auth: using credential '${credUsed}'"
    } else {
        // RC2 hardening (2026-05-28): fail loud instead of silently degrading
        // to PUT-only. Without the admin credential the blobHash PATCH cannot
        // run, db/content/lamad-spa keeps no blobHash, /apps/lamad-spa/ 404s,
        // /lamad goes dark while the build reports green — exactly the drift
        // spa-blob-deploy-drift documented.
        error("stageSpaBlobs auth: neither 'storage-api-key-admin' nor 'doorway-admin-bootstrap-key' is visible at this job's credential scope. The blobHash PATCH (db/content/{slug}) cannot run without it, which leaves lamad-spa blobless and /lamad 404ing. Provision one of these credentials at the App job/folder scope, then re-run. (To intentionally ship a no-content deploy, remove this guard deliberately.)")
    }
    return adminKey
}

def stageAndVerifyAllBundles(List<String> doorwayEprUrls, String adminKey, String gitCommitHash) {
    // Two-phase deploy of the pillar-EPR bundles (Task B21). Deploy-seed is
    // post-build and transient-prone (conductor/doorway 503 during cluster
    // churn); the orchestrator runs this pipeline wait-for-result at Level 0, so
    // a hard FAILURE here aborts the whole dependency graph. catchError ->
    // UNSTABLE keeps the chain alive (the orchestrator treats UNSTABLE as
    // success). The credential-missing guard stays a hard error upstream
    // (resolveStorageAdminKey).
    //
    //   Phase 1 (byte-seed, per host): PUT the content-addressed blob bytes onto
    //     EVERY serving backend — bytes don't auto-replicate P2P yet, so this is
    //     legitimate load-spread, not a divergent write.
    //   Phase 2 (author head, ONCE): PATCH the notarized head exactly once, via
    //     the first doorway that reaches a live conductor bridge. The conductor
    //     authors the DHT entry, the peer network witnesses it, and it gossips to
    //     every peer (run_content_sweep) — so the other backends converge WITHOUT
    //     a per-host head write. This replaces the retired per-host `amber` PATCH
    //     that minted divergent, un-witnessed heads (the per-host stranding class).
    def bundles = [
        [distDir: "${env.WORKSPACE}/app/elohim-app/dist/elohim-app/browser", slug: "elohim-host-landing"],
        [distDir: "${env.WORKSPACE}/app/elohim-app/dist/elohim-app/server",  slug: "elohim-host-landing", kind: "server"],
        [distDir: "${env.WORKSPACE}/app/lamad/dist/lamad/browser",           slug: "lamad-spa"],
        [distDir: "${env.WORKSPACE}/app/lamad/dist/lamad/server",            slug: "lamad-spa", kind: "server"],
    ]
    def outcomes = [:]

    // Phase 1 — byte-seed every backend. Per-(host,slug) isolation lives INSIDE
    // stageSpaBlobs; this outer catchError is a backstop for non-sh throws only.
    catchError(buildResult: 'UNSTABLE', stageResult: 'UNSTABLE') {
        for (int i = 0; i < doorwayEprUrls.size(); i++) {
            stageSpaBlobs(doorwayEprUrls[i], bundles, adminKey, outcomes)
        }
    }

    // Phase 2 — author each bundle's head EXACTLY once (failover to a live
    // bridge). authorHeadOnce swallows a bridgeless 503 to try the next doorway;
    // a bundle whose head NO doorway could witness is NAMED UNSTABLE by
    // emitAppDeployJunit. Unlike the retired per-host PATCH, we never write an
    // un-witnessed local head as a fallback.
    catchError(buildResult: 'UNSTABLE', stageResult: 'UNSTABLE') {
        for (bundle in bundles) {
            authorHeadOnce(doorwayEprUrls, bundle, adminKey, outcomes)
        }
    }
    // Hard gate, OUTSIDE Phase 2's catchError above (so it is a real FAILURE,
    // never swallowed to UNSTABLE): authorHeadOnce recorded 'broken|...' and
    // returned without authoring for every bundle a peer judged BROKEN_HEAD.
    // A plain keySet() loop (not findAll/collect) keeps this CPS-safe.
    def broken = []
    for (k in outcomes.keySet()) {
        if (k.startsWith('broken|')) {
            broken.add("${k.substring(7)}: ${outcomes[k]}")
        }
    }
    if (!broken.isEmpty()) {
        error("Deploy refused: ${broken.size()} bundle(s) cannot boot — ${broken.join('; ')}. The peer judged the bytes; fix the build, do not re-run.")
    }

    // End-to-end serving seatbelt: probe the EPR-routed mounts a human actually
    // visits. Each host serves 200 via its own converged head OR via doorway
    // failover during the convergence window. Skipped on STORAGE_URL override (a
    // raw storage backend has no EPR router). UNSTABLE per the dependency-chain
    // rule.
    if (!env.STORAGE_URL) {
        catchError(buildResult: 'UNSTABLE', stageResult: 'UNSTABLE') {
            for (int i = 0; i < doorwayEprUrls.size(); i++) {
                verifyEprMounts(doorwayEprUrls[i], ['/', '/lamad'])
            }
        }

        // Phase 4 (Track-4 T4-2) — served-vs-declared propagation probe: does the
        // running doorway PROCESS actually serve the head just authored above,
        // not merely a 200'ing mount over a stale materialization? Skipped on
        // STORAGE_URL override for the same reason verifyEprMounts is (a raw
        // storage backend has no health-surface EPR attestation either).
        catchError(buildResult: 'UNSTABLE', stageResult: 'UNSTABLE') {
            verifyProjectedHeads(doorwayEprUrls, bundles, gitCommitHash, outcomes)
        }
    }

    // Moved to the END (was previously emitted before verifyEprMounts/
    // verifyProjectedHeads ran): both later legs now feed named outcomes into
    // this report, so it must run after every leg has populated `outcomes`.
    emitAppDeployJunit((env.BRANCH_NAME ?: 'dev'), doorwayEprUrls, bundles, outcomes)
}

// Emit a junit-style report for the per-(host,slug) SPA-blob deploy (Part B,
// 2026-06-27). One testcase per (host, bundle) cell, classname
// `elohim-app.deploy.<env>`. Registered via junit() so a STALE host surfaces in
// the test-report tab + getTestResults even though the build stays UNSTABLE —
// the orchestrator treats UNSTABLE as success, so a swallowed leg was
// previously invisible (the per-host deploy-lag class: elohim.host stuck on an
// old bundle while alpha advanced). A passing leg => outcomes["host|slug|kind"]
// == true (set inside stageSpaBlobs only on clean return). Mirrors the edge
// emitDeployJunit. Own top-level def = own CPS method; no heredoc (CPS 64KB).
def emitAppDeployJunit(String envName, List<String> doorwayEprUrls, List<Map> bundles, Map outcomes) {
    def safeEnv = (envName ?: 'dev').replaceAll('[^A-Za-z0-9._-]', '-')
    def cases = []
    // Byte-seed legs: one per (host, bundle). Passed => the blob bytes landed on
    // that backend (outcome recorded true inside stageSpaBlobs on clean return).
    doorwayEprUrls.each { url ->
        def host = url.replaceFirst(/^https?:\/\//, '')
        bundles.each { b ->
            def kind = b.kind ?: 'browser'
            cases << [name: "seed ${host} ${b.slug} (${kind})".toString(),
                      kind: 'seed',
                      passed: outcomes["${host}|${b.slug}|${kind}".toString()] == true]
        }
    }
    // Author legs: one per bundle. Passed => some doorway's conductor witnessed
    // the single head (outcomes["author|slug|kind"] holds the authoring host).
    bundles.each { b ->
        def kind = b.kind ?: 'browser'
        cases << [name: "author ${b.slug} (${kind})".toString(),
                  kind: 'author',
                  passed: outcomes["author|${b.slug}|${kind}".toString()] != null]
    }
    // Projected-head legs (Track-4 T4-2): one per (host, server-bundle). Passed
    // => this host's health surface (/health/startup or /health) served the
    // just-authored serverBlobHash, OR the T4-1 attestation isn't deployed
    // there yet (verify-projected-head.sh's FIELD-ABSENT reads as exit 0 —
    // an honest skip, not a failure). Only server-kind bundles carry a
    // servedBundleHeads entry in the contract; a leg is only emitted when the
    // author leg actually recorded a hash to check against (outcomes["hash|…"]).
    doorwayEprUrls.each { url ->
        def host = url.replaceFirst(/^https?:\/\//, '')
        bundles.each { b ->
            def kind = b.kind ?: 'browser'
            if (kind != 'server') { return }
            def key = "projhead|${host}|${b.slug}|${kind}".toString()
            if (!outcomes.containsKey(key)) { return }
            cases << [name: "projected-head ${host} ${b.slug} (${kind})".toString(),
                      kind: 'projhead',
                      passed: outcomes[key] == true]
        }
    }
    def failed = cases.count { !it.passed }
    def lines = cases.collect { c ->
        def attrs = "classname=\"elohim-app.deploy.${safeEnv}\" name=\"${c.name}\" time=\"0\""
        if (c.passed) {
            "  <testcase ${attrs}/>"
        } else {
            def msg
            if (c.kind == 'author') {
                msg = "Head author '${c.name}' failed: NO doorway in the fabric reached a live conductor bridge to author (witness) this bundle's single notarized head. The head cannot green or converge until a conductor bridge is live. Check the alpha peers' conductor health (storage /health, conductor app-WS)."
            } else if (c.kind == 'projhead') {
                msg = "Projected-head probe '${c.name}' failed: this host's health surface (/health/startup or /health) served a serverBlobHash that does NOT match the just-authored declared head, or the host was unreachable after retries. The running doorway PROCESS has not materialized the current SSR bundle (a stale-but-200 host) — check its logs / trigger a restart to pick up the hot-swap. (T4-1 attestation absence alone never fails this leg — see scripts/ci/verify-projected-head.sh.)"
            } else {
                msg = "Blob byte-seed '${c.name}' failed after retries (PUT /admin/seed/blob): this backend did not receive the bundle bytes. A transient 503 during cluster churn is the usual cause (now retried in stage-spa-blob.sh); a persistent failure means the backend is down. Re-run the App pipeline, or check the host storage /health."
            }
            msg = msg.replace('&', '&amp;').replace('<', '&lt;').replace('"', '&quot;')
            "  <testcase ${attrs}><failure message=\"${msg}\" type=\"spa-blob-${c.kind}\"/></testcase>"
        }
    }
    def xml = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        "<testsuite name=\"elohim-app.deploy.${safeEnv}\" tests=\"${cases.size()}\" failures=\"${failed}\">",
        lines.join('\n'),
        '</testsuite>',
    ].join('\n')
    def reportFile = "deploy-app-${safeEnv}-junit.xml"
    writeFile(file: reportFile, text: xml)
    archiveArtifacts(artifacts: reportFile, allowEmptyArchive: true)
    junit(testResults: reportFile, allowEmptyResults: true)
    def passed = cases.size() - failed
    echo "App SPA-blob deploy for ${safeEnv}: ${passed}/${cases.size()} legs landed (byte-seed per host + one witnessed head author per bundle)"
    if (failed > 0) {
        def failedNames = cases.findAll { !it.passed }.collect { it.name }.join('; ')
        echo "App deploy partial failure: ${failed}/${cases.size()} legs failed — ${failedNames}. Build UNSTABLE (test shape); orchestrator proceeds. A failed 'author' leg means the single head was never witnessed; a failed 'seed' leg means a backend lacks the bytes."
    }
}

// ============================================================================
// END HELPER METHODS
// ============================================================================

pipeline {
    agent {
        kubernetes {
            cloud 'kubernetes'
            yaml '''
apiVersion: v1
kind: Pod
spec:
 serviceAccount: jenkins-deployer
 # operations OR edge, operations preferred: the edge label is the 7.6 GB ThinkPads and
 # a memory request alone still lands there (requests count reservations, not the
 # 3-5 GB the peers actually leave free) — elohim #1691 landed on thinkc-p1s again.
 # Same shape as the DNA pipeline's sweettest shards.
 affinity:
   nodeAffinity:
     requiredDuringSchedulingIgnoredDuringExecution:
       nodeSelectorTerms:
         - matchExpressions:
             - key: node-type
               operator: In
               values:
                 - operations
                 - edge
     preferredDuringSchedulingIgnoredDuringExecution:
       - weight: 100
         preference:
           matchExpressions:
             - key: node-type
               operator: In
               values:
                 - operations
 volumes:
  - name: containerd-sock
    hostPath:
     path: /var/snap/microk8s/common/run/containerd.sock
     type: Socket
  - name: buildkit-run
    emptyDir: {}
 containers:
 - name: builder
   image: harbor.ethosengine.com/ethosengine/ci-builder:latest
   # Always: :latest is a moving tag — a cached node can silently serve a stale
   # toolchain (#1218 shape). Freshness > outage-resilience (operator, 2026-06-07).
   imagePullPolicy: Always
   command:
   - cat
   tty: true
   # memory: the Angular build needs ~5 GB; with no request the scheduler put this
   # pod on a 7.6 GB ThinkPad twice (elohim #1689/#1690, 2026-09-04): the node hit
   # 0.13 GB free, the JNLP channel dropped and the controller's exec fallback got
   # HTTP 500 x5. A real request keeps the build on a node that can hold it.
   resources:
     requests:
       memory: "6Gi"
       ephemeral-storage: "2Gi"
     limits:
       memory: "10Gi"
       ephemeral-storage: "5Gi"
   volumeMounts:
   - name: containerd-sock
     mountPath: /run/containerd/containerd.sock
   - name: buildkit-run
     mountPath: /run/buildkit
 - name: buildkitd
   image: moby/buildkit:v0.12.5
   securityContext:
     privileged: true
   args:
   - --addr
   - unix:///run/buildkit/buildkitd.sock
   - --oci-worker=true
   - --containerd-worker=false
   volumeMounts:
   - name: containerd-sock
     mountPath: /run/containerd/containerd.sock
   - name: buildkit-run
     mountPath: /run/buildkit
'''
        }
    }
    
    environment {
        // Only set static values here
        BRANCH_NAME = "${env.BRANCH_NAME ?: 'main'}"
        NPM_TOKEN = credentials('ee-nexus-npm-token')
    }

    options {
        // Skip default checkout - it uses sparse checkout with 0% files
        // We do explicit full checkout in the Checkout stage
        skipDefaultCheckout(true)
        overrideIndexTriggers(false)  // Only orchestrator or manual triggers - no webhook/branch indexing
    }

    parameters {
        string(name: 'STEPS', defaultValue: 'all', description: 'Comma-separated list of build steps to run (from build-manifest.json). "all" runs everything.')
        booleanParam(
            name: 'DEPLOY_ONLY',
            defaultValue: false,
            description: 'No-op for this pipeline. Accepted so the orchestrator can propagate the flag uniformly; orchestrator skips triggering elohim-app when DEPLOY_ONLY=true.'
        )
    }

    // No triggers - orchestrator handles all webhook events
    // triggers { }

    stages {
        stage('Check Trigger') {
            steps {
                script {
                    def validTrigger = currentBuild.getBuildCauses().any { cause ->
                        cause._class.contains('UserIdCause') ||
                        cause._class.contains('UpstreamCause') ||
                        cause._class.contains('BranchIndexingCause')
                    }
                    if (!validTrigger) {
                        echo "⏭️ PIPELINE SKIPPED - Use orchestrator or manual trigger"
                        currentBuild.result = 'NOT_BUILT'
                        currentBuild.displayName = "#${env.BUILD_NUMBER} SKIPPED"
                        env.PIPELINE_SKIPPED = 'true'
                    } else {
                        echo "✅ Valid trigger: ${currentBuild.getBuildCauses()*.shortDescription.join(', ')}"
                    }
                }
            }
        }

        stage('Checkout') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder'){
                    script {
                        // Configure git safe directory before any git operations
                        sh 'git config --global --add safe.directory "*"'

                        // Explicit checkout - bypass sparse checkout config in job
                        checkout([
                            $class: 'GitSCM',
                            branches: [[name: "*/${env.BRANCH_NAME ?: 'dev'}"]],
                            extensions: [
                                [$class: 'CloneOption', shallow: false, noTags: true],
                                [$class: 'CleanBeforeCheckout']
                            ],
                            userRemoteConfigs: [[
                                url: 'https://github.com/ethosengine/elohim.git',
                                credentialsId: 'ee-bot-pat'
                            ]]
                        ])

                        echo "Building branch: ${env.BRANCH_NAME}"
                        echo "Change request: ${env.CHANGE_ID ?: 'None'}"

                        // Verify git state
                        sh 'git rev-parse HEAD | cut -c1-8'
                        sh 'git status'

                        // Enable pnpm via corepack (uses packageManager field in root package.json)
                        sh 'corepack enable'
                        sh 'pnpm --version'
                    }
                }
            }
        }

        stage('Setup Version') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder'){
                    script {
                        sh 'git config --global --add safe.directory "*"'

                        echo "DEBUG - Setup Version: Starting"
                        echo "DEBUG - Branch: ${env.BRANCH_NAME}"

                        // Validate VERSION file
                        if (!fileExists('VERSION')) {
                            error "VERSION file not found in workspace"
                        }

                        // Parse VERSION file in key-value format (APP_VERSION=x.x.x, HAPP_VERSION=x.x.x)
                        def versionContent = readFile('VERSION').trim()
                        def versionMap = [:]
                        versionContent.split('\n').each { line ->
                            def parts = line.split('=')
                            if (parts.length == 2) {
                                versionMap[parts[0].trim()] = parts[1].trim()
                            }
                        }
                        def baseVersion = versionMap['APP_VERSION'] ?: versionMap['HAPP_VERSION'] ?: '1.0.0'
                        echo "DEBUG - Base version: '${baseVersion}'"

                        if (!baseVersion) {
                            error "VERSION file is empty or malformed"
                        }

                        // Get git hash
                        def gitHash = sh(
                            script: 'git rev-parse HEAD | cut -c1-8',
                            returnStdout: true
                        ).trim()
                        echo "DEBUG - Git hash: '${gitHash}'"

                        // Sync package.json version
                        dir('app/elohim-app') {
                            sh "npm version '${baseVersion}' --no-git-tag-version"
                        }

                        // Sanitize branch name for Docker tag (replace / with -)
                        def sanitizedBranch = env.BRANCH_NAME.replaceAll('/', '-')
                        echo "DEBUG - Sanitized branch: '${sanitizedBranch}'"

                        // Create image tag
                        def imageTag = (env.BRANCH_NAME == 'main')
                            ? baseVersion
                            : "${baseVersion}-${sanitizedBranch}-${gitHash}"

                        echo "DEBUG - Image tag: '${imageTag}'"

                        // Write build.env file
                        def buildEnvContent = """BASE_VERSION=${baseVersion}
GIT_COMMIT_HASH=${gitHash}
IMAGE_TAG=${imageTag}
BRANCH_NAME=${env.BRANCH_NAME}"""

                        writeFile file: "${env.WORKSPACE}/build.env", text: buildEnvContent

                        // Verify file was written
                        sh "cat '${env.WORKSPACE}/build.env'"

                        // Archive for debugging
                        archiveArtifacts artifacts: 'build.env', allowEmptyArchive: false
                        
                        echo "Build variables persisted to build.env"
                    }
                }
            }
        }
        
        stage('Fetch WASM Cache Core') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder') {
                    script {
                        echo 'Fetching elohim-cache-core WASM module from Harbor...'

                        def wasmDir = 'elohim/elohim-cache-core/pkg'

                        // Read HAPP_VERSION from VERSION file
                        def versionContent = readFile('VERSION').trim()
                        def versionMap = [:]
                        versionContent.split('\n').each { line ->
                            def parts = line.split('=')
                            if (parts.length == 2) {
                                versionMap[parts[0].trim()] = parts[1].trim()
                            }
                        }
                        def baseVersion = versionMap['HAPP_VERSION'] ?: versionMap['APP_VERSION'] ?: '1.0.0'

                        // Compute Harbor tag using same logic as DNA pipeline producer
                        def happVersion
                        if (env.BRANCH_NAME == 'main') {
                            happVersion = baseVersion
                        } else {
                            def gitHash = sh(script: 'git rev-parse HEAD | cut -c1-8', returnStdout: true).trim()
                            def sanitizedBranch = env.BRANCH_NAME.replaceAll('/', '-')
                            happVersion = "${baseVersion}-${sanitizedBranch}-${gitHash}"
                        }

                        echo "Using HAPP_VERSION: ${happVersion}"

                        // Install oras CLI if not present
                        sh '''
                            if ! command -v oras &> /dev/null; then
                                echo "Installing oras CLI..."
                                curl -sLO https://github.com/oras-project/oras/releases/download/v1.1.0/oras_1.1.0_linux_amd64.tar.gz
                                tar -xzf oras_1.1.0_linux_amd64.tar.gz
                                chmod +x oras
                                mv oras /usr/local/bin/
                                rm oras_1.1.0_linux_amd64.tar.gz
                            fi
                        '''

                        // Fetch WASM from Harbor
                        def fetched = false
                        withCredentials([usernamePassword(
                            credentialsId: 'harbor-robot-registry',
                            usernameVariable: 'HARBOR_USER',
                            passwordVariable: 'HARBOR_PASS'
                        )]) {
                            def result = sh(script: """
                                oras login harbor.ethosengine.com -u \$HARBOR_USER -p \$HARBOR_PASS
                                mkdir -p '${wasmDir}'
                                cd '${wasmDir}'
                                oras pull harbor.ethosengine.com/ethosengine/elohim-wasm-cache-core:${happVersion}
                            """, returnStatus: true)
                            fetched = (result == 0)
                        }

                        if (!fetched) {
                            echo """
                            ⚠️ Could not fetch elohim-cache-core WASM from Harbor.
                            App will use TypeScript fallback (slightly slower but functional).
                            To enable WASM: Run holochain DNA pipeline to push artifacts to Harbor.
                            """
                            // TODO: Make WASM deployment more reliable for alpha/staging.
                            // The 404 on /wasm/elohim-cache-core/elohim_cache_core.js
                            // is harmless (TS fallback works) but creates console noise that
                            // obscures real errors and mismatches production expectations.
                            // Options: (1) pre-seed Harbor with a known-good WASM artifact,
                            // (2) suppress the fetch in the browser when WASM isn't bundled,
                            // (3) make DNA pipeline a dependency of alpha deploys.
                        }

                        if (fileExists("${wasmDir}/elohim_cache_core.js")) {
                            echo "✅ elohim-cache-core WASM module ready"
                            sh "ls -lh ${wasmDir}/"
                        } else {
                            echo "⚠️ WASM module not available - TypeScript fallback will be used"
                        }
                    }
                }
            }
        }

        stage('Install Dependencies') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder'){
                    dir('app/elohim-library') {
                        script {
                            echo 'Installing elohim-library dependencies (required for elohim-service imports)'
                            sh 'pnpm install --frozen-lockfile'
                        }
                    }
                    dir('elohim/sdk/storage-client-ts') {
                        script {
                            echo 'Building storage-client-ts (required for @elohim/storage-client/generated types)'
                            sh 'pnpm install --frozen-lockfile && pnpm run build'
                            sh 'ls -la dist/ dist/generated/'
                            // Publish to Nexus so downstream pipelines (genesis) can resolve without workspace root
                            // Only publish if this version doesn't already exist (prevents tarball overwrite + integrity mismatch)
                            sh '''#!/bin/bash
                                set -euo pipefail
                                PKG_VERSION=$(node -p "require('./package.json').version")
                                echo "Checking if @elohim/storage-client@${PKG_VERSION} exists on Nexus..."
                                if npm view "@elohim/storage-client@${PKG_VERSION}" --registry=https://nexus.ethosengine.com/repository/npm/ version 2>/dev/null; then
                                    echo "ℹ️  @elohim/storage-client@${PKG_VERSION} already published, skipping"
                                else
                                    echo "Publishing @elohim/storage-client@${PKG_VERSION} to Nexus..."
                                    pnpm publish --no-git-checks
                                    echo "✅ Published @elohim/storage-client@${PKG_VERSION}"
                                fi
                            '''
                        }
                    }
                    dir('app/elohim-app') {
                        script {
                            echo 'Installing pnpm dependencies'
                            sh 'pnpm install --frozen-lockfile'

                            // Copy WASM files from fetched location to node_modules
                            // This is needed because Angular expects WASM in node_modules/elohim-cache-core
                            def wasmSrc = '../../elohim/elohim-cache-core/pkg'
                            def wasmDest = 'node_modules/elohim-cache-core'
                            if (fileExists(wasmSrc)) {
                                echo 'Copying elohim-cache-core WASM to node_modules...'
                                sh """
                                    mkdir -p '${wasmDest}'
                                    cp -v '${wasmSrc}'/*.js '${wasmDest}/' 2>/dev/null || true
                                    cp -v '${wasmSrc}'/*.wasm '${wasmDest}/' 2>/dev/null || true
                                    cp -v '${wasmSrc}'/*.ts '${wasmDest}/' 2>/dev/null || true
                                    ls -la '${wasmDest}/' || true
                                """
                            } else {
                                echo "⚠️ WASM source not found at ${wasmSrc} - TypeScript fallback will be used"
                            }
                        }
                    }
                }
            }
        }

        stage('Build Sophia Plugin') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    expression { shouldRunStep('build-sophia-umd') }
                }
            }
            steps { container('builder') { script { buildSophiaPlugin() } } }
        }

        stage('Build Elohim Core') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    expression { shouldRunStep('build-angular') }
                }
            }
            steps {
                container('builder') {
                    sh '''#!/bin/bash
                        set -euo pipefail
                        echo "Building elohim-core (vite library + custom-elements-manifest)..."
                        pnpm --filter elohim-core run build

                        if [ ! -f app/elohim-elements/elohim-core/dist/register.js ]; then
                            echo "ERROR: elohim-core/dist/register.js missing after build"
                            exit 1
                        fi
                        echo "elohim-core build OK"

                        echo "Building elohim-imagodei (vite library + custom-elements-manifest)..."
                        pnpm --filter elohim-imagodei run build

                        if [ ! -f app/elohim-elements/elohim-imagodei/dist/register.js ]; then
                            echo "ERROR: elohim-imagodei/dist/register.js missing after build"
                            exit 1
                        fi
                        echo "elohim-imagodei build OK"

                        echo "Building elohim-qahal (vite library + custom-elements-manifest)..."
                        pnpm --filter elohim-qahal run build

                        if [ ! -f app/elohim-elements/elohim-qahal/dist/register.js ]; then
                            echo "ERROR: elohim-qahal/dist/register.js missing after build"
                            exit 1
                        fi
                        echo "elohim-qahal build OK"
                    '''
                }
            }
        }

        stage('Build App') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    expression { shouldRunStep('build-angular') }
                }
            }
            steps {
                container('builder'){
                    dir('app/elohim-app') {
                        script {
                            def props = loadBuildVars()

                            withBuildVars(props) {
                                echo 'Building Angular application'
                                echo "Using git hash: ${GIT_COMMIT_HASH}"
                                echo "Using image tag: ${IMAGE_TAG}"

                                // Replace placeholders
                                sh """
                                    sed -i "s/GIT_HASH_PLACEHOLDER/${GIT_COMMIT_HASH}/g" src/environments/environment.prod.ts
                                    sed -i "s/GIT_HASH_PLACEHOLDER/${GIT_COMMIT_HASH}/g" src/environments/environment.staging.ts
                                    sed -i "s/GIT_HASH_PLACEHOLDER/${GIT_COMMIT_HASH}/g" src/environments/environment.alpha.ts
                                """

                                // Determine build configuration based on branch
                                // For PR builds, CHANGE_TARGET contains the target branch (e.g., 'dev')
                                // For direct branch builds, use BRANCH_NAME
                                def targetBranch = env.CHANGE_TARGET ?: env.BRANCH_NAME
                                def sourceBranch = env.CHANGE_BRANCH ?: env.BRANCH_NAME

                                def buildConfig = 'production'
                                if (targetBranch == 'staging' || targetBranch ==~ /staging-.+/ ||
                                    sourceBranch == 'staging' || sourceBranch ==~ /staging-.+/) {
                                    buildConfig = 'staging'
                                } else if (targetBranch == 'dev' || targetBranch ==~ /feat-.+/ || targetBranch ==~ /claude\/.+/ || targetBranch.contains('alpha') ||
                                           sourceBranch == 'dev' || sourceBranch ==~ /feat-.+/ || sourceBranch ==~ /claude\/.+/ || sourceBranch.contains('alpha')) {
                                    buildConfig = 'alpha'
                                }

                                echo "Building with configuration: ${buildConfig} (target: ${targetBranch}, source: ${sourceBranch})"
                                sh 'bash scripts/fetch-fonts.sh'
                                // Compile Service Worker (esbuild bundles JSZip into IIFE)
                                // Must run before ng build so the compiled SW is in src/assets/
                                sh 'pnpm run build:sw'
                                sh "pnpm exec ng build --configuration=${buildConfig}"

                                // Generate version.json for deployment verification
                                sh """
cat > dist/elohim-app/browser/version.json << VEOF
{
  "commit": "${GIT_COMMIT_HASH}",
  "version": "${BASE_VERSION}",
  "buildTime": "\$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "environment": "${buildConfig}",
  "service": "elohim-app"
}
VEOF
"""
                                sh 'ls -la dist/'
                            }
                        }
                    }
                }
            }
        }

        stage('Build Lamad Bundle') {
            // Pillar-EPR decomposition (Task B21): app/lamad is the lamad
            // pillar SPA, served as its own EPR at /lamad/... by doorway.
            // Runs AFTER Build App because lamad's tsconfig path aliases
            // reference app/elohim-app codegen artifacts (B18). One ng build
            // emits BOTH dist/lamad/browser (the shell) and dist/lamad/server
            // (the SSR bundle stageSpaBlobs seeds as lamad-spa serverBlobHash
            // — the peer-runtime render source; entry contract gated by
            // app/scripts/lint-ssr-entry.mjs).
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    expression { shouldRunStep('build-angular') }
                }
            }
            steps {
                container('builder') {
                    dir('app/lamad') {
                        sh 'pnpm run build'
                        sh 'ls -la dist/lamad/browser/ | head -20'
                        sh 'ls -la dist/lamad/server/ | head -20'
                    }
                }
            }
        }

        stage('Unit Test') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder'){
                    dir('app/elohim-app') {
                        script {
                            echo 'Running Angular tests with coverage (Vitest)'
                            sh 'pnpm exec vitest run --config vite.config.ts --coverage'
                        }
                    }
                }
            }
        }

        stage('SonarQube Analysis') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    anyOf {
                        branch 'main'
                        branch 'staging'
                        branch 'dev'
                        expression { env.BRANCH_NAME ==~ /staging-.+/ }
                        expression { env.BRANCH_NAME ==~ /feat-.+/ }
                        expression { env.BRANCH_NAME ==~ /claude\/.+/ }
                        changeRequest target: 'main'
                        changeRequest target: 'staging'
                        changeRequest target: 'dev'
                    }
                }
            }
            steps {
                container('builder'){
                    dir('app/elohim-app') {
                        script {
                            def sonarConfig = getSonarProjectConfig()
                            echo "SonarQube Analysis: project=${sonarConfig.projectKey}, env=${sonarConfig.env}, enforce=${sonarConfig.shouldEnforce}"

                            def scannerFailed = false
                            try {
                                withSonarQubeEnv('ee-sonarqube') {
                                    sh """
                                    sonar-scanner \
                                        -Dsonar.projectKey=${sonarConfig.projectKey} \
                                        -Dsonar.sources=src \
                                        -Dsonar.tests=src \
                                        -Dsonar.test.inclusions=**/*.spec.ts \
                                        -Dsonar.typescript.lcov.reportPaths=coverage/vitest/lcov.info \
                                        -Dsonar.javascript.lcov.reportPaths=coverage/vitest/lcov.info \
                                        -Dsonar.coverage.exclusions=**/*.module.ts,**/*-routing.module.ts,**/*.model.ts,**/models/**,**/environments/**,**/main.ts,**/polyfills.ts,**/*.spec.ts,**/index.ts,**/components/**,**/renderers/**,**/content-io/**,**/guards/**,**/interceptors/**,**/pipes/**,**/directives/**,**/parsers/**,**/*.routes.ts \
                                        -Dsonar.qualitygate.wait=false
                                    """
                                }
                            } catch (Exception scannerErr) {
                                scannerFailed = true
                                echo "⚠️ SonarQube scanner CLI failed: ${scannerErr.message}"
                                echo "Most common cause: SonarQube server transient outage (HTTP 5xx) at ${env.SONAR_HOST_URL}."
                                if (sonarConfig.shouldEnforce) {
                                    error "❌ SonarQube scanner failed and quality gate is enforced for this branch — failing build."
                                } else {
                                    echo "Alpha/Staging: marking build UNSTABLE and skipping quality-gate wait."
                                    currentBuild.result = 'UNSTABLE'
                                }
                            }

                            if (scannerFailed) {
                                echo "Skipping waitForQualityGate (no analysis was uploaded)."
                                return
                            }

                            echo "Waiting for SonarQube quality gate..."
                            try {
                                timeout(time: 10, unit: 'MINUTES') {
                                    def qg = waitForQualityGate abortPipeline: false
                                    if (qg.status != 'OK') {
                                        if (sonarConfig.shouldEnforce) {
                                            // Production: Block deployment on quality gate failure
                                            error "❌ SonarQube Quality Gate FAILED: ${qg.status}\nReview issues at: ${env.SONAR_HOST_URL}/dashboard?id=${sonarConfig.projectKey}"
                                        } else {
                                            // Alpha/Staging: Log warning but don't block
                                            echo "⚠️ SonarQube Quality Gate status: ${qg.status}"
                                            echo "Review issues at: ${env.SONAR_HOST_URL}/dashboard?id=${sonarConfig.projectKey}"
                                            currentBuild.result = 'UNSTABLE'
                                        }
                                    } else {
                                        echo "✅ SonarQube quality gate passed (${sonarConfig.env})"
                                    }
                                }
                            } catch (Exception e) {
                                echo "⚠️ SonarQube quality gate check failed: ${e.message}"
                                echo "This may be due to webhook configuration issues."
                                echo "Review results at: ${env.SONAR_HOST_URL}/dashboard?id=${sonarConfig.projectKey}"
                                if (sonarConfig.shouldEnforce) {
                                    currentBuild.result = 'UNSTABLE'
                                    echo "⚠️ Marking build UNSTABLE - production quality gate could not be verified"
                                } else {
                                    echo "Continuing pipeline..."
                                }
                            }
                        }
                    }
                }
            }
        }

        stage('Apply Ingress (pre-upload)') {
            // Apply the per-env ingress manifest BEFORE Upload SPA Blob.
            //
            // The ingress carries `nginx.ingress.kubernetes.io/proxy-body-size`,
            // which nginx-ingress's default (1 MB) violates for the ~10 MB SPA
            // blob PUT that stageSpaBlobs does. Without this stage, the App
            // pipeline can't bootstrap: stageSpaBlobs runs at line ~865 and
            // hits 413; the Deploy to Alpha stage that would normally apply
            // the ingress doesn't run until line ~1126, after stageSpaBlobs.
            //
            // Symptom this stage fixes: App #1460 (and prior) 413 on PUT.
            // Bootstrap chicken-and-egg first written into the ingress
            // manifest comment by 984f6b0e7; this stage closes the loop.
            //
            // kubectl apply is idempotent — the later Deploy to Alpha stage
            // still applies the full sibling-manifest set (configmap, service,
            // ingress) via deployAppToEnvironment. Applying the ingress twice
            // is a no-op.
            //
            // Scope: alpha (dev/feat/claude branches), staging, prod.
            // Each env has its own genesis/orchestrator/manifests/elohim-app/<env>/ingress.yaml.
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    expression { shouldRunStep('build-angular') }
                }
            }
            steps {
                container('builder') {
                    script {
                        def branch = env.BRANCH_NAME ?: 'dev'
                        def env_target
                        if (branch == 'main') {
                            env_target = 'prod'
                        } else if (branch == 'staging' || branch.startsWith('staging-')) {
                            env_target = 'staging'
                        } else {
                            env_target = 'alpha'
                        }
                        def ingressPath = "genesis/orchestrator/manifests/elohim-app/${env_target}/ingress.yaml"
                        echo "Applying ${env_target} ingress (pre-upload) from ${ingressPath}"
                        sh "kubectl apply -f ${ingressPath}"
                    }
                }
            }
        }

        stage('Upload SPA Blob') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    expression { shouldRunStep('build-angular') }
                }
            }
            steps {
                container('builder') {
                    script {
                        // Body lives in three top-level helpers (resolveDoorwayEprUrls /
                        // resolveStorageAdminKey / stageAndVerifyAllBundles) — this block
                        // grew 9,034 → 9,898 bytes with the seatbelt call-sites and broke
                        // the CPS 64KB method limit (___cps___7636, #1519–#1521). Keep it
                        // thin; rationale comments moved with their logic.
                        def doorwayEprUrls = resolveDoorwayEprUrls()
                        echo "stageSpaBlobs doorwayEprUrls: ${doorwayEprUrls}"
                        def adminKey = resolveStorageAdminKey()
                        // gitCommitHash: only for verifyProjectedHeads' cheap, non-gating
                        // browser-bundle /version.json liveness signal (Track-4 T4-2).
                        def buildProps = loadBuildVars()
                        def gitCommitHash = buildProps.GIT_COMMIT_HASH ?: ''
                        stageAndVerifyAllBundles(doorwayEprUrls, adminKey, gitCommitHash)
                    }
                }
            }
        }

        stage('Build Image') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    expression { shouldRunStep('build-site-image') }
                }
            }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()
                        if (!props.IMAGE_TAG || !props.GIT_COMMIT_HASH || !props.BASE_VERSION) {
                            error "Missing required build variables"
                        }
                        
                        withBuildVars(props) {
                            echo 'Building container image'
                            echo "Image tag: ${IMAGE_TAG}"
                            echo "Git hash: ${GIT_COMMIT_HASH}"
                            
                            sh """#!/bin/bash
                                set -euo pipefail

                                # Verify BuildKit
                                buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null

                                # Create build context
                                mkdir -p /tmp/build-context
                                cp -r app/elohim-app /tmp/build-context/
                                cp app/elohim-app/images/Dockerfile /tmp/build-context/
                                cp app/elohim-app/images/nginx.conf /tmp/build-context/
                                
                                # Build image
                                cd /tmp/build-context
                                BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \\
                                  nerdctl -n k8s.io build -t elohim-app:${IMAGE_TAG} -f Dockerfile .

                                # Additional tags
                                nerdctl -n k8s.io tag elohim-app:${IMAGE_TAG} elohim-app:${GIT_COMMIT_HASH}
                                
                                if [ "${BRANCH_NAME}" = "main" ]; then
                                    nerdctl -n k8s.io tag elohim-app:${IMAGE_TAG} elohim-app:latest
                                fi
                            """
                            
                            // Mark build as completed
                            env.DOCKER_BUILD_COMPLETED = 'true'
                            echo 'Container image built successfully'
                        }
                    }
                }
            }
        }

        stage('Push to Harbor Registry') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()
                        withBuildVars(props) {
                            withCredentials([usernamePassword(credentialsId: 'harbor-robot-registry', passwordVariable: 'HARBOR_PASSWORD', usernameVariable: 'HARBOR_USERNAME')]) {
                                echo 'Logging into Harbor registry'
                                sh 'echo $HARBOR_PASSWORD | nerdctl -n k8s.io login harbor.ethosengine.com -u $HARBOR_USERNAME --password-stdin'
                                
                                echo "Tagging and pushing image: ${IMAGE_TAG}"
                                sh """
                                    nerdctl -n k8s.io tag elohim-app:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-site:${IMAGE_TAG}
                                    nerdctl -n k8s.io tag elohim-app:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-site:${GIT_COMMIT_HASH}
                                    
                                    nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-site:${IMAGE_TAG}
                                    nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-site:${GIT_COMMIT_HASH}
                                """
                                
                                if (env.BRANCH_NAME == 'main') {
                                    sh """
                                        nerdctl -n k8s.io tag elohim-app:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-site:latest
                                        nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-site:latest
                                    """
                                }
                                
                                echo 'Successfully pushed to Harbor registry'
                            }
                        }
                    }
                }
            }
        }

        stage('Harbor Security Scan') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()
                        withBuildVars(props) {
                            withCredentials([usernamePassword(credentialsId: 'harbor-robot-registry', passwordVariable: 'HARBOR_PASSWORD', usernameVariable: 'HARBOR_USERNAME')]) {
                                echo "Triggering Harbor scan for: ${IMAGE_TAG}"

                                sh """
                                    AUTH_HEADER="Authorization: Basic \$(echo -n "\$HARBOR_USERNAME:\$HARBOR_PASSWORD" | base64)"

                                    wget --post-data="" \\
                                      --header="accept: application/json" \\
                                      --header="Content-Type: application/json" \\
                                      --header="\$AUTH_HEADER" \\
                                      -S -O- \\
                                      "https://harbor.ethosengine.com/api/v2.0/projects/ethosengine/repositories/elohim-site/artifacts/${IMAGE_TAG}/scan" || \\
                                    echo "Scan request failed"
                                """

                                echo 'Scan initiated, polling for completion...'

                                sh """#!/bin/bash
                                    AUTH_HEADER="Authorization: Basic \$(echo -n "\$HARBOR_USERNAME:\$HARBOR_PASSWORD" | base64)"
                                    MAX_ATTEMPTS=24
                                    ATTEMPT=1

                                    while [ \$ATTEMPT -le \$MAX_ATTEMPTS ]; do
                                        VULN_DATA=\$(wget -q -O- \\
                                          --header="accept: application/json" \\
                                          --header="\$AUTH_HEADER" \\
                                          "https://harbor.ethosengine.com/api/v2.0/projects/ethosengine/repositories/elohim-site/artifacts/${IMAGE_TAG}/additions/vulnerabilities" 2>/dev/null || echo "")

                                        if [ ! -z "\$VULN_DATA" ] && echo "\$VULN_DATA" | grep -q '"scanner"'; then
                                            echo "✅ Scan completed"
                                            break
                                        fi

                                        [ \$((ATTEMPT % 5)) -eq 0 ] && echo "Waiting for scan (attempt \$ATTEMPT/\$MAX_ATTEMPTS)..."
                                        sleep 10
                                        ATTEMPT=\$((ATTEMPT + 1))
                                    done
                                """
                            }
                        }
                    }
                }
            }
        }

        // Note: Holochain infrastructure is built by elohim-edge pipeline,
        // triggered by the orchestrator when holochain source files change.

        stage('Deploy to Staging') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    anyOf { branch 'staging'; expression { env.BRANCH_NAME ==~ /staging-.+/ } }
                }
            }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()
                        withBuildVars(props) {
                            deployAppToEnvironment('staging', 'elohim-staging', 'elohim-site-staging',
                                'genesis/orchestrator/manifests/elohim-app/staging.yaml', IMAGE_TAG)
                        }
                    }
                }
            }
        }

        stage('🚀 Deploy to Alpha') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    expression { env.BRANCH_NAME == 'dev' || env.BRANCH_NAME ==~ /feat-.+/ || env.BRANCH_NAME ==~ /claude\/.+/ }
                }
            }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()
                        withBuildVars(props) {
                            echo """
                            ═══════════════════════════════════════════════════════════
                            🚀 DEPLOYING ELOHIM APP TO ALPHA
                            ═══════════════════════════════════════════════════════════
                            Image Tag: ${IMAGE_TAG}
                            Git Hash: ${GIT_COMMIT_HASH}
                            Target: https://alpha.elohim.host
                            ═══════════════════════════════════════════════════════════
                            """

                            deployAppToEnvironment('alpha', 'elohim-alpha', 'elohim-site-alpha',
                                'genesis/orchestrator/manifests/elohim-app/alpha.yaml', IMAGE_TAG)

                            echo """
                            ═══════════════════════════════════════════════════════════
                            ✅ ALPHA DEPLOYMENT COMPLETE
                            ═══════════════════════════════════════════════════════════
                            App URL: https://alpha.elohim.host
                            Image: ${IMAGE_TAG}
                            ═══════════════════════════════════════════════════════════
                            """
                        }
                    }
                }
            }
        }

        stage('Verify Holochain Health') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    expression { env.BRANCH_NAME == 'dev' || env.BRANCH_NAME ==~ /feat-.+/ || env.BRANCH_NAME ==~ /claude\/.+/ }
                }
            }
            steps {
                container('builder'){
                    script {
                        echo """
                        ═══════════════════════════════════════════════════════════
                        VERIFYING HOLOCHAIN INFRASTRUCTURE
                        ═══════════════════════════════════════════════════════════
                        Alpha app uses: doorway-alpha.elohim.host
                        Seeding is managed by: elohim-genesis pipeline
                        ═══════════════════════════════════════════════════════════
                        """

                        // Check if holochain edge node is running
                        def holochainStatus = sh(
                            script: '''
                                kubectl get deployment elohim-edgenode-alpha -n elohim-alpha -o jsonpath='{.status.availableReplicas}' 2>/dev/null || echo "0"
                            ''',
                            returnStdout: true
                        ).trim()

                        if (holochainStatus == "1") {
                            echo "Holochain Edge Node: Running"
                        } else {
                            echo "Holochain Edge Node: Not available (${holochainStatus} replicas)"
                            echo "Run elohim-edge pipeline with FORCE_DEPLOY=true"
                        }

                        // Check holochain connectivity with retry
                        def holochainHealth = "000"
                        for (int i = 0; i < 3; i++) {
                            holochainHealth = sh(
                                script: '''
                                    timeout 10s curl -sf -o /dev/null -w "%{http_code}" https://doorway-alpha.elohim.host/health 2>/dev/null || echo "000"
                                ''',
                                returnStdout: true
                            ).trim()

                            if (holochainHealth == "200") break
                            if (i < 2) {
                                echo "Health check attempt ${i+1} failed, retrying..."
                                sleep 5
                            }
                        }

                        if (holochainHealth == "200") {
                            echo "Holochain Gateway: Healthy"
                        } else {
                            echo "Holochain Gateway: Unhealthy (HTTP ${holochainHealth})"
                            echo "App will work but holochain features may be unavailable"
                            echo "Run elohim-edge pipeline with FORCE_DEPLOY=true to fix"
                        }

                        echo """
                        ═══════════════════════════════════════════════════════════
                        HOLOCHAIN STATUS
                        ═══════════════════════════════════════════════════════════
                        Edge Node: ${holochainStatus == "1" ? "Running" : "Unavailable"}
                        Gateway: ${holochainHealth == "200" ? "Healthy" : "Unhealthy"}

                        Note: Database seeding is managed by elohim-genesis pipeline.
                        To force seed, run that pipeline with FORCE_SEED=true.
                        ═══════════════════════════════════════════════════════════
                        """
                    }
                }
            }
        }

        stage('E2E Testing - Alpha Validation') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    expression { env.BRANCH_NAME == 'dev' || env.BRANCH_NAME ==~ /feat-.+/ || env.BRANCH_NAME ==~ /claude\/.+/ }
                }
            }
            steps {
                container('builder'){
                    dir('app/elohim-app') {
                        script {
                            def props = loadBuildVars()
                            withBuildVars(props) {
                                // Advisory by design. This E2E runs against the LIVE
                                // alpha target (runE2ETests opens with a `timeout 60s
                                // curl https://alpha.elohim.host`). A down/flapping
                                // alpha must NOT drive this build to FAILURE — a Level-0
                                // FAILURE trips the orchestrator's fail-fast abort
                                // (genesis/orchestrator/Jenkinsfile ~1807), which then
                                // never dispatches elohim-edge (Level 1+), the ONLY
                                // pipeline that runs `kubectl apply`. That deadlocks the
                                // deploy that would FIX alpha behind alpha being up
                                // (observed: orchestrator #1240, 2026-06-13). UNSTABLE is
                                // treated as success by triggerPipeline (success ==
                                // result in [SUCCESS, UNSTABLE]) so the cascade proceeds
                                // and edge deploys; E2E results still publish below. This
                                // mirrors the orchestrator's own post-flight/P2P/fed-smoke
                                // gates, which are all catchError -> UNSTABLE. App
                                // build/compile/Sonar failures upstream still hard-gate.
                                catchError(buildResult: 'UNSTABLE', stageResult: 'UNSTABLE') {
                                    runE2ETests('alpha', 'https://alpha.elohim.host', env.GIT_COMMIT_HASH)
                                }
                            }
                        }
                    }
                }
            }
            post {
                success {
                    echo '✅ E2E tests passed - alpha validation successful!'
                }
                always {
                    dir('app/elohim-app') {
                        script {
                            publishE2EReports('alpha')
                        }
                    }
                }
                failure {
                    echo '❌ E2E tests failed - alpha deployment validation unsuccessful'
                    echo 'Check test artifacts and logs for details'
                }
            }
        }

        stage('E2E Testing - Staging Validation') {
            when {
                allOf {
                    expression { env.PIPELINE_SKIPPED != 'true' }
                    anyOf { branch 'staging'; expression { env.BRANCH_NAME ==~ /staging-.+/ } }
                }
            }
            steps {
                container('builder'){
                    dir('app/elohim-app') {
                        script {
                            def props = loadBuildVars()
                            withBuildVars(props) {
                                runE2ETests('staging', 'https://staging.elohim.host', env.GIT_COMMIT_HASH)
                            }
                        }
                    }
                }
            }
            post {
                success {
                    echo '✅ E2E tests passed - staging validation successful!'
                }
                always {
                    dir('app/elohim-app') {
                        script {
                            publishE2EReports('staging')
                        }
                    }
                }
                failure {
                    echo '❌ E2E tests failed - staging deployment validation unsuccessful'
                    echo 'Check test artifacts and logs for details'
                }
            }
        }

        stage('Deploy to Prod') {
            when { allOf { expression { env.PIPELINE_SKIPPED != 'true' }; branch 'main' } }
            steps {
                container('builder'){
                    script {
                        def props = loadBuildVars()
                        withBuildVars(props) {
                            deployAppToEnvironment('prod', 'elohim-prod', 'elohim-site',
                                'genesis/orchestrator/manifests/elohim-app/prod.yaml', IMAGE_TAG)
                        }
                    }
                }
            }
        }

        stage('Cleanup') {
            when { expression { env.PIPELINE_SKIPPED != 'true' } }
            steps {
                container('builder'){
                    script {
                        dir('app/elohim-app') { sh 'rm -rf node_modules || true' }
                    }
                }
            }
        }
    }

    post {
        success {
            script {
                try {
                    container('builder') {
                        def props = loadBuildVars()
                        echo "Pipeline completed successfully"
                        echo "Docker image: elohim-app:${props.IMAGE_TAG}"
                        echo "Git hash: ${props.GIT_COMMIT_HASH}"
                        echo "Base version: ${props.BASE_VERSION}"
                        echo "Branch: ${props.BRANCH_NAME}"
                    }
                } catch (Exception e) {
                    echo "Pipeline completed successfully"
                }
            }
        }
        failure {
            echo 'Pipeline failed. Check the logs for details.'
        }
        always {
            script {
                if (env.DOCKER_BUILD_COMPLETED == 'true') {
                    try {
                        container('builder') {
                            def props = loadBuildVars()
                            withBuildVars(props) {
                                echo 'Cleaning up Docker images...'
                                sh """
                                    nerdctl -n k8s.io rmi elohim-app:${IMAGE_TAG} || true
                                    nerdctl -n k8s.io rmi elohim-app:${GIT_COMMIT_HASH} || true
                                    nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-site:${IMAGE_TAG} || true
                                    nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-site:${GIT_COMMIT_HASH} || true
                                """
                                if (env.BRANCH_NAME == 'main') {
                                    sh """
                                        nerdctl -n k8s.io rmi elohim-app:latest || true
                                        nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-site:latest || true
                                    """
                                }
                                sh "nerdctl -n k8s.io system prune -af --volumes || true"
                            }
                        }
                    } catch (Exception e) {
                        echo "Cleanup failed: ${e.message}"
                    }
                } else {
                    echo 'Build not completed, skipping cleanup.'
                }
            }
        }
    }
}
