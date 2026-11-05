#!/usr/bin/env python3
"""Address-crossing codemod (regenesis-1 T0b): census | dry-run | apply.

Classes: address | digest | cite | oci | record | prose.  Only `address` hits in
mechanical shapes are ever proposed for change.  Read-only except `apply`
(writes the files of ONE track plus the receipt) and the report outputs.
Use python3 (not -I): PyYAML lives in site-packages.
"""
import argparse, base64, fnmatch, json, os, re, subprocess, sys, time
from collections import Counter, defaultdict
from pathlib import Path

import yaml

HERE = Path(__file__).resolve().parent
OUT_DEFAULT = HERE.parent
ROOT_DEFAULT = HERE.parents[3]
CLASSES = ["address", "digest", "cite", "oci", "record", "prose"]
TRACKS = ["DNA", "eprfs", "elohim-storage", "brit", "rakia+sdk", "doorway",
          "app/SPA", "seeder+CI", "a2o", "steward/node", "other"]

# ---------------------------------------------------------------- tracks
TRACK_PREFIXES = [  # first match wins (longest-specific first)
    ("elohim/holochain/dna/", "DNA"),
    ("elohim/eprfs/", "eprfs"),
    ("elohim/elohim-storage/", "elohim-storage"),
    ("elohim/brit/", "brit"),
    ("elohim/rakia/elohim/brit/", "brit"),
    ("brit/", "brit"),
    ("elohim/rakia/", "rakia+sdk"),
    ("rakia/", "rakia+sdk"),
    ("elohim/sdk/", "rakia+sdk"),
    ("crates/elohim-views/", "rakia+sdk"),
    ("doorway/", "doorway"),
    ("app/", "app/SPA"),
    ("genesis/seeder/", "seeder+CI"),
    ("scripts/ci/", "seeder+CI"),
    ("Jenkinsfile", "seeder+CI"),
    ("elohim/holochain/Jenkinsfile", "seeder+CI"),
    ("genesis/a2o/", "a2o"),
    ("steward/", "steward/node"),
    ("crates/consent-grant/", "steward/node"),
    ("crates/doorway-client/", "doorway"),
    ("genesis/scripts/ci/", "seeder+CI"),
]
BLOB_PLANE_TRACKS = {"DNA", "elohim-storage", "doorway", "seeder+CI", "app/SPA",
                     "a2o", "steward/node", "rakia+sdk", "eprfs"}


def track_of(rel):
    for p, t in TRACK_PREFIXES:
        if rel.startswith(p):
            return t
    return "other"


# ---------------------------------------------------------------- allowlist
def glob_re(g):
    out, i = "", 0
    while i < len(g):
        c = g[i]
        if g[i:i + 3] == "**/":
            out += "(?:.*/)?"; i += 3
        elif g[i:i + 2] == "**":
            out += ".*"; i += 2
        elif c == "*":
            out += "[^/]*"; i += 1
        elif c == "?":
            out += "[^/]"; i += 1
        else:
            out += re.escape(c); i += 1
    return re.compile("^" + out + "$")


class Allow:
    def __init__(self, path):
        d = yaml.safe_load(Path(path).read_text()) or {}
        self.excl_prefix = tuple(d.get("exclude_prefixes", []))
        self.excl_dirnames = set(d.get("exclude_dirnames", []))
        self.excl_globs = [glob_re(g) for g in d.get("exclude_globs", [])]
        self.keep_paths = [(glob_re(k["glob"]), k["class"]) for k in d.get("keep_paths", [])]
        self.keep_patterns = [(re.compile(k["regex"]), k["class"]) for k in d.get("keep_patterns", [])]
        self.skip_mech = [glob_re(g) for g in d.get("skip_mechanical_globs", [])]
        self.coupled = set(d.get("coupled_tracks", []))

    def excluded(self, rel):
        if rel.startswith(self.excl_prefix):
            return True
        if self.excl_dirnames & set(rel.split("/")[:-1]):
            return True
        return any(g.match(rel) for g in self.excl_globs)

    def keep_path_class(self, rel):
        for g, c in self.keep_paths:
            if g.match(rel):
                return c
        return None

    def skip_mechanical(self, rel):
        return any(g.match(rel) for g in self.skip_mech)


