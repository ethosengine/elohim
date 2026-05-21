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

def stageSpaBlob(String storageUrl, String distDir) {
    sh """
        cd '${distDir}'
        zip -r lamad-spa.zip .
        SPA_HASH=\$(sha256sum lamad-spa.zip | awk '{print \$1}')
        echo "SPA blob hash: \${SPA_HASH}"
        echo "SPA blob size: \$(du -h lamad-spa.zip | cut -f1)"

        # Upload ZIP as blob to storage
        curl -f -X PUT \
            -H 'Content-Type: application/zip' \
            --data-binary @lamad-spa.zip \
            "${storageUrl}/blob/\${SPA_HASH}" \
            || echo 'WARNING: Blob upload failed (storage may not be reachable)'

        # Update content node with new blobHash
        curl -f -X PUT \
            -H 'Content-Type: application/json' \
            -d '{"blobHash":"'\${SPA_HASH}'"}' \
            "${storageUrl}/db/content/lamad-spa" \
            || echo 'WARNING: Content node update failed'

        rm -f lamad-spa.zip
    """
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
 nodeSelector:
    node-type: edge
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
   command:
   - cat
   tty: true
   resources:
     requests:
       ephemeral-storage: "2Gi"
     limits:
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
                                sh 'bash ../../scripts/fetch-fonts.sh'
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
                        def storageUrl = env.STORAGE_URL ?: 'http://elohim-matthew-alpha-0.elohim-matthew-alpha-headless:8090'
                        stageSpaBlob(storageUrl, "${env.WORKSPACE}/app/elohim-app/dist/elohim-app/browser")
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
                                runE2ETests('alpha', 'https://alpha.elohim.host', env.GIT_COMMIT_HASH)
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
