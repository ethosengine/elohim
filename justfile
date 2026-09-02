# Elohim developer CLI — one discoverable root entrypoint.
#
# Public surface: gate · test · dev · mesh · seed · look · status · codegen
# Project-specific mechanics stay in private recipes and build-manifest.json.

set dotenv-load := false

root := justfile_directory()
app_dir := root / "app" / "elohim-app"
a2o_dir := root / "genesis" / "a2o"
seeder_dir := root / "genesis" / "seeder"

[private]
default:
    @just --list

# Run the manifest-declared quality gate for changed files, a project name, or a path.
gate target="changed" base="origin/dev":
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ "{{ target }}" != "changed" ]]; then
      exec node "{{ root }}/genesis/orchestrator/gate-runner.mjs" --target "{{ target }}"
    fi
    changes="$(mktemp)"
    trap 'rm -f "$changes"' EXIT
    if git rev-parse --verify "{{ base }}" >/dev/null 2>&1; then
      merge_base="$(git merge-base HEAD "{{ base }}")"
      git diff --name-only "$merge_base"...HEAD >> "$changes"
    else
      git diff-tree --no-commit-id --name-only -r HEAD >> "$changes"
    fi
    git diff --name-only >> "$changes"
    git diff --cached --name-only >> "$changes"
    git ls-files --others --exclude-standard >> "$changes"
    sort -u "$changes" | node "{{ root }}/genesis/orchestrator/gate-runner.mjs" --changed-file-list

