//! Embedding wrapper for fluvio-ingestion.
//!
//! Wraps the BGE-small model from fastembed.
//! Shared across all pipeline operations via Arc<RwLock<Embedder>>.

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use anyhow::Context;

pub struct Embedder {
    model: TextEmbedding,
}

impl Embedder {
    /// Load BGE-small. Called once on service boot — takes ~2-3 seconds.
    pub fn new() -> anyhow::Result<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGESmallENV15)
                .with_show_download_progress(true),
        ).context("failed to load BGE-small embedding model")?;

        Ok(Self { model })
    }

    /// Embed a single text string → 384-dimensional vector.
    pub fn embed(&mut self, text: &str) -> anyhow::Result<Vec<f32>> {
        let mut results = self.model
            .embed(vec![text.to_string()], None)
            .context("embedding failed")?;

        results.pop().context("no embedding vector returned")
    }

    /// Embed multiple texts in one batch — more efficient than calling embed() in a loop.
    pub fn embed_batch(&mut self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }
        self.model
            .embed(texts.to_vec(), None)
            .context("batch embedding failed")
    }
}