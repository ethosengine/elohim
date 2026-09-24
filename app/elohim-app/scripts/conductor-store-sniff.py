#!/usr/bin/env python3
"""conductor-store-sniff — what is a Holochain conductor's data dir made of, and what grows.

Reads a STOPPED (or quiescent) conductor data root offline and reports, per DNA:
file sizes (main / WAL / shm), row counts per table, actions by type and by hour,
distinct authors, op-type mix, validation-receipt fan-in, and the dominant entry
kinds (by printable-string prefix in the entry blob), so the growth driver has a
name rather than a byte count.

It is the durable form of the 2026-09-24 inspection recorded in
genesis/docs/content/elohim-protocol/architecture/2026-09-24-conductor-store-growth-report.md.

Usage:
    conductor-store-sniff.py <conductor-data-root> [--dna <prefix>] [--json] [--top N]
                             [--passphrase <pw>]   (default: $HC_PASSPHRASE or "test")
    "test" is only the household/sandbox dev passphrase; any real node supplies its own via $HC_PASSPHRASE.

    <conductor-data-root> is the dir holding databases/ (e.g. genesis/local-dev/household-dowell/conductors/matthew).

Prerequisites (pure-Python wheels; no system sqlcipher needed):
    pip install --target "$SCRATCH/py" pynacl sqlcipher3-binary && export PYTHONPATH="$SCRATCH/py"

How the key is opened: databases/db.key is base64url(nonce24 || secretbox(key32) || salt16);
the secretbox secret is argon2id(passphrase, salt, MODERATE/MODERATE) — mirrors
holochain_data/src/key.rs. The pragmas mirror key.rs::apply_pragmas. Read-only: the
connection is opened with mode=ro; a WAL-mode open still rebuilds the -shm index,
which is the same thing every conductor start does.

Never run this against a RUNNING conductor: a long read snapshot is exactly the
checkpoint-starvation the report describes.
"""
import argparse
import base64
import collections
import datetime
import json
import os
import re
import sys

try:
    import nacl.pwhash
    import nacl.secret
    import sqlcipher3
except ImportError as e:  # pragma: no cover
    sys.exit(f"missing dependency ({e}); see the module docstring for the pip line")

ACTION_TYPES = {1: "Dna", 2: "AgentValidationPkg", 3: "InitZomesComplete", 4: "Create",
                5: "Update", 6: "Delete", 7: "CreateLink", 8: "DeleteLink", 9: "OpenChain",
                10: "CloseChain"}
OP_TYPES = {1: "StoreRecord", 2: "StoreEntry", 3: "RegisterAgentActivity",
            4: "RegisterUpdatedContent", 5: "RegisterUpdatedRecord", 6: "RegisterDeletedBy",
            7: "RegisterDeletedEntryAction", 8: "RegisterAddLink", 9: "RegisterRemoveLink"}
MB = 1048576.0


def open_db(key_path, db_path, passphrase):
    buf = base64.urlsafe_b64decode(open(key_path).read().strip() + "==")
    nonce, cipher, salt = buf[:24], buf[24:72], buf[72:88]
    secret = nacl.pwhash.argon2id.kdf(
        32, passphrase.encode(), salt,
        opslimit=nacl.pwhash.argon2id.OPSLIMIT_MODERATE,
        memlimit=nacl.pwhash.argon2id.MEMLIMIT_MODERATE)
    key = nacl.secret.SecretBox(secret).decrypt(cipher, nonce)
    c = sqlcipher3.connect(f"file:{db_path}?mode=ro", uri=True)
    for p in (f"PRAGMA key=\"x'{key.hex()}'\"",
              f"PRAGMA cipher_salt=\"x'{salt.hex()}'\"",
              "PRAGMA cipher_compatibility=4",
              "PRAGMA cipher_plaintext_header_size=32"):
        c.execute(p)
    c.execute("select count(*) from sqlite_master")  # fail fast on a wrong passphrase
    return c


def fsize(p):
    try:
        return os.path.getsize(p)
    except OSError:
        return 0


def hour(ts_micros):
    return datetime.datetime.fromtimestamp(ts_micros / 1e6, datetime.UTC).strftime("%m-%d %H")


def entry_kind(blob):
    """Name an entry by the first content-type-ish printable string in its msgpack."""
    # msgpack: key "content_type", then a 1-byte fixstr or 2-byte str8 header, then the value
    m = re.search(rb"content_type.{1,2}?([a-z][a-z0-9:_-]{3,40})", blob)
    if m:
        return m.group(1).decode()
    m = re.search(rb"([a-z_]{4,24}):[A-Za-z0-9]", blob)
    if m:
        return m.group(1).decode() + ":"
    return "?"


# Table/column names come from the database itself and are interpolated into SQL,
# so only plain identifiers pass; anything else is skipped with a one-line warning.
IDENTIFIER = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")


def safe_identifier(name, what):
    if isinstance(name, str) and IDENTIFIER.match(name):
        return True
    print(f"warning: skipping {what} with a non-identifier name {name!r}", file=sys.stderr)
    return False


