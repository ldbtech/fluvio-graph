use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::graph::enums::{Domain, GraphError as FluvioGraphError, NodeKind};
use crate::graph::fluvio_graph::FluvioGraph;
use crate::graph::structs::{DomainGraph, Edge, EdgeId, Node, NodeId};
use crate::graph::EmbeddingContext;
use crate::ingestion_registry::connector::NormalizedChunk;

pub struct IngestionPipeline {
    pub graph: DomainGraph,
    pub embed_ctx: Arc<Mutex<EmbeddingContext>>,
}

impl IngestionPipeline {
    pub fn new(graph: DomainGraph, embed_ctx: Arc<Mutex<EmbeddingContext>>) -> Self {
        Self { graph, embed_ctx }
    }

    fn embed_text(&self, text: &str) -> Result<Vec<f32>, FluvioGraphError> {
        self.embed_ctx
            .lock()
            .unwrap()
            .embed(text)
            .map_err(|e: crate::graph::GraphError| {
                FluvioGraphError::EmbeddingFailed(e.to_string())
            })
    }

    /// Each chunk becomes one node — embedding captures chunk text.
    ///
    /// For PDFs, pass `pdf_document: Some((document_id, filename))` so `source_uri` and metadata
    /// distinguish uploads (`document_id`, `filename` on the node). Otherwise `source_uri` is
    /// `pdf://page/{page}` (legacy, single-doc assumptions).
    pub fn ingest_chunk(
        &mut self,
        text: &str,
        source_hint: &str,
        page: usize,
        pdf_document: Option<(&str, &str)>,
    ) -> anyhow::Result<NodeId> {
        let source_uri = if let Some((doc_id, _)) = pdf_document {
            format!("pdf://{doc_id}/page/{page}")
        } else {
            format!("pdf://page/{page}")
        };
        let mut metadata = HashMap::from([
            ("source".to_string(), source_hint.to_string()),
            ("page".to_string(), page.to_string()),
            ("source_uri".to_string(), source_uri.clone()),
        ]);
        if let Some((doc_id, filename)) = pdf_document {
            metadata.insert("document_id".to_string(), doc_id.to_string());
            metadata.insert("filename".to_string(), filename.to_string());
        }
        let embeddings = self.embed_text(text)?;
        let id = NodeId::from_content("pdf_chunk", &source_uri);
        let node = Node {
            id,
            domain: Domain::Pdf,
            source_uri,
            source_text: text.to_string(),
            embeddings,
            metadata,
            kind: NodeKind::Artifcat,
        };
        self.graph
            .insert_node(node)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Ok(id)
    }

    fn node_from_normalized_chunk(
        &self,
        chunk: &NormalizedChunk,
    ) -> Result<Node, FluvioGraphError> {
        if chunk.text.trim().is_empty() {
            return Err(FluvioGraphError::SerializationError(
                "empty chunk text".to_string(),
            ));
        }
        let mut meta = chunk.metadata.clone();
        let default_source = if matches!(&chunk.domain, Domain::Custom(s) if s == "tools") {
            "tools"
        } else {
            "email"
        };
        meta
            .entry("source".to_string())
            .or_insert_with(|| default_source.to_string());
        meta
            .entry("page".to_string())
            .or_insert_with(|| chunk.chunk_index.to_string());
        meta.insert("source_uri".to_string(), chunk.source_uri.clone());

        let embeddings = self.embed_text(&chunk.text)?;
        let id = NodeId::from_content("normalized_chunk", &chunk.source_uri);

        Ok(Node {
            id,
            domain: chunk.domain.clone(),
            source_uri: chunk.source_uri.clone(),
            source_text: chunk.text.clone(),
            embeddings,
            metadata: meta,
            kind: NodeKind::Artifcat,
        })
    }

    /// Ingest connector [`NormalizedChunk`]s: embed text, merge metadata, add structured edges with labels.
    pub fn ingest_normalized_chunks(
        &mut self,
        chunks: &[NormalizedChunk],
    ) -> Result<(usize, usize), FluvioGraphError> {
        let mut uri_to_id: HashMap<String, NodeId> = HashMap::new();
        let mut nodes_added = 0usize;
        let mut edges_added = 0usize;

        for chunk in chunks {
            if chunk.text.trim().is_empty() {
                continue;
            }
            let node = self.node_from_normalized_chunk(chunk)?;
            let id = node.id;
            self.graph.insert_node(node)?;
            uri_to_id.insert(chunk.source_uri.clone(), id);
            nodes_added += 1;
        }

        for chunk in chunks {
            let Some(&from_id) = uri_to_id.get(&chunk.source_uri) else {
                continue;
            };
            for pe in &chunk.pre_defined_edges {
                if let Some(&to_id) = uri_to_id.get(&pe.to_uri) {
                    let edge = Edge {
                        id: EdgeId::new(),
                        from: from_id,
                        to: to_id,
                        token: pe.token_cost,
                        relationship_probability: pe.relationship_probability,
                        label: pe.label.clone(),
                        metadata: HashMap::new(),
                    };
                    self.graph.insert_edge(edge)?;
                    edges_added += 1;
                }
            }
        }

        Ok((nodes_added, edges_added))
    }

