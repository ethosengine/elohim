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
    def manifest = deepCopy(new JsonSlurper().parseText(content))
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
    // _validationErrors collects structural problems (duplicate pipelines,
    // missing dependencies, cycles) as Strings. @NonCPS functions cannot
    // `throw new RuntimeException(...)` under the Jenkins script-security
    // sandbox, so errors are accumulated and surfaced by walkBuildGraph
    // (CPS context) via error(...).
    def graph = [steps: [:], pipelines: [:], _validationErrors: []]

    for (def manifest : manifests) {
        def pipeline = manifest.pipeline
        if (graph.pipelines.containsKey(pipeline)) {
            graph._validationErrors.add(
                ("Duplicate pipeline name '${pipeline}' in ${manifest._filePath} " +
                 "and ${graph.pipelines[pipeline]._filePath}").toString()
            )
            continue  // skip this manifest; keep collecting other errors
        }
        graph.pipelines[pipeline] = manifest

        manifest.steps.each { stepName, stepDef ->
            // .toString() everywhere we build a map key or a value that will be
            // compared against a map key. Groovy interpolation produces GString,
            // but JSON-parsed values are String, and GString.equals(String) is
            // ALWAYS false — which causes containsKey() to silently fail with
            // the paradoxical "Step X depends on Y which does not exist.
            // Available steps: ... Y ..." error.
            def qualifiedName = "${pipeline}:${stepName}".toString()
            graph.steps[qualifiedName] = [
                pipeline: pipeline,
                localName: stepName,
                description: stepDef.description,
                inputs: stepDef.inputs,
                outputs: stepDef.outputs,
                depends: (stepDef.depends ?: []).collect { dep ->
                    (dep.contains(':') ? dep : "${pipeline}:${dep}").toString()
                },
                executor: stepDef.executor,
                manualOnly: manifest.manualOnly ?: false
            ]
        }
    }

    // Externalize cross-pipeline deps pointing at pipelines with no manifest.
    // These are real dependencies (Sprint 2 will add their manifests and they
    // become internal) but the graph can't validate or cycle-check them yet.
    // Record them in graph._externalDeps and remove from step.depends.
    graph._externalDeps = [] as Set
    graph.steps.each { name, step ->
        step.depends = step.depends.findAll { dep ->
            if (graph.steps.containsKey(dep)) return true
            def targetPipeline = dep.split(':', 2)[0]
            if (!graph.pipelines.containsKey(targetPipeline)) {
                graph._externalDeps.add(dep)
                return false  // prune from the internal graph
            }
            return true  // intra-manifest gap — real error, keep for validation
        }
    }

    // Validate: every remaining dependency target exists
    graph.steps.each { name, step ->
        step.depends.each { dep ->
            if (!graph.steps.containsKey(dep)) {
                graph._validationErrors.add(
                    ("Step '${name}' depends on '${dep}' which does not exist. " +
                     "Available steps: ${graph.steps.keySet().sort().join(', ')}").toString()
                )
            }
        }
    }

    // Detect cycles (accumulates into graph._validationErrors)
    detectCycles(graph)

    return graph
}

@NonCPS
def detectCycles(Map graph) {
    // If composeGraph already recorded structural errors (duplicate pipelines,
    // missing dependencies), the graph may reference steps that don't exist.
    // Running dfs over that would NPE. Skip — walkBuildGraph will surface the
    // prior errors via error() and the user can fix those first.
    if (graph._validationErrors && graph._validationErrors.size() > 0) return

    def visited = [] as Set
    def inStack = [] as Set

    for (def stepName : graph.steps.keySet()) {
        if (!visited.contains(stepName)) {
            dfsDetectCycle(graph, stepName, visited, inStack, [])
        }
    }
}

