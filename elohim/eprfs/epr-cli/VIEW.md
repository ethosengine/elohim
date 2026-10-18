# Declared local views and work compositions

`epr view` reads repository-owned declarations without repository discovery, Git,
network services, or implicit workflow paths. It does not mint records or accept
work. JSON output has `projection`, `snapshot` and `comparison` members; failures
exit 2 with `diagnostics`. The default rendering is a concise native text reading.

```sh
epr view view.json --root /path/to/repository --json
epr view view.json --root /path/to/repository --save /private/view-store --json
epr view view.json --root /path/to/repository --baseline <cid> --store /private/view-store --json
epr view --snapshot <cid> --store /private/view-store --json
epr view --snapshot <cid> --store /private/view-store --source plots/north.md
```

Start with `tests/fixtures/views/orchard/view.json`. That fixture has no Elohim
configuration, `genesis/`, `.claude/`, workflow records, or Git repository. Change
its directory, metadata fields, selection and named readings without recompiling.
The production application is
`elohim/elohim-storage/.epr-meta/idle-is-free.view.json`, relative to this repository.

## Declaration

A declaration is a strict JSON object. Unknown fields, readers and operations refuse.
Its `version` is 1. `purpose` describes the reading, and `sources` maps caller-chosen
names to bindings:

- `path`: one repository-relative path or an array of paths. No absolute paths,
  parent traversal or symlinks, including ancestors. External references remain
  unresolved references; they are never fetched.
- `pattern`: optional basename glob. With it, each path is a recursively traversed
  directory; without it, paths name exact regular files.
- `reader`: `frontmatter`, `json`, `bytes`, `flows`, `actors`, `labels` or `habit`.
  Frontmatter and JSON supply subject fields. Plain documents without frontmatter
  supply no fields and cannot match a scalar or list selection. Bytes can retain arbitrary binary
  artifacts. Flow and actor logs use the existing CID-verified JSONL codecs;
  labels use the existing CID-to-path JSON mapping. These three readers require one
  exact file per binding, preserving each log's append order. Habit uses the native habit
  declaration validator, including its `<id>.habit.md` naming convention.
- `optional`: missing inputs yield explicit unavailable readings. Malformed
  existing inputs still refuse.

`subjects` names a frontmatter or JSON binding; `identity` names its stable string
identity field. It is independent of location and exact byte version. Duplicate
selected identities refuse. `fields` lists the metadata fields to display.

`select` is a bounded expression: `{"op":"eq","field":"kind","value":"task"}`,
`{"op":"contains","field":"tags","value":"quiet"}`, or
`{"op":"all|any","rules":[...]}`. Equality and membership use typed scalar values.
An optional `exclude` uses the same language, for example membership in an `index`
tag. The language contains no executable expressions. Missing displayed fields
are null; missing identity or required relationships refuse.

`relationships` maps names to `{field, source, target, required?}`. Each scalar or
list item in `field` must resolve uniquely in the target binding's metadata field.
`target: "$path"` uses native cite parsing to match a captured source path.
Unresolved relationships remain visible, including external references. A resolved
edge carries the exact target byte CID.

`readings` maps caller-chosen display names to `{reader, input, required?}`:

- `reconciliation`: existing native intent/commitment, production, technical review
  and independently appointed acceptance reading. It uses the subject's canonical
  body CID, the declared flow log, and only explicitly declared supporting bytes,
  actors and labels. Undeclared evidence cannot establish current acceptance.
- `notes`: native notes on the subject's body CID.
- `measurements`: native structured observations keyed by source path, or by the
  string metadata field named in `subject_field`. An optional `measure` selects
  an exact version pin. Environment labels remain data; no household/fleet columns
  or acceptance rule are invented.
- `habit`: the existing habit reading from its declared input. An optional
  `relationship` limits it to targets resolved by that relationship.

Absent structured evidence is unavailable, even if a narrative says work passed.
`required: true` makes such absence a diagnostic failure. Declarations cannot
promote an authored status, measurement, or technical approval into acceptance.

`bounds` requires `max_files`, `max_bytes` and `max_depth`. These apply before
source reads, including declaration and governance bytes. Hard ceilings are
100,000 entries, 256 MiB captured inputs and depth 32. Snapshot metadata and
tree nodes each have a separate 64 MiB budget, checked before writing. Selection expressions have
at most 16 nested levels and 64 children per combination. Special files refuse;
nonblocking no-follow descriptor traversal prevents FIFO hangs and symlink reads.

