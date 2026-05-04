/**
 * Shared deploy helpers for orchestrator-managed pipelines.
 *
 * Loaded via `def helpers = load 'genesis/orchestrator/scripts/deploy-helpers.groovy'`
 * from any downstream Jenkinsfile.
 *
 * Conventions:
 *   - All manifests use SITE_TAG_PLACEHOLDER and DEPLOY_VERSION_PLACEHOLDER tokens
 *   - Substituted via sed at deploy time
 *   - kubectl rollout status waits up to 300s
 *   - Pre-deploy: ingress conflict check via genesis/orchestrator/scripts/check-ingress-conflicts.sh
 *   - Post-deploy: stale resource detection (advisory only)
 */

def deployAppToEnvironment(String environment, String namespace, String deploymentName, String manifestPath, String imageTag) {
    echo "Deploying to ${environment}: ${imageTag}"

    // Validate ConfigMap exists (skipped automatically when caller does not require one)
    sh "kubectl get configmap elohim-config-${environment} -n ${namespace} || { echo 'ConfigMap missing'; exit 1; }"

    // Update deployment manifest with image tag and deploy version label
    def outputFile = manifestPath.replace('.yaml', "-${environment}.rendered.yaml")
    sh "sed -e 's/SITE_TAG_PLACEHOLDER/${imageTag}/g' -e 's/DEPLOY_VERSION_PLACEHOLDER/${imageTag}/g' ${manifestPath} > ${outputFile}"

    // Fail fast if any placeholders remain
    def remaining = sh(script: "grep -c '_PLACEHOLDER' ${outputFile} || true", returnStdout: true).trim()
    if (remaining != '0') {
        error "Unresolved placeholders in ${outputFile}!"
    }

    // Preview manifest
    sh """
        echo '==== Deployment manifest preview ===='
        grep 'image:\\|app.kubernetes.io/version:' ${outputFile}
        echo '===================================='
    """

    // Pre-deploy: check for ingress hostname conflicts
    sh "bash genesis/orchestrator/scripts/check-ingress-conflicts.sh ${outputFile} ${namespace}"

    // Deploy and rollout
    sh "kubectl apply -f ${outputFile}"
    sh "kubectl rollout restart deployment/${deploymentName} -n ${namespace}"
    sh "kubectl rollout status deployment/${deploymentName} -n ${namespace} --timeout=300s"

    // Verify deployed image
    sh """
        echo '==== Verifying deployed image ===='
        kubectl get deployment ${deploymentName} -n ${namespace} -o jsonpath='{.spec.template.spec.containers[0].image}'
        echo ''
        echo '=================================='
    """

    // Post-deploy: detect stale resources (advisory only)
    sh "bash genesis/orchestrator/scripts/detect-stale-resources.sh ${namespace} ${imageTag} elohim-site || true"

    echo "${environment} deployment completed!"
}

/**
 * Same as deployAppToEnvironment but does NOT validate a configmap.
 * Used by elohim-storybook (fully static, no runtime config injection).
 */
def deployStaticToEnvironment(String environment, String namespace, String deploymentName, String manifestPath, String imageTag, String imageNameForStaleCheck) {
    echo "Deploying static site to ${environment}: ${imageTag}"

    // Update deployment manifest with image tag and deploy version label
    def outputFile = manifestPath.replace('.yaml', "-${environment}.rendered.yaml")
    sh "sed -e 's/STORYBOOK_TAG_PLACEHOLDER/${imageTag}/g' -e 's/DEPLOY_VERSION_PLACEHOLDER/${imageTag}/g' ${manifestPath} > ${outputFile}"

    // Fail fast if any placeholders remain
    def remaining = sh(script: "grep -c '_PLACEHOLDER' ${outputFile} || true", returnStdout: true).trim()
    if (remaining != '0') {
        error "Unresolved placeholders in ${outputFile}!"
    }

    // Preview manifest
    sh """
        echo '==== Deployment manifest preview ===='
        grep 'image:\\|app.kubernetes.io/version:' ${outputFile}
        echo '===================================='
    """

    // Pre-deploy: check for ingress hostname conflicts
    sh "bash genesis/orchestrator/scripts/check-ingress-conflicts.sh ${outputFile} ${namespace}"

    // Deploy and rollout
    sh "kubectl apply -f ${outputFile}"
    sh "kubectl rollout restart deployment/${deploymentName} -n ${namespace}"
    sh "kubectl rollout status deployment/${deploymentName} -n ${namespace} --timeout=300s"

    // Verify deployed image
    sh """
        echo '==== Verifying deployed image ===='
        kubectl get deployment ${deploymentName} -n ${namespace} -o jsonpath='{.spec.template.spec.containers[0].image}'
        echo ''
        echo '=================================='
    """

    // Post-deploy: detect stale resources (advisory only)
    sh "bash genesis/orchestrator/scripts/detect-stale-resources.sh ${namespace} ${imageTag} ${imageNameForStaleCheck} || true"

    echo "${environment} deployment completed!"
}

return this