def sniff_dna(root, dna_file, passphrase, top):
    db = os.path.join(root, "databases", dna_file)
    out = {"dna": dna_file[4:-3], "main_mb": fsize(db) / MB, "wal_mb": fsize(db + "-wal") / MB,
           "shm_mb": fsize(db + "-shm") / MB}
    c = open_db(os.path.join(root, "databases", "db.key"), db, passphrase)
    tables = [r[0] for r in c.execute(
        "select name from sqlite_master where type='table' and name not like 'sqlite_%' and name not like '_sqlx%'")]
    out["page_count"] = c.execute("pragma page_count").fetchone()[0]
    rows = {}
    for t in tables:
        if not safe_identifier(t, "table"):
            continue
        n = c.execute(f'select count(*) from "{t}"').fetchone()[0]
        if n:
            cols = [k for k in (r[1] for r in c.execute(f'pragma table_info("{t}")'))
                    if safe_identifier(k, f"column of {t}")]
            expr = "+".join(f'coalesce(length("{k}"),0)' for k in cols) or "0"
            rows[t] = {"rows": n, "payload_mb": c.execute(f"select coalesce(sum({expr}),0) from \"{t}\"").fetchone()[0] / MB}
    out["tables"] = rows
    if "Action" in rows:
        out["actions_by_type"] = {ACTION_TYPES.get(k, k): v for k, v in
                                  c.execute("select action_type,count(*) from Action group by 1 order by 2 desc")}
        out["distinct_authors"] = c.execute("select count(distinct author) from Action").fetchone()[0]
        out["top_authors"] = [(base64.urlsafe_b64encode(b"\x84\x20\x24" + a).decode()[:16], n) for a, n in
                              c.execute("select author,count(*) from Action group by 1 order by 2 desc limit ?", (top,))]
        out["actions_per_hour"] = list(c.execute(
            "select substr(datetime(timestamp/1000000,'unixepoch'),6,8), count(*) from Action group by 1 order by 1"))
    if "ChainOp" in rows:
        out["ops_by_type"] = {OP_TYPES.get(k, k): v for k, v in
                              c.execute("select op_type,count(*) from ChainOp group by 1 order by 2 desc")}
        out["ops_unintegrated"] = c.execute("select count(*) from ChainOp where when_integrated is null").fetchone()[0]
    if "ValidationReceipt" in rows:
        out["receipts_per_op"] = dict(c.execute(
            "select n,count(*) from (select op_hash,count(*) n from ValidationReceipt group by 1) group by n"))
    if "Link" in rows:
        out["links_by_type_tag"] = [(zt, lt, (tag or b"")[:24].decode("latin1"), n) for zt, lt, tag, n in c.execute(
            "select zome_index,link_type,tag,count(*) from Link group by 1,2,substr(tag,1,24) order by 4 desc limit ?", (top,))]
    if "Entry" in rows:
        kinds = collections.Counter()
        for (blob,) in c.execute("select blob from Entry"):
            kinds[entry_kind(blob)] += 1
        out["entry_kinds"] = kinds.most_common(top)
        out["entry_size_modes"] = list(c.execute(
            "select length(blob),count(*) from Entry group by 1 order by 2 desc limit 5"))
    c.close()
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("root")
    ap.add_argument("--dna", help="only DNAs whose hash starts with this prefix")
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--top", type=int, default=10)
    ap.add_argument("--passphrase", default=os.environ.get("HC_PASSPHRASE", "test"))
    a = ap.parse_args()
    dbdir = os.path.join(a.root, "databases")
    if not os.path.isdir(dbdir):
        sys.exit(f"{dbdir}: not a conductor data root")
    report = {"root": a.root, "wasm_mb": (fsize(os.path.join(dbdir, "wasm.db")) + fsize(os.path.join(dbdir, "wasm.db-wal"))) / MB,
              "conductor_mb": (fsize(os.path.join(dbdir, "conductor.db")) + fsize(os.path.join(dbdir, "conductor.db-wal"))) / MB,
              "dnas": []}
    for f in sorted(os.listdir(dbdir)):
        if f.startswith("dht-") and f.endswith(".db") and (not a.dna or f[4:].startswith(a.dna)):
            report["dnas"].append(sniff_dna(a.root, f, a.passphrase, a.top))
    report["dnas"].sort(key=lambda d: -(d["main_mb"] + d["wal_mb"]))
    if a.json:
        json.dump(report, sys.stdout, indent=1, default=str)
        return
    print(f"{a.root}\n  wasm.db {report['wasm_mb']:.0f} MB (compiled modules, fixed per pin)   conductor.db {report['conductor_mb']:.1f} MB")
    for d in report["dnas"]:
        print(f"\n== DNA {d['dna'][:20]}…  main {d['main_mb']:.0f} MB  WAL {d['wal_mb']:.0f} MB  "
              f"(WAL/main {d['wal_mb'] / max(d['main_mb'], 0.1):.1f}x)  pages {d['page_count']}")
        for t, v in sorted(d["tables"].items(), key=lambda kv: -kv[1]["rows"])[:8]:
            print(f"   {v['rows']:9d} rows {v['payload_mb']:7.1f} MB  {t}")
        if "actions_by_type" in d:
            print("   actions:", d["actions_by_type"], f"authors={d['distinct_authors']}")
            print("   top authors:", d["top_authors"][:5])
            ph = d["actions_per_hour"]
            if ph:
                peak = max(ph, key=lambda r: r[1])
                print(f"   actions/hour: {len(ph)} buckets, first {ph[0]}, peak {peak}, last {ph[-1]}")
        if "ops_by_type" in d:
            print("   ops:", d["ops_by_type"], f"unintegrated={d['ops_unintegrated']}")
        if "receipts_per_op" in d:
            print("   receipts per op:", d["receipts_per_op"])
        if "links_by_type_tag" in d:
            print("   links (zome,type,tag,n):", d["links_by_type_tag"][:6])
        if "entry_kinds" in d:
            print("   entry kinds:", d["entry_kinds"][:8])


if __name__ == "__main__":
    main()