@NonCPS
def dfsDetectCycle(Map graph, String node, Set visited, Set inStack, List path) {
    def step = graph.steps[node]
    if (step == null) return  // defense: missing dep already recorded by composeGraph

    visited.add(node)
    inStack.add(node)
    path = path + [node]

    for (def dep : step.depends) {
        if (!visited.contains(dep)) {
            dfsDetectCycle(graph, dep, visited, inStack, path)
        } else if (inStack.contains(dep)) {
            def cycleStart = path.indexOf(dep)
            def cycle = path.subList(cycleStart, path.size()) + [dep]
            graph._validationErrors.add(("Dependency cycle detected: ${cycle.join(' -> ')}").toString())
            inStack.remove(node)
            return  // abort this dfs branch; other branches may still find more cycles
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

// ============================================================
// EXECUTION PLANNING
// ============================================================

def topoSortAndLevel(Map graph, Set staleSteps) {
    if (staleSteps.isEmpty()) return []

    def levels = []
    def placed = new HashSet()
    def remaining = new HashSet(staleSteps)

    while (!remaining.isEmpty()) {
        def level = remaining.findAll { name ->
            def step = graph.steps[name]
            step.depends.every { dep ->
                !remaining.contains(dep) || placed.contains(dep)
            }
        } as Set

        if (level.isEmpty()) {
            echo "WARNING: Cannot resolve remaining steps, adding all: ${remaining}"
            levels.add(remaining.sort() as List)
            break
        }

        levels.add(level.sort() as List)
        placed.addAll(level)
        remaining.removeAll(level)
    }

    return levels
}

@NonCPS
def groupByPipeline(Set staleSteps, Map graph) {
    def pipelineSteps = [:]
    staleSteps.each { name ->
        def step = graph.steps[name]
        def pipeline = step.pipeline
        if (!pipelineSteps.containsKey(pipeline)) {
            pipelineSteps[pipeline] = []
        }
        pipelineSteps[pipeline].add(step.localName)
    }
    return pipelineSteps
}

// ============================================================
// DECISION MATRIX
// ============================================================

@NonCPS
def formatDecisionMatrix(Map graph, Map staleMap) {
    def lines = []
    lines.add('╔══════════════════════════════════════════════════════════════════════════╗')
    lines.add('║                       BUILD GRAPH DECISION MATRIX                        ║')
    lines.add('╠══════════════════════════════════════════════════════════════════════════╣')

    def sortedSteps = graph.steps.entrySet().sort { a, b ->
        def cmp = a.value.pipeline <=> b.value.pipeline
        cmp != 0 ? cmp : a.key <=> b.key
    }

    def currentPipeline = ''
    for (def entry : sortedSteps) {
        def name = entry.key
        def step = entry.value
        def info = staleMap[name] ?: [stale: false]

        if (step.pipeline != currentPipeline) {
            currentPipeline = step.pipeline
            def pipelineHeader = "║ [${currentPipeline}]"
            lines.add(pipelineHeader.padRight(76) + '║')
        }

        def status
        def icon
        if (step.manualOnly) {
            status = 'MANUAL'
            icon = '🔒'
        } else if (info.stale) {
            status = 'BUILD '
            icon = '🔨'
        } else {
            status = 'SKIP  '
            icon = '⏭️ '
        }

        def reason = info.stale ? info.reason : 'no changes'
        if (step.manualOnly && info.stale) {
            reason = "would build (${info.reason}) but manual-only"
        }

        def displayName = step.localName.padRight(26)
        def line = "║ ${icon} ${displayName}│ ${status} │ ${reason}"
        if (line.length() > 75) {
            line = line.substring(0, 72) + '...'
        }
        lines.add(line.padRight(76) + '║')
    }

    lines.add('╚══════════════════════════════════════════════════════════════════════════╝')
    return lines.join('\n')
}

@NonCPS
def formatPerFileMatrix(Map graph, Map staleMap, List changedFiles) {
    def lines = []
    lines.add('╔══════════════════════════════════════════════════════════════════════════╗')
    lines.add('║                       PER-FILE ROUTING MATRIX                            ║')
    lines.add('╠══════════════════════════════════════════════════════════════════════════╣')

    if (changedFiles == null || changedFiles.isEmpty()) {
        lines.add('║ (no changed files)                                                       ║')
        lines.add('╚══════════════════════════════════════════════════════════════════════════╝')
        return lines.join('\n')
    }

    // Build file -> [matched step qualified names] map by inspecting staleMap reasons.
    // Reason formats produced by checkSourceChanges / checkBuildProcessChanges:
    //   "source: <file> matches <pattern>"
    //   "buildProcess: cannot read <file>"
    //   "buildProcess: function <fn> not found in <file>"
    //   "buildProcess: <file> hash changed"           (label = file)
    //   "buildProcess: <file>@<fn> hash changed"      (label = file@func)
    //   "depends: <stepName>"                         (propagated, no file)
    def fileRouting = [:]
    for (def file : changedFiles) {
        fileRouting[file] = []
    }

    graph.steps.each { qualifiedName, step ->
        def info = staleMap[qualifiedName]
        if (!info?.stale) return
        def reason = info.reason ?: ''
        String matchedFile = null

        def srcMatcher = (reason =~ /^source: (\S+) matches /)
        if (srcMatcher.find()) {
            matchedFile = srcMatcher.group(1)
        } else {
            def bpReadMatcher = (reason =~ /^buildProcess: cannot read (\S+)$/)
            if (bpReadMatcher.find()) {
                matchedFile = bpReadMatcher.group(1)
            } else {
                def bpFnMatcher = (reason =~ /^buildProcess: function \S+ not found in (\S+)$/)
                if (bpFnMatcher.find()) {
                    matchedFile = bpFnMatcher.group(1)
                } else {
                    def bpHashMatcher = (reason =~ /^buildProcess: (\S+) hash changed$/)
                    if (bpHashMatcher.find()) {
                        // Strip optional @function suffix to get the file path
                        matchedFile = bpHashMatcher.group(1).split('@')[0]
                    }
                }
            }
        }

        if (matchedFile != null && fileRouting.containsKey(matchedFile)) {
            fileRouting[matchedFile].add(qualifiedName)
        }
    }

    def routedCount = 0
    def skippedCount = 0
    for (def entry : fileRouting.entrySet()) {
        def file = entry.key
        def steps = entry.value
        def displayFile = file.length() > 45 ? '...' + file.substring(file.length() - 42) : file
        def status
        if (steps.isEmpty()) {
            status = 'SKIP — no manifest source matched'
            skippedCount++
        } else {
            def shown = steps.take(2).join(', ')
            def extra = steps.size() > 2 ? ' +' + (steps.size() - 2) : ''
            status = "→ ${shown}${extra}"
            routedCount++
        }
        def line = "║ ${displayFile.padRight(45)} ${status}"
        if (line.length() > 75) line = line.substring(0, 72) + '...'
        lines.add(line.padRight(76) + '║')
    }

    lines.add('╠══════════════════════════════════════════════════════════════════════════╣')
    def summary = "║ ${routedCount} routed, ${skippedCount} unrouted of ${changedFiles.size()} total"
    lines.add(summary.padRight(76) + '║')
    lines.add('╚══════════════════════════════════════════════════════════════════════════╝')

    if (skippedCount > 0) {
        lines.add("ℹ️  ${skippedCount} file(s) matched no manifest. If this is unexpected, check build-manifest.json source globs.")
    }

    return lines.join('\n')
}

// =============================================================================
// KNOWN DIVERGENCES — Sprint 1 transitional allowlist
// =============================================================================
// Each entry is an asymmetric divergence between the legacy PIPELINES map
// and the manifest-driven build graph. Sprint 2 closes these gaps and
// empties the list. Any divergence NOT in this list is a hard failure — it
// represents unexpected drift between the two source-of-truths.
//
// To remove an entry: reconcile the asymmetry (create a missing manifest
// OR add a missing PIPELINES entry OR migrate off the legacy algorithm),
// verify the comparison matrix shows ZERO divergences for that pipeline
// across several real builds, then drop the entry from this list.
@NonCPS
def getKnownDivergences() {
    return [
        // Legacy-only: pipeline exists in PIPELINES, no build-manifest.json yet.
        // Resolution (Sprint 2): add a build-manifest.json covering the sophia
        // submodule's source pattern (sophia/ gitlink + sophia.Jenkinsfile shim).
        'elohim-sophia',

        // Graph-only: manifest exists, no PIPELINES entry. The orchestrator
        // doesn't currently route changes to these pipelines via the legacy
        // path. Resolution (Sprint 2): either add legacy PIPELINES entries so
        // both algorithms agree, OR migrate the orchestrator fully off the
        // legacy algorithm so manifests become authoritative.
        'elohim-compute',           // elohim/elohim-compute/build-manifest.json
        'elohim-doorway-app',       // doorway/doorway-app/build-manifest.json
        'elohim-orchestrator',      // genesis/orchestrator/build-manifest.json
                                    // (covers the orchestrator's own self-edits;
                                    // legacy treats those as CI-only via ciOnlyFiles)
    ]
}

@NonCPS
def formatComparisonMatrix(Map pipelinesAnalysis, Map graphStaleMap, Map graph) {
    def lines = []
    lines.add('╔══════════════════════════════════════════════════════════════════════════╗')
    lines.add('║                    CHANGESET ANALYSIS COMPARISON                         ║')
    lines.add('╠══════════════════════════════════════════════════════════════════════════╣')
    lines.add('║ Pipeline               │ PIPELINES │ Build Graph │ Match?               ║')
    lines.add('╟────────────────────────┼───────────┼─────────────┼──────────────────────╢')

    def graphPipelines = [:]
    graph.steps.each { name, step ->
        def pipeline = step.pipeline
        if (!graphPipelines.containsKey(pipeline)) {
            graphPipelines[pipeline] = [shouldBuild: false, reasons: []]
        }
        def info = graphStaleMap[name]
        if (info?.stale && !step.manualOnly) {
            graphPipelines[pipeline].shouldBuild = true
            graphPipelines[pipeline].reasons.add(info.reason)
        }
    }

    def divergences = 0
    def unexpectedDivergences = []
    def known = getKnownDivergences()
    def allPipelines = (pipelinesAnalysis.keySet() + graphPipelines.keySet()).sort().unique()

    for (def pipeline : allPipelines) {
        def pResult = pipelinesAnalysis[pipeline]?.shouldRun ?: false
        def gResult = graphPipelines[pipeline]?.shouldBuild ?: false
        def match = (pResult == gResult)
        if (!match) {
            divergences++
            if (!known.contains(pipeline)) {
                unexpectedDivergences.add(pipeline)
            }
        }

        def pStatus = pResult ? 'BUILD' : 'SKIP '
        def gStatus = gResult ? 'BUILD' : 'SKIP '
        def matchIcon = match ? '  ✓   ' : '  ✗   '
        def detail = ''
        if (!match && gResult) {
            detail = graphPipelines[pipeline].reasons.take(1).join()
        }
        if (!match && !gResult && pResult) {
            detail = 'PIPELINES false positive?'
        }

        def name = pipeline.padRight(23)
        def line = "║ ${name}│ ${pStatus}    │ ${gStatus}       │${matchIcon}│ ${detail}"
        if (line.length() > 75) line = line.substring(0, 72) + '...'
        lines.add(line.padRight(76) + '║')
    }

    lines.add('╚══════════════════════════════════════════════════════════════════════════╝')

    if (divergences > 0) {
        lines.add("⚠️  ${divergences} DIVERGENCE(S) detected between PIPELINES and Build Graph")
    } else {
        lines.add("✓ PIPELINES and Build Graph agree on all pipelines")
    }

    return [
        text: lines.join('\n'),
        unexpectedDivergences: unexpectedDivergences
    ]
}

// ============================================================
// BUILD STATE PERSISTENCE
// ============================================================

def loadBuildState() {
    try {
        copyArtifacts(
            projectName: env.JOB_NAME,
            selector: lastSuccessful(),
            filter: 'build-state.json',
            optional: true,
            fingerprintArtifacts: false
        )
        if (fileExists('build-state.json')) {
            def content = readFile(file: 'build-state.json')
            return parseJson(content)
        }
    } catch (Exception e) {
        echo "No previous build state found: ${e.message}"
    }
    return [version: '1.0', lastSuccessfulCommit: null, stepStates: [:]]
}

@NonCPS
def parseJson(String content) {
    return deepCopy(new JsonSlurper().parseText(content))
}

/**
 * Deep-convert LazyMap/LazyList from JsonSlurper into regular HashMap/ArrayList.
 * LazyMap is not CPS-serializable and breaks when crossing @NonCPS → pipeline boundary.
 *
 * The previous implementation round-tripped through JSON, but
 * `new JsonSlurper().parseText(...)` ALSO returns LazyMap — so the round-trip
 * was a no-op and the resulting graph state still threw NotSerializableException
 * the moment it crossed back into CPS. The bug was masked for months by the
 * try/catch in runBuildGraphShadow swallowing the exception. Sprint 1 removed
 * that try/catch; this is the fix.
 */
@NonCPS
def deepCopy(obj) {
    if (obj instanceof Map) {
        def m = [:]
        obj.each { k, v -> m[k] = deepCopy(v) }
        return m
    }
    if (obj instanceof List) {
        def l = []
        obj.each { l.add(deepCopy(it)) }
        return l
    }
    return obj
}

def saveBuildState(Map graph, Map staleMap, Map buildProcessHashes, String commitHash, Map previousState) {
    def stepStates = [:]

    graph.steps.each { name, step ->
        def info = staleMap[name]
        def previousStepState = previousState?.stepStates?.get(name)
        stepStates[name] = [
            lastBuiltCommit: info?.stale ? commitHash : (previousStepState?.lastBuiltCommit ?: null),
            buildProcessHashes: buildProcessHashes[name] ?: [:],
            outputVerified: false
        ]
    }

    def state = [
        version: '1.0',
        lastSuccessfulCommit: commitHash,
        stepStates: stepStates
    ]

    writeFile(file: 'build-state.json', text: serializeJson(state))
    archiveArtifacts(artifacts: 'build-state.json', fingerprint: true)
}

@NonCPS
def serializeJson(Object data) {
    return JsonOutput.prettyPrint(JsonOutput.toJson(data))
}

// ============================================================
// MAIN ENTRY POINT
// ============================================================

def walkBuildGraph(List changedFiles) {
    echo '=== Build Graph Walker ==='
    echo "Analyzing ${changedFiles.size()} changed files"

    def manifests = discoverAndParseManifests()
    echo "Parsed ${manifests.size()} build manifests"

    def graph = composeGraph(manifests)

    // Surface structural validation errors from composeGraph (duplicate pipelines,
    // missing dependencies, cycles). @NonCPS cannot throw under the sandbox, so
    // composeGraph accumulates errors; walkBuildGraph (CPS) turns them into a
    // loud orchestrator failure here.
    if (graph._validationErrors && graph._validationErrors.size() > 0) {
        error """
═══════════════════════════════════════════════════════════════════════════════
  ❌ BUILD GRAPH VALIDATION FAILED
═══════════════════════════════════════════════════════════════════════════════
  composeGraph found ${graph._validationErrors.size()} structural error(s) in the
  manifest graph:

  ${graph._validationErrors.collect { '  • ' + it }.join('\n')}

  Fix the offending build-manifest.json file(s) and push again.
═══════════════════════════════════════════════════════════════════════════════
"""
    }

    echo "Composed graph: ${graph.steps.size()} steps across ${graph.pipelines.size()} pipelines"
    if (graph._externalDeps && graph._externalDeps.size() > 0) {
        echo "ℹ️  ${graph._externalDeps.size()} external dep(s) to unmanaged pipelines (Sprint 2): ${graph._externalDeps.sort().join(', ')}"
    }

    def buildState = loadBuildState()
    if (buildState.lastSuccessfulCommit) {
        echo "Previous build state: commit ${buildState.lastSuccessfulCommit}, ${buildState.stepStates.size()} step hashes"
    } else {
        echo "No previous build state — all steps with buildProcess references will be marked stale"
    }

    def detection = detectAllStaleness(graph, changedFiles, buildState)
    def staleMap = detection.staleMap
    def buildProcessHashes = detection.buildProcessHashes

    echo formatDecisionMatrix(graph, staleMap)

    def staleSteps = staleMap.findAll { k, v -> v.stale && !graph.steps[k].manualOnly }.keySet() as Set
    def manualStaleSteps = staleMap.findAll { k, v -> v.stale && graph.steps[k].manualOnly }.keySet() as Set

    def levels = topoSortAndLevel(graph, staleSteps)
    def pipelineSteps = groupByPipeline(staleSteps, graph)

    echo "Rebuild set: ${staleSteps.size()} steps across ${pipelineSteps.size()} pipelines"
    if (levels) {
        levels.eachWithIndex { level, i ->
            echo "  Level ${i}: ${level.join(', ')}"
        }
    }
    if (manualStaleSteps) {
        echo "Manual-only steps with changes (not auto-triggered): ${manualStaleSteps.join(', ')}"
    }

    return [
        graph: graph,
        staleMap: staleMap,
        staleSteps: staleSteps,
        levels: levels,
        pipelineSteps: pipelineSteps,
        buildProcessHashes: buildProcessHashes,
        previousState: buildState
    ]
}

return this