`capture: "selected"` retains selected subjects, declared supporting bindings,
resources actually consulted by native readers, the declaration and applicable
governance. `"all"` additionally retains unselected candidates. All consulted input
versions remain in provenance. Directory membership and input bytes are rechecked
before reporting success. This is optimistic consistency, not a filesystem-wide
transaction against arbitrary hostile writers.

## Saved sources and comparison

Saving is explicit. The local store contains immutable raw blobs and canonical
EPRFS directory trees, addressed by the existing `BlobCid` codec. The snapshot
retains the result, declaration, source-tree identity, input versions, resolved
governance and compiled native-reader source fingerprints. New store directories
are owner-only. Failed saves do not replace earlier snapshots; unreferenced objects
from an interrupted save may remain.

`projection.sources[path]` names the exact raw blob file in the store. Reading that
object opens the full source, including frontmatter; `view::reopen` verifies and
returns those same bytes to native consumers. Use `--snapshot <cid> --source <path>` to inspect a verified source directly. JSON
returns UTF-8 text or base64 for binary data, with its CID and encoding.
Reopening verifies every retained
blob and tree and never falls back to a checkout. The offline scenario suite can
run in a network namespace without interfaces:

```sh
cd genesis/a2o
unshare -n env EPR_BIN=/absolute/path/to/epr pnpm exec cucumber-js -p declared-views
```

Baseline comparison preserves the original cohort and includes each original
subject plus its current reading or an explicit missing state. Additions, missing
subjects, changed authored fields and changed readings are separate lists. A
missing subject is never counted as complete. Capture a new snapshot to retain
later edits; the previous CID and bytes remain usable.

## Local Avodah board (Sprint 2)

From the repository root:

```sh
just dev view elohim/elohim-storage/.epr-meta/idle-is-free.view.json
```

The launcher builds the production Angular app and the small `elohim-node`
`local-view-only` feature, starts the node's hub reader on loopback `8097` and
serves a pinned copy of the application on the existing Angular port `4200`.
It starts no conductor, swarm, agent service or alpha connection. It writes an
owner-only `launch-link.txt` under `genesis/local-dev/declared-views/<view-id>/`.
Open that link within five minutes. Its single-use fragment is exchanged for an
8-hour, HttpOnly, SameSite session and removed from browser history. The view ID
includes the repository/declaration binding, keeping different collections apart.
`LOCAL_VIEW_SKIP_BUILD=1` explicitly reuses an already built production bundle.

The board groups by authored status. Evidence, production, review and acceptance
remain native readings. Optional `presentation` bindings contain `titleField`,
`statusField`, ordered `columns` (`value`/`label`), `terminalValues`, `readings`,
and `reconciliationReading`. `$identity` binds the stable subject identity.
Bindings name declared fields/readings; 64 columns/values/readings and 256-byte
labels are the limits. Unknown statuses remain visible. Presentation never defines
acceptance rules. Old declarations and snapshots remain readable.

Opening and **Refresh** each capture once. Browsing observations does not capture.
History is private operational state, capped at 1,000 observations per binding;
a failed capture leaves previous observations readable. The chart starts with the
first observation's subject cohort unless another baseline is selected. Missing
subjects remain unknown, additions are separate, and only nonempty current native
assertion sets entirely reported `accepted` count as accepted. A declaration or
reader change remains in that observation's captured bytes and provenance.

**Keep offline** explicitly verifies and retains the selected collection's whole
history, exact source objects and one pinned app build, including lazy chunks and
fonts. The UI names the retained observation number; later refreshes do not update
that copy automatically. A failed save leaves the previous generation intact.
The worker is scoped to `/avodah/views/`; the existing `/apps/` worker is independent.
**Forget offline copy** removes that collection's retained copy. Session expiry
alone does not delete it.

### Workspace and doorway delivery

`LOCAL_VIEW_ORIGIN` is the exact externally visible origin of the existing Angular
endpoint, for example the workspace's HTTPS `angular-dev` URL. It is configuration,
not a new hostname convention; `app/workspace-runtime/workspace-doorway.ts` remains
the hostname boundary. All browser API requests are same-origin.

For a local doorway, set `LOCAL_VIEW_READER_URL=http://127.0.0.1:8097` on its process
and `LOCAL_VIEW_DOORWAY_URL=http://127.0.0.1:8888` on the launcher. The doorway only
relays `/api/local-views`, forwards the launcher capability unchanged, and never
caches the response. An isolated relay can run as:

