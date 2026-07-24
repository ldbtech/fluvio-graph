//! # fluvio-graph
//!
//! An embeddable knowledge graph engine. Ingest documents, retrieve grounded
//! context for a question, and traverse the resulting graph — all in-process,
//! with no server running.
//!
//! This crate is a **facade**: it re-exports a curated, stable surface from the
//! internal `fluvio-*-core` crates. Depend on this one. The internals behind it
//! are free to change without breaking you; see `CHANGELOG.md` for the rules.
//!
//! ## Quick start
//!
//! ```no_run
//! use fluvio_graph::prelude::*;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to storage. Config is always injected — the library never reads
//! // the environment.
//! let store = SurrealStorage::connect(&SurrealConfig {
//!     url: "ws://127.0.0.1:8000".into(),
//!     ..SurrealConfig::default()
//! })
//! .await?;
//! store.init_schema().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## What's where
//!
//! - [`storage`] — SurrealDB-backed persistence ([`SurrealStorage`], [`SurrealConfig`])
//! - [`query`] — grounded retrieval ([`QueryContext`], [`QueryConfig`])
//! - [`embeddings`] — the embedding model context
//! - [`graph`] — the in-memory graph and its registry
//! - [`types`] — the domain vocabulary ([`Node`], [`Edge`], [`Domain`], …)
//! - [`ingestion`] — the document intake pipeline (feature `ingestion`, on by default)

#![forbid(unsafe_code)]

// ── Storage ───────────────────────────────────────────────────────────────────

/// SurrealDB-backed persistence for nodes, edges, and vectors.
pub mod storage {
    pub use fluvio_graph_core::storage::surreal::{SurrealConfig, SurrealStorage};
}

// ── Retrieval ─────────────────────────────────────────────────────────────────

/// Grounded retrieval: turn a question into a relevant subgraph.
pub mod query {
    pub use fluvio_graph_core::query_context::{QueryConfig, QueryContext, QueryRoute};
}

// ── Embeddings ────────────────────────────────────────────────────────────────

/// The embedding model context used for vector search.
pub mod embeddings {
    pub use fluvio_graph_core::embeddings::EmbeddingContext;
}

// ── Graph ─────────────────────────────────────────────────────────────────────

/// The in-memory graph, its registry, and the trait they implement.
pub mod graph {
    pub use fluvio_graph_core::graph::FluvioGraph;
    pub use fluvio_graph_core::registry::GraphRegistry;
    pub use fluvio_types::DomainGraph;
}

// ── Domain vocabulary ─────────────────────────────────────────────────────────

/// The shared domain types: nodes, edges, identifiers, and their enums.
pub mod types {
    pub use fluvio_types::{
        Domain, Edge, EdgeId, ExternalRef, GraphError, GraphEvent, GraphId, GraphQuery,
        GraphResult, Node, NodeId, NodeKind, NodeKindFilter, NodePredicate,
    };
}

// ── Ingestion ─────────────────────────────────────────────────────────────────

/// The document intake pipeline: extract, chunk, embed, and write to the graph.
#[cfg(feature = "ingestion")]
pub mod ingestion {
    pub use fluvio_ingestion_core::pipeline::chunker::{
        Chunk, chunk_text, chunk_text_with_config,
    };
    pub use fluvio_ingestion_core::pipeline::{IngestResult, IngestionPipeline};
}

// ── Prelude ───────────────────────────────────────────────────────────────────

/// The things most consumers want, in one import.
///
/// ```
/// use fluvio_graph::prelude::*;
/// ```
pub mod prelude {
    pub use crate::embeddings::EmbeddingContext;
    pub use crate::graph::{DomainGraph, FluvioGraph, GraphRegistry};
    pub use crate::query::{QueryConfig, QueryContext};
    pub use crate::storage::{SurrealConfig, SurrealStorage};
    pub use crate::types::{Domain, Edge, Node, NodeId, NodeKind};

    #[cfg(feature = "ingestion")]
    pub use crate::ingestion::IngestionPipeline;
}

// ── Escape hatch ──────────────────────────────────────────────────────────────

/// Direct access to the underlying crates, for consumers who need something the
/// curated surface above does not expose.
///
/// Nothing here is covered by the facade's stability promise — if you find
/// yourself reaching in, that is a signal the facade is missing something.
#[doc(hidden)]
pub mod internal {
    pub use fluvio_graph_core;
    #[cfg(feature = "ingestion")]
    pub use fluvio_ingestion_core;
    pub use fluvio_types;
}