# ---------------------------------------------------------------- scanning
IDENTS = ["serverBlobHash", "server_blob_hash", "blobHashes", "blob_hashes", "blobHash", "blob_hash",
          "shard_hashes", "shardHashes", "shard_hash", "shardHash", "BlobHash", "ShardHash",
          "blake3Hash", "blake3_hash"]
RENAME = {"serverBlobHash": "serverBlobCid", "server_blob_hash": "server_blob_cid",
          "blobHashes": "blobCids", "blob_hashes": "blob_cids",
          "blobHash": "blobCid", "blob_hash": "blob_cid",
          "shard_hashes": "shard_cids", "shardHashes": "shardCids",
          "shard_hash": "shard_cid", "shardHash": "shardCid",
          "BlobHash": "BlobCid", "ShardHash": "ShardCid"}
IDENT_ALT = "|".join(sorted(IDENTS, key=len, reverse=True))
MASTER = re.compile(
    r"(?P<sha_dash>sha256-(?:[0-9a-fA-F]{6,}|\{|%s|\$))"
    r"|(?P<sha_colon>sha256:[0-9a-fA-F]{8,})"
    r"|(?P<b3_dash>blake3-(?:[0-9a-fA-F]{6,}|\{|%s|\$))"
    r"|(?P<bafkrei>bafkrei(?:[a-z2-7]{20,})?)"
    r"|(?P<bafyrei>bafyrei)"
    r"|(?P<ident>" + IDENT_ALT + r")"
    r"|(?P<content_hash>(?<![A-Za-z0-9])content_hash(?![A-Za-z0-9]))"
    r"|(?P<bare>(?<![0-9a-fA-F])[0-9a-f]{64}(?![0-9a-fA-F]))")
BARE_CTX = re.compile(r"/blob/|/shard/|/apps/|blobHash|blob_hash|shardHash|shard_hash|serverBlobHash|X-Blob-Hash", re.I)
CITE_CTX = re.compile(r"cites?:|fingerprint|drift_fingerprint", re.I)
OCI_CTX = re.compile(r"@sha256:|\bimage\b|docker|harbor|registry|oci|manifest\.json|imageDigest", re.I)
OCI_PATH = re.compile(r"(^|/)(Jenkinsfile[^/]*|Dockerfile[^/]*|devfile[^/]*\.ya?ml|.*\.groovy)$|^che-devworkspaces/")
DIGEST_CTX = re.compile(r"dedup|checksum|etag|integrity|\bsri\b|Sha256::|sha2::|hasher|digest|hash_key|cache_key|hmac|nonce", re.I)
BLOB_CTX = re.compile(r"blob|shard|cid|/apps/|content-address", re.I)
REGEX_FORM = re.compile(r"\^bafkrei|bafkrei\\?\[a-z2-7|bafkrei\{|bafkrei\)|\"bafkrei\"|'bafkrei'|`bafkrei`")
TEXT_EXT = {".md", ".mdx", ".txt", ".adoc", ".rst"}
CODE_EXT = {".rs", ".ts", ".tsx", ".js", ".mjs", ".cjs", ".sh", ".bash", ".py", ".json", ".yaml",
            ".yml", ".html", ".feature", ".graphql", ".groovy", ".toml", ".sql", ".bats"}
BIN_EXT = {".png", ".jpg", ".jpeg", ".gif", ".webp", ".ico", ".wasm", ".zip", ".gz", ".tgz", ".woff",
           ".woff2", ".ttf", ".pdf", ".dna", ".happ", ".bundle", ".so", ".o", ".rlib", ".sqlite",
           ".db", ".mp4", ".mp3", ".svg", ".webm", ".bin", ".lock"}
PROSE_PREFIXES = ("genesis/local-dev/", "genesis/docs/", "genesis/research/")
MAXSIZE = 1_500_000


