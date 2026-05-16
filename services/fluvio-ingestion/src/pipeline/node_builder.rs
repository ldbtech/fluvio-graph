//! Node builder — assembles fluvio_types::Node structs from ingestion chunks.

use std::collections::HashMap;
use uuid::Uuid;

use fluvio_types::{Node, NodeId, NodeKind, Domain};
use crate::pipeline::chunker::Chunk;

/// Build a graph node from a text chunk.
///
/// `source_uri` is the canonical locator for this chunk:
///   - PDF:      `pdf://{owner_id}/{filename}/chunk/{index}`
///   - Text:     `text://{owner_id}/{source_uri}/chunk/{index}`
///   - Codebase: `git://{repo_url}/file/{path}#L{line}`
pub fn build_node(
    owner_id:   Uuid,
    chunk:      &Chunk,
    source_uri: &str,
    domain:     Domain,
    embeddings: Vec<f32>,
    extra_meta: Option<HashMap<String, String>>,
) -> Node {
    let mut metadata = extra_meta.unwrap_or_default();
    metadata.insert("chunk_index".into(), chunk.chunk_index.to_string());
    metadata.insert("token_count".into(), chunk.token_count.to_string());
    metadata.insert("owner_id".into(), owner_id.to_string());

    let node_id = NodeId::from_content("chunk", source_uri);

    Node {
        id:          node_id,
        domain,
        source_uri:  source_uri.to_string(),
        source_text: chunk.text.clone(),
        embeddings,
        metadata,
        kind: NodeKind::Artifcat,
    }
}

/// Build the canonical source URI for a PDF chunk.
pub fn pdf_chunk_uri(owner_id: Uuid, filename: &str, chunk_index: usize) -> String {
    format!("pdf://{owner_id}/{filename}/chunk/{chunk_index}")
}

/// Build the canonical source URI for a plain text chunk.
pub fn text_chunk_uri(owner_id: Uuid, source_hint: &str, chunk_index: usize) -> String {
    format!("text://{owner_id}/{source_hint}/chunk/{chunk_index}")
}

/// Build the canonical source URI for a codebase file.
pub fn code_chunk_uri(owner_id: Uuid, repo: &str, file_path: &str, chunk_index: usize) -> String {
    format!("git://{owner_id}/{repo}/{file_path}/chunk/{chunk_index}")
}