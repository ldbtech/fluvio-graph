use std::collections::HashMap;
use crate::graph::{Graph, NodeId, TextChunk};

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