# Run one test family; `mesh` is the Act I a2o lane, optionally scoped to a path or tag expression.
test target="changed" scope="":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ target }}" in
      changed) exec just --justfile "{{ root }}/justfile" gate ;;
      app) cd "{{ app_dir }}"; exec pnpm test ;;
      a2o) cd "{{ a2o_dir }}"; exec pnpm run test:unit ;;
      mesh|mesh-browser)
        # Act I lane — the a2o `mesh` cucumber profile against the household mesh THIS host owns.
        # `mesh-browser` is the same act, same mesh, through a real browser: it selects the
        # `mesh-browser` profile (@browser/@browser-only, which `mesh` excludes) and points the
        # app URL at the doorway, which serves the app AND proxies the sign-in portal, so both
        # live on one origin and a portal return-URL is an ordinary same-origin redirect.
        # Env comes from the same two sources the CI mesh stage uses: hc-mesh.sh's `mesh_seed_env`
        # (seed/probe block) and hc-mesh-prologue.sh's "a2o env" block, so a local run and the
        # pipeline read the same names. Bring the mesh up first: `just mesh start && just mesh prologue`.
        # `set +e` around the source because hc-mesh.sh is `set -u` only and its optional-binary
        # probes (mongod, the conductor fork) legitimately exit non-zero at load.
        set +e
        # shellcheck source=/dev/null
        source "{{ app_dir }}/scripts/hc-mesh.sh"
        set -e
        mesh_seed_env
        DOORWAY_B_PORT="${DOORWAY_B_PORT:-8889}"
        export E2E_DOORWAY_ALPHA="$DOORWAY_URL"
        export E2E_DOORWAY_B="http://localhost:$DOORWAY_B_PORT"
        export E2E_DOORWAY_BETA="$E2E_DOORWAY_B"
        export E2E_STORAGE_URL="$STORAGE_URL"
        i=0
        for peer in "${PEERS[@]}"; do
          peer_url="http://localhost:$(http_port $i)"
          if [[ "$i" -eq 1 ]]; then export E2E_STORAGE_B="$peer_url"; fi
          export "E2E_STORAGE_$(echo "$peer" | tr '[:lower:]' '[:upper:]')=$peer_url"
          i=$((i+1))
        done
        export E2E_DOORWAY_POOL_STORAGE_URLS="$(peer_url_csv)"
        household_fixture="$MESH_DIR/household-fixture.json"
        if [[ -f "$household_fixture" ]]; then
          export E2E_HOUSEHOLD_FIXTURE_PATH="$household_fixture"
        else
          unset E2E_HOUSEHOLD_FIXTURE_PATH
        fi
        export ELOHIM_CLUSTER_STATE_PATH_OVERRIDE="{{ root }}/genesis/manifests/cluster-state.act1-household.yaml"
        export ELOHIM_REMOTE_COMPUTE_STATUS=unavailable
        # DURABLE TRACE. Reports live under the REPO (genesis/a2o/reports/, gitignored),
        # not under $MESH_DIR: /tmp is wiped on container restart, so every honest local
        # run's evidence died with the container and counted for nothing. See
        # genesis/docs/superpowers/specs/2026-08-22-verification-as-memoized-derivation-guidestar.md
        # (§S1/§S4: the lane with the highest discovery value produced the least durable
        # evidence — this lane is the ONLY one where destructive chapters can run at all).
        reports_dir="{{ a2o_dir }}/reports"
        mkdir -p "$reports_dir"
        # Default, not a force: a caller-supplied CUCUMBER_JSON_REPORT survives, so a
        # scoped run's report isn't clobbered by a later full-lane run.
        export CUCUMBER_JSON_REPORT="${CUCUMBER_JSON_REPORT:-$reports_dir/cucumber-mesh.json}"
        mkdir -p "$(dirname "$CUCUMBER_JSON_REPORT")"
        # Law II env input: the peer count this run addressed. The household fixture also
        # carries it, but the fixture is written by `just mesh prologue` and can be absent,
        # while the mesh roster is always in scope here. Nothing else is forced from this
        # recipe on purpose — the fixture declares processControl, and an undeclared
        # network stage / DNA hash is reported `unknown` by the builder rather than guessed.
        export A2O_PEER_COUNT="${#PEERS[@]}"
        # The transport-parity story must name its mode inside Gherkin. A matrix
        # caller pins the requested mode; an ordinary mesh run derives it from
        # every peer's live status, so neither path trusts a launcher variable.
        export E2E_EXPECTED_TRANSPORT="${E2E_EXPECTED_TRANSPORT:-$(mesh_transport_backend_from_status)}"
        # Run id: sortable UTC stamp + the commit measured. Lexicographic order IS
        # chronological order, and a mesh run takes minutes, so one second is collision-free.
        run_id="${A2O_RUN_ID:-$(date -u +%Y%m%dT%H%M%SZ)-$(git -C "{{ root }}" rev-parse --short=8 HEAD 2>/dev/null || echo nogit)}"
        a2o_profile=mesh
        if [[ "{{ target }}" == "mesh-browser" ]]; then
          a2o_profile=mesh-browser
          export E2E_DEVICE_MODE=playwright
          export E2E_APP_URL="${E2E_APP_URL:-$DOORWAY_URL}"
          export CUCUMBER_JSON_REPORT="${CUCUMBER_JSON_REPORT:-$reports_dir/cucumber-mesh-browser.json}"
        fi
        cd "{{ a2o_dir }}"
        # NO `exec`: exec replaces the shell, so nothing after cucumber ever ran and a run
        # left no report. Capture the verdict instead and propagate it at the end.
        rc=0
        if [[ -z "{{ scope }}" ]]; then
          "{{ a2o_dir }}/node_modules/.bin/cucumber-js" --profile "$a2o_profile" || rc=$?
        else
          # SCOPING: cucumber MERGES a profile's `paths` with CLI positionals instead of replacing
          # them, so a bare `--profile mesh features/x.feature` runs the WHOLE tree plus that file.
          # Generate a config carrying the mesh profile MINUS `paths` so the positional (or --tags)
          # actually narrows. cucumber does `path.join(cwd, configFile)`, so --config must be a path
          # RELATIVE to genesis/a2o — an absolute one is silently mangled.
          export A2O_PROFILE="$a2o_profile"
          cfg="$reports_dir/cucumber-$a2o_profile-scoped.mjs"
          printf '%s\n' \
            '// GENERATED by `just test mesh <scope>`: the mesh profile MINUS `paths`, so a CLI' \
            '// positional or tag expression scopes the run instead of merging with the whole' \
            '// feature tree. Regenerated every run; never commit it.' \
            "import profiles from 'file://{{ a2o_dir }}/cucumber.mjs';" \
            'export default function () {' \
            '  const name = process.env.A2O_PROFILE || "mesh";' \
            '  const { paths: _paths, ...profile } = profiles()[name];' \
            '  return { [name]: profile };' \
            '}' > "$cfg"
          cfg_rel="$(realpath --relative-to="{{ a2o_dir }}" "$cfg")"
          case "{{ scope }}" in
            @*|*" and "*|*" or "*|*"not "*)
              "{{ a2o_dir }}/node_modules/.bin/cucumber-js" --config "$cfg_rel" --profile "$a2o_profile" --tags "{{ scope }}" || rc=$? ;;
            *)
              "{{ a2o_dir }}/node_modules/.bin/cucumber-js" --config "$cfg_rel" --profile "$a2o_profile" "{{ scope }}" || rc=$? ;;
          esac
        fi
        # Built REGARDLESS of the verdict: a red run that leaves no evidence is the exact
        # defect this slice exists to fix. Run-identified under the stable glob
        # reports/sprint-report-household-*.json, so no run overwrites another. A scoped run
        # still reports every unexercised DECLARED concern as NOT MEASURED (Law I) — a
        # re-scope must not make missing coverage vanish.
        # --console-dir / --coverage-gap are LANE-OWNED slots, not the builder's ambient
        # defaults (reports/console, reports/coverage-gap.json). Those two paths accumulate
        # across every lane and are never cleared per run: with the defaults, this household
        # report attributed 22 console-error findings from an Aug-20 BROWSER run to a
        # 1-scenario mesh run. A report may under-count its own findings; it may not carry
        # another run's. (The steps hardcode `reports/console`, so wiring the household lane's
        # own capture into this slot is a genesis/a2o/steps change, not a justfile one.)
        report_rc=0
        # Stamp what every running peer proves, not the launcher selection. A
        # partial dual restart resolves to unknown rather than forging a dual
        # evidence key from MESH_TRANSPORT_BACKEND alone.
        observed_transport="$(mesh_transport_backend_from_status)"
        node --import tsx scripts/build-sprint-report.ts \
          --cucumber "$CUCUMBER_JSON_REPORT" \
          --console-dir  "$reports_dir/console-household" \
          --coverage-gap "$reports_dir/coverage-gap-household.json" \
          --out-json "${A2O_SPRINT_REPORT_JSON:-$reports_dir/sprint-report-household-$run_id.json}" \
          --out-md   "${A2O_SPRINT_REPORT_MD:-$reports_dir/sprint-report-household-$run_id.md}" \
          --profile  mesh \
          --lane     household \
          --transport "$observed_transport" \
          --run-id   "$run_id" \
          --doorway  "$E2E_DOORWAY_ALPHA" || report_rc=$?
        # Cucumber's code wins so a red run stays red; a GREEN run whose evidence could not
        # be written is not a green run either, so the builder's code is the fallback.
        if [[ "$rc" -ne 0 ]]; then exit "$rc"; fi
        exit "$report_rc"
        ;;
      seeder) cd "{{ seeder_dir }}"; exec pnpm test ;;
      storage) exec node "{{ root }}/genesis/orchestrator/gate-runner.mjs" --target elohim-storage ;;
      doorway) exec node "{{ root }}/genesis/orchestrator/gate-runner.mjs" --target doorway ;;
      node) exec node "{{ root }}/genesis/orchestrator/gate-runner.mjs" --target steward-node ;;
      sophia) cd "{{ root }}/sophia"; exec pnpm test --ci ;;
      *) echo "test target must be changed|app|a2o|mesh|seeder|storage|doorway|node|sophia" >&2; exit 2 ;;
    esac

