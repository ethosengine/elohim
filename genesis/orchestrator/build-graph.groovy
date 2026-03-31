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

// ============================================================
// CHANGE DETECTION
// ============================================================

@NonCPS
def matchesGlob(String filePath, String pattern) {
    def normalizedFile = filePath.startsWith('./') ? filePath.substring(2) : filePath
    def normalizedPattern = pattern.startsWith('./') ? pattern.substring(2) : pattern

    def regex = normalizedPattern
        .replace('.', '\\.')
        .replace('**/','(.+/)?')
        .replace('**', '.*')
        .replace('*', '[^/]*')
        .replace('?', '[^/]')

    return normalizedFile.matches(regex)
}

@NonCPS
def checkSourceChanges(List changedFiles, Map step) {
    def sources = step.inputs?.sources ?: []
    if (sources.isEmpty()) return [stale: false]

    for (def file : changedFiles) {
        for (def pattern : sources) {
            if (matchesGlob(file, pattern)) {
                return [stale: true, reason: "source: ${file} matches ${pattern}"]
            }
        }
    }
    return [stale: false]
}

@NonCPS
def extractFunctionBody(String fileContent, String functionName) {
    def pattern = ~/def\s+${functionName}\s*\([^)]*\)\s*\{/
    def matcher = pattern.matcher(fileContent)
    if (!matcher.find()) return null

    int start = matcher.end()
    int depth = 1
    int pos = start
    while (pos < fileContent.length() && depth > 0) {
        char c = fileContent.charAt(pos)
        if (c == '{' as char) depth++
        else if (c == '}' as char) depth--
        pos++
    }
    return fileContent.substring(start, pos - 1).trim()
}

@NonCPS
def sha256(String content) {
    def digest = MessageDigest.getInstance('SHA-256')
    def hash = digest.digest(content.getBytes('UTF-8'))
    return hash.collect { String.format('%02x', it) }.join()
}

def checkBuildProcessChanges(Map step, Map buildState, String qualifiedName) {
    def refs = step.inputs?.buildProcess ?: []
    if (refs.isEmpty()) return [stale: false, hashes: [:]]

    def currentHashes = [:]

    for (def ref : refs) {
        def parts = ref.split('@', 2)
        def fileName = parts[0]
        def funcName = parts.length > 1 ? parts[1] : null

        def fileContent
        try {
            fileContent = readFile(file: fileName)
        } catch (Exception e) {
            echo "WARNING: Cannot read '${fileName}' referenced by '${qualifiedName}': ${e.message}"
            return [stale: true, reason: "buildProcess: cannot read ${fileName}", hashes: [:]]
        }

        String contentToHash
        if (funcName) {
            contentToHash = extractFunctionBody(fileContent, funcName)
            if (contentToHash == null) {
                echo "WARNING: Function '${funcName}' not found in '${fileName}' (referenced by '${qualifiedName}')"
                return [stale: true, reason: "buildProcess: function ${funcName} not found in ${fileName}", hashes: [:]]
            }
        } else {
            contentToHash = fileContent
        }

        def currentHash = sha256(contentToHash)
        currentHashes[ref] = currentHash

        def previousHash = buildState?.stepStates?.get(qualifiedName)?.buildProcessHashes?.get(ref)
        if (previousHash == null || currentHash != previousHash) {
            def label = funcName ? "${fileName}@${funcName}" : fileName
            return [stale: true, reason: "buildProcess: ${label} hash changed", hashes: currentHashes]
        }
    }

    return [stale: false, hashes: currentHashes]
}

@NonCPS
def propagateStaleness(Map graph, Map staleMap) {
    def changed = true
    while (changed) {
        changed = false
        graph.steps.each { name, step ->
            if (staleMap[name]?.stale) return

            def staleDep = step.depends.find { dep -> staleMap[dep]?.stale }
            if (staleDep) {
                staleMap[name] = [stale: true, reason: "depends: ${staleDep}"]
                changed = true
            }
        }
    }
    return staleMap
}

def detectAllStaleness(Map graph, List changedFiles, Map buildState) {
    def staleMap = [:]
    def allHashes = [:]

    for (def entry : graph.steps.entrySet()) {
        def name = entry.key
        def step = entry.value

        def sourceResult = checkSourceChanges(changedFiles, step)
        if (sourceResult.stale) {
            staleMap[name] = sourceResult
            continue
        }

        def processResult = checkBuildProcessChanges(step, buildState, name)
        allHashes[name] = processResult.hashes
        if (processResult.stale) {
            staleMap[name] = [stale: true, reason: processResult.reason]
            continue
        }

        staleMap[name] = [stale: false]
    }

    staleMap = propagateStaleness(graph, staleMap)

    return [staleMap: staleMap, buildProcessHashes: allHashes]
}

return this