def is_test_path(rel):
    parts = rel.split("/")
    name = parts[-1]
    if {"tests", "test", "__tests__", "fixtures", "fixture", "features", "testdata"} & set(parts[:-1]):
        return True
    return bool(re.search(r"\.(test|spec)\.|_tests?\.rs$|(^|/)tests?\.rs$|\.feature$|\.selftest\.", name))


class Hit:
    __slots__ = ("path", "line", "col", "end", "kind", "text", "match", "cls", "track", "allow",
                 "rule", "repl", "note")

    def __init__(self, **k):
        for s in self.__slots__:
            setattr(self, s, k.get(s))

    def row(self):
        return {s: getattr(self, s) for s in
                ("path", "line", "kind", "match", "cls", "track", "rule", "note", "text")}


def list_files(root, allow, walk):
    rels = []
    if not walk and (root / ".git").exists():
        try:
            out = subprocess.run(["git", "-C", str(root), "ls-files", "-z", "--recurse-submodules"],
                                 capture_output=True, check=True).stdout
            rels = [p for p in out.decode("utf-8", "replace").split("\0") if p]
        except Exception:
            rels = []
    if not rels:
        for dp, dns, fns in os.walk(root):
            relp = os.path.relpath(dp, root)
            relp = "" if relp == "." else relp
            dns[:] = [d for d in dns if d not in allow.excl_dirnames and d != ".git"
                      and not (relp + "/" + d + "/").lstrip("/").startswith(allow.excl_prefix)]
            for f in fns:
                rels.append((relp + "/" + f).lstrip("/"))
    return sorted(r for r in rels if not allow.excluded(r))


def read_text(p):
    try:
        if p.is_symlink() or not p.is_file() or p.stat().st_size > MAXSIZE:
            return None
        b = p.read_bytes()
    except OSError:
        return None
    if b"\0" in b[:8000]:
        return None
    return b.decode("utf-8", "replace")


def hex_to_blake3_cid(h):
    raw = bytes([0x01, 0x55, 0x1E, 0x20]) + bytes.fromhex(h)
    return "b" + base64.b32encode(raw).decode().lower().rstrip("=")


def classify(rel, track, ext, line, kind, m, ctx):
    """-> (cls, allowlisted)"""
    a = ctx["allow"]
    pc = a.keep_path_class(rel)
    if pc:
        return pc, True
    for rx, c in a.keep_patterns:
        if rx.search(line):
            return c, True
    sha16 = kind == "sha_colon" and re.fullmatch(r"sha256:[0-9a-fA-F]{16}", m)
    if ctx["md"]:
        if sha16 or CITE_CTX.search(line) and kind in ("sha_colon", "bafkrei"):
            return "cite", False
        return "prose", False   # all markdown/text is prose (fenced snippets included; see report)
    elif rel.startswith(PROSE_PREFIXES):
        if sha16:
            return "cite", False
        return "prose", False
    if kind == "bafyrei":
        return "record", False
    if kind == "sha_colon":
        if sha16:
            return "cite", False
        if OCI_PATH.search(rel) or OCI_CTX.search(line) and not BLOB_CTX.search(line):
            return "oci", False
        if CITE_CTX.search(line):
            return "cite", False
        return ("address", False) if BLOB_CTX.search(line) and not DIGEST_CTX.search(line) else ("digest", False)
    if kind == "bafkrei" and CITE_CTX.search(line):
        return "cite", False
    if kind == "bare":
        if not BARE_CTX.search(line):
            return None, False   # not an address hit
        return ("digest", False) if DIGEST_CTX.search(line) else ("address", False)
    if kind == "content_hash":
        return ("address", False) if track in BLOB_PLANE_TRACKS else ("digest", False)
    return "address", False


