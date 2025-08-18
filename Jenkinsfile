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
       ephemeral-storage: "2Gi"
   volumeMounts:
   - name: containerd-sock
     mountPath: /run/containerd/containerd.sock
   - name: buildkit-run
     mountPath: /run/buildkit
 - name: buildkitd
   image: moby/buildkit:v0.13.1
   securityContext:
     privileged: true
   args:
   - --addr
   - unix:///run/buildkit/buildkitd.sock
   - --oci-worker=false
   - --containerd-worker=true
   - --containerd-worker-namespace=k8s.io
   volumeMounts:
   - name: containerd-sock
     mountPath: /run/containerd/containerd.sock
   - name: buildkit-run
     mountPath: /run/buildkit
'''
        }
    }
    
    stages {

        stage('Checkout') {
            steps {
                container('builder'){
                    script {
                        //scm checkout
                        checkout([$class: 'GitSCM', 
                                 branches: [[name: '*/main']], 
                                 userRemoteConfigs: [[url: 'https://github.com/ethosengine/elohim.git']]])
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
        
        stage('Build') {
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

        stage('Test') {
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
                                    -Dsonar.typescript.lcov.reportPaths=coverage/lcov.info \
                                    -Dsonar.javascript.lcov.reportPaths=coverage/lcov.info
                                '''
                            }
                        }
                    }
                }
            }
        }

        stage('Build Docker Image') {
            steps {
                container('builder'){
                    script {
                        echo 'Building container image using containerd'
                        
                        sh '''
                            set -euxo pipefail

                            echo "Sockets available:"
                            ls -l /run/containerd/containerd.sock
                            ls -l /run/buildkit/buildkitd.sock

                            echo "Versions:"
                            nerdctl version || true
                            buildctl --addr unix:///run/buildkit/buildkitd.sock --version

                            # Sanity: ensure worker is up
                            buildctl --addr unix:///run/buildkit/buildkitd.sock debug workers

                            # Build
                            BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \
                              nerdctl -n k8s.io build -t elohim-app:${BUILD_NUMBER} -f images/Dockerfile .

                            nerdctl -n k8s.io tag elohim-app:${BUILD_NUMBER} elohim-app:latest
                        '''
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
                            
                            echo 'Tagging image for Harbor registry'
                            sh 'nerdctl -n k8s.io tag elohim-app:${BUILD_NUMBER} harbor.ethosengine.com/ethosengine/elohim-site:${BUILD_NUMBER}'
                            sh 'nerdctl -n k8s.io tag elohim-app:${BUILD_NUMBER} harbor.ethosengine.com/ethosengine/elohim-site:latest'
                            
                            echo 'Pushing images to Harbor registry'
                            sh 'nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-site:${BUILD_NUMBER}'
                            sh 'nerdctl -n k8s.io push harbor.ethosengine.com/ethosengine/elohim-site:latest'
                            
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
                                echo "Triggering scan for artifact: ${BUILD_NUMBER}"
                                wget --post-data="" \
                                  --header="accept: application/json" \
                                  --header="Content-Type: application/json" \
                                  --header="$AUTH_HEADER" \
                                  -S \
                                  -O- \
                                  "https://harbor.ethosengine.com/api/v2.0/projects/ethosengine/repositories/elohim-site/artifacts/${BUILD_NUMBER}/scan" || \
                                echo "Scan request failed - check error response above"
                            '''
                            
                            echo 'Vulnerability scan initiated. Polling for completion...'
                            
                            // Poll for scan completion with smart retry logic
                            sh '''
                                AUTH_HEADER="Authorization: Basic $(echo -n "$HARBOR_USERNAME:$HARBOR_PASSWORD" | base64)"
                                
                                # Polling configuration
                                MAX_ATTEMPTS=24  # 24 attempts = 4 minutes max
                                ATTEMPT=1
                                POLL_INTERVAL=10  # 10 seconds between polls
                                
                                echo "Polling for scan completion (max ${MAX_ATTEMPTS} attempts, ${POLL_INTERVAL}s intervals)..."
                                
                                while [ $ATTEMPT -le $MAX_ATTEMPTS ]; do
                                    echo "Attempt $ATTEMPT/$MAX_ATTEMPTS: Checking scan status..."
                                    
                                    VULN_DATA=$(wget -q -O- \
                                      --header="accept: application/json" \
                                      --header="$AUTH_HEADER" \
                                      "https://harbor.ethosengine.com/api/v2.0/projects/ethosengine/repositories/elohim-site/artifacts/${BUILD_NUMBER}/additions/vulnerabilities" 2>/dev/null || echo "")
                                    
                                    # Check if we got valid scan data (not empty and contains scanner info)
                                    if [ ! -z "$VULN_DATA" ] && echo "$VULN_DATA" | grep -q '"scanner"'; then
                                        echo "✅ Scan completed after $ATTEMPT attempts!"
                                        break
                                    fi
                                    
                                    if [ $ATTEMPT -eq $MAX_ATTEMPTS ]; then
                                        echo "❌ Scan did not complete within timeout period"
                                        echo "Last response: $VULN_DATA"
                                        VULN_DATA=""  # Clear data to trigger fallback message
                                        break
                                    fi
                                    
                                    echo "Scan not ready yet, waiting ${POLL_INTERVAL}s..."
                                    sleep $POLL_INTERVAL
                                    ATTEMPT=$((ATTEMPT + 1))
                                done
                                
                                if [ ! -z "$VULN_DATA" ]; then
                                    echo "Vulnerability scan completed successfully!"
                                    
                                    # Extract scanner info
                                    SCANNER=$(echo "$VULN_DATA" | grep -o '"scanner":{"name":"[^"]*","vendor":"[^"]*","version":"[^"]*"}' || echo "Scanner info not found")
                                    echo "Scanner: $SCANNER"
                                    
                                    # Extract generated timestamp
                                    GENERATED=$(echo "$VULN_DATA" | grep -o '"generated_at":"[^"]*"' || echo "Timestamp not found")
                                    echo "Generated: $GENERATED"
                                    
                                    # Count vulnerabilities
                                    VULN_COUNT=$(echo "$VULN_DATA" | grep -o '"vulnerabilities":\\[.*\\]' | grep -o '\\[.*\\]' | tr ',' '\\n' | wc -l)
                                    if echo "$VULN_DATA" | grep -q '"vulnerabilities":\\[\\]'; then
                                        echo "✅ Security Status: CLEAN - No vulnerabilities found!"
                                    else
                                        echo "⚠️  Security Status: $VULN_COUNT vulnerabilities detected"
                                        echo "Raw vulnerability data:"
                                        echo "$VULN_DATA" | head -c 1000
                                    fi
                                else
                                    echo "No vulnerability data available yet. Scan may still be running."
                                    echo "Check Harbor UI for scan progress: https://harbor.ethosengine.com"
                                fi
                            '''
                        }
                    }
                }
            }
        }

        stage('Stage Deploy') {
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
                        
                        // Update image tag in deployment manifest
                        sh "sed 's/BUILD_NUMBER_PLACEHOLDER/${BUILD_NUMBER}/g' manifests/deployment.yaml > manifests/deployment-${BUILD_NUMBER}.yaml"
                        
                        // Deploy staging only
                        sh 'kubectl apply -f manifests/deployment-${BUILD_NUMBER}.yaml'
                        
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

        stage('Prod Deploy') {
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
                        sh 'kubectl apply -f manifests/deployment-${BUILD_NUMBER}.yaml'
                        
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
            echo 'Pipeline completed successfully. Docker image elohim-app:${BUILD_NUMBER} is ready.'
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
                            sh 'nerdctl -n k8s.io rmi elohim-app:${BUILD_NUMBER} || true'
                            sh 'nerdctl -n k8s.io rmi elohim-app:latest || true'
                            sh 'nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-site:${BUILD_NUMBER} || true'
                            sh 'nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-site:latest || true'
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