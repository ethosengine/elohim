//! `eprfs` — the single-sourced content-addressing CLI. The one tool Python/JS shell out to when
//! they need a full CID over a file (no other language re-implements CID/base32 encoding — that is
//! the single-source invariant of the cite↔CID convergence, see
//! `genesis/docs/superpowers/specs/2026-07-12-cite-fingerprint-cid-convergence-design.md`).
//!
//! Usage:
//!   eprfs cid <path> [--body] [--short]
//!     --body   address the frontmatter-excluded, trimmed canonical body (byte-matches the Python
//!              cite oracle's body_of+strip) instead of the whole file bytes.
//!     --short  print the `sha256:hex16` short form (the cite fingerprint) instead of the CID.
//!
//! Bytes → raw codec 0x55 (`bafkrei…`): file bytes and document bodies are arbitrary bytes, not
//! canonical dag-cbor atoms.

use std::process::ExitCode;

use eprfs_core::BlobCid;

/// Split a document into `(has_frontmatter, body)`, then return the frontmatter-excluded, trimmed
/// body. Byte-matches the parent cite oracle (`cite_graph.body_of` + `.strip()`) and brit's
/// `engine::frontmatter::canonical_body` — the parity is pinned by the shared corpus test in
/// `.claude/scripts/_lib/__tests__` (Slice-3) and brit's `cite_parity.rs`.
fn canonical_body(content: &str) -> String {
    let body = match content.strip_prefix("---\n") {
        None => content, // no opening fence → whole thing is body
        Some(rest) => {
            if let Some(end) = rest.find("\n---\n") {
                &rest[end + "\n---\n".len()..]
            } else if rest.strip_suffix("\n---").is_some() {
                "" // frontmatter-only doc, empty body
            } else {
                content // unterminated frontmatter → treat all as body
            }
        }
    };
    body.trim().to_string()
}

fn usage() -> ExitCode {
    eprintln!("usage: eprfs cid <path> [--body] [--short]");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("cid") => run_cid(&args[1..]),
        _ => usage(),
    }
}

fn run_cid(args: &[String]) -> ExitCode {
    let mut path: Option<&str> = None;
    let mut body = false;
    let mut short = false;
    for a in args {
        match a.as_str() {
            "--body" => body = true,
            "--short" => short = true,
            s if s.starts_with("--") => {
                eprintln!("unknown flag: {s}");
                return usage();
            }
            s => {
                if path.replace(s).is_some() {
                    eprintln!("cid takes exactly one path");
                    return usage();
                }
            }
        }
    }
    let Some(path) = path else { return usage() };

    let raw = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("eprfs: cannot read {path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let bytes: Vec<u8> = if body {
        // canonical body is defined over UTF-8 text (lossy decode mirrors the Python oracle's
        // read_text(errors="replace")).
        canonical_body(&String::from_utf8_lossy(&raw)).into_bytes()
    } else {
        raw
    };

    let cid = BlobCid::compute_raw(&bytes);
    if short {
        println!("{}", cid.short_fingerprint());
    } else {
        println!("{cid}");
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_body_excludes_frontmatter_and_trims() {
        let c = "---\nid: foo\n---\n\n  body text  \n\n";
        assert_eq!(canonical_body(c), "body text");
    }

    #[test]
    fn no_frontmatter_is_whole_trimmed_body() {
        let c = "# Title\nno frontmatter here\n";
        assert_eq!(canonical_body(c), "# Title\nno frontmatter here");
    }

    #[test]
    fn frontmatter_only_doc_has_empty_body() {
        assert_eq!(canonical_body("---\nid: x\n---"), "");
    }

    #[test]
    fn short_form_of_body_matches_python_oracle() {
        // cite_graph.fingerprint_text("ok target body") — the same value eprfs-core pins.
        let body = canonical_body("---\nid: target-ok\n---\nok target body\n");
        assert_eq!(body, "ok target body");
        let cid = BlobCid::compute_raw(body.as_bytes());
        assert_eq!(cid.short_fingerprint(), "sha256:4e72cded3d6affd3");
    }
}