def scan_text(rel, text, allow):
    track = track_of(rel)
    ext = os.path.splitext(rel)[1].lower()
    is_md = ext in TEXT_EXT
    hits = []
    if not MASTER.search(text):
        return hits
    fence = False
    in_fm = text.startswith("---")
    fm_end = None
    lines = text.split("\n")
    for i, line in enumerate(lines, 1):
        if is_md:
            if line.lstrip().startswith("```"):
                fence = not fence
        if len(line) > 4000:
            line = line[:4000]
        for m in MASTER.finditer(line):
            kind = m.lastgroup
            s = m.group(0)
            if kind == "bafkrei" and REGEX_FORM.search(line) and len(s) == 7:
                kind = "bafkrei_regex"
            elif kind == "bafkrei" and len(s) == 7:
                kind = "bafkrei_prefix"
            elif kind == "bafkrei":
                kind = "bafkrei_literal"
            ctx = {"allow": allow, "md": is_md, "fence": fence}
            ckind = "bafkrei" if kind.startswith("bafkrei") else kind
            cls, al = classify(rel, track, ext, line, ckind, s, ctx)
            if cls is None:
                continue
            hits.append(Hit(path=rel, line=i, col=m.start(), end=m.end(), kind=kind, match=s,
                            text=line.strip()[:160], cls=cls, track=track, allow=al))
    return hits


def census_scan(root, allow, walk=False):
    hits = []
    for rel in list_files(root, allow, walk):
        if os.path.splitext(rel)[1].lower() in BIN_EXT:
            continue
        t = read_text(root / rel)
        if t is None:
            continue
        hits.extend(scan_text(rel, t, allow))
    return hits


# ---------------------------------------------------------------- mechanical plan
def plan_edits(rel, text, allow, hits=None):
    """Return list of edits {line, col, end, before, after, rule, cls} for this file."""
    if hits is None:
        hits = scan_text(rel, text, allow)
    ext = os.path.splitext(rel)[1].lower()
    notes = []
    edits = []
    if ext not in CODE_EXT:
        return edits, notes
    addr = [h for h in hits if h.cls == "address" and not h.allow]
    if not addr:
        return edits, notes
    if allow.skip_mechanical(rel):
        notes.append(("skipped", "coupled/generated/prose path (track-owned)", len(addr)))
        return edits, notes
    track = track_of(rel)
    test = is_test_path(rel)
    present = set(re.findall(r"[A-Za-z0-9_]+", text))
    for h in addr:
        if h.kind == "ident" and h.match in RENAME:
            tgt = RENAME[h.match]
            # collision: the target spelled as a standalone token already exists in this file
            if tgt in present:
                notes.append(("collision", f"{h.match} -> {tgt} (target already in file)", 1))
                continue
            edits.append(dict(line=h.line, col=h.col, end=h.end, before=h.match, after=tgt,
                              rule="ident-rename", cls="address"))
        elif h.kind == "bafkrei_regex":
            edits.append(dict(line=h.line, col=h.col, end=h.end, before=h.match, after="bafkr4i",
                              rule="regex-bafkrei", cls="address"))
        elif h.kind == "bafkrei_prefix" and ext in (".rs", ".ts", ".tsx", ".js", ".mjs", ".sh"):
            notes.append(("manual", "bare 'bafkrei' prefix (starts_with / comment): verify surface", 1))
        elif test and h.kind == "bafkrei_literal":
            edits.append(dict(line=h.line, col=h.col, end=h.end, before=h.match,
                              after="bafkr4i" + h.match[7:], rule="fixture-bafkrei", cls="address"))
        elif test and h.kind == "sha_dash" and re.fullmatch(r"sha256-[0-9a-fA-F]{64}", h.match):
            edits.append(dict(line=h.line, col=h.col, end=h.end, before=h.match,
                              after=hex_to_blake3_cid(h.match[7:].lower()),
                              rule="fixture-sha256", cls="address"))
        elif h.kind in ("sha_dash", "b3_dash", "bafkrei_literal", "bare", "content_hash", "ident"):
            notes.append(("manual", f"{h.kind} {h.match[:40]}", 1))
    edits.sort(key=lambda e: (e["line"], e["col"]))
    return edits, notes


