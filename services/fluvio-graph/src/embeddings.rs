use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

use fluvio_types::GraphError;

pub struct EmbeddingContext {
    model: TextEmbedding,
}

impl EmbeddingContext {
    pub fn new() -> anyhow::Result<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGESmallENV15)
                .with_show_download_progress(true),
        )?;
        Ok(Self { model })
    }

    pub fn embed(&mut self, text: &str) -> Result<Vec<f32>, GraphError> {
        let mut embeddings = self
            .model
            .embed(vec![text.to_string()], None)
            .map_err(|e| GraphError::EmbeddingFailed(e.to_string()))?;

        embeddings
            .pop()
            .ok_or_else(|| GraphError::EmbeddingFailed("no vectors generated".to_string()))
    }
}
