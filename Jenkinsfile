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
    node-type: operations
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
       ephemeral-storage: "1Gi"
     limits:
       ephemeral-storage: "2.5Gi"
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
        // Initialize version variables
        GIT_COMMIT_HASH = ''
        BRANCH_NAME = "${env.BRANCH_NAME ?: 'main'}"
        BASE_VERSION = ''
        IMAGE_TAG = ''
    }
    
    stages {

        stage('Checkout') {
            when {
                anyOf {
                    branch 'main'
                    branch 'dev'
                    branch 'review-*'
                    branch 'feat-*'
                    changeRequest()
                }
            }
            steps {
                container('builder'){
                    script {
                        // Multibranch SCM checkout - supports filtered branches and PRs
                        checkout scm
                        
                        // Validate that we're building an allowed branch
                        echo "Building branch: ${env.BRANCH_NAME}"
                        echo "Change request: ${env.CHANGE_ID ?: 'None'}"
                    }
                }
            }
        }

        stage('Setup Version') {
            steps {
                container('builder'){
                    script {
                        // Fix git safe.directory issue for Jenkins workspace
                        sh 'git config --global --add safe.directory $(pwd)'
                        
                        // Read version from root VERSION file
                        def baseVersion = readFile('VERSION').trim()
                        env.BASE_VERSION = baseVersion
                        
                        // Get the git commit hash
                        env.GIT_COMMIT_HASH = sh(
                            script: 'git rev-parse --short HEAD',
                            returnStdout: true
                        ).trim()
                        
                        echo "DEBUG: Read base version: '${baseVersion}'"
                        echo "DEBUG: ENV base version: '${env.BASE_VERSION}'"
                        
                        // Sync package.json version for build artifacts only (don't commit back)
                        dir('elohim-app') {
                            sh "npm version '${baseVersion}' --no-git-tag-version"
                        }
                        
                        // Create comprehensive image tag
                        if (env.BRANCH_NAME == 'main') {
                            env.IMAGE_TAG = "${baseVersion}"
                        } else {
                            env.IMAGE_TAG = "${baseVersion}-${env.BRANCH_NAME}-${env.GIT_COMMIT_HASH}"
                        }
                        
                        echo "Branch: ${env.BRANCH_NAME}"
                        echo "Git Commit: ${env.GIT_COMMIT_HASH}"
                        echo "Base Version: ${env.BASE_VERSION}"
                        echo "Image Tag: ${env.IMAGE_TAG}"
                    }
                }
            }
        }
        
        stage('Install Dependencies') {
            steps {
                container('builder'){
                    dir('elohim-app') {
                        script {
                            echo 'Installing npm dependencies'
                            sh 'npm ci'
                        }
                    }
                }
            }
        }
        
        stage('Build App') {
            steps {
                container('builder'){
                    dir('elohim-app') {
                        script {
                            echo 'Building Angular application'
                            sh 'npm run build'
                            echo 'Build output:'
                            sh 'ls -la dist/'
                        }
                    }
                }
            }
        }

        stage('Unit Test') {
            steps {
                container('builder'){
                    dir('elohim-app') {
                        script {
                            echo 'Running Angular tests with coverage'
                            sh 'npm run test -- --watch=false --browsers=ChromeHeadless --code-coverage'
                        }
                    }
                }
            }
        }

        stage('SonarQube Analysis') {
            steps {
                container('builder'){
                    dir('elohim-app') {
                        script {
                            withSonarQubeEnv('ee-sonarqube') {
                                sh '''
                                sonar-scanner \
                                    -Dsonar.projectKey=elohim-app \
                                    -Dsonar.sources=src \
                                    -Dsonar.tests=src \
                                    -Dsonar.test.inclusions=**/*.spec.ts \
                                    -Dsonar.typescript.lcov.reportPaths=coverage/elohim-app/lcov.info \
                                    -Dsonar.javascript.lcov.reportPaths=coverage/elohim-app/lcov.info
                                '''
                            }
                            
                            // Wait for webhook-based quality gate result
                            echo "Waiting for SonarQube webhook to deliver quality gate result..."
                            timeout(time: 4, unit: 'MINUTES') {
                                def qg = waitForQualityGate()
                                if (qg.status != 'OK') {
                                    echo "❌ SonarQube Quality Gate FAILED with status: ${qg.status}"
                                    echo "Quality gate conditions not met. Check SonarQube for details."
                                    error "🚫 PIPELINE FAILED: SonarQube Quality Gate is ${qg.status} - fix code quality issues before proceeding"
                                }
                                echo "✅ SonarQube Quality Gate passed"
                            }
                        }
                    }
                }
            }
        }

        stage('Build Image') {
            steps {
                container('builder'){
                    script {
                        echo 'Building container image using containerd'
                        
                        sh """#!/bin/bash
                            set -euo pipefail

                            echo "Verifying BuildKit..."
                            buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers > /dev/null

                            # Create clean build context
                            mkdir -p /tmp/build-context
                            cp -r elohim-app /tmp/build-context/
                            cp images/Dockerfile /tmp/build-context/
                            
                            # Build container image with semantic versioning
                            cd /tmp/build-context
                            BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \\
                              nerdctl -n k8s.io build -t elohim-app:${env.IMAGE_TAG} -f Dockerfile .

                            # Tag with commit hash for traceability
                            nerdctl -n k8s.io tag elohim-app:${env.IMAGE_TAG} elohim-app:${env.GIT_COMMIT_HASH}
                            
                            # Only tag as latest for main branch
                            if [ "${env.BRANCH_NAME}" = "main" ]; then
                                nerdctl -n k8s.io tag elohim-app:${env.IMAGE_TAG} elohim-app:latest
                            fi
                        """
                        
                        env.DOCKER_BUILD_COMPLETED = 'true'
                        echo 'Container image built successfully'
                    }
                }
            }
        }

        stage('Push to Harbor Registry') {
            steps {
                container('builder'){
                    script {
                        withCredentials([usernamePassword(credentialsId: 'harbor-robot-registry', passwordVariable: 'HARBOR_PASSWORD', usernameVariable: 'HARBOR_USERNAME')]) {
                            echo 'Logging into Harbor registry'
                            sh 'echo $HARBOR_PASSWORD | nerdctl -n k8s.io login harbor.ethosengine.com -u $HARBOR_USERNAME --password-stdin'
                            
                            echo 'Tagging image for Harbor registry with semantic versioning'
                            sh 'nerdctl -n k8s.io tag elohim-app:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-site:${IMAGE_TAG}'
                            sh 'nerdctl -n k8s.io tag elohim-app:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-site:${GIT_COMMIT_HASH}'
                            
                            // Only tag and push latest for main branch
                            if (env.BRANCH_NAME == 'main') {
                                sh 'nerdctl -n k8s.io tag elohim-app:${IMAGE_TAG} harbor.ethosengine.com/ethosengine/elohim-site:latest'
                            }
                            
                            echo 'Pushing images to Harbor registry'
                            sh 'nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-site:${IMAGE_TAG}'
                            sh 'nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-site:${GIT_COMMIT_HASH}'
                            
                            if (env.BRANCH_NAME == 'main') {
                                sh 'nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-site:latest'
                            }
                            
                            echo 'Successfully pushed to Harbor registry'
                        }
                    }
                }
            }
        }

        stage('Harbor Security Scan') {
            steps {
                container('builder'){
                    script {
                        withCredentials([usernamePassword(credentialsId: 'harbor-robot-registry', passwordVariable: 'HARBOR_PASSWORD', usernameVariable: 'HARBOR_USERNAME')]) {
                            echo 'Triggering Harbor vulnerability scan'
                            
                            // Trigger scan via Harbor API using wget with basic auth
                            sh '''
                                AUTH_HEADER="Authorization: Basic $(echo -n "$HARBOR_USERNAME:$HARBOR_PASSWORD" | base64)"
                                echo "Triggering scan for artifact: ${IMAGE_TAG}"
                                wget --post-data="" \
                                  --header="accept: application/json" \
                                  --header="Content-Type: application/json" \
                                  --header="$AUTH_HEADER" \
                                  -S \
                                  -O- \
                                  "https://harbor.ethosengine.com/api/v2.0/projects/ethosengine/repositories/elohim-site/artifacts/${IMAGE_TAG}/scan" || \
                                echo "Scan request failed - check error response above"
                            '''
                            
                            echo 'Vulnerability scan initiated. Polling for completion...'
                            
                            // Poll for scan completion with smart retry logic
                            sh '''#!/bin/bash
                                AUTH_HEADER="Authorization: Basic $(echo -n "$HARBOR_USERNAME:$HARBOR_PASSWORD" | base64)"
                                
                                # Polling configuration
                                MAX_ATTEMPTS=24  # 24 attempts = 4 minutes max
                                ATTEMPT=1
                                POLL_INTERVAL=10  # 10 seconds between polls
                                
                                echo "Polling for scan completion..."
                                
                                while [ $ATTEMPT -le $MAX_ATTEMPTS ]; do
                                    VULN_DATA=$(wget -q -O- \
                                      --header="accept: application/json" \
                                      --header="$AUTH_HEADER" \
                                      "https://harbor.ethosengine.com/api/v2.0/projects/ethosengine/repositories/elohim-site/artifacts/${IMAGE_TAG}/additions/vulnerabilities" 2>/dev/null || echo "")
                                    
                                    # Check if we got valid scan data
                                    if [ ! -z "$VULN_DATA" ] && echo "$VULN_DATA" | grep -q '"scanner"'; then
                                        echo "✅ Scan completed after $ATTEMPT attempts"
                                        break
                                    fi
                                    
                                    if [ $ATTEMPT -eq $MAX_ATTEMPTS ]; then
                                        echo "❌ Scan timeout after $MAX_ATTEMPTS attempts"
                                        VULN_DATA=""
                                        break
                                    fi
                                    
                                    [ $((ATTEMPT % 5)) -eq 0 ] && echo "Waiting for scan (attempt $ATTEMPT/$MAX_ATTEMPTS)..."
                                    sleep $POLL_INTERVAL
                                    ATTEMPT=$((ATTEMPT + 1))
                                done
                                
                                if [ ! -z "$VULN_DATA" ]; then
                                    # Check vulnerabilities
                                    if echo "$VULN_DATA" | grep -q '"vulnerabilities":\\[\\]'; then
                                        echo "✅ Security Status: CLEAN - No vulnerabilities found"
                                    else
                                        VULN_COUNT=$(echo "$VULN_DATA" | grep -o '"vulnerabilities":\\[.*\\]' | grep -o '\\[.*\\]' | tr ',' '\\n' | wc -l)
                                        echo "⚠️  Security Status: $VULN_COUNT vulnerabilities detected"
                                    fi
                                else
                                    echo "❌ Scan data unavailable. Check Harbor UI: https://harbor.ethosengine.com"
                                fi
                            '''
                        }
                    }
                }
            }
        }

        stage('Deploy to Staging') {
            steps {
                container('builder'){
                    script {
                        echo 'Deploying to Staging Environment'
                        
                        // Validate staging dependencies exist
                        sh '''
                            echo "Validating staging ConfigMap exists..."
                            kubectl get configmap elohim-config-staging -n ethosengine || {
                                echo "❌ ERROR: elohim-config-staging ConfigMap missing"
                                echo "Run: kubectl apply -f manifests/configmap-staging.yaml"
                                exit 1
                            }
                            echo "✅ Staging ConfigMap validated"
                        '''
                        
                        // Update image tag in deployment manifest with semantic version
                        sh "sed 's/BUILD_NUMBER_PLACEHOLDER/${IMAGE_TAG}/g' manifests/deployment.yaml > manifests/deployment-${IMAGE_TAG}.yaml"
                        
                        // Deploy staging only
                        sh 'kubectl apply -f manifests/deployment-${IMAGE_TAG}.yaml'
                        
                        // Wait for staging deployment to be ready
                        sh 'kubectl rollout status deployment/elohim-site-staging -n ethosengine --timeout=300s'
                        
                        // Show staging deployment status
                        sh 'kubectl get deployments,services,pods -l app=elohim-site-staging -n ethosengine'
                        
                        echo 'Staging deployment completed successfully!'
                        echo 'Staging URL: https://staging.elohim.host'
                    }
                }
            }
        }

        stage('E2E Testing - Staging Validation') {
            steps {
                container('builder'){
                    dir('elohim-app') {
                        script {
                            echo 'Running E2E tests to validate staging deployment'
                            
                            // Mark that E2E tests are being attempted
                            env.E2E_TESTS_RAN = 'true'
                            
                            // Install E2E test dependencies if not already cached
                            sh '''
                                if [ ! -d "node_modules/cypress" ]; then
                                    echo "Installing Cypress and dependencies..."
                                    npm install cypress @badeball/cypress-cucumber-preprocessor @cypress/browserify-preprocessor @bahmutov/cypress-esbuild-preprocessor
                                else
                                    echo "Cypress dependencies already installed"
                                fi
                            '''
                            
                            // Verify staging endpoint is responding
                            sh '''#!/bin/bash
                                echo "Verifying staging endpoint..."
                                timeout 60s bash -c 'until curl -s -o /dev/null -w "%{http_code}" https://staging.elohim.host | grep -q "200\\|302\\|301"; do 
                                    sleep 5
                                done'
                                echo "✅ Staging site is responding"
                            '''
                            
                            // Run Cypress E2E tests against staging with explicit environment targeting
                            sh '''#!/bin/bash
                                export CYPRESS_baseUrl=https://staging.elohim.host
                                export CYPRESS_ENV=staging
                                export NO_COLOR=1
                                export DISPLAY=:99
                                echo "Running E2E tests against: $CYPRESS_baseUrl"
                                
                                # Start display server
                                Xvfb :99 -screen 0 1024x768x24 -ac > /dev/null 2>&1 &
                                XVFB_PID=$!
                                sleep 2
                                
                                # Verify Cypress
                                echo "Verifying Cypress..."
                                npx cypress verify > /dev/null || {
                                    echo "❌ Cypress verification failed"
                                    exit 1
                                }
                                
                                echo "Creating reports directory..."
                                mkdir -p cypress/reports
                                
                                echo "DEBUG: Checking cucumber preprocessor config..."
                                cat package.json | grep -A 10 "cypress-cucumber-preprocessor"
                                
                                echo "DEBUG: Listing installed cucumber packages..."
                                npm list | grep cucumber || echo "No cucumber packages found"
                                
                                echo "Running E2E tests..."
                                if npx cypress run \
                                    --headless \
                                    --browser chromium \
                                    --spec "cypress/e2e/staging-validation.feature"; then
                                    echo "✅ E2E tests passed successfully!"
                                    TEST_RESULT="passed"
                                else
                                    echo "❌ E2E tests failed!"
                                    TEST_RESULT="failed"
                                fi
                                
                                echo "DEBUG: Checking for reports after test execution..."
                                ls -la cypress/reports/ 2>/dev/null || echo "No reports directory found after test"
                                find . -name "*cucumber*" -type f 2>/dev/null || echo "No cucumber files found after test"
                                
                                # Cleanup
                                kill $XVFB_PID 2>/dev/null || true
                                
                                # Exit with appropriate code
                                if [ "$TEST_RESULT" = "failed" ]; then
                                    exit 1
                                fi
                            '''
                            
                            echo '✅ Staging validation tests passed successfully!'
                            echo 'Staging site is ready for production deployment'
                        }
                    }
                }
            }
            post {
                success {
                    echo '✅ E2E tests passed - staging validation successful!'
                }
                always {
                    dir('elohim-app') {
                        script {
                            // Only try to publish cucumber reports if E2E tests actually ran
                            if (env.E2E_TESTS_RAN == 'true') {
                                echo '📊 Publishing cucumber reports...'
                                
                                // Debug: Show what files exist in cypress directory
                                sh 'echo "DEBUG: Contents of cypress directory:"'
                                sh 'find cypress -type f -name "*" 2>/dev/null || echo "cypress directory not found"'
                                
                                // Debug: Show specifically what's in reports directory
                                sh 'echo "DEBUG: Contents of cypress/reports directory:"'
                                sh 'ls -la cypress/reports/ 2>/dev/null || echo "cypress/reports directory not found"'
                                
                                // Debug: Show absolute paths for cucumber plugin
                                sh 'echo "DEBUG: Current working directory: $(pwd)"'
                                sh 'echo "DEBUG: Absolute path to cucumber report: $(pwd)/cypress/reports/cucumber-report.json"'
                                sh 'test -f cypress/reports/cucumber-report.json && echo "DEBUG: File exists and is readable" || echo "DEBUG: File does not exist or is not readable"'
                                
                                // Publish cucumber reports using cucumber plugin
                                if (fileExists('cypress/reports/cucumber-report.json')) {
                                cucumber([
                                    reportTitle: 'E2E Test Results',
                                    fileIncludePattern: 'cucumber-report.json',
                                    jsonReportDirectory: 'cypress/reports',
                                    buildStatus: 'UNSTABLE',
                                    failedFeaturesNumber: -1,
                                    failedScenariosNumber: -1,
                                    failedStepsNumber: -1,
                                    skippedStepsNumber: -1,
                                    pendingStepsNumber: -1,
                                    undefinedStepsNumber: -1
                                ])
                                    echo 'Cucumber reports published successfully with cucumber plugin'
                                } else {
                                    echo 'No cucumber reports found to publish'
                                }
                            } else {
                                echo 'E2E tests did not run - skipping cucumber report publishing'
                            }
                            
                            // Archive test artifacts if E2E tests ran
                            if (env.E2E_TESTS_RAN == 'true') {
                                sh 'echo "Archiving test artifacts..."'
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
                    }
                }
                failure {
                    echo '❌ E2E tests failed - staging deployment validation unsuccessful'
                    echo 'Check test artifacts and logs for details'
                }
            }
        }

        stage('Deploy to Prod') {
            steps {
                container('builder'){
                    script {
                        echo 'Deploying to Production Environment'
                        
                        // Validate production dependencies exist
                        sh '''
                            echo "Validating production ConfigMap exists..."
                            kubectl get configmap elohim-config-prod -n ethosengine || {
                                echo "❌ ERROR: elohim-config-prod ConfigMap missing"
                                echo "Run: kubectl apply -f manifests/configmap-prod.yaml"
                                exit 1
                            }
                            echo "✅ Production ConfigMap validated"
                        '''
                        
                        // Deploy production (using same deployment file)
                        sh 'kubectl apply -f manifests/deployment-${IMAGE_TAG}.yaml'
                        
                        // Wait for production deployment to be ready
                        sh 'kubectl rollout status deployment/elohim-site -n ethosengine --timeout=300s'
                        
                        // Show production deployment status
                        sh 'kubectl get deployments,services,pods -l app=elohim-site -n ethosengine'
                        
                        echo 'Production deployment completed successfully!'
                        echo 'Production URL: https://elohim.host'
                    }
                }
            }
        }

        stage('Cleanup') {
            steps {
                container('builder'){
                    script {
                        echo 'Cleaning up to save space'
                        dir('elohim-app') {
                            sh 'rm -rf node_modules || true'
                        }
                    }
                }
            }
        }

    }

    post {
        success {
            echo 'Pipeline completed successfully. Docker image elohim-app:${IMAGE_TAG} (${GIT_COMMIT_HASH}) is ready.'
            echo 'Base version: ${BASE_VERSION}'
            echo 'Branch: ${BRANCH_NAME}'
        }
        failure {
            echo 'Pipeline failed. Check the logs for details.'
        }
        always {
            script {
                if (env.DOCKER_BUILD_COMPLETED == 'true') {
                    try {
                        container('builder') {
                            echo 'Cleaning up nerdctl images...'
                            sh 'nerdctl -n k8s.io rmi elohim-app:${IMAGE_TAG} || true'
                            sh 'nerdctl -n k8s.io rmi elohim-app:${GIT_COMMIT_HASH} || true'
                            sh 'nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-site:${IMAGE_TAG} || true'
                            sh 'nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-site:${GIT_COMMIT_HASH} || true'
                            if (env.BRANCH_NAME == 'main') {
                                sh 'nerdctl -n k8s.io rmi elohim-app:latest || true'
                                sh 'nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-site:latest || true'
                            }
                            sh 'nerdctl -n k8s.io system prune -af --volumes || true'
                            echo 'nerdctl cleanup completed.'
                        }
                    } catch (Exception e) {
                        echo "nerdctl cleanup failed: ${e.message}"
                    }
                } else {
                    echo 'Build was not completed, skipping image cleanup.'
                }
            }
        }
    }
}