---
id: artifact-store-k9-two-device-receipt
status: active
habits: [sprint-lands-each-closure-once]
---

# K9 receipt: two devices diff their doorway input lines, 2026-11-12

Both of one person's devices built doorway through `just mesh build doorway` at the same source
commit 8b273f826, each with an `epr` built at that commit, and each recorded a crossing Produce.
Each then ran `epr artifact inputs diff <method descriptor> --device <the other>` with an `epr`
and peer daemon at dev 6060f8347. Both directions paired the same two records and printed the
same 11 differing lines, mirrored.

| | ethosengine | shem |
|---|---|---|
| device | `did:key:z6MkpZRh…VMZB` | `did:key:z6Mkm5de…bowj` |
| key | `bafyreihe2pgpl5c2kpsbduud6ykm347zbnx66hiibe6qyq3knzjttt5lhi` | `bafyreifeinwqu6ra7iqj3j7dxlutl5xrocsmjhxch3wrtnpxfadl7vnrlm` |
| input (document CID) | `bafkr4idc7yezxlwekw6d2zkv2dlqqdwqbhi6mkz6xudmq47ei7zhbjab7u` | `bafkr4ia6eoasc3ghidccfpyq45p72iwt5lcgpjnqq4xsa72ba6bbyqca74` |
| manifest | `bafyreicillwaz7zd7zpj7vvghdnml3d4xvpvphcypvz6dlog3ltybmn4sm` | `bafyreifsium2yd3u2y7544vdxskwk6zov4lgcjxckl5a42fgb4aekkr2wi` |
| doorway output | `bafkr4idqk2r47ztcwzq3bon7xkxbvkuxo5kfve7ldwmqpgc6wi6wbb7him` | `bafkr4iegrxripmxuvkkrwl4q76jjenva4vtxcf65wkgufcgzg7glw5g5su` |
| OS | RHEL 10.0 | AlmaLinux 10.2 |
| glibc, glibc-devel | 2.39-145.el10_2.x86_64 | 2.39-128.el10_2.alma.1.x86_64_v2 |
| gcc, libgcc | 14.3.1-4.4.el10.x86_64 | 14.3.1-4.4.el10.alma.2.x86_64_v2 |
| diff exit | 1 (22 lines) | 1 (22 lines) |

The differing lines, at identical paths on both devices: `link:Scrt1.o`, `link:crt1.o`,
`link:libc.so>/lib64/ld-linux-x86-64.so.2`, `link:libc.so>/lib64/libc.so.6`,
`link:libc.so>/usr/lib64/libc_nonshared.a`, `link:libc_nonshared.a`, `link:libgcc.a`,
`link:libgcc_s.so>/lib64/libgcc_s.so.1`, `link:libm.so>/lib64/libm.so.6`,
`link:libm.so>/lib64/libmvec.so.1`, `tool:link-driver:target` (`/usr/bin/gcc`). Every other
input line agrees, including the compiler's predefined-macro dump.

ethosengine's direction: [stdout](k9/inputs-diff-ethosengine.stdout),
[stderr](k9/inputs-diff-ethosengine.stderr), [exit](k9/inputs-diff-ethosengine.exit). shem's
direction is kept on shem at `genesis/local-dev/hostcompat-wave1/k9/inputs-diff-6060.{stdout,stderr,exit}`.

## Reading

- The plan predicted `link:Scrt1.o`, `link:crt1.o` and the macro dump. The macro dump agrees.
  What differs is the whole glibc and gcc link floor: the same upstream versions, built three
  ways apart (vendor rebuild, glibc release -145 against -128, and the base ISA build against
  the x86_64_v2 variant). Paths cannot tell these apart; content addressing does. This is the
  floor a declared-target build (K6) keys by CID.
- Both doorway outputs record; they differ, as their keys differ. shem rebuilt two hours later
  and reproduced the same output and key, a small reproducibility receipt.

## What the receipt took (defects found and fixed on the way)

1. **Crossing was denied on both devices.** Each peer daemon read
   `genesis/agentic/pool-policy.json` from a main checkout behind dev, whose
   `berth.channel.crosses` lacks `artifact-produce@1`. The allow-list is applied when a device
   projects its channel and again when it folds a sibling's, so no Produce crossed in either
   direction, silently. Both daemons were restarted with `--policy` at dev's file. The F2r
   `no-record` on 2026-11-09 (`f2r-fetch-ethosengine.md`) fits the same cause. Open: a daemon
   whose policy file is behind the device's head should say so.
2. **Pairing and serving chose "the newest record for this key".** A key does not change when
   only files outside its view change, so one device holds several records with one key and
   output that differ in commit and manifest. The diff paired on the wrong commit, and the
   daemon served the newest manifest instead of the one requested. Fixed at 228204ad6 and
   6060f8347: record-exact serving, pairing on the newest commit both devices recorded, and a
   serve index still bounded by distinct outputs.
3. **Open: a relay-only link hides the manifest-stage error.** `eprfs-peer/src/fetch.rs`
   reports any failed manifest request over a relay-only link as `transport-cannot-deliver`;
   an older daemon's rejection of the new request variant read that way.
4. **Open: the device's current epr release (caf7cf491, 2026-11-10) predates K9.** The receipt
   ran each daemon on an explicit `EPR_BIN` candidate (steward-approved for this receipt);
   ethosengine's daemon is back on the current release, with dev's policy.
