//! Ingestion pipeline — orchestrates extract → chunk → embed → store → wire.

pub mod chunker;
pub mod edge_wirer;
pub mod embedder;
pub mod job_store;
pub mod node_builder;

use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use fluvio_types::Domain;

use crate::client::GraphClient;
use crate::extractor::{pdf::PdfExtractor, text::TextExtractor};
use crate::pipeline::{
    chunker::chunk_text,
    edge_wirer::{wire_edges, SEMANTIC_EDGE_THRESHOLD},
    embedder::Embedder,
    job_store::{JobStatus, JobStore},
    node_builder::{build_node, pdf_chunk_uri, text_chunk_uri},
};

// ── IngestResult ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct IngestResult {
    pub job_id:      String,
    pub node_ids:    Vec<String>,
    pub chunk_count: usize,
}

// ── IngestionPipeline ─────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct IngestionPipeline {
    pub embedder:     Arc<RwLock<Embedder>>,
    pub graph_client: Arc<GraphClient>,
    pub job_store:    JobStore,
}

impl IngestionPipeline {
    pub fn new(embedder: Embedder, graph_client: GraphClient) -> Self {
        Self {
            embedder:     Arc::new(RwLock::new(embedder)),
            graph_client: Arc::new(graph_client),
            job_store:    JobStore::new(),
        }
    }

    // ── PDF ingestion ─────────────────────────────────────────────────────────

    /// Ingest a PDF file asynchronously.
    /// Returns immediately with a job_id — processing runs in background.
    pub async fn ingest_pdf_async(
        &self,
        owner_id: Uuid,
        bytes:    Vec<u8>,
        filename: String,
        zone:     i16,
    ) -> String {
        let job = job_store::IngestJob::new(owner_id, &filename);
        let job_id = job.id.clone();
        self.job_store.insert(job);

        // Clone what we need for the background task
        let pipeline    = self.clone();
        let job_id_bg   = job_id.clone();

        tokio::spawn(async move {
            pipeline.job_store.update_status(&job_id_bg, JobStatus::Running);

            match pipeline.run_pdf_pipeline(owner_id, bytes, &filename, zone).await {
                Ok(result) => {
                    pipeline.job_store.complete(
                        &job_id_bg,
                        result.node_ids,
                        result.chunk_count,
                    );
                    tracing::info!(job_id = %job_id_bg, "PDF ingestion complete");
                }
                Err(e) => {
                    pipeline.job_store.fail(&job_id_bg, e.to_string());
                    tracing::error!(job_id = %job_id_bg, error = %e, "PDF ingestion failed");
                }
            }
        });

        job_id
    }

    /// Run the full PDF pipeline synchronously (called from background task).
    async fn run_pdf_pipeline(
        &self,
        owner_id: Uuid,
        bytes:    Vec<u8>,
        filename: &str,
        zone:     i16,
    ) -> anyhow::Result<IngestResult> {
        // 1. Extract text from PDF
        let text = PdfExtractor::extract(&bytes)?;

        if text.trim().is_empty() {
            anyhow::bail!("PDF produced no extractable text");
        }

        tracing::info!(filename, chars = text.len(), "PDF text extracted");

        // 2. Chunk
        let chunks = chunk_text(&text);
        tracing::info!(chunk_count = chunks.len(), "PDF chunked");

        // 3. Embed all chunks in one batch
        let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        let embeddings = {
            let mut emb = self.embedder.write().await;
            emb.embed_batch(&texts)?
        };

        // 4. Build nodes and write to fluvio-graph
        let mut node_ids        = Vec::with_capacity(chunks.len());
        let mut nodes_with_embs = Vec::with_capacity(chunks.len());

        for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
            let source_uri = pdf_chunk_uri(owner_id, filename, chunk.chunk_index);
            let node = build_node(
                owner_id,
                chunk,
                &source_uri,
                Domain::Pdf,
                embedding.clone(),
                Some(std::collections::HashMap::from([
                    ("filename".into(), filename.to_string()),
                ])),
            );

            let node_id = self.graph_client
                .upsert_node(owner_id, &node, zone)
                .await?;

            nodes_with_embs.push((node.id, embedding.clone()));
            node_ids.push(node_id);
        }

        tracing::info!(node_count = node_ids.len(), "nodes written to fluvio-graph");

        // 5. Wire semantic edges between chunks
        let edges = wire_edges(
            owner_id,
            &nodes_with_embs,
            &self.graph_client,
            SEMANTIC_EDGE_THRESHOLD,
        ).await?;

        tracing::info!(edges_wired = edges, "semantic edges wired");

        Ok(IngestResult {
            job_id:      String::new(), // filled by caller
            node_ids,
            chunk_count: chunks.len(),
        })
    }

    // ── Raw text ingestion ────────────────────────────────────────────────────

    /// Ingest plain text synchronously — text is small enough to not need async.
    pub async fn ingest_text(
        &self,
        owner_id:   Uuid,
        text:       String,
        source_uri: String,
        domain:     Domain,
        zone:       i16,
    ) -> anyhow::Result<IngestResult> {
        let chunks = chunk_text(&text);

        let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        let embeddings = {
            let mut emb = self.embedder.write().await;
            emb.embed_batch(&texts)?
        };

        let mut node_ids        = Vec::with_capacity(chunks.len());
        let mut nodes_with_embs = Vec::with_capacity(chunks.len());

        for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
            let uri  = text_chunk_uri(owner_id, &source_uri, chunk.chunk_index);
            let node = build_node(
                owner_id,
                chunk,
                &uri,
                domain.clone(),
                embedding.clone(),
                None,
            );

            let node_id = self.graph_client
                .upsert_node(owner_id, &node, zone)
                .await?;

            nodes_with_embs.push((node.id, embedding.clone()));
            node_ids.push(node_id);
        }

        wire_edges(
            owner_id,
            &nodes_with_embs,
            &self.graph_client,
            SEMANTIC_EDGE_THRESHOLD,
        ).await?;

        Ok(IngestResult {
            job_id:      String::new(),
            node_ids,
            chunk_count: chunks.len(),
        })
    }
}