# Manage the single-peer local stack. Safe default: status.
dev action="status" profile="isolated" seed="false" build="false":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ action }}" in
      start)
        args=()
        [[ "{{ seed }}" == "true" ]] && args+=(--seed)
        [[ "{{ build }}" == "true" ]] && args+=(--build)
        case "{{ profile }}" in
          isolated) export NETWORK_PROFILE=isolated ;;
          alpha) export NETWORK_PROFILE=join-alpha ;;
          *) echo "dev profile must be isolated|alpha" >&2; exit 2 ;;
        esac
        exec "{{ app_dir }}/scripts/hc-start.sh" "${args[@]}"
        ;;
      conductor)
        # The T3 hybrid rung: `just dev conductor profile=alpha` = a workspace conductor joined to
        # alpha's network (sovereign peer). The profile mapping is the same as `start` — before
        # 2026-08-28 this arm ignored `profile`, so profile=alpha silently started an isolated node.
        # Usage: just dev conductor alpha [CONDUCTOR_RELEASE_CHANNELS=<channel>=observe]
        case "{{ profile }}" in
          isolated) export NETWORK_PROFILE=isolated ;;
          alpha) export NETWORK_PROFILE=join-alpha ;;
          *) echo "dev profile must be isolated|alpha" >&2; exit 2 ;;
        esac
        exec "{{ app_dir }}/scripts/hc-start.sh" --conductor
        ;;
      app) cd "{{ app_dir }}"; exec pnpm start ;;
      stop)
        pkill -x holochain 2>/dev/null || true
        fuser -k 8888/tcp 8090/tcp 8095/tcp 2>/dev/null || true
        find "{{ root }}/elohim/holochain/local-dev" -maxdepth 1 -name '.hc_live_*' -delete 2>/dev/null || true
        echo "local stack stopped"
        ;;
      status) just --justfile "{{ root }}/justfile" status runtime ;;
      *) echo "dev action must be start|conductor|app|stop|status" >&2; exit 2 ;;
    esac

