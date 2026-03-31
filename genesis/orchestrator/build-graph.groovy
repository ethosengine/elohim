// build-graph.groovy
// Build Graph Walker for Jenkins Orchestrator
//
// Discovers per-pipeline build-manifest.json files, composes them into
// a unified DAG, and walks it against a changeset to determine the
// minimal set of build steps needed. Zero guessing.
//
// Usage (from Jenkinsfile):
//   def buildGraph = load('genesis/orchestrator/build-graph.groovy')
//   def result = buildGraph.walkBuildGraph(changedFiles)

import groovy.json.JsonSlurper
import groovy.json.JsonOutput
import java.security.MessageDigest

// ============================================================
// DISCOVERY & PARSING
// ============================================================

/**
 * Discover and parse all build-manifest.json files in the workspace.
 * CPS-compatible (uses pipeline steps: sh, readFile).
 */
def discoverAndParseManifests() {
    def paths = sh(
        script: "find . -name 'build-manifest.json' -not -path '*/node_modules/*' -not -path '*/.superpowers/*' | sort",
        returnStdout: true
    ).trim().split('\n').findAll { it }

    echo "Found ${paths.size()} build manifests: ${paths.join(', ')}"

    def manifests = []
    for (def path : paths) {
        def content = readFile(file: path)
        def manifest = parseManifest(content, path)
        manifests.add(manifest)
    }
    return manifests
}

@NonCPS
def parseManifest(String content, String filePath) {
    def manifest = new JsonSlurper().parseText(content)
    manifest._filePath = filePath
    return manifest
}

// ============================================================
// COMPOSITION
// ============================================================

/**
 * Compose all manifests into a unified build graph.
 * Returns [steps: [:], pipelines: [:]] where each step has qualified name.
 */
@NonCPS
def composeGraph(List manifests) {
    def graph = [steps: [:], pipelines: [:]]

    for (def manifest : manifests) {
        def pipeline = manifest.pipeline
        if (graph.pipelines.containsKey(pipeline)) {
            throw new RuntimeException(
                "Duplicate pipeline name '${pipeline}' in ${manifest._filePath} " +
                "and ${graph.pipelines[pipeline]._filePath}"
            )
        }
        graph.pipelines[pipeline] = manifest

        manifest.steps.each { stepName, stepDef ->
            def qualifiedName = "${pipeline}:${stepName}"
            graph.steps[qualifiedName] = [
                pipeline: pipeline,
                localName: stepName,
                description: stepDef.description,
                inputs: stepDef.inputs,
                outputs: stepDef.outputs,
                depends: (stepDef.depends ?: []).collect { dep ->
                    dep.contains(':') ? dep : "${pipeline}:${dep}"
                },
                executor: stepDef.executor,
                manualOnly: manifest.manualOnly ?: false
            ]
        }
    }

    // Validate: every dependency target exists
    graph.steps.each { name, step ->
        step.depends.each { dep ->
            if (!graph.steps.containsKey(dep)) {
                throw new RuntimeException(
                    "Step '${name}' depends on '${dep}' which does not exist. " +
                    "Available steps: ${graph.steps.keySet().sort().join(', ')}"
                )
            }
        }
    }

    // Detect cycles
    detectCycles(graph)

    return graph
}

@NonCPS
def detectCycles(Map graph) {
    def visited = new HashSet()
    def inStack = new HashSet()

    for (def stepName : graph.steps.keySet()) {
        if (!visited.contains(stepName)) {
            dfsDetectCycle(graph, stepName, visited, inStack, [])
        }
    }
}

@NonCPS
def dfsDetectCycle(Map graph, String node, Set visited, Set inStack, List path) {
    visited.add(node)
    inStack.add(node)
    path = path + [node]

    def step = graph.steps[node]
    for (def dep : step.depends) {
        if (!visited.contains(dep)) {
            dfsDetectCycle(graph, dep, visited, inStack, path)
        } else if (inStack.contains(dep)) {
            def cycleStart = path.indexOf(dep)
            def cycle = path.subList(cycleStart, path.size()) + [dep]
            throw new RuntimeException("Dependency cycle detected: ${cycle.join(' -> ')}")
        }
    }

    inStack.remove(node)
}

return this