def apply_edits(text, edits):
    lines = text.split("\n")
    by = defaultdict(list)
    for e in edits:
        by[e["line"]].append(e)
    for ln, es in by.items():
        s = lines[ln - 1]
        for e in sorted(es, key=lambda e: -e["col"]):
            s = s[:e["col"]] + e["after"] + s[e["end"]:]
        lines[ln - 1] = s
    return "\n".join(lines)


# ---------------------------------------------------------------- reports
def tally(hits):
    tc = {t: Counter() for t in TRACKS}
    for h in hits:
        tc[h.track][h.cls] += 1
    return tc


def md_table(tc):
    out = ["| track | " + " | ".join(CLASSES) + " | total |", "|---|" + "---:|" * (len(CLASSES) + 1)]
    tot = Counter()
    for t in TRACKS:
        row = [tc[t][c] for c in CLASSES]
        for c, v in zip(CLASSES, row):
            tot[c] += v
        out.append(f"| {t} | " + " | ".join(map(str, row)) + f" | {sum(row)} |")
    out.append("| **total** | " + " | ".join(f"**{tot[c]}**" for c in CLASSES) + f" | **{sum(tot.values())}** |")
    return "\n".join(out)


def cmd_census(a):
    root, allow = Path(a.root), Allow(a.allowlist)
    hits = census_scan(root, allow, a.walk)
    tc = tally(hits)
    files = Counter(h.path for h in hits if h.cls == "address")
    kinds = Counter((h.cls, h.kind) for h in hits)
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    js = {"generated": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "root": str(root),
          "totals_by_class": {c: sum(tc[t][c] for t in TRACKS) for c in CLASSES},
          "by_track_class": {t: dict(tc[t]) for t in TRACKS},
          "by_class_kind": {f"{c}/{k}": n for (c, k), n in sorted(kinds.items())},
          "address_files": dict(files.most_common()),
          "hits": [h.row() for h in hits]}
    (out / f"census-{a.label}.json").write_text(json.dumps(js, indent=1))
    md = [f"# Address census {a.label}", "", f"root `{root}`; {len(hits)} hits in "
          f"{len({h.path for h in hits})} files; generated {js['generated']}.", "",
          "## Track x class", "", md_table(tc), "", "## Class x kind", "",
          "| class | kind | hits |", "|---|---|---:|"]
    md += [f"| {c} | {k} | {n} |" for (c, k), n in sorted(kinds.items())]
    md += ["", f"## Per-file counts, class `address` ({len(files)} files)", "", "| hits | track | file |",
           "|---:|---|---|"]
    md += [f"| {n} | {track_of(p)} | `{p}` |" for p, n in files.most_common()]
    (out / f"census-{a.label}.md").write_text("\n".join(md) + "\n")
    print(md_table(tc))
    print("\nTop 15 files by address hits:")
    for p, n in files.most_common(15):
        print(f"{n:6d}  {p}")
    print(f"\nwrote {out}/census-{a.label}.md and .json")


def plan_all(root, allow, walk, track=None):
    plan = {}
    notes_all = defaultdict(list)
    for rel in list_files(root, allow, walk):
        if track and track_of(rel) != track:
            continue
        if os.path.splitext(rel)[1].lower() in BIN_EXT:
            continue
        t = read_text(root / rel)
        if t is None:
            continue
        hits = scan_text(rel, t, allow)
        edits, notes = plan_edits(rel, t, allow, hits)
        if edits:
            plan[rel] = edits
        if notes:
            notes_all[rel] = notes
    return plan, notes_all