    /// Skips chunks whose `source_uri` already exists; edges can still attach to existing nodes.
    pub fn ingest_normalized_chunks_merge_uris(
        &mut self,
        chunks: &[NormalizedChunk],
    ) -> Result<(usize, usize, usize), FluvioGraphError> {
        let mut uri_to_id: HashMap<String, NodeId> = HashMap::new();
        for n in self.graph.nodes.values() {
            if !n.source_uri.is_empty() {
                uri_to_id.insert(n.source_uri.clone(), n.id);
            }
            if let Some(uri) = n.metadata.get("source_uri").filter(|u| !u.is_empty()) {
                uri_to_id.insert(uri.clone(), n.id);
            }
        }

        let mut skipped = 0usize;
        let mut nodes_added = 0usize;

        for chunk in chunks {
            if chunk.text.trim().is_empty() {
                continue;
            }
            if uri_to_id.contains_key(&chunk.source_uri) {
                skipped += 1;
                continue;
            }
            let node = self.node_from_normalized_chunk(chunk)?;
            let id = node.id;
            self.graph.insert_node(node)?;
            uri_to_id.insert(chunk.source_uri.clone(), id);
            nodes_added += 1;
        }

        let mut edges_added = 0usize;
        for chunk in chunks {
            let Some(&from_id) = uri_to_id.get(&chunk.source_uri) else {
                continue;
            };
            for pe in &chunk.pre_defined_edges {
                if let Some(&to_id) = uri_to_id.get(&pe.to_uri) {
                    let dup = self
                        .graph
                        .adj
                        .get(&from_id)
                        .map(|edges| edges.iter().any(|e| e.to == to_id && e.label == pe.label))
                        .unwrap_or(false);
                    if dup {
                        continue;
                    }
                    let edge = Edge {
                        id: EdgeId::new(),
                        from: from_id,
                        to: to_id,
                        token: pe.token_cost,
                        relationship_probability: pe.relationship_probability,
                        label: pe.label.clone(),
                        metadata: HashMap::new(),
                    };
                    self.graph.insert_edge(edge)?;
                    edges_added += 1;
                }
            }
        }

        Ok((nodes_added, edges_added, skipped))
    }

    pub fn wire_edges(&mut self, threshold: f32) {
        let ids: Vec<NodeId> = self.graph.nodes.keys().copied().collect();
        self.wire_edges_for_nodes(&ids, threshold);
    }

    /// Same as [`Self::wire_edges`], but only runs from `ids` outward. Use after bulk PDF ingest so
    /// a 100-page upload does not re-score every existing workspace node against every other node
    /// (that was O(|V|²) and blew memory / runtime on large graphs).
    pub fn wire_edges_for_nodes(&mut self, ids: &[NodeId], threshold: f32) {
        for &id in ids {
            let query = match self.graph.nodes.get(&id) {
                Some(n) => n.embeddings.clone(),
                None => continue,
            };

            let neighbors = self.graph.similarity_search(&query, 6);

            for (neighbor_id, sim) in neighbors {
                if neighbor_id == id || sim < threshold {
                    continue;
                }

                let already = self
                    .graph
                    .adj
                    .get(&id)
                    .map(|edges| {
                        edges.iter().any(|e| {
                            e.to == neighbor_id && e.label == "semantic_neighbor"
                        })
                    })
                    .unwrap_or(false);

                if already {
                    continue;
                }

                let token_cost = ((1.0 - sim) * 10_000.0) as i32;
                let edge = Edge {
                    id: EdgeId::new(),
                    from: id,
                    to: neighbor_id,
                    token: token_cost,
                    relationship_probability: sim as f64,
                    label: "semantic_neighbor".to_string(),
                    metadata: HashMap::new(),
                };
                let _ = self.graph.insert_edge(edge);
            }
        }
    }
}
