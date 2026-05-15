//! storage/cache.rs
//!
//! ## What belongs here (future)
//!
//! This module will hold a bounded LRU cache for hot nodes —
//! nodes that are accessed frequently across many requests.
//!
//! The cache sits between the GraphQL resolver and SurrealDB:
//!
//! ```text
//! Resolver → check cache → hit  → return node (μs)
//!                        → miss → SurrealDB → insert into cache → return
//! ```
//!
//! ## Design constraints
//! - Max size: configurable, default 10K nodes (~20MB with embeddings)
//! - Eviction: LRU (least recently used)
//! - Invalidation: when a node is upserted in SurrealDB, evict from cache
//! - Scope: process-local only — not distributed (Redis is for token cache
//!   in fluvio-connectors, not graph nodes)
//!
//! ## Why not implement it now
//! Premature optimisation. SurrealDB with proper indexes is fast enough
//! for the current scale. Add the cache when profiling shows DB latency
//! is the bottleneck — not before.
//!
//! ## Crate to use when ready
//! `lru` crate: https://crates.io/crates/lru
//! ```toml
//! lru = "0.12"
//! ```

// Placeholder — no implementation yet.
// The module is declared in lib.rs so it compiles as part of the crate.