# Manage or measure the local multi-peer mesh. Safe default: status.
mesh action="status" *args:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{ action }}" in
      start|stop|status|probe|prologue) exec "{{ app_dir }}/scripts/hc-mesh.sh" "{{ action }}" ;;
      quiesce) exec "{{ app_dir }}/scripts/hc-mesh-quiesce.sh" ;;
      monitor) exec python3 "{{ app_dir }}/scripts/hc-mesh-monitor.py" ;;
      matrix) exec "{{ app_dir }}/scripts/hc-mesh-transport-matrix.sh" ;;
      recovery) exec "{{ app_dir }}/scripts/hc-mesh-recovery.sh" {{ args }} ;;
      recovery-matrix) exec "{{ app_dir }}/scripts/hc-mesh-recovery-matrix.sh" ;;
      # Restart arms (ratchet lane D, rung D2 pawls — 2026-08-28): the same hc-mesh.sh actions the
      # recovery harness drives, reachable through the verb so a shift never has to know the script.
      conductors-restart) exec "{{ app_dir }}/scripts/hc-mesh.sh" conductors-restart ;;
      storage-restart) exec "{{ app_dir }}/scripts/hc-mesh.sh" storage-restart {{ args }} ;;
      join-peer) exec "{{ app_dir }}/scripts/hc-mesh.sh" join-peer {{ args }} ;;
      *) echo "mesh action must be start|stop|status|probe|prologue|quiesce|monitor|matrix|recovery|recovery-matrix|conductors-restart|storage-restart [peer...]|join-peer <fresh-name>" >&2; exit 2 ;;
    esac

