/**
 * Adapters - Wire format normalization for multi-transport content pipeline.
 *
 * Content arrives from three transports with different wire formats:
 * - Storage HTTP: camelCase ContentView (already normalized by Rust serde)
 * - Conductor zome: snake_case with metadata_json string (needs normalization)
 * - IndexedDB: pre-transformed ContentNode (already in app format)
 *
 * The conductor normalizer ensures all transports produce the same
 * ContentView shape before entering the typed pipeline.
 */
export {
  normalizeConductorContent,
  normalizeConductorContentBatch,
  isConductorResponse,
  ensureContentView,
  type ConductorContentResponse,
} from './conductor-normalizer';