```sh
LOCAL_VIEW_READER_URL=http://127.0.0.1:8097 doorway --local-view-relay 127.0.0.1:8888
LOCAL_VIEW_ORIGIN=https://YOUR-EXISTING-ANGULAR-ENDPOINT \
LOCAL_VIEW_DOORWAY_URL=http://127.0.0.1:8888 \
just dev view elohim/elohim-storage/.epr-meta/idle-is-free.view.json
```

Omit the doorway URL on a laptop: the same adapter proxies directly to the same
hub reader. Browser requests cannot select roots, declarations or filesystem paths.
`LOCAL_VIEW_ROOT`, `LOCAL_VIEW_ID`, `LOCAL_VIEW_PORT` and `LOCAL_VIEW_READER_PORT`
are launcher configuration only. To exercise orchard vocabulary, set
`LOCAL_VIEW_ROOT=.../elohim/eprfs/epr-cli/tests/fixtures/views/orchard`,
`LOCAL_VIEW_ID=orchard` and pass `board.view.json`.

**Delivery TODO — signed-in identity:** replace the launcher capability adapter
with the existing verified signed-in identity plus an owner binding. A private
launcher session is not a signed-in protocol identity. This does not change
projection semantics or introduce publication/acceptance authority.

### Verification

`just gate elohim-hub` verifies history, confinement, corruption, generated client
contracts and launcher access. The EPRFS, node, doorway and app gates own their
respective changes. The browser journey runs against a production build:

```sh
cd genesis/a2o
ELOHIM_NODE_BIN=/absolute/pool/path/local-view/elohim-node \
  pnpm exec cucumber-js -p local-view-board
```

It closes the entire browser, stops both serving processes, disables networking,
and reopens the retained board, chart and sources in the same browser profile.
Receipts and light/dark/cold-offline captures remain under
`genesis/a2o/reports/recovery/local-views/`.

## Domain compositions (version 2)

Avodah consumes a data-only composition of version 1 views. The coding application
lives in `elohim/rakia/work-composition/`; its app manifest declares
`workComposition.$ref`. The shared contract is
`elohim/sdk/schemas/v1/manifest/work-composition.schema.json`.

```sh
LOCAL_VIEW_ID=rakia just dev view elohim/rakia/work-composition/.epr-meta/progress.json
```

The launcher builds the native reader with the separate optimized `local-view`
Cargo profile. Its first build compiles dependencies; subsequent launches reuse
them. Unoptimized debug captures of repository-sized compositions can exceed a
workspace proxy's timeout. The production browser story requires both opening
and manual captures to return successfully within 60 seconds, independently of
whether an older observation can still render.

Each group names a repository-relative child view, a label, and a role (`focus`,
`work`, or `support`). Focus groups appear on Progress; work groups supply the
burndown cohort. Board can inspect any group. Explicit links join declared fields
and retain unresolved references. No domain names or status vocabulary are built
into Avodah. Orchard's `composition.json` uses the same reader and client.

Optional `conditionField`, `claimPrefix`, and `titlePrefix` bind source material.
Claims retain complete paragraphs, source lines and captured byte identities;
dated claims are never backfilled as historical observations. Conditions are
authored declarations, with native verification displayed separately. Tagged-text
views can read explicit Gherkin tags; selection supports `exists`, and `$path` is
an explicit identity option for documents without a declared ID.

Progress compares with the preceding observation by default, or an explicitly
chosen earlier observation. It distinguishes source changes, claims, newly
accepted assertions, reopened acceptance, missing subjects and lost evidence.
Changes to the composition, child declarations or native readers are labeled
interpretation changes. Counts never combine habits with their linked work.

Version 1 child views may declare bounded `evidence_roots`. The native reader
follows only explicit acceptance-report and evidence references associated with
selected subjects, captures their exact bytes, and uses the existing native
reconciler. Missing or excluded dependencies cannot become acceptance. Every child
declaration, app-manifest binding, source and reader version stays with the snapshot
and explicit offline copy. Version 1 snapshots remain readable without migration.

This first composition is labeled a local interpretation without an adopted
commons rule. It grants no authority to change work or adopt a shared policy.
Executable plugins, external issue import, publishing, editing, Gantt and waterfall
renderers remain future extensions. The Rakia package explicitly lists the current
habit directories; adding a new owning directory requires updating that declaration.

Run the complete native and production-browser story with:

```sh
cd genesis/a2o
EPR_BIN=/absolute/path/to/epr pnpm exec cucumber-js -p work-composition
```