# Seed content or validate a corpus facet (profile: local|alpha|mesh). False content dry-run modes are intentionally absent.
seed action="validate" profile="local" scope="content" limit="":
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{ seeder_dir }}"
    export CONDUCTOR_URLS="${CONDUCTOR_URLS:-ws://localhost:4445}"
    case "{{ profile }}" in
      local) ;;
      alpha)
        export DOORWAY_URL=https://doorway-alpha.elohim.host
        export DOORWAY_API_KEY=dev-elohim-auth-2024
        export STORAGE_URL=https://storage-alpha.elohim.host
        export HOLOCHAIN_ADMIN_URL='wss://doorway-alpha.elohim.host?apiKey=dev-elohim-auth-2024'
        export ADMIN_PROXY_URL=https://doorway-alpha.elohim.host
        ;;
      mesh)
        # The household mesh THIS host owns (`just mesh start`): the seed-chain env comes from
        # hc-mesh.sh's `mesh_seed_env` — the one source of truth `just test mesh` and the CI mesh
        # stage already read (DOORWAY_URL/STORAGE_URL/DOORWAY_API_KEY/ADMIN_PROXY_URL + the cast's
        # CONDUCTOR_URLS). `set +e` around the source: hc-mesh.sh is `set -u` only and its
        # optional-binary probes exit non-zero at load.
        set +e
        # shellcheck source=/dev/null
        source "{{ app_dir }}/scripts/hc-mesh.sh"
        set -e
        mesh_seed_env
        export ADMIN_PROXY_URL="${ADMIN_PROXY_URL:-$DOORWAY_URL}"
        ;;
      *) echo "seed profile must be local|alpha|mesh" >&2; exit 2 ;;
    esac
    case "{{ action }}" in
      validate)
        case "{{ scope }}" in
          content) exec pnpm exec tsx src/schema-validation.ts ../data/lamad/content ;;
          all) exec pnpm run validate:all ;;
          collectives|humans|presences|account-packages|devices|deployments|corpora)
            exec pnpm run "validate:{{ scope }}"
            ;;
          *) echo "validation scope must be content|all|collectives|humans|presences|account-packages|devices|deployments|corpora" >&2; exit 2 ;;
        esac
        ;;
      apply)
        case "{{ scope }}" in
          content)
            args=()
            if [[ -n "{{ limit }}" ]]; then
              [[ "{{ limit }}" =~ ^[0-9]+$ ]] || { echo "seed limit must be a positive integer" >&2; exit 2; }
              args+=(--limit "{{ limit }}")
            fi
            exec pnpm exec tsx src/seed.ts "${args[@]}"
            ;;
          conductors) exec pnpm exec tsx src/seed-conductor-identities.ts ;;
          agent-bindings) exec pnpm exec tsx src/seed-agent-bindings.ts ;;
          humans) exec pnpm exec tsx src/seed-humans.ts ;;
          collectives) exec pnpm exec tsx src/seed-collectives.ts ;;
          presences) exec pnpm exec tsx src/seed-presences.ts ;;
          accounts) exec pnpm exec tsx src/seed-accounts.ts ;;
          nodes) exec pnpm exec tsx src/seed-nodes.ts ;;
          epr-atoms) exec pnpm exec tsx src/seed-epr-atom.ts ;;
          commitments) exec pnpm exec tsx src/seed-commitments.ts ;;
          household) exec pnpm exec tsx src/seed-household-formation.ts ;;
          delegates) exec pnpm exec tsx src/seed-delegates-compute.ts ;;
          test-admin) exec pnpm exec tsx src/seed-test-admin.ts ;;
          *) echo "apply scope must be content|conductors|agent-bindings|humans|collectives|presences|accounts|nodes|epr-atoms|commitments|household|delegates|test-admin" >&2; exit 2 ;;
        esac
        ;;
      stats) exec pnpm exec tsx src/stats.ts ;;
      diagnose) exec pnpm exec tsx src/diagnose.ts ;;
      *) echo "seed action must be validate|apply|stats|diagnose" >&2; exit 2 ;;
    esac

# Render a page or the Graphos component system.
look kind="page" target="":
    #!/usr/bin/env bash
    set -euo pipefail
    cd "{{ a2o_dir }}"
    case "{{ kind }}" in
      page) [[ -n "{{ target }}" ]] || { echo "usage: just look page <url>" >&2; exit 2; }; exec pnpm look "{{ target }}" ;;
      graphos) [[ -n "{{ target }}" ]] || { echo "usage: just look graphos <list|story|sheet>" >&2; exit 2; }; exec pnpm graphos "{{ target }}" ;;
      *) echo "look kind must be page|graphos" >&2; exit 2 ;;
    esac

