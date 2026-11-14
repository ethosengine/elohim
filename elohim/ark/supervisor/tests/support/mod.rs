use std::{fs, os::unix::fs::PermissionsExt, sync::Mutex};

use ark_supervisor::admission::{resolve_recorded, RecordedRequirement, ResolveGap};
use cid::Cid;
use elohim_epr::cid::compute_cid;
use host_contract::Requires;
use serde_json::json;

static READER_ENV: Mutex<()> = Mutex::new(());

pub fn resolve(
    output: Cid,
    requires: Option<&Requires>,
    own: bool,
) -> Result<RecordedRequirement, ResolveGap> {
    let reply = json!({"output":output.to_string(), "requires":compute_cid(b"requirements").to_string(),
        "self":own, "produce":compute_cid(b"this device's Produce").to_string(), "path":"WRITE_PATH"});
    stub(
        output,
        requires,
        reply,
        if requires.is_some() { 0 } else { 3 },
        "",
    )
}

pub fn stub(
    output: Cid,
    requires: Option<&Requires>,
    reply: serde_json::Value,
    status: i32,
    stderr: &str,
) -> Result<RecordedRequirement, ResolveGap> {
    let _lock = READER_ENV.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path();
    fs::write(
        path.join("document"),
        serde_json::to_vec(&requires).unwrap(),
    )
    .unwrap();
    fs::write(path.join("reply"), reply.to_string()).unwrap();
    fs::write(path.join("stderr"), stderr).unwrap();
    let reader = path.join("epr");
    fs::write(
        &reader,
        format!(
            r##"#!/bin/sh
set -eu
[ "$1 $2 $3 $5 $7" = 'artifact requires --output --write --json' ]
[ "$4" = '{output}' ]
base="$(dirname "$0")"
cat "$base/stderr" >&2
if [ {status} != 0 ]; then exit {status}; fi
cp "$base/document" "$6"
sed "s|WRITE_PATH|$6|g" "$base/reply"
"##
        ),
    )
    .unwrap();
    fs::set_permissions(&reader, fs::Permissions::from_mode(0o700)).unwrap();
    let old = std::env::var_os("EPR_BIN");
    std::env::set_var("EPR_BIN", &reader);
    let result = resolve_recorded(output);
    match old {
        Some(value) => std::env::set_var("EPR_BIN", value),
        None => std::env::remove_var("EPR_BIN"),
    }
    result
}
