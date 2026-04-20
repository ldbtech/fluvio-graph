use std::collections::HashMap;

use crate::graph::{Graph, GraphError, NodeId, TextChunk};
use crate::ingestion_registry::connector::NormalizedChunk;

pub struct IngestionPipeline {
    pub graph: Graph,
}

impl IngestionPipeline {
    pub fn new (graph: Graph) -> Self {
        Self { graph }
    }

    /// Each chunk becomes one node - Embedding captures its meaning 
    pub fn ingest_chunk(
        &mut self,
        text: &str,
        source_hint: &str,
        page: usize,
    ) -> anyhow::Result<NodeId>{
        let doc = Box::new(TextChunk {
            text: text.to_string(),
            metadata: HashMap::from([
                ("source".to_string(), source_hint.to_string()),
                ("page".to_string(),   page.to_string()),
            ])
        });

        let id = self.graph.add_node(doc)?;
        Ok(id)
    }

    /// After we ingest all the chunks, wire edges by cosine similarity
    /// I know this is not optimal algorithm. But later we will try to connect
    /// Edges while creating nodes using threading or similar solution.
    /// Ingest connector [`NormalizedChunk`]s (e.g. Gmail): embed text, merge metadata, add structured edges.
    pub fn ingest_normalized_chunks(
        &mut self,
        chunks: &[NormalizedChunk],
    ) -> Result<(usize, usize), GraphError> {
        let mut uri_to_id: HashMap<String, NodeId> = HashMap::new();
        let mut nodes_added = 0usize;
        let mut edges_added = 0usize;

        for chunk in chunks {
            if chunk.text.trim().is_empty() {
                continue;
            }
            let mut meta = chunk.metadata.clone();
            meta
                .entry("source".to_string())
                .or_insert_with(|| "email".to_string());
            meta
                .entry("page".to_string())
                .or_insert_with(|| chunk.chunk_index.to_string());
            meta.insert("source_uri".to_string(), chunk.source_uri.clone());

            let doc = Box::new(TextChunk {
                text: chunk.text.clone(),
                metadata: meta,
            });
            let id = self.graph.add_node(doc)?;
            uri_to_id.insert(chunk.source_uri.clone(), id);
            nodes_added += 1;
        }

        for chunk in chunks {
            let Some(&from_id) = uri_to_id.get(&chunk.source_uri) else {
                continue;
            };
            for pe in &chunk.pre_defined_edges {
                if let Some(&to_id) = uri_to_id.get(&pe.to_uri) {
                    self.graph.add_edge(
                        from_id,
                        to_id,
                        pe.token_cost,
                        pe.relationship_probability,
                    )?;
                    edges_added += 1;
                }
            }
        }

        Ok((nodes_added, edges_added))
    }

    pub fn wire_edges(&mut self, threshold: f32) {
        let ids: Vec<NodeId> = self.graph.nodes.keys().copied().collect();

        for &id in &ids {
            let query = match self.graph.nodes.get(&id) {
                Some(n) => n.embeddings.clone(),
                None => continue,
            };

            let neighbors = self.graph.similarity_search(&query, 6);

            for (neighbor_id, sim) in neighbors {
                if neighbor_id == id || sim < threshold {
                    continue;
                }

                let already = self.graph.adj_list
                    .get(&id)
                    .map(|edges| edges.iter().any(|e| e.to == neighbor_id))
                    .unwrap_or(false);

                if already { continue; }

                let token_cost = ((1.0 - sim) * 10_000.0) as i32;
                let _ = self.graph.add_edge(id, neighbor_id, token_cost, sim as f64);
            }
        }
    }
}