# Show the runtime, habits, saga, CI preview, seed, or RAM-guard state.
status view="all":
    #!/usr/bin/env bash
    set -euo pipefail
    runtime() {
      for endpoint in 'Doorway|http://localhost:8888/health' 'Storage|http://localhost:8090/health'; do
        name="${endpoint%%|*}"; url="${endpoint#*|}"
        if curl -sf -m 2 "$url" >/dev/null; then printf '%-10s UP\n' "$name"; else printf '%-10s down\n' "$name"; fi
      done
      "{{ app_dir }}/scripts/hc-mesh.sh" status 2>/dev/null || true
    }
    case "{{ view }}" in
      runtime) runtime ;;
      habits) python3 "{{ root }}/.claude/scripts/habits-status.py" --full ;;
      saga) python3 "{{ root }}/.claude/scripts/saga-status.py" ;;
      ci) node "{{ root }}/genesis/orchestrator/preview.mjs" origin/dev ;;
      seed) cd "{{ seeder_dir }}"; exec pnpm exec tsx src/stats.ts ;;
      ram) exec python3 "{{ root }}/genesis/agentic/bin/ram-guard" status ;;
      all) runtime; python3 "{{ root }}/genesis/agentic/bin/ram-guard" status --brief || true; python3 "{{ root }}/.claude/scripts/habits-status.py" ;;
      *) echo "status view must be all|runtime|habits|saga|ci|seed|ram" >&2; exit 2 ;;
    esac

# Generate or verify derived interfaces. Safe default: verify.
codegen target="all" mode="verify":
    #!/usr/bin/env bash
    set -euo pipefail
    [[ "{{ mode }}" == "verify" || "{{ mode }}" == "write" ]] || { echo "codegen mode must be verify|write" >&2; exit 2; }
    run_target() {
      case "$1:{{ mode }}" in
        schema:verify) pnpm run schema:codegen:ts -- --verify; pnpm run schema:codegen:rs -- --verify ;;
        schema:write) pnpm run schema:codegen:ts; pnpm run schema:codegen:rs ;;
        domains:verify) pnpm run manifest:codegen:verify; pnpm run lamad:codegen:verify; pnpm run imagodei:codegen:verify; pnpm run shefa:codegen:verify; pnpm run qahal:codegen:verify; pnpm run avodah:codegen:verify ;;
        domains:write) pnpm run lamad:codegen; pnpm run imagodei:codegen; pnpm run shefa:codegen; pnpm run qahal:codegen; pnpm run avodah:codegen ;;
        routes:verify) pnpm run route-claims:codegen:verify ;;
        routes:write) pnpm run route-claims:codegen ;;
        agents:verify) node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify ;;
        agents:write) node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs project --write-fixtures --write-runtime ;;
        wire-types:verify) pnpm run wire-types:generate; git diff --exit-code -- elohim/sdk/storage-client-ts/src/generated ;;
        wire-types:write) pnpm run wire-types:generate ;;
        elements:verify) pnpm run elements:codegen:verify ;;
        elements:write) pnpm run elements:codegen ;;
        rakia:verify) pnpm run rakia:codegen:rs:verify ;;
        rakia:write) pnpm run rakia:codegen:rs ;;
        *) echo "codegen target must be all|schema|domains|routes|agents|wire-types|elements|rakia" >&2; exit 2 ;;
      esac
    }
    if [[ "{{ target }}" == all ]]; then
      for target in schema domains routes agents elements rakia; do run_target "$target"; done
    else
      run_target "{{ target }}"
    fi

# ---- Private manifest runners -------------------------------------------------

# Executable contract probe for run-local-gate.sh's env handling. Not registered
# in any manifest; invoked directly by gate-runner.test.mjs so that
# `rustflags: ""` collapsing back to inherit can never regress silently.
_gate-selftest-env:
    @echo "RUSTFLAGS=[${RUSTFLAGS-<unset>}] CARGO_TARGET_DIR=[${CARGO_TARGET_DIR-<unset>}]"

_gate-elohim-library:
    cd app/elohim-library && pnpm exec eslint projects/elohim-service/src projects/lamad-ui/src projects/html5-app-plugin/src
    cd app/elohim-library/projects/elohim-service && pnpm exec tsc --noEmit && pnpm exec tsc --noEmit -p tsconfig.spec.json && pnpm exec vitest run

