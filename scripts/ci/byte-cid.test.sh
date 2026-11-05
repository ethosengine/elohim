#!/usr/bin/env bash
# lib/byte-cid.mjs pins the shared byte-profile vectors (raw + blake3-256, bafkr4i…):
# "hello world", "abc", "" — the same rows the kernel and the seeder pin.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
T="$(mktemp -d)"; trap 'rm -rf "$T"' EXIT
fail=0
want() { # <content> <expected cid>
  printf '%s' "$1" > "$T/f"
  got="$(node "$HERE/lib/byte-cid.mjs" "$T/f")"
  if [ "$got" = "$2" ]; then echo "ok   '$1' -> $got"; else echo "FAIL '$1': got $got want $2"; fail=1; fi
}
want 'hello world' bafkr4igxjga67jykbseaxdmmdgc5a5o3zp3htom2l6mrjznk7fvyggu6eq
want 'abc' bafkr4ideg6z2yocgkez77nr3outtvdnvjdcvqrs5phnqh7jvtrwnlpm5qu
want '' bafkr4ifpcne3t5pzugtkaqcn5i3nzskjtpfslsnnyejlpte2spfoihzsmi
[ "$fail" = 0 ] && echo "ALL PASS"; exit "$fail"
