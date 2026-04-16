use crate::graph::enums::Domain;
use crate::graph::structs::NodeId;
use std::collections::HashMap;

struct Document {
    id: NodeId, // content-addressed id
    domain: Domain, // pdf, email, whatsapp, music, codebase, etc.
    source_uri: String, // file path, message id, spotify uri, etc.
    content: String, // extracted text
    metadata: HashMap<String, String>, // page, author, timestamp, language..etc.
    chunk_size: usize,
    related_documents: Vec<DocumentRelationship>, // explicit edges to create
}

struct DocumentRelationship {
    to_uri: String,
    label: String,
    probability: f64,
}