def cmd_dry(a):
    root, allow = Path(a.root), Allow(a.allowlist)
    plan, notes = plan_all(root, allow, a.walk)
    per = defaultdict(Counter)
    rules = Counter()
    for rel, es in plan.items():
        for e in es:
            per[track_of(rel)][e["rule"]] += 1
            rules[e["rule"]] += 1
    nt = defaultdict(Counter)
    for rel, ns in notes.items():
        for k, why, n in ns:
            nt[track_of(rel)][k] += n
    md = ["# Dry-run 0 (mechanical classes only; nothing written)", "",
          f"{sum(rules.values())} substitutions in {len(plan)} files. Never touched: cite, oci, record, prose, "
          "digest, refs/notes, archive, .eprfs, genesis/data.", "",
          "## Per track", "", "| track | ident-rename | regex-bafkrei | fixture-bafkrei | fixture-sha256 | "
          "skipped (track-owned) | collision | manual |", "|---|---:|---:|---:|---:|---:|---:|---:|"]
    for t in TRACKS:
        c, n = per[t], nt[t]
        md.append(f"| {t} | {c['ident-rename']} | {c['regex-bafkrei']} | {c['fixture-bafkrei']} | "
                  f"{c['fixture-sha256']} | {n['skipped']} | {n['collision']} | {n['manual']} |")
    md += ["", "Identifier renames in tracks " + ", ".join(sorted(allow.coupled)) +
           " are coupled to schema/DNA; `apply` needs `--allow-coupled`.", "",
           "## Exact substitutions per file", ""]
    for rel in sorted(plan):
        md.append(f"### `{rel}` ({track_of(rel)})")
        text = (root / rel).read_text(errors="replace").split("\n")
        for e in plan[rel]:
            md.append(f"- L{e['line']} `{e['rule']}`: `{e['before']}` -> `{e['after']}`")
        md.append("")
    md += ["## Not mechanical (checklist for the owning track)", ""]
    for rel in sorted(notes):
        ns = notes[rel]
        agg = Counter()
        for k, why, n in ns:
            agg[(k, why)] += n
        for (k, why), n in sorted(agg.items()):
            if k == "manual" and why.startswith(("ident", "sha_dash", "bare", "content_hash", "b3_dash")):
                why = why.split(" ")[0]
            md.append(f"- `{rel}` [{k}] {why} x{n}")
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / f"dry-run-{a.label}.md").write_text("\n".join(md) + "\n")
    print(f"{sum(rules.values())} mechanical substitutions in {len(plan)} files; rules={dict(rules)}")
    print(f"wrote {out}/dry-run-{a.label}.md")


def cmd_apply(a):
    root, allow = Path(a.root), Allow(a.allowlist)
    if a.track not in TRACKS:
        sys.exit(f"unknown track {a.track}; one of {TRACKS}")
    plan, _ = plan_all(root, allow, a.walk, a.track)
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    rec = out / "codemod-receipt.jsonl"
    n = 0
    with rec.open("a") as rf:
        for rel, es in sorted(plan.items()):
            if a.track in allow.coupled and not a.allow_coupled:
                es = [e for e in es if e["rule"] != "ident-rename"]
            if not es:
                continue
            p = root / rel
            text = p.read_text(errors="replace")
            p.write_text(apply_edits(text, es))
            for e in es:
                rf.write(json.dumps({"path": rel, "class": e["cls"], "rule": e["rule"], "line": e["line"],
                                     "before": e["before"], "after": e["after"], "track": a.track}) + "\n")
                n += 1
    print(f"applied {n} substitutions for track {a.track}")


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--root", default=str(ROOT_DEFAULT))
    ap.add_argument("--out", default=str(OUT_DEFAULT))
    ap.add_argument("--allowlist", default=str(HERE / "allowlist.yaml"))
    ap.add_argument("--label", default="0")
    ap.add_argument("--walk", action="store_true", help="os.walk instead of git ls-files (tracked only)")
    sp = ap.add_subparsers(dest="cmd", required=True)
    c = sp.add_parser("census"); c.add_argument("--json", action="store_true"); c.set_defaults(f=cmd_census)
    d = sp.add_parser("dry-run"); d.set_defaults(f=cmd_dry)
    p = sp.add_parser("apply"); p.add_argument("--track", required=True)
    p.add_argument("--allow-coupled", action="store_true"); p.set_defaults(f=cmd_apply)
    a = ap.parse_args()
    a.f(a)


if __name__ == "__main__":
    main()
