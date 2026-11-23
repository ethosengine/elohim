#!/usr/bin/env bash
set -euo pipefail
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
bin=$script_dir/bin
unset EPR_BIN GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null GIT_EDITOR=true

setup() {
    tmp=$(mktemp -d "${TMPDIR:-/tmp}/landing-tests.XXXXXXXX")
    trap 'rm -rf -- "$tmp"' EXIT
    repo=$tmp/repo wt=$tmp/worktree
    mkdir "$tmp/hooks"
    git init -q -b source "$repo"
    git -C "$repo" config user.name 'Landing Test'
    git -C "$repo" config user.email landing@example.invalid
    git -C "$repo" config core.hooksPath "$tmp/hooks"
    git -C "$repo" config commit.gpgsign false
    echo base > "$repo/file"
    git -C "$repo" add file
    git -C "$repo" commit -qm base
    base=$(git -C "$repo" rev-parse HEAD)
    git -C "$repo" worktree add -qb topic "$wt" "$base"
}
source_commits() {
    echo one > "$repo/one"
    git -C "$repo" add one
    git -C "$repo" commit -qm one
    one=$(git -C "$repo" rev-parse HEAD)
    echo two > "$repo/two"
    git -C "$repo" add two
    git -C "$repo" commit -qm two
    two=$(git -C "$repo" rev-parse HEAD)
}
run() {
    rc=0
    "$@" > "$tmp/out" 2> "$tmp/err" || rc=$?
    output=$(cat "$tmp/out")
}
expect() {
    if [[ $rc != "$1" ]]; then
        cat "$tmp/out" "$tmp/err" >&2
        return 1
    fi
}
first_line() { [[ ${output%%$'\n'*} == 'Git fallback: '* ]]; }
pick_two() {
    source_commits
    run "$bin/landing-pick" -C "$wt" "$one" "$two"
    expect 0
    first_line
    [[ $(git -C "$wt" log --reverse --format=%s "$base..HEAD") == $'one\ntwo' ]]
    [[ ${output##*$'\n'} == "$(git -C "$wt" rev-parse --short HEAD)" ]]
}
pick_range() {
    source_commits
    run "$bin/landing-pick" -C "$wt" "$base..$two"
    expect 0
    [[ $(git -C "$wt" log --reverse --format=%s "$base..HEAD") == $'one\ntwo' ]]
}
main_checkout() {
    source_commits
    run "$bin/landing-pick" -C "$repo" "$one"
    expect 3
}
dirty() {
    source_commits
    echo dirty >> "$wt/file"
    run "$bin/landing-pick" -C "$wt" "$one"
    expect 3
    git -C "$wt" add file
    run "$bin/landing-pick" -C "$wt" "$one"
    expect 3
    run "$bin/landing-retrailer" -C "$wt" "$base"
    expect 3
    run "$bin/landing-tagged-tip" -C "$wt" --tag '[build:app]' --subject test --branch tagged
    expect 3
}
conflict_setup() {
    echo source > "$repo/file"
    git -C "$repo" add file
    git -C "$repo" commit -qm conflict
    conflict=$(git -C "$repo" rev-parse HEAD)
    echo topic > "$wt/file"
    git -C "$wt" add file
    git -C "$wt" commit -qm topic
    before=$(git -C "$wt" rev-parse HEAD)
    run "$bin/landing-pick" -C "$wt" "$conflict"
    expect 4
    [[ $'\n'$output$'\n' == *$'\nfile\n'* ]]
    git -C "$wt" rev-parse --verify CHERRY_PICK_HEAD >/dev/null
}
conflict_continue() {
    conflict_setup
    echo resolved > "$wt/file"
    git -C "$wt" add file
    run "$bin/landing-pick" -C "$wt" --continue
    expect 0
    [[ $(cat "$wt/file") == resolved ]]
    [[ $(git -C "$wt" rev-parse HEAD) != "$before" ]]
    ! git -C "$wt" rev-parse --verify CHERRY_PICK_HEAD 2>/dev/null
}
conflict_abort() {
    conflict_setup
    run "$bin/landing-pick" -C "$wt" --abort
    expect 0
    [[ $(git -C "$wt" rev-parse HEAD) == "$before" ]]
    [[ $(cat "$wt/file") == topic ]]
}
protected_branch() {
    source_commits
    git -C "$wt" branch -m dev
    run "$bin/landing-pick" -C "$wt" "$one"
    expect 3
}
retrailer() {
    source_commits
    "$bin/landing-pick" -C "$wt" "$one" "$two" >/dev/null
    before=$(git -C "$wt" rev-parse HEAD)
    cat > "$tmp/hooks/prepare-commit-msg" <<'HOOK'
#!/usr/bin/env bash
set -eu
[[ ! ${EPR_BIN+x} ]]
printf '\nTest-Trailer: present\n' >> "$1"
HOOK
    chmod +x "$tmp/hooks/prepare-commit-msg"
    run env EPR_BIN=must-be-unset "$bin/landing-retrailer" -C "$wt" "$base"
    expect 0
    first_line
    [[ $(git -C "$wt" rev-parse HEAD) != "$before" ]]
    commits=$(git -C "$wt" rev-list "$base..HEAD")
    count=0
    for commit in $commits; do
        [[ $(git -C "$wt" show -s --format=%B "$commit") == *'Test-Trailer: present'* ]]
        count=$((count + 1))
    done
    [[ $count == 2 ]]
}
remote_refusal() {
    source_commits
    "$bin/landing-pick" -C "$wt" "$one" "$two" >/dev/null
    git init --bare -q "$tmp/remote.git"
    git -C "$wt" remote add origin "$tmp/remote.git"
    git -C "$wt" push -q origin HEAD:published
    git -C "$wt" fetch -q origin
    before=$(git -C "$wt" rev-parse HEAD)
    run "$bin/landing-retrailer" -C "$wt" "$base"
    expect 3
    [[ $(git -C "$wt" rev-parse HEAD) == "$before" ]]
}
tagged_tip() {
    before=$(git -C "$wt" rev-parse HEAD)
    run "$bin/landing-tagged-tip" -C "$wt" --tag '[build:app]' --subject 'Build app' --branch tagged
    expect 0
    first_line
    tip=$(git -C "$wt" rev-parse tagged)
    [[ $'\n'$output$'\n' == *$'\n'"$tip"$'\n'* ]]
    [[ $(git -C "$wt" rev-parse HEAD) == "$before" ]]
    [[ $(git -C "$wt" rev-parse 'tagged^{tree}') == "$(git -C "$wt" rev-parse 'HEAD^{tree}')" ]]
    [[ $(git -C "$wt" show -s --format=%P tagged) == "$before" ]]
    [[ $(git -C "$wt" show -s --format=%B tagged) == $'Build app [build:app]\n\nNot for merge.' ]]
    [[ ${output##*$'\n'} == "$(git -C "$wt" rev-parse --short HEAD)" ]]
}
tagged_refusals() {
    run "$bin/landing-tagged-tip" -C "$wt" --tag bad --subject test --branch tagged
    expect 2
    git -C "$wt" branch tagged
    run "$bin/landing-tagged-tip" -C "$wt" --tag '[build:app]' --subject test --branch tagged
    expect 3
    run "$bin/landing-tagged-tip" -C "$wt" --tag '[build:app]' --subject $'bad\nsubject' --branch other
    expect 2
}
arguments() {
    for script in landing-pick landing-retrailer landing-tagged-tip; do
        run "$bin/$script"
        expect 2
        run "$bin/$script" -C "$wt" --unknown
        expect 2
    done
}
# Separate processes keep errexit active within each test function.
if [[ ${1:-} == --case ]]; then
    setup
    "$2"
    exit 0
fi
failures=0
for name in pick_two pick_range main_checkout dirty conflict_continue conflict_abort protected_branch retrailer remote_refusal tagged_tip tagged_refusals arguments; do
    if bash "$0" --case "$name"; then
        printf 'ok %s\n' "$name"
    else
        printf 'not ok %s\n' "$name"
        failures=$((failures + 1))
    fi
done
(( failures == 0 ))
