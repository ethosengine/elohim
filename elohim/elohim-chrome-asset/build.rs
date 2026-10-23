//! Splices the generated chrome vocabulary (frame capabilities + page-info
//! sections, projected from the SDK schemas into `src/generated/`) into
//! `omni-element.js`, so the served element carries ONE vocabulary and no
//! hand-written table. The placeholder is `/*__CHROME_VOCABULARY__*/null`.

use std::{env, fs, path::PathBuf};

const PLACEHOLDER: &str = "/*__CHROME_VOCABULARY__*/null";

fn main() {
    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let js_path = dir.join("src/omni-element.js");
    let vocab_path = dir.join("src/generated/chrome-vocabulary.json");
    println!("cargo:rerun-if-changed={}", js_path.display());
    println!("cargo:rerun-if-changed={}", vocab_path.display());
    println!("cargo:rerun-if-changed=build.rs");

    let js = fs::read_to_string(&js_path).expect("read omni-element.js");
    let vocab = fs::read_to_string(&vocab_path)
        .expect("read src/generated/chrome-vocabulary.json (run `pnpm run schema:codegen:ts`)");
    // Compact the JSON onto one line so the served bytes stay small and stable.
    let value: serde_json::Value = serde_json::from_str(&vocab).expect("vocabulary is JSON");
    let compact = serde_json::to_string(&value).expect("compact vocabulary");
    assert_eq!(
        js.matches(PLACEHOLDER).count(),
        1,
        "exactly one vocabulary placeholder"
    );
    let out = js.replace(PLACEHOLDER, &compact);

    let out_path = PathBuf::from(env::var("OUT_DIR").expect("out dir")).join("omni-element.js");
    fs::write(out_path, out).expect("write spliced element");
}
