//! Apollo Federation v2 SDL codegen — manifest-driven.
//!
//! `generate_sdl_from_manifests` emits a full Apollo Federation v2 subgraph SDL
//! by concatenating `CORE_TYPES_SDL` with per-manifest extension blocks.
//!
//! For this sprint the implementation is template-driven (lamad + shefa
//! are the two registered manifests). Future sprints may generalize to fully
//! dynamic SDL emission by iterating `ManifestRegistry` entries.

#![cfg(feature = "graph-native")]

/// Generate Apollo Federation v2 subgraph SDL from the named manifests.
///
/// Core types are always emitted first. Each manifest name appends its
/// type-extension block. Unknown manifest names are silently ignored —
/// callers should validate names against `ManifestRegistry` before calling.
pub fn generate_sdl_from_manifests(manifest_names: &[&str]) -> String {
    let mut sdl = String::new();
    sdl.push_str(CORE_TYPES_SDL);
    for name in manifest_names {
        match *name {
            "lamad" => sdl.push_str(LAMAD_SUBGRAPH_SDL),
            "shefa" => sdl.push_str(SHEFA_SUBGRAPH_SDL),
            _ => {}
        }
    }
    sdl
}

/// Core Apollo Federation v2 schema: `@link` directive + `EprHead` entity +
/// shared types (`LamadContext`, `ShefaContext`, `QahalContext`) + query roots.
///
/// The `extend schema @link(...)` declaration is what makes this a valid
/// Federation v2 subgraph SDL. `@key(fields: "cid")` marks `EprHead` as a
/// federated entity that can be resolved by CID across subgraphs.
const CORE_TYPES_SDL: &str = r#"
extend schema
  @link(url: "https://specs.apollo.dev/federation/v2.3",
        import: ["@key", "@external", "@requires", "@shareable"])

type EprHead @key(fields: "cid") {
  cid: ID!
  slug: String!
  contentCid: String!
  version: Int!
  authorDid: String
  updatedAt: String!
  lamad: LamadContext
  shefa: ShefaContext
  qahal: QahalContext
}

type LamadContext {
  title: String!
  contentType: String!
  description: String
  contentFormat: String
  tags: [String!]!
}

type ShefaContext {
  stewards: [String!]!
  allocations: [Float!]!
}

type QahalContext {
  reach: String
  layer: String
  attestationRequirements: [String!]!
}

type Query {
  eprHead(cid: ID!): EprHead
  contributor(did: ID!): Contributor
}

type Contributor @key(fields: "did") {
  did: ID!
  displayName: String
}
"#;

/// Lamad subgraph SDL extension — adds `prerequisites` and `teaches` graph traversal
/// fields to `EprHead`. Both fields resolve through the CozoDB NEIGHBORHOOD primitive.
const LAMAD_SUBGRAPH_SDL: &str = r#"
extend type EprHead {
  prerequisites(maxDepth: Int = 3): [EprHead!]!
  teaches: [EprHead!]!
}
"#;

/// Shefa subgraph SDL extension — adds household topology and reciprocity flow
/// fields to `Contributor`.
const SHEFA_SUBGRAPH_SDL: &str = r#"
extend type Contributor {
  household: Household
  reciprocityInbound: [ReciprocityFlow!]!
}

type Household {
  cid: ID!
  members: [Contributor!]!
  devices: [Device!]!
}

type Device {
  id: String!
  metrics: String
}

type ReciprocityFlow {
  from: String!
  amount: Float!
}
"#;