_gate-elohim-storybook:
    cd app/elohim-library && pnpm run build-storybook && pnpm run test-storybook:ci

_gate-elements-codegen:
    pnpm run elements:codegen:verify
    pnpm --filter elohim-core test

_gate-elohim-compute:
    cd elohim/elohim-compute && cargo fmt --check && cargo clippy -- -D warnings && cargo test

_gate-elohim-epr:
    cd elohim && cargo fmt --check && cargo clippy -p elohim-epr -p elohim-epr-rea -- -D warnings && cargo test -p elohim-epr -p elohim-epr-rea --all-targets

# `pnpm build` first: @elohim/epr ships no dist/ in the tree, so a consumer's
# import fails at module resolution rather than at type-check. Testing a package
# that cannot be imported proves the tests, not the package.
_gate-epr-ts:
    cd elohim/sdk/epr-ts && pnpm build && pnpm test

# elohim/eprfs is its own native workspace with no per-project justfile; the
# recipe lives here so the manifest's typed contract has a real target.
_gate-eprfs:
    cd elohim/eprfs && cargo fmt --check
    cd elohim/eprfs && cargo clippy --workspace --all-targets -- -D warnings
    cd elohim/eprfs && cargo test --workspace

_gate-seam-contracts:
    cd crates/seam-contracts && cargo test --all-features
    cd crates/seam-contracts && cargo clippy --all-features -- -D warnings
    cd crates/seam-contracts && cargo fmt --check
    cd crates/seam-contracts && cargo build --target wasm32-unknown-unknown --no-default-features

_gate-epr-storage:
    cd elohim/elohim-storage && cargo build && cargo test --test schema_contract && cargo test --test schema_contract_diesel_epr
    cd elohim/sdk/storage-client-ts && pnpm test

_gate-elohim-sdk:
    bash scripts/ci/elohim-sdk-feature-matrix.sh

_gate-schema-dna:
    pnpm run schema:check-dna

_gate-manifest-hygiene:
    cargo test --manifest-path elohim/holochain/tests/manifest-hygiene/Cargo.toml

_gate-sweettest-check:
    #!/usr/bin/env bash
    set -euo pipefail
    clang_include="$(ls -1d /usr/lib/clang/*/include 2>/dev/null | sort -V | tail -1)"
    BINDGEN_EXTRA_CLANG_ARGS="${BINDGEN_EXTRA_CLANG_ARGS:--I$clang_include}" OPENSSL_NO_VENDOR=1 \
      cargo check --manifest-path elohim/holochain/tests/sweettest/Cargo.toml --tests

_gate-schema-validate:
    pnpm run schema:validate

_gate-schema-codegen:
    pnpm run schema:codegen:ts -- --verify
    pnpm run schema:codegen:rs -- --verify
    pnpm run route-claims:codegen:verify

_gate-constants-sync:
    pnpm run schema:codegen:ts -- --verify
    cd genesis/seeder && pnpm exec vitest run src/__tests__/constants-sync.test.ts

_gate-genesis-a2o:
    cd genesis/a2o && pnpm run lint && pnpm run format:check && pnpm run typecheck

_gate-gherkin-prepush-lint:
    cd genesis/a2o && pnpm run lint:gherkin

_gate-reach-drift:
    node genesis/seeder/scripts/check-reach-drift.mjs

_gate-domain-types:
    #!/usr/bin/env bash
    set -euo pipefail
    for domain in imagodei infrastructure lamad qahal shefa avodah; do
      cargo check --manifest-path "elohim/sdk/domains/$domain/types/Cargo.toml"
    done

_gate-rakia-codegen:
    pnpm run rakia:codegen:rs:verify

_gate-rakia-validate:
    pnpm run rakia:schema:validate

_gate-cargo-coverage:
    pnpm run validate:cargo-coverage

_gate-pipeline-list-fresh:
    node genesis/orchestrator/scripts/generate-pipeline-list.mjs
    git diff --exit-code -- genesis/orchestrator/pipeline-list.json
