---
title: Elohim Rust Style Guide
id: elohim-rust-style-guide
status: Accepted
class: process-meta
created: 2026-10-22
updated: 2026-10-22
---

# Elohim Rust Style Guide

This is the target standard for first-party Rust in the Elohim repository. It applies to
libraries, services, command-line tools, Holochain zomes, tests, and build scripts. It covers
design and maintainability as well as formatting. New code must follow it; existing code
converges through the [debt ratchet](#rs-gov-02).

**Accepted standard does not mean fully automated enforcement.** The
[coverage table](#current-enforcement-coverage) records what existing tools check. The ratchet,
strict-scope selection, and exception validation described here are accepted requirements for
subsequent tooling work, not capabilities installed by this document.

Vendored code and submodules retain their own standards. Change generators rather than their
outputs. Generated Rust remains subject to compilation and applicable safety and interface
checks; generated formatting or naming exceptions must identify the generator and scope.

## Foundations and precedence

Follow the public Rust conventions wherever this guide does not explicitly depart from them:

- [The official Rust Style Guide](https://doc.rust-lang.org/style-guide/) defines Rust's
  formatting conventions. **rustfmt handles most formatting automatically**; use the
  repository's pinned toolchain and the owning project's configuration.
- [The Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) and their
  [checklist](https://rust-lang.github.io/api-guidelines/checklist.html) guide public API design,
  naming, documentation, interoperability, and compatibility. They require contextual judgment;
  a successful Clippy run does not establish API quality.
- [Google's Rust Style Guide](https://google.github.io/styleguide/rustguide.html) is the source
  adapted here. Its platform-specific instructions do not become Elohim requirements merely
  by appearing upstream. The [adoption map](#upstream-adoption-map) records every disposition.

Applicable repository governance and protocol contracts take precedence, followed by explicit
Elohim rules here, then the public Rust guides. A directory may tighten the standard. A necessary
departure must follow RS-GOV-04; local convention or existing debt alone is not justification.
Changes to an upstream website do not silently change our accepted rules or lint policy.

Prefer designs that a maintainer can understand, test, and change safely. Reuse an existing
trait or domain type before introducing another abstraction. Make failure, authority, ownership,
and resource lifetime visible. Every additional rule must justify its maintenance cost.

### How to read a rule

Rule IDs are permanent references for reviews, diagnostics, and exceptions. Do not reuse a
retired ID for a different requirement. **Must** is a requirement; **prefer** is the default
unless a concrete tradeoff supports another choice. An exception explicitly permitted in a rule
is part of that rule; broader departures need RS-GOV-04 documentation.

Each rule names its enforcement:

| Label                  | Meaning                                                                                                                                      |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| **Automatic, partial** | An existing compiler, rustfmt, or Clippy check catches a stated subset in the gate configurations that run it. Review still covers the rest. |
| **Planned**            | A specific detector or integration is identified but is not a repository-wide enforced requirement yet.                                      |
| **Review**             | Correctness depends on context; no claimed automatic check proves the rule.                                                                  |

Examples inside backticks are illustrative API shapes or expressions. The two fenced `rust`
examples are self-contained compilation examples; the `text` exception example is prose, not a
supported `.epr-meta` configuration format.

## Contents

- [Governance and gradual adoption](#governance-and-gradual-adoption)
- [Organization and imports](#organization-and-imports)
- [Formatting](#formatting)
- [Ownership and borrowing](#ownership-and-borrowing)
- [Function inputs, outputs, and errors](#function-inputs-outputs-and-errors)
- [Types and invariants](#types-and-invariants)
- [Traits and construction](#traits-and-construction)
- [Concurrency and lifecycle](#concurrency-and-lifecycle)
- [Macros and generated interfaces](#macros-and-generated-interfaces)
- [Naming](#naming)
- [Documentation](#documentation)
- [Safety and API evolution](#safety-and-api-evolution)
- [Implementation practices](#implementation-practices)
- [Testing and repository integration](#testing-and-repository-integration)
- [Current enforcement coverage](#current-enforcement-coverage)
- [Upstream adoption map](#upstream-adoption-map)
- [Attribution and maintenance](#attribution-and-maintenance)

## Governance and gradual adoption

<a id="rs-gov-01"></a>

### RS-GOV-01 — Apply requirements at the actual boundary

Published libraries and APIs consumed across crates must account for downstream compatibility.
Private application code can choose simpler concrete types when callers and changes are controlled
together. Rust visibility, wire compatibility, and protocol authority are separate boundaries:
`pub` alone does not mean an independently published API, and a private serialization type can
still define a public wire contract.

For example, a storage implementation may return `Vec<Peer>` internally while a shared library
offers an iterator. A wire enum change still needs schema and consumer verification. Review the
boundary before applying generic API advice. **Enforcement: Review.**

<a id="rs-gov-02"></a>

### RS-GOV-02 — Block new violations and reduce existing debt

For each activated rule, compare findings with the baseline at the change's integration merge
base, normally against `origin/dev`. Fail any new, unexcepted finding. Removing an unrelated
finding does not buy permission to introduce another.

A crate touched through Rust source, a Cargo manifest, or applicable Rust lint policy must remove
at least one existing finding if it has debt under active rules. Evaluate the complete change,
not every editor save. A crate with no debt must stay clean. Documentation-only changes and
crates compiled only because they are dependencies do not trigger paydown. A shared lint-policy
change touches its applicable crates; adding a rule uses the explicit rollout procedure below.

For example, a crate starting with twelve findings must finish with at most eleven, with no new
finding identities. Twelve different findings fail even though the count is unchanged. Fixing an
unexplained suppression by supplying a defensible reason resolves that hygiene finding; adding a
suppression solely to hide another finding is not paydown. Moving code or weakening a rule is
not paydown either. Actual deletion of obsolete code can resolve its findings.

Baselines shrink as debt is fixed. Never refresh a baseline to absorb ordinary new violations.
A new rule or toolchain migration gets an explicit, reviewed baseline migration, with its scope
and diagnostic differences recorded. That bootstrap does not demand unrelated cleanup in every
crate, and it must preserve enforcement of rules already active. **Enforcement: Automatic** for
RS-GOV-04 through the native Rust style gate. The first policy enrollment blocks new findings;
per-touch paydown starts once that policy is in the integration baseline.

<a id="rs-gov-03"></a>

### RS-GOV-03 — Keep refactored scopes strict

Designate a crate or filesystem-scoped module as strict before a major refactor. Its completion
requires zero unexcepted findings under active rules. Compile the owning crate with its required
configurations even when the strict boundary is smaller. Inline modules share their file's scope
until a validator explicitly supports semantic module selection.

Strictness persists after the refactor. For example, making `src/reconcile/` strict does not force
unrelated legacy debt in `src/p2p/` to disappear, but ordinary crate paydown still applies. Removing
or narrowing the designation is a policy change requiring a documented decision, not routine
cleanup. Necessary approved exceptions remain visible inside strict scopes.

Strictness covers active automated rules; review must also assess the rest of this standard.
Passing a zero-debt check does not certify architectural compliance. **Enforcement: Automatic**
for RS-GOV-04 through the permanent `rust-style-strict@1` binding described below.

<a id="rs-gov-04"></a>

### RS-GOV-04 — Explain necessary divergence

Keep local lint exceptions on the smallest relevant item, with a meaningful `reason`. Prefer
`#[expect(lint, reason = "...")]` when the finding must exist in that build configuration;
remove the expectation when it becomes unfulfilled. Use a reasoned `allow` when an expectation
would legitimately be absent in some supported configurations. Do not suppress an entire lint
group to resolve one diagnostic.

Record architectural, module-wide, or crate-wide divergence in the body of the owning `.epr-meta`
file (or `.epr-meta/manifest.md` for directory-form governance). Include the rule ID, precise
scope, constraint, rationale, rejected alternative and tradeoff, evidence or regression check,
responsible maintainer/team, and a revisit condition. Reference that record from the local
suppression where applicable. A rule's allowed local case needs no redundant exception ledger;
a wider architectural decision does.

Illustrative record, **not executable configuration**:

```text
Rule: RS-CON-01
Scope: src/cache.rs, Cache::reset_after_poison
Constraint: cache contents are disposable projections of verified input.
Decision: discard all poisoned cache state and rebuild before serving it.
Tradeoff: recovery preserves availability but loses the cached work.
Evidence: regression check proves no pre-reset entry can be returned.
Responsible: the crate's maintainers.
Revisit: if the cache acquires authoritative state or reset is no longer complete.
```

A reason must explain why the code is sound and useful; “needed for Clippy” explains neither.
A recovery exception cannot justify skipping a safety precondition, accepting invalid authority,
or misrepresenting verification. The existence of prose does not prove the decision is correct.

The first automation slice is exception hygiene.
The native syntax checker requires a nonempty literal `reason` on every `allow` and `expect`
attribute, including tests, inactive cfg branches, and literal attributes in macro bodies. Its
Git-derived ratchet blocks new unexplained attributes. Existing compiler gates with `-D warnings`
reject unfulfilled lint expectations in compiled configurations. **Enforcement: Automatic, partial**
for attribute reasons and compiled expectations; **Planned** for structured exception validation;
**Review** for justification. Cargo lint tables, command-line suppressions, procedural macro output,
and the meaning of a reason are outside this syntax check.

<a id="rs-gov-05"></a>

### RS-GOV-05 — Make enforcement visible and reproducible

When touching a Rust crate, surface its active rules, strict scopes, existing debt, current-change
paydown, exceptions, and measurement freshness. Edit-time advice may use cached results; it must
identify them as cached. Missing, skipped, failed, or stale runs are unmeasured, not clean.

Keep one decision authority behind local gates, pre-push, and CI. Use the existing
`build-manifest.json` project selection and native governance evaluator. A hook may explain the
policy; it must not pretend to have compiled the crate. Pin measurements to source, toolchain,
rule versions, target, and feature selection. For example, a passing default-feature run cannot
certify storage's iroh configuration. **Enforcement: Automatic, partial**: the syntax gate prints
per-crate before/after counts, touched state, strict-file counts and its Git baseline; `--json`
includes findings and failures. Agent adapters identify cached checker use and unmeasured calls.
Durable receipts for every compiler target/feature combination remain **Planned**.

## Organization and imports

<a id="rs-org-01"></a>

### RS-ORG-01 — Let crate boundaries represent real seams

Prefer cohesive crates with a clear purpose, dependency direction, and independently useful test
boundary. Do not split crates just to meet a line-count target or copy Chromium's build model.
For example, reuse a shared verification crate rather than duplicating its invariant in a
service. A small cohesive implementation may remain in its current crate. **Enforcement: Review.**

<a id="rs-org-02"></a>

### RS-ORG-02 — Use Cargo's module layout

Use ordinary Cargo source trees, with `lib.rs` or `main.rs` as entry points and domain modules
under `src/`. Both `peer.rs` with `peer/` children and an existing `peer/mod.rs` layout are valid;
avoid churn between equivalent layouts. Chromium's flat `rust_library` layout and `chromium::import!`
do not apply. Declare dependencies in Cargo and import through Rust paths. **Enforcement: Review.**

<a id="rs-org-03"></a>

### RS-ORG-03 — Name modules for the abstraction they contain

Use a primary type or a cohesive domain operation to name a leaf module: `binding_proof.rs` for
binding-proof logic, for example. Give a multi-type concern a descriptive domain name. Avoid
catch-all `utils` modules that accumulate unrelated behavior. Existing public module paths may
require a compatibility re-export during a rename. **Enforcement: Review.**

<a id="rs-org-04"></a>

### RS-ORG-04 — Keep module entry points focused on composition

Non-leaf modules should primarily declare children and expose the intended API. Put substantial
implementation in leaf modules so readers can distinguish the public surface from its machinery.
For example, `reconcile/mod.rs` can re-export a worker implemented in `reconcile/worker.rs`.
Small glue functions, inline tests, and binding modules are allowed. A large existing `mod.rs`
is refactoring debt, not justification for adding unrelated implementation. **Enforcement: Review**;
the existing source-size advisory is only a signal, not an implementation-placement validator.

<a id="rs-org-05"></a>

### RS-ORG-05 — Keep imported names attributable

Prefer explicit imports. `use super::*` in inline tests, locally scoped enum-variant imports,
and established framework preludes such as `hdk::prelude::*` or `hdi::prelude::*` are allowed.
Use explicit public re-exports to keep a library's API stable; a wildcard re-export needs a
deliberate facade or generated-binding rationale. Do not invent an application-wide prelude to
hide dependencies. **Enforcement: Planned**, `clippy::wildcard_imports` offers partial detection;
framework and facade applicability still needs review.

<a id="rs-org-06"></a>

### RS-ORG-06 — Group imports by their immediate module

Write `use std::collections::{BTreeMap, BTreeSet};` separately from `use std::path::Path;`.
Keep nested module paths out of the same brace group so edits have a predictable location.
Preserve generated imports at their source. **Enforcement: Review** today; do not claim stable
rustfmt enforces `imports_granularity = "Module"` on the pinned toolchain. An unstable formatter
option is not a reason to move the repository to nightly.

<a id="rs-org-07"></a>

### RS-ORG-07 — Scope trait imports to where they are used

Import a trait inside a function when only that function needs its methods; use a module-level
import when the trait is used throughout the module or in public bounds. For example,
`use std::fmt::Write as _;` can live inside the function that writes into a `String`. This
limits implicit method-resolution context. **Enforcement: Review.**

<a id="rs-org-08"></a>

### RS-ORG-08 — Qualify free functions and import ordinary type names

Prefer `fs::read(path)` after importing `std::fs`, and `Path` after importing `std::path::Path`.
This makes operations attributable while keeping type signatures readable. Qualify ambiguous
types such as `io::Result` or `sync::Mutex`, and use an explicit alias for unavoidable collisions.
Established framework helpers or tightly related local functions may remain unqualified when
their source is clear. **Enforcement: Review.**

## Formatting

<a id="rs-fmt-01"></a>

### RS-FMT-01 — Let the pinned rustfmt decide layout

Use `cargo fmt` in the owning workspace and its existing configuration; verify through the
project gate. Do not manually align code against formatter output. For example, accept the
formatter's wrapping of a long function call. Preserve the crate's declared edition; edition
migrations and formatter upgrades are explicit changes, not incidental cleanup.

The public default uses four-space indentation and a 100-character line width. Doorway's
checked-in configuration explicitly selects those values and edition 2021; it is not a global
configuration inherited by every Rust workspace. **Enforcement: Automatic, partial**, through
the projects that declare `cargo fmt --check`.

<a id="rs-fmt-02"></a>

### RS-FMT-02 — Justify formatter exclusions

Use `#[rustfmt::skip]` only on the smallest item whose deliberate layout communicates something
rustfmt cannot preserve, such as a readable protocol test vector. Add an adjacent explanation.
Do not skip whole modules to avoid fixing formatting. Generated output must be fixed or exempted
at its generator boundary. **Enforcement: Review**; reason validation is **Planned** and is not
provided by `allow_attributes_without_reason`.

## Ownership and borrowing

<a id="rs-own-01"></a>

### RS-OWN-01 — Choose shared ownership deliberately

Start with direct ownership and borrowed access. Use `Arc` for actual cross-task or cross-thread
shared ownership, and `Rc` only within a single-threaded boundary. For example, sharing a storage
service across Tokio tasks can require `Arc<Service>`; passing `&Service` to a synchronous helper
usually does not. Explain interior mutability, shutdown, and cycle avoidance where ownership
is shared. `Arc` alone does not make the inner type thread-safe. **Enforcement: Review.**

<a id="rs-own-02"></a>

### RS-OWN-02 — Borrow when ownership is unnecessary

Accept a borrow for read-only work and move values when the caller gives up ownership. Clone
when an independent lifetime, snapshot, or asynchronous task needs owned data; do not clone
merely to silence the borrow checker without understanding the lifetime. For example, borrow a
content ID for lookup but own it in a queued job. **Enforcement: Planned**, selected clone lints
such as `clippy::redundant_clone` can detect a subset; lifetime and snapshot intent need review.

## Function inputs, outputs, and errors

<a id="rs-api-01"></a>

### RS-API-01 — Borrow the data rather than its container

Prefer `&str`, `&[T]`, `&T`, and `Option<&T>` to unnecessary `&String`, `&Vec<T>`, `&Box<T>`,
and `&Option<T>`. For example, a CID lookup should accept the existing domain type or a string
slice, not require the caller to allocate a `String`. Keep the concrete container when its
identity, capacity, mutation, or specialized representation is part of the operation.
**Enforcement: Automatic, partial**, `clippy::ptr_arg` and `clippy::borrowed_box`.

<a id="rs-api-02"></a>

### RS-API-02 — Return iterators when deferred consumption helps callers

Reusable libraries should offer an iterator when callers can stream, filter, or stop early.
For example, expose `impl Iterator<Item = &Peer>` over stored peers when that borrow is suitable.
Return a collection when the implementation already owns one, callers need indexing or a
snapshot, or a collected result makes application code simpler. Do not hold a lock across
arbitrary caller iteration without documenting that cost. **Enforcement: Review.**

<a id="rs-api-03"></a>

### RS-API-03 — Accept iteration without requiring allocation

Use `IntoIterator` in reusable APIs that only traverse input; use a slice when contiguous access,
indexing, repeated passes, or straightforward application code benefits. For example, a batch
encoder may accept `impl IntoIterator<Item = Record>`, while a checksum over bytes accepts
`&[u8]`. Avoid extra type parameters with no caller benefit. **Enforcement: Review.**

<a id="rs-api-04"></a>

### RS-API-04 — Use conversion parameters where they improve the API

An `impl Into<T>` parameter is useful when a library genuinely accepts several owned forms.
Prefer a concrete type in internal functions with known callers. For example, a reusable builder
may accept `impl Into<String>`; a hot internal lookup should accept `&str`. Consider inference
quality, monomorphization, and whether the conversion allocates. **Enforcement: Review.**

<a id="rs-api-05"></a>

### RS-API-05 — Name choices and encode valid states

Use an enum for a domain choice whose Boolean spelling hides meaning: `SyncMode::VerifyOnly`
is clearer than a positional `true`. A Boolean remains appropriate for an actual predicate or
an obvious single flag. Couple dependent results in one type: return `Result<VerifiedClaims,
VerifyError>` rather than a `valid` flag plus optional claims. Do not let representable invalid
states leak across trust boundaries. **Enforcement: Review.**

<a id="rs-api-06"></a>

### RS-API-06 — Represent expected failures explicitly

Use `Option` for ordinary absence when no explanation is needed, and `Result` for failures the
caller must distinguish. Preserve structured error types and source context in libraries;
application boundaries may add operational context and map errors to their transport contract.
For example, distinguish missing content from corrupt content rather than returning `None` for
both. Network input, unavailable peers, malformed content, and denied authority are expected
failure modes, not reasons to panic. Avoid secrets in error messages.

A panic can signal a violated internal invariant that is a programming defect; it is not the
normal error channel. Tests may panic on failed assertions. Developer tools should still report
ordinary I/O and usage errors cleanly. **Enforcement: Review**, supplemented by RS-IMP-02/03.

## Types and invariants

<a id="rs-typ-01"></a>

### RS-TYP-01 — Use a newtype when the distinction matters

Reuse existing content-address and verified-value types. Add a newtype only for a distinct
invariant, capability, or unit that callers must not interchange; a type alias does not enforce
that distinction. For example, validated input should be constructed through a checked path,
not a public tuple field that bypasses validation. An alias is suitable for a convenient name
with identical semantics. **Enforcement: Review.**

<a id="rs-typ-02"></a>

### RS-TYP-02 — Give structured data named fields

Use `struct RetryPolicy { attempts: u32, delay: Duration }` instead of an unexplained tuple of
two integers. Single-field newtypes and small conventional tuples such as `(key, value)` are
fine. Named fields make additions and reviews safer, particularly for wire types; serialization
changes still need contract verification. **Enforcement: Review.**

## Traits and construction

<a id="rs-trt-01"></a>

### RS-TRT-01 — Implement traits for real contracts

Add implementations that callers need or an intentionally published contract requires. Reuse
an existing trait before adding a near-duplicate. Prefer inherent methods for behavior on one
concrete type; introduce a trait for an actual polymorphic boundary, including a meaningful
test seam. For example, use an existing storage interface for a second backend rather than
creating a parallel `StoreLike` abstraction. **Enforcement: Review.**

<a id="rs-trt-02"></a>

### RS-TRT-02 — Honor standard trait semantics and derive mechanical code

Use standard traits such as `Display`, `From`, `TryFrom`, and `Iterator` when their contracts fit.
Use derives when field-wise behavior is correct; implement manually when invariants, redaction,
or domain equality require it. For example, derive equality for a value object, but redact secrets
in its `Debug` output. Never use infallible `From` for a validation that can reject input.
**Enforcement: Automatic, partial**, `clippy::should_implement_trait`,
`clippy::derivable_impls`, and related correctness checks; semantics need review.

<a id="rs-trt-03"></a>

### RS-TRT-03 — Choose dispatch by the boundary's needs

Use generics or `impl Trait` for static composition; use `dyn Trait` when heterogeneous values,
runtime selection, or reduced code duplication justify dynamic dispatch. For example, a runtime
backend registry may hold trait objects while a small decoding helper is generic. Neither is
universally faster or simpler; measure important costs. **Enforcement: Review.**

<a id="rs-trt-04"></a>

### RS-TRT-04 — Keep extension traits recognizable and controlled

Name extension traits with `Ext`, keep them near the adapter that needs them, and prefer adding
an inherent method when we control the underlying type. A bridge may use a private extension
trait for a foreign type; a public extension API needs a stable purpose and collision review.
For example, `ForeignRecordExt` should translate a specific external representation, not inject
unrelated methods into every type. **Enforcement: Review.**

<a id="rs-trt-05"></a>

### RS-TRT-05 — Treat common trait implementations as promises

Use `Copy` for small values expected to remain copyable; do not discard useful error context to
make an error `Copy`. Use `Clone` only when duplication has sensible semantics, especially for
resource handles and authority-bearing types. Implement `Default` only for a valid, unsurprising
state. For example, a retry policy can default, but a verified grant cannot default into authority.

Implement `Eq` and `Hash` consistently for keys. Offer `Ord` only when there is a meaningful
total order, consistent with equality; use partial comparison when values are genuinely unordered.
Public-field value objects should expose useful common traits when field semantics support them.
**Enforcement: Automatic, partial**, including `clippy::new_without_default` and trait consistency
checks; the semantic commitments remain **Review**.

<a id="rs-trt-06"></a>

### RS-TRT-06 — Make important construction paths discoverable

Provide a named constructor for important conversions when it helps users find and understand
the invariant, alongside a suitable standard conversion trait. For example, a checked
`try_from_bytes` may accompany `TryFrom<&[u8]>`. Use `new`, `with_capacity`, `from_bytes`, or
another conventional descriptive name; `try_new` is useful when distinguishing a fallible path.
Elohim does not require renaming every established `new() -> Result<...>` API. Avoid redundant
constructor aliases with no discoverability benefit. **Enforcement: Review.**

<a id="rs-trt-07"></a>

### RS-TRT-07 — Give optional configuration safe defaults

Options structures should implement `Default` when all fields have safe defaults, enabling
`Options { attempts: 3, ..Options::default() }`. Put required identity, authority, and connection
material in required constructor arguments instead of fabricating defaults. For an external
`#[non_exhaustive]` options type, provide constructors or builders because downstream struct
literals and update syntax are unavailable. **Enforcement: Review**, with partial default-related
Clippy checks where applicable.

## Concurrency and lifecycle

<a id="rs-con-01"></a>

### RS-CON-01 — Do not silently trust poisoned state

A poisoned synchronization primitive signals that protected invariants may have failed. Do not
silently call `into_inner()` and continue using that state. Propagate a meaningful failure or
terminate the affected operation according to the service contract. For example, poisoned
authorization or replay-protection state must not produce an allow decision.

Recovery is permitted only when the owning code can discard, reconstruct, or revalidate the
whole affected invariant before reuse. Document that decision under RS-GOV-04 and test it.
Choosing a non-poisoning lock does not itself establish recovery safety. **Enforcement: Review**;
semantic detection is **Planned**, and a text search is only an audit aid.

<a id="rs-con-02"></a>

### RS-CON-02 — Use the owning runtime and bound concurrent work

Tokio and `async`/`.await` are supported in our native services. Follow the crate's runtime
contract, keep blocking work off executor workers, and make cancellation, task failure,
shutdown, and queue bounds explicit. For example, a supervisor must observe a worker's failure
rather than discard its join handle and assume success. Holochain zomes follow host API and WASM
execution constraints; do not transplant a native runtime into them.

Use `std::thread` for a justified dedicated native thread with a lifecycle owner. Chromium's
thread and async prohibitions do not apply. **Enforcement: Review.**

<a id="rs-con-03"></a>

### RS-CON-03 — Select locks by access pattern

Use the existing synchronization primitive appropriate to the boundary. Keep synchronous lock
scopes short and do not hold their guards across `.await`. Async locks are appropriate when an
asynchronous critical section is necessary, but still require contention and cancellation review.
For example, copy a small snapshot under a lock before awaiting network I/O. Fuchsia's mutex
requirement does not apply. **Enforcement: Automatic, partial**, `clippy::await_holding_lock`
and `clippy::await_holding_refcell_ref`; these do not detect every deadlock or lifetime hazard.

## Macros and generated interfaces

<a id="rs-mac-01"></a>

### RS-MAC-01 — Keep generated behavior inspectable

Prefer ordinary functions and types for business logic. A small `macro_rules!` helper is suitable
for repetitive syntax; new procedural macros or external generators need a concrete benefit,
useful diagnostics, and a verification path. Established Serde, ts-rs, Holochain, and error derives
are supported tools, not exceptions to remove mechanically.

For example, preserve the Rust-to-TypeScript pipeline: edit the authoritative schema/type,
regenerate, and verify the resulting interface. Do not fix generated casing in a client adapter
or manually edit generated bindings. Check cross-crate ts-rs import paths when moving types;
use the repository's coordinated migration discipline. **Enforcement: Automatic, partial** via
the applicable schema/codegen gates; macro design is **Review**.

## Naming

<a id="rs-nam-01"></a>

### RS-NAM-01 — Make names communicate Rust's ownership conventions

Use Rust casing conventions and standard conversion names: `as_bytes` for a cheap borrowed view,
`to_owned` for a borrowed conversion producing ownership, and `into_parts` for consumption.
Choose names consistent with the actual cost and receiver. Established trait or external API
names retain their contract. **Enforcement: Automatic, partial**, rustc's style lints and
`clippy::wrong_self_convention`.

<a id="rs-nam-02"></a>

### RS-NAM-02 — Give lifetime and generic parameters useful names

Use descriptive lifetimes for distinct domain relationships, such as `'session` and `'store`.
Elision, `'_`, and a local `'a` are fine for simple relationships. Use `T` for a genuinely generic
value; use `Reader`, `Codec`, or a concise bound-related name when the role matters. Do not turn
simple signatures into verbose naming exercises. **Enforcement: Review.**

<a id="rs-nam-03"></a>

### RS-NAM-03 — Prefer recognizable domain words

Avoid unexplained local abbreviations and names formed by removing letters. Established terms
such as CID, DHT, HTTP, and WASM are appropriate when their domain is clear; explain domain
vocabulary in module documentation. For example, `reconciliation_state` is preferable to
`rcn_st`. Extension traits follow RS-TRT-04; constructors follow RS-TRT-06.
**Enforcement: Review**, with partial casing checks from RS-NAM-01.

## Documentation

<a id="rs-doc-01"></a>

### RS-DOC-01 — Document the contract, failure modes, and boundaries

Public APIs must explain behavior a caller cannot infer from the signature: invariants,
side effects, absence, errors, panic conditions, units, and important resource costs. Use
`# Safety` for unsafe preconditions and `# Errors`/`# Panics` where relevant. Document trust and
verification guarantees where they matter. A cryptographic check's documentation must say what
was verified, not imply broader authority.

Use `//!` for crate/module context, `///` for item documentation, and `//` for implementation
notes. Use readable Markdown with intra-doc links and examples for meaningful workflows. Avoid
comments that merely repeat a name. A self-explanatory enum variant can rely on the enum's
contract; use a scoped reason if a local documentation lint requires an exemption.

Self-contained example (the payload check is illustrative, not an EPR verifier):

```rust
/// Rejects an empty payload before it enters a decoder.
///
/// # Errors
/// Returns an error if no bytes were supplied.
pub fn require_payload(bytes: &[u8]) -> Result<&[u8], &'static str> {
    if bytes.is_empty() {
        return Err("payload is empty");
    }
    Ok(bytes)
}
```

**Enforcement: Automatic, partial**, `clippy::missing_safety_doc` and locally enabled
`missing_docs`; **Planned** for broader documentation ratchets and selected
`clippy::missing_errors_doc`/`clippy::missing_panics_doc`. Meaning and completeness need **Review**.

## Safety and API evolution

<a id="rs-saf-01"></a>

### RS-SAF-01 — Isolate unsafe operations and explain their proof

Use safe Rust when it can meet the requirement. Keep unavoidable unsafe operations in a small,
reviewable abstraction, separate from domain decisions. Each unsafe block or implementation needs
an adjacent `SAFETY:` explanation of the actual preconditions and why they hold. An unsafe API
must document the caller's obligations. “FFI call” is not a safety argument.

For example, a native process hook must explain platform validity, lifetimes, and which operations
are permitted in that execution context. A performance justification needs a relevant benchmark.
Use explicit unsafe blocks even within unsafe functions; prefer `forbid(unsafe_code)` in crates
that require none, after checking macro and platform requirements. **Enforcement: Automatic,
partial** for existing safety-doc checks; **Planned** for
`clippy::undocumented_unsafe_blocks` and an explicit `unsafe_op_in_unsafe_fn` policy. Proof remains
**Review**; counting unsafe sites is not a proof.

<a id="rs-saf-02"></a>

### RS-SAF-02 — Use stable features and the declared toolchain

Do not introduce nightly language/library features or unstable formatter requirements into the
normal build. Follow the project's toolchain and minimum supported version when it declares one.
An unavoidable target-specific requirement needs an isolated, documented exception and a matching
build lane. For example, an embedded toolchain exception does not change all native service gates.
**Enforcement: Automatic, partial**, stable compilation rejects gated language/library features;
dependency and toolchain choices need **Review**.

<a id="rs-evo-01"></a>

### RS-EVO-01 — Make accessor behavior predictable

Use `value()` for access, `value_mut()` for mutable borrowing, `take_value()` for removal,
`set_value()` for replacement, and `replace_value()` when returning the previous value matters.
For closure accessors, use `with_value` or `with_mut_value` and document the callback's scope.
For example, a closure may avoid exposing a lock guard, but must document reentrancy and must
not call arbitrary user code under a lock without considering deadlocks. Existing builder-style
`with_*` methods are valid when their consuming receiver makes the distinction clear.
**Enforcement: Automatic, partial**, `clippy::misnamed_getters`; API semantics are **Review**.

<a id="rs-evo-02"></a>

### RS-EVO-02 — Provide the companion operations users expect

If a public collection offers `len`, also offer `is_empty`. Provide a coherent usable API rather
than disconnected conveniences. For example, test both operations against the same empty-state
invariant. A type whose length has no meaningful emptiness needs a documented exception.
**Enforcement: Automatic, partial**, `clippy::len_without_is_empty`.

<a id="rs-evo-03"></a>

### RS-EVO-03 — Respect hidden dependencies and constant-evaluation contracts

Treat another crate's `#[doc(hidden)]` items as implementation details unless an explicitly
supported generated-code contract requires them. Do not build on them because they happen to be
public. Mark a public function `const` only when constant evaluation is an intended commitment;
removing it can break callers. For example, a stable units constructor can be `const`, while an
operation likely to acquire runtime validation should not promise it casually. **Enforcement: Review.**

<a id="rs-evo-04"></a>

### RS-EVO-04 — Keep operators conventional

Implement arithmetic and logical operators only for behavior their notation reasonably denotes.
For example, adding quantities can be appropriate; using `+` to publish a record or grant a
capability is not. Prefer a named method when effects or failure deserve attention.
**Enforcement: Automatic, partial**, suspicious-operator Clippy checks; domain meaning is **Review**.

<a id="rs-evo-05"></a>

### RS-EVO-05 — Plan how public enums and structs evolve

Use `#[non_exhaustive]` for independently consumed enums expected to gain variants, and for
public-field structs expected to gain fields; provide usable constructors for those structs.
Closed ontologies and APIs whose consumers migrate together may remain exhaustive when that
choice is deliberate. For example, an extensible library error can be non-exhaustive while a
closed protocol state machine uses exhaustive matching to force coordinated changes.

Do not mass-apply the attribute to serialized schema types. It changes Rust source compatibility,
not wire-format versioning or validation; adding it to an existing public type can itself break
callers. Respect schema authority and verify generated clients. **Enforcement: Review**;
`clippy::exhaustive_enums` and `clippy::exhaustive_structs` are **Planned** candidates only for
scopes where their assumptions match the API.

<a id="rs-evo-06"></a>

### RS-EVO-06 — Make consequential return values hard to discard

Use `#[must_use]` when ignoring a return defeats the operation or loses important information.
Put it on a reusable result type, lazy computation, or guard when that expresses the contract;
otherwise put it on the function. `Result` already carries this protection. For example, a
verification predicate returning `bool` should explain that callers must check it; better still,
return the existing verified type when the boundary supports that contract.

Self-contained example:

```rust
#[must_use = "check the result before scheduling a retry"]
pub fn retry_budget_remaining(attempts: u32, limit: u32) -> bool {
    attempts < limit
}
```

Do not silence an important result with `let _ = ...` merely to pass the lint. Explicit discard
requires an understood, documented consequence. **Enforcement: Automatic, partial**,
`unused_must_use` checks annotated results; **Planned**, `clippy::must_use_candidate` can suggest
some missing annotations. Neither proves that a security decision was used correctly.

<a id="rs-evo-07"></a>

### RS-EVO-07 — Initialize runtime behavior explicitly

Start threads, initialize services, and perform I/O through the explicit startup path. Do not
use pre-`main` constructors to hide runtime effects. For example, initialize a service in its
composition root instead of through a global registration side effect. Compile-time constants
are fine; lazy initialization still needs a clear failure and lifecycle contract.
**Enforcement: Review**; targeted disallowed-API checks are **Planned** if needed.

## Implementation practices

<a id="rs-imp-01"></a>

### RS-IMP-01 — Use inference where the result remains clear

Annotate types at boundaries or when a conversion's result would otherwise surprise the reader.
For example, `let records: Vec<Record> = iterator.collect();` states an allocation and intended
collection. Let obvious local values infer their type; do not restate every literal's type.
**Enforcement: Review.**

<a id="rs-imp-02"></a>

### RS-IMP-02 — Match the state you need

Use `match`, `if let`, `let ... else`, or `?` rather than checking a variant and then unwrapping
it. For example, `let Some(peer) = peers.get(id) else { return Ok(None); };` ties the successful
branch to the value it uses. Use a domain result type when multiple fields must agree, as in
RS-API-05. **Enforcement: Automatic, partial**, `clippy::unnecessary_unwrap`; cross-field
invariants need **Review**.

<a id="rs-imp-03"></a>

### RS-IMP-03 — Reserve panicking extraction for proven invariants

Propagate expected errors instead of replacing `unwrap` mechanically with `expect`. When an
internal invariant makes failure a defect, use `expect` with the reason the value must exist,
such as `expect("the validated header contains a length")`. Keep the proof close to the call.
Tests may use panicking extraction, but messages should help diagnose fixture failures.

Do not impose a blanket ban on every `expect`; review each failure contract.
**Enforcement: Planned**, `clippy::unwrap_used` identifies panicking extraction for review;
`clippy::expect_used` may support selected strict scopes but is not a universal ban. No lint
establishes that the claimed invariant is true.

<a id="rs-imp-04"></a>

### RS-IMP-04 — Handle known variants explicitly

When matching another crate's non-exhaustive enum, list known variants and reserve the wildcard
for future ones unless the operation genuinely groups known and future cases. For example,
logging may group several failures, while authorization must define a safe unknown-case outcome.
Do not use a catch-all to conceal a newly meaningful case. **Enforcement: Review**; do not claim
stable enforcement from `non_exhaustive_omitted_patterns` merely because it appears in lint help.

<a id="rs-imp-05"></a>

### RS-IMP-05 — Make assertions explain the broken invariant

Use `assert_eq!`/`assert_ne!` when values are being compared and add context for ambiguous failures.
For example, include the non-sensitive operation and expected state, not a credential or whole
private record. Tiny tests with an already clear comparison do not need repetitive prose.
**Enforcement: Review**, with partial assertion-related Clippy assistance.

<a id="rs-imp-06"></a>

### RS-IMP-06 — Prefer the clearest control flow

Use iterator adapters for straightforward transformations and `for` loops when branching,
mutation, early exit, or error context is clearer. For example, a multi-step reconciliation
operation can use an explicit loop without contorting it into nested combinators. Avoid allocating
a collection merely to iterate it immediately unless it deliberately establishes a snapshot.
**Enforcement: Review**; an adapter suggestion is not a command to obscure domain logic.

<a id="rs-imp-07"></a>

### RS-IMP-07 — Limit panic containment to an explicit boundary

Do not use `catch_unwind` as an error channel or a substitute for input validation. A supervised
native boundary may use it as defense in depth only with a documented recovery invariant,
observed panic outcome, and tests that prevent reuse of possibly corrupted state. Explain any
use of `AssertUnwindSafe`. For example, discard a failed isolated worker's state before restarting
it; do not report its operation as successful.

Containment works only for unwinding panics; it does not catch aborts. No security, durability,
or correctness guarantee may depend on it being available. Holochain/WASM behavior must follow
its actual build and host contract. **Enforcement: Review**; targeted detection is **Planned**.

<a id="rs-imp-08"></a>

### RS-IMP-08 — Use standard formatting for constructed text

Prefer `format!` for substitutions into a new string and `write!`/`writeln!` when extending a
buffer or writing to a destination. For example, `format!("peer {peer_id}: {state}")` makes the
message structure visible. A simple `push_str` or two-part append is fine; use existing measured
buffering paths in hot loops. Handle write failures where they are possible.
**Enforcement: Automatic, partial**, formatting-related Clippy checks; readability is **Review**.

## Testing and repository integration

<a id="rs-tst-01"></a>

### RS-TST-01 — Prove the changed contract through its owning gate

Use `#[test]` for ordinary Rust tests, `#[tokio::test]` when asynchronous behavior needs a runtime,
and the repository's Holochain/sweettest and a2o lanes for their respective contracts. Chromium
and Fuchsia test frameworks do not apply. Test failure paths and invariants, including invalid
input, unavailable dependencies, and scoped recovery when they are relevant to the change.
Do not treat ignored or unrun tests as evidence.

Use `just gate <project-or-owning-path>` for the manifest's verification boundary. It resolves
native Cargo pool slots, toolchain, resource caps, and crate-specific `RUSTFLAGS`; DNA/WASM
workspaces retain their required target layout. For example, a native doorway command must
clear the environment's WASM-oriented flags, while storage retains its declared backend flags.
Dependency changes need executed tests, not only `cargo check`. Read the existing schema and
codegen commands before running a test that writes generated bindings.

**Enforcement: Automatic, partial**, existing manifest-declared gates; choosing adequate
behavioral coverage remains **Review**. This guide introduces no alternative project detector.

## Current enforcement coverage

Inspected on 2026-10-22 against the working tree. This is a coverage statement, not a claim that
all crates passed a fresh lint run. The local gate toolchain is pinned to **1.96.1** in
[`pool-policy.json`](../agentic/pool-policy.json); verify the pin when changing policy.

| Mechanism                                                         | Observed coverage                                                                                                                                                                                                | Limit                                                                                       |
| ----------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| rustfmt + Clippy with `-D warnings`                               | [Doorway](../../doorway/doorway-service/justfile), [storage](../../elohim/elohim-storage/justfile), [steward/node](../../steward/node/justfile), and several [root gate recipes](../../justfile) including EPRFS | Only selected packages, targets, and features are compiled; local suppressions still apply. |
| Storage feature linting                                           | Default configuration plus the explicit `p2p p2p-iroh` all-targets lane                                                                                                                                          | Does not imply every possible feature combination is tested.                                |
| Clippy configuration                                              | [Doorway `clippy.toml`](../../doorway/doorway-service/clippy.toml) sets cognitive complexity to 25 and argument count to 7                                                                                       | Parameters do not enable optional lints; this file is not a repository-wide policy.         |
| Documentation/unsafe floor                                        | Default `missing_safety_doc` in active Clippy lanes; [`seam-contracts`](../../crates/seam-contracts/src/lib.rs) separately forbids unsafe code and denies missing docs                                           | Does not establish identical coverage for all crates.                                       |
| Local/pre-push selection                                          | [`gate-runner.mjs`](../orchestrator/gate-runner.mjs) and [pre-push](../../.husky/pre-push.bash) use manifest project boundaries                                                                                  | Skipped/deferred heavy gates cannot count as a fresh style measurement.                     |
| CI                                                                | The [orchestrator](../orchestrator/Jenkinsfile) runs the native syntax gate before downstream dispatch; project pipelines retain Cargo/Clippy lanes.                                                             | First integration jobs without a prior successful baseline refuse as unmeasured.            |
| New-finding baseline, per-touch paydown, persistent strict scopes | [Native checker](../orchestrator/rust-style/) and [pre-push](../../.husky/pre-push.bash) enforce RS-GOV-04 across first-party `.rs` files.                                                                       | Reason quality and structured design-divergence review remain human responsibilities.       |

### Running and graduating the active gate

Run `just gate rust-style` for the full repository check and its regression tests. Every explicit
Cargo project gate also runs this style floor for that project's directory. The manifest's global
Rust/config inputs select the full gate when a Rust file is touched, including a new crate that
has not yet acquired its own compiler lane. This does not substitute for that crate's compiler gate.

For a focused report, use `bash genesis/orchestrator/rust-style.sh --scope path/to/crate --json`.
The default base is `origin/dev`; missing Git history, malformed Rust or missing policy is an
error, never a clean result. Working-tree checks include untracked first-party Rust files.
Pre-push checks the actual pushed commit for each ref; dirty fixes cannot hide committed findings.
Feature pushes and CI builds use the integration merge base. Pushes to `dev`/`main` compare with
the prior remote commit; their CI builds compare with the prior successful build. CI requires
those objects to exist, and refuses an integration job with no successful baseline. A rebuild of
an already successful commit can recheck its last commit. No first build silently uses `HEAD^`.

There is no editable debt count or baseline refresh command. Git supplies the old source, and
finding identity includes the crate owner, path, enclosing Rust item and normalized attribute,
with multiplicity retained. Line movement is harmless; moving debt to another path or Cargo
owner is a new finding. Touched crates with baseline debt must remove at least one finding.
Source, Cargo manifests, lint configuration and applicable `.epr-meta` edits count as touches;
documentation-only edits do not. Fixing a reason once satisfies the change, not just one save.

To graduate a crate or module, merge this binding into the existing owning manifest's `rules`
list (retain all existing rules and frontmatter):

```yaml
rules:
  - id: rust-style-strict
    policy: rust-style-strict@1
```

Place it at the crate root or the intended module directory. The native resolver owns inheritance;
`when` and `params` overrides are rejected for this binding. Strict scopes require zero unexplained
attributes and cannot be weakened relative to the baseline. The checker itself is the first strict
scope. A meaningful reason satisfies exception hygiene; it does not establish that the underlying
lint suppression is necessary or approve an architectural divergence.

The shared, package-owned agent adapter is registered for Claude and Codex. Reconstructable Write
and Edit calls get prospective native checks; shell calls and freeform patches receive an explicit
unmeasured advisory and the command to run. Missing or stale cached binaries also say unmeasured.
Run `just gate rust-style` to build/refresh the checker. Hook configuration must be loaded and trusted
by the runtime; pre-push and CI remain independent of an agent's tool interface or hook support.

[Policy configuration](../../.epr-meta/rust-style-policy.json) currently excludes only `vendor/`
(third-party source) and `.cargo-pool/` (build cache). Submodules are separate repositories and Git
supplies their gitlinks, not their internal source. All other first-party sources, including
code-generated files, are scanned. A policy or exclusion change requires an explicit checker
migration; version 1 rejects changes to that configuration after enrollment.

### Extending linting without creating a second enforcement system

Clippy supports named lint selection and parameters, not arbitrary new lint implementations in
`clippy.toml`. Use [Cargo lint tables](https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section),
compiler attributes, or gate arguments for lint levels. Workspace inheritance must be explicitly
enabled by members; our separate Cargo workspaces do not acquire one policy automatically.
[Clippy configuration](https://doc.rust-lang.org/clippy/configuration.html) searches upward for a
configuration file; it does not merge every ancestor's settings.

Select individual additional lints after checking their applicability on the pinned toolchain.
Do not turn on the complete `restriction` group, and do not assume all `pedantic` suggestions
match our design choices. `-D warnings` escalates active warnings; it does not activate every
allow-by-default lint. Preserve existing compiler failures and warning-denying checks while
introducing a baseline for newly selected rules.

Use Cargo's [JSON diagnostic messages](https://doc.rust-lang.org/cargo/reference/external-tools.html#json-messages)
for finding identity and attribution. If a rule cannot be expressed with existing tools, specify
a narrowly scoped native validator using syntax or semantic analysis appropriate to the rule.
A regex occurrence count can inform an audit, but cannot reliably distinguish Rust syntax,
macro expansion, tests, or justified behavior.

Directory policy belongs in the existing `.epr-meta` cascade. Reuse the native evaluator and
versioned policy registry; prove parameter resolution, supported selectors, and validator
registration before publishing a configuration example as executable. The schema does not
currently accept arbitrary `rust-style` keys, and a policy label is not evidence that a detector
runs. Agent-facing hooks/docs remain package-first where their package is authoritative.

The first active rollout is RS-GOV-04 exception hygiene. Subsequent independent candidates are unsafe
documentation, borrowed argument types, panic sites, documentation coverage, and consequential
return values. Each activation needs positive/negative fixtures, scope and feature coverage,
an explicit legacy baseline, and the same verdict through local and CI entry points. Acceptance
cases must include a new violation hidden by equal total counts, renamed/moved debt, false paydown
through suppression, stale expectations, strict-subtree debt, and missing measurements.

## Upstream adoption map

**Adopt** keeps the upstream intent. **Adapt** changes its scope or prescription for Elohim.
**Not applicable** rejects a platform-specific requirement and names the relevant replacement.
This table covers every normative upstream heading, including rules expressed at section level.
Background, scope, and goals are represented by this guide's introduction and foundations.

| Google guide topic                     | Decision                                                     | Elohim rule                          |
| -------------------------------------- | ------------------------------------------------------------ | ------------------------------------ |
| Public Rust style rules                | Adopt                                                        | Foundations; [RS-FMT-01](#rs-fmt-01) |
| API design in context                  | Adopt                                                        | [RS-GOV-01](#rs-gov-01)              |
| Small crates (Chromium)                | Adapt: cohesive Cargo seams, no mechanical size split        | [RS-ORG-01](#rs-org-01)              |
| Flat `rust_library` layout (Chromium)  | Not applicable: Cargo modules                                | [RS-ORG-02](#rs-org-02)              |
| Source/leaf-module names               | Adopt                                                        | [RS-ORG-03](#rs-org-03)              |
| Implementation in non-leaf modules     | Adapt: small composition glue allowed                        | [RS-ORG-04](#rs-org-04)              |
| Glob imports                           | Adapt: established Holochain preludes, controlled facades    | [RS-ORG-05](#rs-org-05)              |
| One module per import statement        | Adopt; review until stable automation is available           | [RS-ORG-06](#rs-org-06)              |
| Locally scoped trait imports           | Adopt                                                        | [RS-ORG-07](#rs-org-07)              |
| Qualified free-function calls          | Adapt: clear framework/local exceptions                      | [RS-ORG-08](#rs-org-08)              |
| Unqualified type names                 | Adopt, retaining ambiguity exceptions                        | [RS-ORG-08](#rs-org-08)              |
| Chromium import mechanism              | Not applicable: Cargo and ordinary Rust paths                | [RS-ORG-02](#rs-org-02)              |
| Formatting tool                        | Adapt: pinned project rustfmt                                | [RS-FMT-01](#rs-fmt-01)              |
| Formatting opt-outs                    | Adopt, require explanation                                   | [RS-FMT-02](#rs-fmt-02)              |
| Avoiding `Rc`/`Arc`                    | Adapt: deliberate runtime shared ownership                   | [RS-OWN-01](#rs-own-01)              |
| Borrowing versus deep copies           | Adopt                                                        | [RS-OWN-02](#rs-own-02)              |
| Flexible immutable references          | Adopt                                                        | [RS-API-01](#rs-api-01)              |
| Iterator versus collection returns     | Adopt, retain application/snapshot exceptions                | [RS-API-02](#rs-api-02)              |
| Iterator versus slice parameters       | Adopt                                                        | [RS-API-03](#rs-api-03)              |
| `Into` parameters                      | Adopt                                                        | [RS-API-04](#rs-api-04)              |
| Enums versus Boolean API choices       | Adopt                                                        | [RS-API-05](#rs-api-05)              |
| Error reporting                        | Adapt: ordinary tool/service failures remain explicit        | [RS-API-06](#rs-api-06)              |
| Newtypes versus aliases                | Adopt; reuse existing domain types                           | [RS-TYP-01](#rs-typ-01)              |
| Named fields versus tuples             | Adopt                                                        | [RS-TYP-02](#rs-typ-02)              |
| Adding trait implementations as needed | Adopt                                                        | [RS-TRT-01](#rs-trt-01)              |
| Standard traits versus custom methods  | Adopt                                                        | [RS-TRT-02](#rs-trt-02)              |
| Derives                                | Adopt, retain invariant/redaction exceptions                 | [RS-TRT-02](#rs-trt-02)              |
| Generics versus dynamic dispatch       | Adopt                                                        | [RS-TRT-03](#rs-trt-03)              |
| Trait versus inherent implementation   | Adopt                                                        | [RS-TRT-01](#rs-trt-01)              |
| Extension traits                       | Adapt: explicit bridge-local foreign-type extensions         | [RS-TRT-04](#rs-trt-04)              |
| Important/common traits                | Adapt: meaningful semantics and authority before convenience | [RS-TRT-05](#rs-trt-05)              |
| Important `From` constructors          | Adapt: useful discoverability, no redundant aliases          | [RS-TRT-06](#rs-trt-06)              |
| Options and `Default`                  | Adapt: no fabricated required authority/identity             | [RS-TRT-07](#rs-trt-07)              |
| Poison handling                        | Adapt: explicit, proven repair before reuse                  | [RS-CON-01](#rs-con-01)              |
| Async prohibition (Chromium)           | Not applicable: owning runtime contract                      | [RS-CON-02](#rs-con-02)              |
| Standard-thread prohibition (Chromium) | Not applicable: lifecycle-owned native threads               | [RS-CON-02](#rs-con-02)              |
| Fuchsia mutex                          | Not applicable: access-pattern-based primitives              | [RS-CON-03](#rs-con-03)              |
| Macros and generation                  | Adapt: established schema/framework derives supported        | [RS-MAC-01](#rs-mac-01)              |
| Method naming                          | Adopt                                                        | [RS-NAM-01](#rs-nam-01)              |
| Lifetime naming                        | Adopt                                                        | [RS-NAM-02](#rs-nam-02)              |
| Generic parameter naming               | Adopt                                                        | [RS-NAM-02](#rs-nam-02)              |
| Extension trait naming                 | Adopt                                                        | [RS-TRT-04](#rs-trt-04)              |
| Initialization naming                  | Adapt: conventional fallible constructors remain valid       | [RS-TRT-06](#rs-trt-06)              |
| Abbreviations                          | Adopt, with explained protocol vocabulary                    | [RS-NAM-03](#rs-nam-03)              |
| Documentation                          | Adapt: trust, wire, and operational contracts included       | [RS-DOC-01](#rs-doc-01)              |
| Unsafe code                            | Adopt; proof and measurement remain review obligations       | [RS-SAF-01](#rs-saf-01)              |
| Unstable features                      | Adapt: stable default, isolated target exceptions            | [RS-SAF-02](#rs-saf-02)              |
| Getters and setters                    | Adopt                                                        | [RS-EVO-01](#rs-evo-01)              |
| Closure accessors                      | Adopt, add reentrancy and lock review                        | [RS-EVO-01](#rs-evo-01)              |
| Expected companion APIs                | Adopt                                                        | [RS-EVO-02](#rs-evo-02)              |
| Foreign hidden items                   | Adopt, supported generator contracts distinguished           | [RS-EVO-03](#rs-evo-03)              |
| Public `const` functions               | Adopt                                                        | [RS-EVO-03](#rs-evo-03)              |
| Operator overloading                   | Adopt                                                        | [RS-EVO-04](#rs-evo-04)              |
| Non-exhaustive enums                   | Adapt: actual compatibility boundary and closed schemas      | [RS-EVO-05](#rs-evo-05)              |
| Non-exhaustive public-field structs    | Adapt: construction and schema compatibility                 | [RS-EVO-05](#rs-evo-05)              |
| Function `must_use`                    | Adopt                                                        | [RS-EVO-06](#rs-evo-06)              |
| Type `must_use`                        | Adopt                                                        | [RS-EVO-06](#rs-evo-06)              |
| Initialization before `main`           | Adopt                                                        | [RS-EVO-07](#rs-evo-07)              |
| Type inference                         | Adopt                                                        | [RS-IMP-01](#rs-imp-01)              |
| Matching versus check-then-unwrap      | Adopt                                                        | [RS-IMP-02](#rs-imp-02)              |
| `expect` versus `unwrap`               | Adapt: first reconsider whether failure should propagate     | [RS-IMP-03](#rs-imp-03)              |
| Known non-exhaustive variants          | Adopt, retain meaningful grouping exceptions                 | [RS-IMP-04](#rs-imp-04)              |
| Assertion context                      | Adopt                                                        | [RS-IMP-05](#rs-imp-05)              |
| Iteration readability                  | Adopt                                                        | [RS-IMP-06](#rs-imp-06)              |
| `catch_unwind`                         | Adapt: documented, supervised defense in depth only          | [RS-IMP-07](#rs-imp-07)              |
| Text formatting                        | Adopt                                                        | [RS-IMP-08](#rs-imp-08)              |
| Per-codebase test framework            | Adapt: Rust/Tokio, Holochain, and a2o lanes                  | [RS-TST-01](#rs-tst-01)              |

## Attribution and maintenance

Adapted from **Google's Rust Style Guide**, by Google and its contributors, under
[Creative Commons Attribution 3.0 Unported](https://creativecommons.org/licenses/by/3.0/).
The source repository's [license](https://github.com/google/styleguide/blob/2ac1584cd3c9854760f02418ec24687ec1698fb8/LICENSE)
and [reviewed guide revision](https://github.com/google/styleguide/blob/2ac1584cd3c9854760f02418ec24687ec1698fb8/rustguide.md)
are pinned to `2ac1584cd3c9854760f02418ec24687ec1698fb8`, retrieved 2026-10-22.
This adaptation rewrites the guidance, examples, platform applicability, governance, and rollout
requirements for Elohim. It is not endorsed by Google. This document is distributed under the
same CC BY 3.0 license; that notice does not change the license of repository code.

Maintain this document as the human-readable standard. Change a rule and its detector mapping
together when automation lands, retaining its stable ID and recording any policy version change.
Update the coverage date only after inspecting the relevant gate/configuration evidence. Link
the actual checks; do not let an intended gate become an “enforced” claim through documentation
alone. The [Rust Reference on lint reasons and expectations](https://doc.rust-lang.org/reference/attributes/diagnostics.html#lint-reasons)
defines attribute behavior; installed tools and fresh gate results determine actual enforcement.
